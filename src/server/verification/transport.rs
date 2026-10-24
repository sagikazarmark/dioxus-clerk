//! The HTTP client the JWKS fetch runs over, chosen per target.
//!
//! Verification needs one thing from HTTP: send a `GET`, get back the status,
//! headers, and a bounded body. jwk-simple's [`HttpTransport`] is exactly that
//! contract, so the fetch path is written once against it and only the client
//! behind it differs per target.
//!
//! Two properties are enforced here rather than after the fact, because no
//! layer between the fetch and the socket reports the final URL, so a
//! post-fetch origin check is not available:
//!
//! - **Redirects are refused.** The signing keys are the entire trust root:
//!   following a redirect (compromised CDN, open redirect, an `https`→`http`
//!   downgrade) would source them from another origin, and the secret key
//!   rides on the request as a bearer token. A 3xx comes back as itself and
//!   the caller fails closed on the status.
//! - **The whole exchange is bounded in time**, body included, so a hung
//!   upstream cannot hold the refresh lock forever.

use std::sync::Arc;
#[cfg(any(not(target_arch = "wasm32"), feature = "worker"))]
use std::time::Duration;

use jwk_simple::jwks::HttpTransport;

use crate::core::ClerkError;

/// Bounds a whole JWKS fetch: connect, headers, and body.
#[cfg(any(not(target_arch = "wasm32"), feature = "worker"))]
const JWKS_FETCH_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn default_transport() -> Result<Arc<dyn HttpTransport>, ClerkError> {
    let client = reqwest::Client::builder()
        .timeout(JWKS_FETCH_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| {
            ClerkError::InvalidConfig(format!("failed to build Clerk JWKS HTTP client: {error}"))
        })?;

    Ok(Arc::new(jwk_simple::jwks::ReqwestTransport::new(client)))
}

#[cfg(all(target_arch = "wasm32", feature = "worker"))]
pub(super) fn default_transport() -> Result<Arc<dyn HttpTransport>, ClerkError> {
    use axum::body::Bytes;
    use axum::http::Request;
    use http_body_util::Full;
    use jwk_simple::jwks::{BoxError, TowerTransport, TransportErrorKind};
    use tower_worker_fetch::{FetchService, Redirect, Stage, Timeout};

    // Both knobs are per-request extensions on Workers. `fetch` follows
    // redirects by default and, unlike a browser, forwards `Authorization` to
    // the redirect target even on another host, so `Manual` is what keeps the
    // secret key where it was sent. There is no tokio timer on Workers (a
    // `tower::timeout` layer would panic), so the deadline is an
    // `AbortSignal.timeout` on the request instead.
    let service = tower::ServiceBuilder::new()
        .map_request(|mut request: Request<Full<Bytes>>| {
            request.extensions_mut().insert(Redirect::Manual);
            request.extensions_mut().insert(Timeout(JWKS_FETCH_TIMEOUT));
            request
        })
        .service(FetchService::new());

    // Only diagnostics depend on the classification: every transport error
    // fails closed the same way. It just makes a Workers timeout log as one.
    let transport = TowerTransport::new(service).with_classifier(|error: &BoxError| {
        let error = error.downcast_ref::<tower_worker_fetch::Error>()?;
        Some(if error.is_timeout() {
            TransportErrorKind::Timeout
        } else if error.stage() == Stage::Body {
            TransportErrorKind::Body
        } else {
            TransportErrorKind::Other
        })
    });

    Ok(Arc::new(transport))
}

#[cfg(all(target_arch = "wasm32", not(feature = "worker")))]
pub(super) fn default_transport() -> Result<Arc<dyn HttpTransport>, ClerkError> {
    // `server` compiles for browser wasm only so features stay additive: a web
    // build that unifies `server` in keeps the client path (see `lib.rs`).
    // Nothing can serve requests there, so rather than ship a browser HTTP
    // client for a path that cannot run, refuse at construction and name the
    // feature that can.
    Err(ClerkError::InvalidConfig(
        "fetching the Clerk JWKS on wasm32 requires the `worker` feature; a browser build cannot verify session tokens".into(),
    ))
}
