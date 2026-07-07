//! Server verification outcome implementation.
//!
//! Converts request credentials into the canonical `VerificationOutcome` used
//! by the tower layer, server-function context reader, and SSR initial state path.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::core::{ClerkAuth, ClerkError, InvalidTokenReason, VerificationOutcome};
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use jwk_simple::KeySet;
use web_time::SystemTime;

use self::credentials::RequestCredentials;
use self::jwks::{JwksCache, JwksCacheLookup, JwksCachePolicy};
use self::jwt::JwtVerificationProfile;
use super::claims::VerifiedClerkClaims;
use super::config::ClerkAuthLayerConfig;

mod credentials;
mod jwks;
mod jwt;

const JWKS_FETCH_TIMEOUT: Duration = Duration::from_secs(30);
/// Upper bound on a JWKS response body. Real Clerk keysets are a few KiB;
/// anything larger indicates a misconfigured endpoint and is rejected before
/// parsing (via Content-Length up front when present, and via the buffered
/// length otherwise — buffering stays bounded by the fetch timeout).
const JWKS_MAX_RESPONSE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VerificationFailure {
    Invalid(InvalidTokenReason),
    Unavailable,
}

impl VerificationFailure {
    fn invalid() -> Self {
        Self::Invalid(InvalidTokenReason::Other)
    }
}

impl From<VerificationFailure> for VerificationOutcome {
    fn from(value: VerificationFailure) -> Self {
        match value {
            VerificationFailure::Invalid(reason) => Self::Invalid(reason),
            VerificationFailure::Unavailable => Self::Unavailable,
        }
    }
}

/// Server-side JWT verifier with in-memory JWKS cache.
#[derive(Clone)]
pub(crate) struct Verifier {
    inner: Arc<VerifierInner>,
}

struct VerifierInner {
    secret_key: String,
    jwks_url: reqwest::Url,
    client: reqwest::Client,
    profile: JwtVerificationProfile,
    cache: Mutex<JwksCache>,
    /// Cache TTL / backoff / max-stale knobs, fixed at construction so a
    /// future config option extends [`Verifier::new`] instead of every
    /// lookup call site.
    cache_policy: JwksCachePolicy,
    /// Serializes JWKS refreshes so a burst of requests on a cold or expired
    /// cache produces one upstream fetch instead of a thundering herd.
    refresh_lock: futures_util::lock::Mutex<()>,
}

impl Verifier {
    pub(crate) fn new(config: ClerkAuthLayerConfig) -> Result<Self, ClerkError> {
        if config.secret_key.is_empty() {
            return Err(ClerkError::InvalidConfig(
                "Clerk secret key must not be empty".into(),
            ));
        }
        let jwks_url = config.jwks_url()?;
        let builder = reqwest::Client::builder()
            .user_agent(concat!("dioxus-clerk/", env!("CARGO_PKG_VERSION")));
        // The signing keys are the entire trust root: never follow a redirect
        // (compromised CDN, open redirect, or an `https`→`http` downgrade) that
        // would source them from another origin. HTTPS is already enforced on
        // the configured base URL, but a redirect target is not re-checked.
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder
            .timeout(JWKS_FETCH_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none());
        let client = builder.build().map_err(|error| {
            ClerkError::InvalidConfig(format!("failed to build Clerk JWKS HTTP client: {error}"))
        })?;

        Ok(Self {
            inner: Arc::new(VerifierInner {
                secret_key: config.secret_key,
                jwks_url,
                client,
                profile: JwtVerificationProfile::new(
                    config.authorized_parties,
                    config.audiences,
                    config.issuers,
                    config.clock_skew,
                    config.require_session_id,
                ),
                cache: Mutex::new(JwksCache::default()),
                cache_policy: JwksCachePolicy::default(),
                refresh_lock: futures_util::lock::Mutex::new(()),
            }),
        })
    }

    /// Locks the cache, recovering from poisoning: the guarded state is a
    /// keyset plus timestamps and cannot be left logically corrupt by a panic
    /// mid-write, while treating poisoning as fatal would turn one panic into
    /// permanent 503s for every refresh-path request.
    fn lock_cache(&self) -> std::sync::MutexGuard<'_, JwksCache> {
        self.inner
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn cache_lookup(&self, kid: &str) -> JwksCacheLookup {
        self.lock_cache()
            .lookup(kid, self.inner.cache_policy, SystemTime::now())
    }

    async fn keyset_for_kid(&self, kid: &str) -> Result<Arc<KeySet>, VerificationFailure> {
        match self.cache_lookup(kid) {
            JwksCacheLookup::Hit(keyset) => Ok(keyset),
            JwksCacheLookup::Refresh => self.refresh_keyset_for_kid(kid).await,
            JwksCacheLookup::RejectUnknownKid => Err(VerificationFailure::invalid()),
            JwksCacheLookup::Unavailable => Err(VerificationFailure::Unavailable),
        }
    }

    async fn refresh_keyset_for_kid(&self, kid: &str) -> Result<Arc<KeySet>, VerificationFailure> {
        let _refresh_guard = self.inner.refresh_lock.lock().await;

        // Re-check after acquiring the lock: a concurrent request may have
        // refreshed the cache — or recorded a fetch failure — while this one
        // waited. The failure-backoff re-check is what keeps a burst of
        // requests during an outage from serializing into one slow upstream
        // fetch each.
        match self.cache_lookup(kid) {
            JwksCacheLookup::Hit(keyset) => return Ok(keyset),
            JwksCacheLookup::RejectUnknownKid => return Err(VerificationFailure::invalid()),
            JwksCacheLookup::Unavailable => return Err(VerificationFailure::Unavailable),
            JwksCacheLookup::Refresh => {}
        }

        let keyset = match self.fetch_keyset().await {
            Ok(keyset) => Arc::new(keyset),
            Err(failure) => {
                // Stale-while-error: a failed refresh serves the stored
                // keyset for a known kid instead of failing closed, bounded
                // by the policy's max-stale window.
                let mut cache = self.lock_cache();
                cache.record_failure();
                if let Some(stale) =
                    cache.stale_keyset_for_kid(kid, self.inner.cache_policy, SystemTime::now())
                {
                    tracing::warn!(
                        "Clerk JWKS refresh failed; serving stale cached keyset until the next successful refresh"
                    );
                    return Ok(stale);
                }
                return Err(failure);
            }
        };
        let contains_kid = keyset.get_by_kid(kid).is_some();
        self.lock_cache().store(Arc::clone(&keyset));

        if contains_kid {
            Ok(keyset)
        } else {
            Err(VerificationFailure::invalid())
        }
    }

    async fn fetch_keyset(&self) -> Result<KeySet, VerificationFailure> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // The reqwest client timeout bounds the whole fetch on native.
            self.fetch_and_parse_keyset().await
        }
        #[cfg(target_arch = "wasm32")]
        {
            // reqwest has no client timeout on wasm; race the fetch against a
            // JS timer so a hung upstream cannot hold the refresh lock forever.
            use futures_util::future::{Either, select};
            let fetch = std::pin::pin!(self.fetch_and_parse_keyset());
            let timeout = std::pin::pin!(gloo_timers::future::TimeoutFuture::new(
                JWKS_FETCH_TIMEOUT.as_millis() as u32
            ));
            match select(fetch, timeout).await {
                Either::Left((result, _)) => result,
                Either::Right(((), _)) => {
                    tracing::warn!("Clerk JWKS fetch timed out");
                    Err(VerificationFailure::Unavailable)
                }
            }
        }
    }

    async fn fetch_and_parse_keyset(&self) -> Result<KeySet, VerificationFailure> {
        let response = self
            .inner
            .client
            .get(self.inner.jwks_url.clone())
            .bearer_auth(&self.inner.secret_key)
            .send()
            .await
            .map_err(|error| {
                tracing::warn!(error = ?error, "failed to fetch Clerk JWKS");
                VerificationFailure::Unavailable
            })?;

        // The signing keys are the entire trust root: reject a response sourced
        // from another origin. On native, redirects are already refused
        // (`Policy::none()`), so this only re-confirms the origin. On the
        // `worker` (wasm) target, reqwest maps to the browser/Workers `fetch`,
        // which follows 3xx with no redirect-policy control — this post-fetch
        // check is the only place a cross-origin redirect can be caught there.
        if response.url().origin() != self.inner.jwks_url.origin() {
            tracing::warn!(
                "Clerk JWKS response came from an unexpected origin (redirect?); refusing to source signing keys from it"
            );
            return Err(VerificationFailure::Unavailable);
        }

        let status = response.status();
        // A 401/403 from `/jwks` almost always means a wrong or revoked
        // CLERK_SECRET_KEY, not a Clerk outage. It still fails closed
        // (`Unavailable`), but say so distinctly so an operator can tell a
        // misconfiguration from an upstream incident.
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            tracing::warn!(
                status = %status,
                "Clerk JWKS endpoint rejected the request; check that CLERK_SECRET_KEY is set correctly and not revoked"
            );
            return Err(VerificationFailure::Unavailable);
        }
        if !status.is_success() {
            tracing::warn!(status = %status, "Clerk JWKS endpoint returned an error status");
            return Err(VerificationFailure::Unavailable);
        }

        if response
            .content_length()
            .is_some_and(|length| length > JWKS_MAX_RESPONSE_BYTES)
        {
            tracing::warn!("Clerk JWKS response exceeds the size limit");
            return Err(VerificationFailure::Unavailable);
        }

        let bytes = response.bytes().await.map_err(|error| {
            tracing::warn!(error = ?error, "failed to read Clerk JWKS response body");
            VerificationFailure::Unavailable
        })?;

        if bytes.len() as u64 > JWKS_MAX_RESPONSE_BYTES {
            tracing::warn!("Clerk JWKS response exceeds the size limit");
            return Err(VerificationFailure::Unavailable);
        }

        serde_json::from_slice::<KeySet>(&bytes).map_err(|error| {
            tracing::warn!(error = ?error, "failed to parse Clerk JWKS response");
            VerificationFailure::Unavailable
        })
    }
}

pub(crate) enum VerifiedRequest {
    Forward(Request<Body>),
    Unavailable(Response<Body>),
}

pub(crate) async fn verify_request(req: Request<Body>, verifier: Verifier) -> VerifiedRequest {
    let credentials = RequestCredentials::from_request(&req);
    let outcome = credentials.verify(verifier).await;
    verified_request_from_outcome(req, outcome)
}

fn unavailable_response() -> Response<Body> {
    let mut response = Response::new(Body::from("Clerk verification unavailable"));
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    response
}

fn verified_request_from_outcome(
    mut req: Request<Body>,
    outcome: VerificationOutcome,
) -> VerifiedRequest {
    // Unavailable is the one outcome that fails closed.
    if matches!(outcome, VerificationOutcome::Unavailable) {
        return VerifiedRequest::Unavailable(unavailable_response());
    }

    req.extensions_mut().remove::<ClerkAuth>();
    req.extensions_mut().insert(outcome);
    VerifiedRequest::Forward(req)
}

async fn verify_token(token: &str, verifier: Verifier) -> VerificationOutcome {
    match verify_token_claims(token, verifier).await {
        Ok(claims) => claims.into_outcome(),
        Err(failure) => failure.into(),
    }
}

async fn verify_token_claims(
    token: &str,
    verifier: Verifier,
) -> Result<VerifiedClerkClaims, VerificationFailure> {
    let kid = verifier.inner.profile.key_id_from_token_header(token)?;

    let keyset = verifier.keyset_for_kid(&kid).await?;
    let profile = &verifier.inner.profile;
    let key = profile.rs256_key_for_kid(&keyset, &kid)?;
    profile.verify_claims(&key, token, &kid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};

    #[test]
    fn missing_outcome_is_inserted_on_forwarded_request() {
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();

        let verified = verified_request_from_outcome(req, VerificationOutcome::Missing);

        match verified {
            VerifiedRequest::Forward(req) => assert!(matches!(
                req.extensions().get::<VerificationOutcome>(),
                Some(VerificationOutcome::Missing)
            )),
            VerifiedRequest::Unavailable(_) => panic!("missing outcome should pass through"),
        }
    }

    #[test]
    fn invalid_outcome_removes_stale_raw_auth_extension() {
        let mut req = Request::builder().uri("/").body(Body::empty()).unwrap();
        req.extensions_mut().insert(sample_auth());

        let verified = verified_request_from_outcome(
            req,
            VerificationOutcome::Invalid(InvalidTokenReason::Other),
        );

        match verified {
            VerifiedRequest::Forward(req) => {
                assert!(matches!(
                    req.extensions().get::<VerificationOutcome>(),
                    Some(VerificationOutcome::Invalid(_))
                ));
                assert!(req.extensions().get::<ClerkAuth>().is_none());
            }
            VerifiedRequest::Unavailable(_) => panic!("invalid outcome should pass through"),
        }
    }

    #[test]
    fn valid_outcome_is_inserted_without_raw_auth_extension() {
        let auth = sample_auth();
        let mut req = Request::builder().uri("/").body(Body::empty()).unwrap();
        req.extensions_mut().insert(auth.clone());

        let verified = verified_request_from_outcome(req, VerificationOutcome::Valid(auth));

        match verified {
            VerifiedRequest::Forward(req) => {
                assert!(matches!(
                    req.extensions().get::<VerificationOutcome>(),
                    Some(VerificationOutcome::Valid(auth)) if auth.user_id == "user_2abc"
                ));
                assert!(req.extensions().get::<ClerkAuth>().is_none());
            }
            VerifiedRequest::Unavailable(_) => panic!("valid outcome should pass through"),
        }
    }

    #[test]
    fn unavailable_outcome_fails_closed_without_forwarding() {
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();

        let verified = verified_request_from_outcome(req, VerificationOutcome::Unavailable);

        match verified {
            VerifiedRequest::Forward(_) => panic!("unavailable outcome should fail closed"),
            VerifiedRequest::Unavailable(response) => {
                assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            }
        }
    }

    fn sample_auth() -> ClerkAuth {
        let mut auth = ClerkAuth::new("user_2abc", 1_700_000_000);
        auth.session_id = Some("sess_2def".into());
        auth.nbf = 1_699_999_000;
        auth.iat = 1_699_999_000;
        auth
    }
}
