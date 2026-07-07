//! Configuration for server-side Clerk request verification.

use std::time::Duration;

use crate::core::ClerkError;

const DEFAULT_BACKEND_API_BASE_URL: &str = "https://api.clerk.com/v1";
pub(crate) const DEFAULT_JWKS_CACHE_TTL: Duration = Duration::from_secs(60 * 60);
pub(crate) const DEFAULT_UNKNOWN_KID_REFRESH_INTERVAL: Duration = Duration::from_secs(60 * 5);
/// After a failed JWKS fetch, further refresh attempts are suppressed for this
/// long so an upstream outage does not serialize every request into its own
/// slow fetch while holding the refresh lock.
pub(crate) const DEFAULT_JWKS_FAILURE_BACKOFF: Duration = Duration::from_secs(10);
/// How long past the cache TTL a stored keyset may still be served when a
/// refresh fails (stale-while-error), so a Clerk outage does not turn every
/// credentialed request into a 503 while the cached keys would still verify.
pub(crate) const DEFAULT_JWKS_MAX_STALE: Duration = Duration::from_secs(60 * 60 * 24);
pub(crate) const DEFAULT_CLOCK_SKEW: Duration = Duration::from_secs(5);

/// Configuration for [`crate::server::ClerkAuthLayer`].
///
/// # Security: default claim acceptance
///
/// By default — with no issuers, audiences, or authorized parties configured
/// — verification accepts **any** RS256 JWT that is signed by your instance's
/// JWKS and passes standard `exp`/`nbf` validation. The `iss`, `aud`, and
/// `azp` claims are not checked.
///
/// This is safe for the common single-instance case: the JWKS endpoint is
/// scoped to your Clerk instance by the secret key, so a token that verifies
/// against it was necessarily minted by *your* instance. It is not a bypass.
///
/// # Security: session tokens only, by default
///
/// Verification accepts only Clerk **session** tokens by default: a token must
/// carry a `sid` (session id) claim. Clerk **JWT-template** tokens (minted with
/// `getToken({ template })` for third-party integrations) are signed by the
/// same instance JWKS and carry a `sub`, but omit `sid`, so without this check
/// a leaked template token could be replayed to authenticate as its user.
/// Requiring `sid` rejects them while accepting every genuine session token.
/// Opt out with [`allow_non_session_tokens`](Self::allow_non_session_tokens)
/// only if you deliberately verify non-session Clerk JWTs here.
///
/// The claim checks below are further defense-in-depth for setups where the
/// single-instance assumption is weaker — multiple apps or environments sharing
/// an instance, satellite domains, or tokens minted for a different audience
/// that you do not want one service to accept for another. For those, harden
/// verification:
///
/// - [`add_authorized_party`](Self::add_authorized_party) — restrict the
///   `azp` origins that may present tokens (Clerk's recommended check; tokens
///   without `azp` are still accepted, matching Clerk's guidance).
/// - [`add_issuer`](Self::add_issuer) — pin the instance frontend origin.
/// - [`add_audience`](Self::add_audience) — require a specific `aud` when you
///   mint audience-scoped tokens.
#[derive(Clone)]
pub struct ClerkAuthLayerConfig {
    pub(crate) secret_key: String,
    backend_api_base_url: String,
    allow_insecure_backend_api_base_url: bool,
    pub(crate) authorized_parties: Vec<String>,
    pub(crate) audiences: Vec<String>,
    pub(crate) issuers: Vec<String>,
    pub(crate) clock_skew: Duration,
    pub(crate) require_session_id: bool,
}

impl ClerkAuthLayerConfig {
    /// Creates a config using Clerk's default Backend API base URL.
    pub fn new(secret_key: impl Into<String>) -> Self {
        Self {
            secret_key: secret_key.into(),
            backend_api_base_url: DEFAULT_BACKEND_API_BASE_URL.into(),
            allow_insecure_backend_api_base_url: false,
            authorized_parties: vec![],
            audiences: vec![],
            issuers: vec![],
            clock_skew: DEFAULT_CLOCK_SKEW,
            require_session_id: true,
        }
    }

    /// Creates a config from the conventional `CLERK_SECRET_KEY` environment variable.
    pub fn from_env() -> Result<Self, ClerkError> {
        let secret_key = std::env::var("CLERK_SECRET_KEY").map_err(|_| {
            ClerkError::InvalidConfig("missing CLERK_SECRET_KEY environment variable".into())
        })?;
        Ok(Self::new(secret_key))
    }

    /// Sets the HTTPS Clerk Backend API base URL, including API version.
    ///
    /// For example: `https://api.clerk.com/v1`. The JWKS endpoint is derived
    /// by appending `/jwks` to this base URL.
    pub fn with_backend_api_base_url(mut self, url: impl Into<String>) -> Self {
        self.backend_api_base_url = url.into();
        self.allow_insecure_backend_api_base_url = false;
        self
    }

    /// Sets a local/test-only HTTP Backend API base URL.
    ///
    /// This explicitly allows `http://` URLs for local JWKS mocks. Do not use
    /// it in production; plain HTTP allows JWKS response tampering.
    pub fn with_insecure_backend_api_base_url(mut self, url: impl Into<String>) -> Self {
        let url = url.into();
        tracing::warn!(
            url = %url,
            "Clerk backend API base URL configured over insecure HTTP: JWKS signing keys can be tampered with in transit. Use with_backend_api_base_url with an https:// URL outside of local development and tests."
        );
        self.backend_api_base_url = url;
        self.allow_insecure_backend_api_base_url = true;
        self
    }

    /// Adds a single allowed `azp` authorized-party origin to the configured set.
    pub fn add_authorized_party(mut self, party: impl Into<String>) -> Self {
        self.authorized_parties.push(party.into());
        self
    }

    /// Sets allowed `azp` authorized-party origins.
    ///
    /// When configured, a token with an `azp` claim must match one of these
    /// values. Tokens without `azp` continue to be accepted, matching Clerk's
    /// manual verification guidance.
    pub fn with_authorized_parties(
        mut self,
        parties: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.authorized_parties = parties.into_iter().map(Into::into).collect();
        self
    }

    /// Adds a single allowed JWT audience to the configured set.
    pub fn add_audience(mut self, audience: impl Into<String>) -> Self {
        self.audiences.push(audience.into());
        self
    }

    /// Sets allowed JWT audiences.
    pub fn with_audiences(
        mut self,
        audiences: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.audiences = audiences.into_iter().map(Into::into).collect();
        self
    }

    /// Adds a single allowed JWT issuer to the configured set.
    ///
    /// Clerk session tokens carry the instance frontend origin as `iss`
    /// (for example `https://your-app.clerk.accounts.dev`). When configured,
    /// tokens whose `iss` claim is missing or does not match are rejected.
    pub fn add_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuers.push(issuer.into());
        self
    }

    /// Sets allowed JWT issuers.
    pub fn with_issuers(mut self, issuers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.issuers = issuers.into_iter().map(Into::into).collect();
        self
    }

    /// Sets accepted clock skew for JWT `exp`, `nbf`, and `iat` validation.
    pub fn with_clock_skew(mut self, clock_skew: Duration) -> Self {
        self.clock_skew = clock_skew;
        self
    }

    /// Accept instance-signed JWTs that are not session tokens (tokens without
    /// a `sid` claim, such as Clerk JWT-template tokens).
    ///
    /// By default verification requires a `sid` so a leaked JWT-template token
    /// cannot be replayed as a session (see the type-level security note). Call
    /// this only when you deliberately verify non-session Clerk JWTs here, and
    /// pair it with [`add_audience`](Self::add_audience) to keep tokens from one
    /// template being accepted for another.
    pub fn allow_non_session_tokens(mut self) -> Self {
        self.require_session_id = false;
        self
    }

    pub(crate) fn jwks_url(&self) -> Result<reqwest::Url, ClerkError> {
        let jwks_url = format!("{}/jwks", self.backend_api_base_url.trim_end_matches('/'));
        let url = reqwest::Url::parse(&jwks_url).map_err(|error| {
            ClerkError::InvalidConfig(format!("invalid Clerk Backend API base URL: {error}"))
        })?;

        if url.scheme() != "https" && !self.allow_insecure_backend_api_base_url {
            return Err(ClerkError::InvalidConfig(
                "Clerk Backend API base URL must use https; use with_insecure_backend_api_base_url only for local tests".into(),
            ));
        }

        Ok(url)
    }
}

impl std::fmt::Debug for ClerkAuthLayerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClerkAuthLayerConfig")
            .field("secret_key", &"<redacted>")
            .field("backend_api_base_url", &self.backend_api_base_url)
            .field(
                "allow_insecure_backend_api_base_url",
                &self.allow_insecure_backend_api_base_url,
            )
            .field("authorized_parties", &self.authorized_parties)
            .field("audiences", &self.audiences)
            .field("issuers", &self.issuers)
            .field("clock_skew", &self.clock_skew)
            .field("require_session_id", &self.require_session_id)
            .finish()
    }
}
