//! Test helpers for applications that authenticate with Clerk through this
//! crate.
//!
//! Tokens are minted locally from an RSA key this crate generates, so tests
//! exercise the real [`ClerkAuthLayer`](crate::server::ClerkAuthLayer)
//! verification path with no Clerk instance, no network, and no JWKS mock.
//!
//! # Testing your application
//!
//! When authentication is not the subject of the test — you just need a
//! signed-in user so you can test what your app does — [`TestClerk`] is the
//! whole API:
//!
//! ```no_run
//! use dioxus_clerk::testing::TestClerk;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let clerk = TestClerk::new()?;
//!
//! let layer = clerk.layer()?;              // wire into your router
//! let cookie = clerk.cookie("user_2abc")?; // send with a request
//! # let _ = (layer, cookie);
//! # Ok(())
//! # }
//! ```
//!
//! # Testing authentication itself
//!
//! When the auth behaviour *is* the subject — org permissions, expiry, tokens
//! that should be rejected — [`TestSession`] builds the claims and
//! [`TestIssuer`] signs them:
//!
//! ```no_run
//! use dioxus_clerk::server::{ClerkAuthLayer, ClerkAuthLayerConfig};
//! use dioxus_clerk::testing::{TestIssuer, TestSession};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let issuer = TestIssuer::generate()?;
//!
//! // The layer verifies against the issuer's keys; nothing is fetched.
//! let config = ClerkAuthLayerConfig::new("").with_static_jwks(issuer.jwks_json()?);
//! let layer = ClerkAuthLayer::from_config(config)?;
//!
//! let admin = issuer.sign(
//!     &TestSession::new("user_2abc")
//!         .with_organization("org_2ghi")
//!         .with_organization_role("org:admin"),
//! )?;
//! let expired = issuer.sign(&TestSession::new("user_2abc").expired())?;
//! # let _ = (layer, admin, expired);
//! # Ok(())
//! # }
//! ```
//!
//! For the full setup — sharing a key with a browser test runner, SSR tests
//! that need no token, and Playwright configuration including the
//! `window.Clerk` fake that keeps a browser suite offline — see the
//! [testing guide](https://github.com/sagikazarmark/dioxus-clerk/blob/main/docs/testing.md).
//!
//! # Choosing a key
//!
//! This crate deliberately ships no key material. Pick whichever fits:
//!
//! - [`TestIssuer::generate`] — a fresh keypair, nothing on disk. Best when the
//!   tokens and the verifier live in the same process.
//! - [`TestIssuer::from_pem_file`] — load a key you generated ahead of time.
//!   Needed when something *outside* the test process (a browser test runner, a
//!   separately spawned server) must sign or verify with the same key.
//! - [`TestIssuer::from_pem_file_or_generate`] — load it, or create it on first
//!   use. Lets a gitignored key work on a fresh checkout with no setup step.
//!
//! Note that RSA-2048 key generation takes on the order of 100ms and is
//! variable, which is fine per suite but adds up per test. Generate once and
//! share it:
//!
//! ```no_run
//! use std::sync::LazyLock;
//! use dioxus_clerk::testing::TestIssuer;
//!
//! static ISSUER: LazyLock<TestIssuer> =
//!     LazyLock::new(|| TestIssuer::generate().expect("test issuer"));
//! ```
//!
//! # Do not ship this
//!
//! Everything here mints tokens that a correctly configured verifier accepts.
//! Keep the `testing` feature under `[dev-dependencies]`, and never point a
//! production [`ClerkAuthLayer`](crate::server::ClerkAuthLayer) at a
//! [`jwks_json`](TestIssuer::jwks_json) from this module.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ct_codecs::{Base64UrlSafeNoPadding, Encoder};
use jwt_simple::prelude::{JWTClaims, RS256KeyPair, RSAKeyPairLike};
use serde_json::{Value, json};

/// Default `kid` for a [`TestIssuer`], used in both the signed token header and
/// the emitted JWKS so the two always agree.
const DEFAULT_KEY_ID: &str = "dioxus-clerk-test-key";

/// Default session lifetime, matching Clerk's own session token TTL.
const DEFAULT_SESSION_LIFETIME: Duration = Duration::from_secs(60);

/// RSA modulus size for [`TestIssuer::generate`]. Clerk signs with RS256, and
/// the verifier rejects every other algorithm, so this is not configurable.
const KEY_MODULUS_BITS: usize = 2048;

/// Something went wrong minting a test token or loading a test key.
#[derive(Debug, thiserror::Error)]
pub enum TestIssuerError {
    /// The key file could not be read or written.
    #[error("failed to access test key at {path}: {source}")]
    KeyFile {
        /// The path that could not be read or written.
        path: PathBuf,
        /// The underlying filesystem error.
        source: std::io::Error,
    },

    /// The key could not be generated, parsed, or serialized.
    #[error("test key error: {0}")]
    Key(String),

    /// The token could not be signed.
    #[error("failed to sign test token: {0}")]
    Sign(String),

    /// A session was built with organization permissions that are not in
    /// Clerk's `org:<feature>:<permission>` form.
    #[error(
        "organization permission {0:?} is not in `org:<feature>:<permission>` form; \
         use with_v1_organization_claims to emit unencoded permissions instead"
    )]
    OrganizationPermission(String),

    /// The auth layer could not be built from this issuer's keys.
    #[error("failed to build a test Clerk auth layer: {0}")]
    Layer(String),
}

/// A ready-made Clerk setup for tests that are about the application rather
/// than about authentication.
///
/// Wraps a [`TestIssuer`] so the common case — "this request is signed in as
/// someone, now test what my app does" — is two calls and needs no knowledge of
/// JWKS or token claims:
///
/// ```no_run
/// # use dioxus_clerk::testing::TestClerk;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let clerk = TestClerk::new()?;
///
/// // Wire the layer into your router, then send authenticated requests.
/// let layer = clerk.layer()?;
/// let cookie = clerk.cookie("user_2abc")?;
/// # let _ = (layer, cookie);
/// # Ok(())
/// # }
/// ```
///
/// Reach past it when authentication *is* what you are testing: [`session`] and
/// [`cookie_for`] take a full [`TestSession`], and [`issuer`] exposes the
/// underlying [`TestIssuer`].
///
/// [`session`]: Self::session
/// [`cookie_for`]: Self::cookie_for
/// [`issuer`]: Self::issuer
pub struct TestClerk {
    issuer: TestIssuer,
}

impl TestClerk {
    /// Generates a fresh key and a Clerk setup around it.
    ///
    /// Costs one RSA-2048 key generation (~100ms, variable). Build it once per
    /// suite and share it — see the [module docs](self).
    pub fn new() -> Result<Self, TestIssuerError> {
        Ok(Self::from_issuer(TestIssuer::generate()?))
    }

    /// Builds a Clerk setup around an existing issuer, so the key can be shared
    /// with another process. See
    /// [`TestIssuer::from_pem_file_or_generate`].
    pub fn from_issuer(issuer: TestIssuer) -> Self {
        Self { issuer }
    }

    /// The underlying issuer, for anything this wrapper does not cover.
    pub fn issuer(&self) -> &TestIssuer {
        &self.issuer
    }

    /// The JWKS document for this setup's key.
    pub fn jwks_json(&self) -> Result<String, TestIssuerError> {
        self.issuer.jwks_json()
    }

    /// A signed-in session for `user_id`, to customize before signing.
    ///
    /// `TestClerk::session("user_2abc")` is `TestSession::new("user_2abc")`;
    /// it is here so a test does not have to import both types.
    pub fn session(&self, user_id: impl Into<String>) -> TestSession {
        TestSession::new(user_id)
    }

    /// A session token for `user_id`.
    pub fn token(&self, user_id: impl Into<String>) -> Result<String, TestIssuerError> {
        self.token_for(&TestSession::new(user_id))
    }

    /// A `__session` cookie header value for `user_id`, the credential Clerk's
    /// browser SDK sends.
    pub fn cookie(&self, user_id: impl Into<String>) -> Result<String, TestIssuerError> {
        self.cookie_for(&TestSession::new(user_id))
    }

    /// An `Authorization` header value for `user_id`.
    pub fn bearer(&self, user_id: impl Into<String>) -> Result<String, TestIssuerError> {
        self.bearer_for(&TestSession::new(user_id))
    }

    /// A session token for a customized session.
    pub fn token_for(&self, session: &TestSession) -> Result<String, TestIssuerError> {
        self.issuer.sign(session)
    }

    /// A `__session` cookie header value for a customized session.
    pub fn cookie_for(&self, session: &TestSession) -> Result<String, TestIssuerError> {
        self.issuer.session_cookie(session)
    }

    /// An `Authorization` header value for a customized session.
    pub fn bearer_for(&self, session: &TestSession) -> Result<String, TestIssuerError> {
        Ok(format!("Bearer {}", self.issuer.sign(session)?))
    }
}

#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
impl TestClerk {
    /// A layer config that verifies against this setup's key and never reaches
    /// the network.
    ///
    /// Use this when the layer needs further configuration; otherwise
    /// [`layer`](Self::layer) builds it directly.
    pub fn config(&self) -> Result<crate::server::ClerkAuthLayerConfig, TestIssuerError> {
        // The secret key only authenticates JWKS fetches, and a static keyset
        // never fetches, so there is nothing meaningful to pass here.
        Ok(crate::server::ClerkAuthLayerConfig::new("").with_static_jwks(self.jwks_json()?))
    }

    /// A `ClerkAuthLayer` that verifies tokens minted by this setup.
    pub fn layer(&self) -> Result<crate::server::ClerkAuthLayer, TestIssuerError> {
        crate::server::ClerkAuthLayer::from_config(self.config()?)
            .map_err(|error| TestIssuerError::Layer(error.to_string()))
    }
}

impl std::fmt::Debug for TestClerk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestClerk")
            .field("issuer", &self.issuer)
            .finish()
    }
}

/// Mints Clerk-shaped session tokens signed by a local RSA key, and emits the
/// matching JWKS for
/// [`with_static_jwks`](crate::server::ClerkAuthLayerConfig::with_static_jwks).
///
/// See the [module docs](self) for how to choose between generating a key and
/// loading one from disk.
pub struct TestIssuer {
    keypair: RS256KeyPair,
    key_id: String,
}

impl TestIssuer {
    /// Generates a fresh RSA-2048 keypair, held in memory only.
    pub fn generate() -> Result<Self, TestIssuerError> {
        let keypair = RS256KeyPair::generate(KEY_MODULUS_BITS)
            .map_err(|error| TestIssuerError::Key(error.to_string()))?;

        Ok(Self::from_keypair(keypair))
    }

    /// Loads a PKCS#1 or PKCS#8 RSA private key from PEM.
    pub fn from_pem(pem: &str) -> Result<Self, TestIssuerError> {
        let keypair =
            RS256KeyPair::from_pem(pem).map_err(|error| TestIssuerError::Key(error.to_string()))?;

        Ok(Self::from_keypair(keypair))
    }

    /// Loads a private key from a PEM file.
    ///
    /// Use this when the key must outlive the test process or be shared with
    /// one — a browser test runner minting its own cookies, say. Prefer
    /// [`from_pem_file_or_generate`](Self::from_pem_file_or_generate) if the
    /// file is gitignored, so a fresh checkout does not have to run a setup
    /// step first.
    pub fn from_pem_file(path: impl AsRef<Path>) -> Result<Self, TestIssuerError> {
        let path = path.as_ref();
        let pem = std::fs::read_to_string(path).map_err(|source| TestIssuerError::KeyFile {
            path: path.to_path_buf(),
            source,
        })?;

        Self::from_pem(&pem)
    }

    /// Loads a private key from a PEM file, generating and writing one if the
    /// file does not exist yet.
    ///
    /// This is the path for a gitignored key: the first run creates it, later
    /// runs reuse it, and no separate `openssl` script is needed. Missing
    /// parent directories are created.
    ///
    /// Concurrent first runs are safe. If another process wins the race to
    /// create the file, this returns the key that process wrote rather than the
    /// one generated here, so every process ends up on the same key.
    pub fn from_pem_file_or_generate(path: impl AsRef<Path>) -> Result<Self, TestIssuerError> {
        let path = path.as_ref();
        match std::fs::read_to_string(path) {
            Ok(pem) => return Self::from_pem(&pem),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(TestIssuerError::KeyFile {
                    path: path.to_path_buf(),
                    source,
                });
            }
        }

        let issuer = Self::generate()?;
        issuer.write_pem_file(path)?;

        // Read back rather than returning `issuer`: a concurrent process may
        // have created the file first, in which case its key is the one on
        // disk and the one any other process will load.
        Self::from_pem_file(path)
    }

    /// Writes this issuer's private key to `path`, creating parent directories.
    ///
    /// Leaves an existing file alone, so racing writers converge on whichever
    /// key landed first.
    fn write_pem_file(&self, path: &Path) -> Result<(), TestIssuerError> {
        let key_file_error = |source: std::io::Error| TestIssuerError::KeyFile {
            path: path.to_path_buf(),
            source,
        };

        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(key_file_error)?;
        }

        // Write to a process-unique temporary file, then rename into place:
        // a reader never observes a half-written key.
        let temporary = path.with_extension(format!("tmp{}", std::process::id()));
        std::fs::write(&temporary, self.to_pem()?).map_err(key_file_error)?;
        restrict_key_file_permissions(&temporary);

        let renamed = std::fs::rename(&temporary, path);
        if renamed.is_err() {
            // Either the rename genuinely failed, or another process created
            // the file first on a platform where rename refuses to clobber.
            // The caller re-reads `path`, so a present file means success.
            let _ = std::fs::remove_file(&temporary);
            if !path.exists() {
                return renamed.map_err(key_file_error);
            }
        }

        Ok(())
    }

    fn from_keypair(keypair: RS256KeyPair) -> Self {
        Self {
            keypair: keypair.with_key_id(DEFAULT_KEY_ID),
            key_id: DEFAULT_KEY_ID.to_string(),
        }
    }

    /// Overrides the `kid` used in signed token headers and the emitted JWKS.
    pub fn with_key_id(mut self, key_id: impl Into<String>) -> Self {
        let key_id = key_id.into();
        self.keypair = self.keypair.with_key_id(&key_id);
        self.key_id = key_id;
        self
    }

    /// The `kid` this issuer signs with.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Serializes the private key as PKCS#1 PEM.
    pub fn to_pem(&self) -> Result<String, TestIssuerError> {
        self.keypair
            .to_pem()
            .map_err(|error| TestIssuerError::Key(error.to_string()))
    }

    /// The JWKS document for this issuer's public key.
    ///
    /// Pass it to
    /// [`with_static_jwks`](crate::server::ClerkAuthLayerConfig::with_static_jwks)
    /// to verify offline, or serve it from a mock JWKS endpoint to exercise the
    /// fetching and caching path as well.
    pub fn jwks_json(&self) -> Result<String, TestIssuerError> {
        let components = self.keypair.public_key().to_components();
        let encode = |bytes: &[u8]| {
            Base64UrlSafeNoPadding::encode_to_string(bytes)
                .map_err(|error| TestIssuerError::Key(error.to_string()))
        };

        let jwks = json!({
            "keys": [{
                "use": "sig",
                "kty": "RSA",
                "kid": self.key_id,
                "alg": "RS256",
                "n": encode(&components.n)?,
                "e": encode(&components.e)?,
            }]
        });

        serde_json::to_string(&jwks).map_err(|error| TestIssuerError::Key(error.to_string()))
    }

    /// Signs `session` into a Clerk-shaped RS256 session token.
    pub fn sign(&self, session: &TestSession) -> Result<String, TestIssuerError> {
        let claims: JWTClaims<Value> = serde_json::from_value(session.to_claims()?)
            .map_err(|error| TestIssuerError::Sign(error.to_string()))?;

        self.keypair
            .sign(claims)
            .map_err(|error| TestIssuerError::Sign(error.to_string()))
    }

    /// Signs `session` and formats it as a `__session` cookie header value,
    /// the credential Clerk's browser SDK sends and this crate reads.
    pub fn session_cookie(&self, session: &TestSession) -> Result<String, TestIssuerError> {
        Ok(format!("__session={}", self.sign(session)?))
    }
}

impl std::fmt::Debug for TestIssuer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestIssuer")
            .field("key_id", &self.key_id)
            .field("keypair", &"<redacted>")
            .finish()
    }
}

#[cfg(unix)]
fn restrict_key_file_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    // Best-effort: a test key is not a secret worth failing a test run over.
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_key_file_permissions(_path: &Path) {}

/// A Clerk session to mint a token for.
///
/// Defaults to a currently-valid personal-account session: a `sid`, an `iat`
/// and `nbf` of now, and an `exp` one minute out. Every part is overridable,
/// including into shapes the verifier is supposed to reject — see
/// [`expired`](Self::expired) and
/// [`without_session_id`](Self::without_session_id).
#[derive(Debug, Clone)]
pub struct TestSession {
    user_id: String,
    session_id: Option<String>,
    issuer: Option<String>,
    audience: Option<String>,
    authorized_party: Option<String>,
    issued_at: Option<i64>,
    not_before: Option<i64>,
    expires_at: Option<i64>,
    lifetime: Duration,
    organization_id: Option<String>,
    organization_slug: Option<String>,
    organization_role: Option<String>,
    organization_permissions: Vec<String>,
    v1_organization_claims: bool,
    extra_claims: BTreeMap<String, Value>,
}

impl TestSession {
    /// A currently-valid session for `user_id`, with a derived `sid`.
    pub fn new(user_id: impl Into<String>) -> Self {
        let user_id = user_id.into();

        Self {
            session_id: Some(format!("sess_{user_id}")),
            user_id,
            issuer: None,
            audience: None,
            authorized_party: None,
            issued_at: None,
            not_before: None,
            expires_at: None,
            lifetime: DEFAULT_SESSION_LIFETIME,
            organization_id: None,
            organization_slug: None,
            organization_role: None,
            organization_permissions: vec![],
            v1_organization_claims: false,
            extra_claims: BTreeMap::new(),
        }
    }

    /// Overrides the `sid` session id claim.
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Omits the `sid` claim, producing a token shaped like a Clerk JWT-template
    /// token rather than a session token.
    ///
    /// Verification rejects these by default, so this is how to test that an
    /// endpoint is not accepting them (or, with
    /// [`allow_non_session_tokens`](crate::server::ClerkAuthLayerConfig::allow_non_session_tokens),
    /// that it deliberately does).
    pub fn without_session_id(mut self) -> Self {
        self.session_id = None;
        self
    }

    /// Sets the `iss` claim, to test
    /// [`add_issuer`](crate::server::ClerkAuthLayerConfig::add_issuer) pinning.
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Sets the `aud` claim, to test
    /// [`add_audience`](crate::server::ClerkAuthLayerConfig::add_audience).
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    /// Sets the `azp` claim, to test
    /// [`add_authorized_party`](crate::server::ClerkAuthLayerConfig::add_authorized_party).
    pub fn with_authorized_party(mut self, authorized_party: impl Into<String>) -> Self {
        self.authorized_party = Some(authorized_party.into());
        self
    }

    /// Sets how long after `iat` the token expires. Defaults to one minute.
    pub fn with_lifetime(mut self, lifetime: Duration) -> Self {
        self.lifetime = lifetime;
        self
    }

    /// Pins `iat` and `nbf` to a fixed Unix timestamp instead of now.
    pub fn with_issued_at(mut self, issued_at: i64) -> Self {
        self.issued_at = Some(issued_at);
        self
    }

    /// Pins `nbf` to a fixed Unix timestamp, independent of `iat`.
    ///
    /// Set it in the future to produce a not-yet-valid token.
    pub fn with_not_before(mut self, not_before: i64) -> Self {
        self.not_before = Some(not_before);
        self
    }

    /// Pins `exp` to a fixed Unix timestamp, ignoring
    /// [`with_lifetime`](Self::with_lifetime).
    pub fn with_expires_at(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Makes the token already expired, for testing rejection and refresh paths.
    ///
    /// Backdates `iat`/`nbf` far enough that the token is expired well beyond
    /// the configured clock skew.
    pub fn expired(mut self) -> Self {
        let issued_at = self.issued_at.unwrap_or_else(unix_now) - 3600;
        self.issued_at = Some(issued_at);
        self.expires_at = Some(issued_at + 60);
        self
    }

    /// Puts the session in an organization.
    ///
    /// Emits Clerk's v2 `o` claim by default; see
    /// [`with_v1_organization_claims`](Self::with_v1_organization_claims) for
    /// the older flat shape.
    pub fn with_organization(mut self, organization_id: impl Into<String>) -> Self {
        self.organization_id = Some(organization_id.into());
        self
    }

    /// Sets the organization slug (`o.slg`, or `org_slug` on v1).
    pub fn with_organization_slug(mut self, slug: impl Into<String>) -> Self {
        self.organization_slug = Some(slug.into());
        self
    }

    /// Sets the organization role (`o.rol`, or `org_role` on v1).
    ///
    /// Accepts either `admin` or `org:admin`; verification normalizes both to
    /// the `org:`-prefixed form.
    pub fn with_organization_role(mut self, role: impl Into<String>) -> Self {
        self.organization_role = Some(role.into());
        self
    }

    /// Sets the organization permissions, as `org:<feature>:<permission>`
    /// strings such as `org:dashboard:read`.
    ///
    /// On v2 these are encoded into Clerk's packed `fea`/`per`/`fpm` claim
    /// trio, which is what the verifier decodes back into
    /// [`ClerkAuth::org_permissions`](crate::core::ClerkAuth::org_permissions).
    /// A permission not in that three-part form is an error at
    /// [`sign`](TestIssuer::sign) time, since it could not round-trip.
    pub fn with_organization_permissions(
        mut self,
        permissions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.organization_permissions = permissions.into_iter().map(Into::into).collect();
        self
    }

    /// Emits pre-v2 flat `org_id` / `org_slug` / `org_role` / `org_permissions`
    /// claims instead of the packed `o` claim.
    ///
    /// Clerk issues v2 claims now; this covers the still-supported older shape,
    /// and accepts permission strings in any form.
    pub fn with_v1_organization_claims(mut self) -> Self {
        self.v1_organization_claims = true;
        self
    }

    /// Sets an arbitrary top-level claim, overriding anything above.
    ///
    /// The escape hatch for claims this builder does not model — and for
    /// deliberately malformed tokens.
    pub fn with_claim(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.extra_claims.insert(name.into(), value.into());
        self
    }

    /// The claim set this session serializes to.
    ///
    /// Exposed so a test can assert on claims directly, or hand them to another
    /// signer.
    pub fn to_claims(&self) -> Result<Value, TestIssuerError> {
        let issued_at = self.issued_at.unwrap_or_else(unix_now);
        let mut claims = json!({
            "sub": self.user_id,
            "iat": issued_at,
            "nbf": self.not_before.unwrap_or(issued_at),
            "exp": self.expires_at.unwrap_or_else(|| {
                issued_at + i64::try_from(self.lifetime.as_secs()).unwrap_or(i64::MAX)
            }),
        });

        let object = claims
            .as_object_mut()
            .expect("claims are built as a JSON object");

        for (name, value) in [
            ("sid", self.session_id.as_ref()),
            ("iss", self.issuer.as_ref()),
            ("aud", self.audience.as_ref()),
            ("azp", self.authorized_party.as_ref()),
        ] {
            if let Some(value) = value {
                object.insert(name.to_string(), Value::String(value.clone()));
            }
        }

        for (name, value) in self.organization_claims()? {
            object.insert(name, value);
        }

        for (name, value) in &self.extra_claims {
            object.insert(name.clone(), value.clone());
        }

        Ok(claims)
    }

    fn organization_claims(&self) -> Result<Vec<(String, Value)>, TestIssuerError> {
        let Some(organization_id) = self.organization_id.as_ref() else {
            return Ok(vec![]);
        };

        if self.v1_organization_claims {
            let mut claims = vec![("org_id".to_string(), json!(organization_id))];
            if let Some(slug) = &self.organization_slug {
                claims.push(("org_slug".to_string(), json!(slug)));
            }
            if let Some(role) = &self.organization_role {
                claims.push(("org_role".to_string(), json!(role)));
            }
            if !self.organization_permissions.is_empty() {
                claims.push((
                    "org_permissions".to_string(),
                    json!(self.organization_permissions),
                ));
            }
            return Ok(claims);
        }

        let mut organization = json!({ "id": organization_id });
        let object = organization
            .as_object_mut()
            .expect("organization claim is built as a JSON object");
        if let Some(slug) = &self.organization_slug {
            object.insert("slg".to_string(), json!(slug));
        }
        if let Some(role) = &self.organization_role {
            object.insert("rol".to_string(), json!(role));
        }

        let mut claims = vec![];
        if !self.organization_permissions.is_empty() {
            let packed = PackedPermissions::encode(&self.organization_permissions)?;
            object.insert("per".to_string(), json!(packed.permissions));
            object.insert("fpm".to_string(), json!(packed.feature_permission_map));
            claims.push(("fea".to_string(), json!(packed.features)));
        }
        claims.push(("o".to_string(), organization));

        Ok(claims)
    }
}

/// Clerk's v2 organization permission encoding: a feature list (`fea`), a
/// permission list (`per`), and one bitmask per feature over the permission
/// list (`fpm`).
struct PackedPermissions {
    features: String,
    permissions: String,
    feature_permission_map: String,
}

impl PackedPermissions {
    fn encode(permissions: &[String]) -> Result<Self, TestIssuerError> {
        // Ordered-unique, because a bit index refers to a position in `per` and
        // the decoder emits features in `fea` order.
        let mut feature_names: Vec<&str> = vec![];
        let mut permission_names: Vec<&str> = vec![];
        let mut pairs: Vec<(usize, usize)> = vec![];

        for permission in permissions {
            let (feature, verb) = permission
                .strip_prefix("org:")
                .and_then(|rest| rest.split_once(':'))
                .filter(|(feature, verb)| !feature.is_empty() && !verb.is_empty())
                .ok_or_else(|| TestIssuerError::OrganizationPermission(permission.clone()))?;

            let feature_index = index_of_or_push(&mut feature_names, feature);
            let permission_index = index_of_or_push(&mut permission_names, verb);
            pairs.push((feature_index, permission_index));
        }

        // The decoder rejects a `per` longer than a u128 has bits, and so does
        // Clerk; a test session with that many distinct permission verbs is a
        // mistake rather than a case worth encoding.
        if permission_names.len() > u128::BITS as usize {
            return Err(TestIssuerError::OrganizationPermission(format!(
                "{} distinct permissions exceeds the {} the v2 claim encoding allows",
                permission_names.len(),
                u128::BITS
            )));
        }

        let mut masks = vec![0u128; feature_names.len()];
        for (feature_index, permission_index) in pairs {
            masks[feature_index] |= 1 << permission_index;
        }

        Ok(Self {
            features: feature_names
                .iter()
                .map(|feature| format!("o:{feature}"))
                .collect::<Vec<_>>()
                .join(","),
            permissions: permission_names.join(","),
            feature_permission_map: masks
                .iter()
                .map(u128::to_string)
                .collect::<Vec<_>>()
                .join(","),
        })
    }
}

fn index_of_or_push<'a>(names: &mut Vec<&'a str>, name: &'a str) -> usize {
    match names.iter().position(|existing| *existing == name) {
        Some(index) => index,
        None => {
            names.push(name);
            names.len() - 1
        }
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}
