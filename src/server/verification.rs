//! Server verification outcome implementation.
//!
//! Converts request credentials into the canonical `VerificationOutcome` used
//! by the tower layer, server-function context reader, and SSR initial state path.

use std::sync::{Arc, Mutex};

use crate::core::{ClerkAuth, ClerkError, InvalidTokenReason, VerificationOutcome};
use axum::body::{Body, Bytes};
use axum::http::header::{AUTHORIZATION, USER_AGENT};
use axum::http::{HeaderValue, Method, Request, Response, StatusCode, Uri};
use jwk_simple::KeySet;
use jwk_simple::jwks::{HttpTransport, TransportErrorKind};
use web_time::SystemTime;

use self::credentials::RequestCredentials;
use self::jwks::{JwksCache, JwksCacheLookup, JwksCachePolicy};
use self::jwt::JwtVerificationProfile;
use super::claims::VerifiedClerkClaims;
use super::config::ClerkAuthLayerConfig;

mod credentials;
mod jwks;
mod jwt;
mod transport;

/// Upper bound on a JWKS response body. Real Clerk keysets are a few KiB;
/// anything larger indicates a misconfigured endpoint and is rejected before
/// parsing: the transport refuses it up front when `Content-Length` says so,
/// and stops reading at the limit otherwise, so buffering stays bounded.
const JWKS_MAX_RESPONSE_BYTES: usize = 1024 * 1024;

const JWKS_USER_AGENT: &str = concat!("dioxus-clerk/", env!("CARGO_PKG_VERSION"));

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

/// Server-side JWT verifier.
#[derive(Clone)]
pub(crate) struct Verifier {
    inner: Arc<VerifierInner>,
}

struct VerifierInner {
    keys: KeySource,
    profile: JwtVerificationProfile,
}

/// Where the verifier gets its signing keys.
enum KeySource {
    /// Keys fetched from Clerk's JWKS endpoint and cached in memory. Boxed
    /// because it carries an HTTP transport, the prepared request, and two
    /// locks, while the static variant is a pointer.
    Fetched(Box<FetchedKeys>),
    /// A fixed keyset from
    /// [`with_static_jwks`](ClerkAuthLayerConfig::with_static_jwks). There is
    /// nothing to fetch, cache, or refresh, so verification here can never fail
    /// as `Unavailable`.
    Static(Arc<KeySet>),
}

/// Fetched-keys state: the HTTP transport, the JWKS cache, and the refresh
/// coordination that only the fetching path needs.
struct FetchedKeys {
    jwks_uri: Uri,
    /// `Bearer <secret key>`, validated once at construction: Clerk's
    /// `/v1/jwks` requires the secret key, and a key that cannot be sent as a
    /// header is a configuration error, not an outage to retry.
    authorization: HeaderValue,
    transport: Arc<dyn HttpTransport>,
    cache: Mutex<JwksCache>,
    /// Cache TTL / backoff / max-stale knobs, fixed at construction so a
    /// future config option extends [`FetchedKeys::new`] instead of every
    /// lookup call site.
    cache_policy: JwksCachePolicy,
    /// Serializes JWKS refreshes so a burst of requests on a cold or expired
    /// cache produces one upstream fetch instead of a thundering herd.
    refresh_lock: futures_util::lock::Mutex<()>,
}

impl Verifier {
    pub(crate) fn new(config: ClerkAuthLayerConfig) -> Result<Self, ClerkError> {
        let keys = match config.static_jwks.as_deref() {
            Some(jwks_json) => KeySource::Static(Arc::new(parse_static_jwks(jwks_json)?)),
            None => KeySource::Fetched(Box::new(FetchedKeys::new(&config)?)),
        };

        Ok(Self {
            inner: Arc::new(VerifierInner {
                keys,
                profile: JwtVerificationProfile::new(
                    config.authorized_parties,
                    config.audiences,
                    config.issuers,
                    config.clock_skew,
                    config.require_session_id,
                ),
            }),
        })
    }

    async fn keyset_for_kid(&self, kid: &str) -> Result<Arc<KeySet>, VerificationFailure> {
        match &self.inner.keys {
            // A fixed keyset has nothing to refresh on an unknown kid, so the
            // whole set is returned and `rs256_key_for_kid` reports the missing
            // key as an invalid token — the same outcome the fetching path
            // reaches after a refresh that still lacks the kid.
            KeySource::Static(keyset) => Ok(Arc::clone(keyset)),
            KeySource::Fetched(keys) => keys.keyset_for_kid(kid).await,
        }
    }
}

/// Parses a configured JWKS document, rejecting one that cannot verify a Clerk
/// session token: it would verify nothing, and failing at construction points at
/// the config instead of surfacing later as every token being invalid.
fn parse_static_jwks(jwks_json: &str) -> Result<KeySet, ClerkError> {
    let keyset: KeySet = serde_json::from_str(jwks_json).map_err(|error| {
        ClerkError::InvalidConfig(format!("invalid static Clerk JWKS: {error}"))
    })?;

    if keyset.is_empty() {
        return Err(ClerkError::InvalidConfig(
            "static Clerk JWKS contains no keys".into(),
        ));
    }

    if !jwt::has_rs256_verification_key(&keyset) {
        return Err(ClerkError::InvalidConfig(
            "static Clerk JWKS contains no RS256 signing key with a key id".into(),
        ));
    }

    Ok(keyset)
}

impl FetchedKeys {
    fn new(config: &ClerkAuthLayerConfig) -> Result<Self, ClerkError> {
        if config.secret_key.is_empty() {
            return Err(ClerkError::InvalidConfig(
                "Clerk secret key must not be empty".into(),
            ));
        }
        let jwks_url = config.jwks_url()?;
        let jwks_uri = Uri::try_from(jwks_url.as_str()).map_err(|error| {
            ClerkError::InvalidConfig(format!("invalid Clerk Backend API base URL: {error}"))
        })?;
        let mut authorization = HeaderValue::from_str(&format!("Bearer {}", config.secret_key))
            .map_err(|_| {
                ClerkError::InvalidConfig(
                    "Clerk secret key contains characters that cannot be sent in an HTTP header"
                        .into(),
                )
            })?;
        authorization.set_sensitive(true);

        Ok(Self {
            jwks_uri,
            authorization,
            transport: transport::default_transport()?,
            cache: Mutex::new(JwksCache::default()),
            cache_policy: JwksCachePolicy::default(),
            refresh_lock: futures_util::lock::Mutex::new(()),
        })
    }

    /// Locks the cache, recovering from poisoning: the guarded state is a
    /// keyset plus timestamps and cannot be left logically corrupt by a panic
    /// mid-write, while treating poisoning as fatal would turn one panic into
    /// permanent 503s for every refresh-path request.
    fn lock_cache(&self) -> std::sync::MutexGuard<'_, JwksCache> {
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn cache_lookup(&self, kid: &str) -> JwksCacheLookup {
        self.lock_cache()
            .lookup(kid, self.cache_policy, SystemTime::now())
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
        let _refresh_guard = self.refresh_lock.lock().await;

        // Re-check after acquiring the lock: a concurrent request may have
        // refreshed the cache (or recorded a fetch failure) while this one
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
                    cache.stale_keyset_for_kid(kid, self.cache_policy, SystemTime::now())
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

    fn jwks_request(&self) -> Request<Bytes> {
        let mut request = Request::new(Bytes::new());
        *request.method_mut() = Method::GET;
        *request.uri_mut() = self.jwks_uri.clone();
        let headers = request.headers_mut();
        headers.insert(AUTHORIZATION, self.authorization.clone());
        headers.insert(USER_AGENT, HeaderValue::from_static(JWKS_USER_AGENT));
        request
    }

    async fn fetch_keyset(&self) -> Result<KeySet, VerificationFailure> {
        let response = self
            .transport
            .send(self.jwks_request(), JWKS_MAX_RESPONSE_BYTES)
            .await
            .map_err(|error| {
                match error.kind() {
                    TransportErrorKind::Timeout => tracing::warn!("Clerk JWKS fetch timed out"),
                    TransportErrorKind::TooLarge => {
                        tracing::warn!("Clerk JWKS response exceeds the size limit")
                    }
                    _ => tracing::warn!(error = %error, "failed to fetch Clerk JWKS"),
                }
                VerificationFailure::Unavailable
            })?;

        let status = response.status();
        // The transport never follows redirects (see `transport`), so a 3xx
        // here is the endpoint trying to source the signing keys from
        // somewhere else. It fails closed like any other non-success status;
        // it is only named so the log points at the cause.
        if status.is_redirection() {
            tracing::warn!(
                status = %status,
                "Clerk JWKS endpoint redirected; refusing to source signing keys from another location"
            );
            return Err(VerificationFailure::Unavailable);
        }
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

        serde_json::from_slice::<KeySet>(response.body()).map_err(|error| {
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
