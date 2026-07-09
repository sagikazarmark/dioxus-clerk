//! Non-rejecting tower layer that records request credential verification
//! outcomes for downstream handlers / server functions to gate on.
//!
//! This layer never short-circuits with 401: when no token is present (or the
//! token fails to validate), the request
//! is forwarded with a [`VerificationOutcome`](crate::core::VerificationOutcome) extension describing the
//! result. Server functions should use the context reader exposed by
//! [`crate::server::current_auth`] / [`crate::server::current_auth_opt`]; handlers that need lower-level access can
//! inspect `VerificationOutcome` directly. This lets public and private routes
//! coexist under the same layer.

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use super::config::ClerkAuthLayerConfig;
use super::verification::{VerifiedRequest, Verifier, verify_request};
use axum::body::Body;
use axum::http::{Request, Response};
use tower::{Layer, Service};

#[cfg(not(target_arch = "wasm32"))]
trait MaybeSend: Send {}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + ?Sized> MaybeSend for T {}

#[cfg(target_arch = "wasm32")]
trait MaybeSend {}

#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> MaybeSend for T {}

#[cfg(not(target_arch = "wasm32"))]
type BoxServiceFuture<E> = Pin<Box<dyn Future<Output = Result<Response<Body>, E>> + Send>>;

#[cfg(target_arch = "wasm32")]
type BoxServiceFuture<E> = Pin<Box<dyn Future<Output = Result<Response<Body>, E>>>>;

#[cfg(all(target_arch = "wasm32", feature = "worker"))]
type ServiceFuture<E> = send_wrapper::SendWrapper<BoxServiceFuture<E>>;

#[cfg(not(all(target_arch = "wasm32", feature = "worker")))]
type ServiceFuture<E> = BoxServiceFuture<E>;

#[cfg(all(target_arch = "wasm32", feature = "worker"))]
fn service_future<E>(future: BoxServiceFuture<E>) -> ServiceFuture<E> {
    send_wrapper::SendWrapper::new(future)
}

#[cfg(not(all(target_arch = "wasm32", feature = "worker")))]
fn service_future<E>(future: BoxServiceFuture<E>) -> ServiceFuture<E> {
    future
}

/// Tower layer that verifies Clerk session JWTs and inserts a
/// [`VerificationOutcome`](crate::core::VerificationOutcome) into request extensions. Valid bearer tokens (or
/// `__session` cookies) produce `VerificationOutcome::Valid(auth)`. The
/// layer is **non-rejecting** for missing or invalid credentials: it records
/// the outcome and lets the request continue so downstream code can decide
/// how to handle anonymous requests.
///
/// # Restricting accepted tokens
///
/// With the default configuration, **any** JWT signed by your Clerk instance
/// key verifies, including tokens minted from Clerk JWT templates for
/// third-party integrations, which legitimately reach browsers. If your
/// instance uses JWT templates, configure
/// [`ClerkAuthLayerConfig::with_issuers`] and
/// [`ClerkAuthLayerConfig::with_authorized_parties`] (and audiences where
/// applicable) via [`ClerkAuthLayer::from_config`] so integration tokens
/// cannot pass as session tokens.
///
/// # Verification model and limitations
///
/// Verification is **stateless**: each request is checked purely against the
/// cached JWKS signing keys, with no call back to Clerk to consult live session
/// state. Two consequences follow, both inherent to networkless JWT
/// verification and matching Clerk's own backend model:
///
/// - **No revocation window.** A token whose session has since been signed out
///   or revoked stays accepted until its `exp`. Clerk session tokens are
///   short-lived (about a minute), so the exposure is bounded by that lifetime
///   rather than by revocation. Gate anything that must react to revocation
///   immediately on a fresh check rather than on a still-valid token.
/// - **Key-rotation lag.** A token signed with a `kid` not in the cached JWKS
///   triggers at most one refresh per unknown-kid refresh interval (5 minutes);
///   within that window an unknown `kid` is rejected as invalid. Clerk
///   pre-publishes new keys before signing with them, so this affects only
///   rotations faster than the refresh floor, and the floor exists to keep an
///   attacker from forcing unbounded JWKS refetches.
///
/// # `worker` feature
///
/// With the `worker` feature (server on wasm), the service future is wrapped
/// in `SendWrapper` to satisfy Axum's `Send` bound. This assumes a
/// single-threaded runtime such as Cloudflare Workers: the future must be
/// polled and dropped on the thread that created it, and doing otherwise
/// panics deterministically.
#[derive(Clone)]
pub struct ClerkAuthLayer {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for ClerkAuthLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClerkAuthLayer").finish_non_exhaustive()
    }
}

struct Inner {
    verifier: Verifier,
}

impl ClerkAuthLayer {
    /// Build a layer that verifies tokens against Clerk's live JWKS using
    /// the given backend secret key. The JWKS is fetched lazily on the
    /// first valid-looking request and cached in memory.
    ///
    /// Uses the default configuration; see the type-level docs on
    /// [restricting accepted tokens](ClerkAuthLayer#restricting-accepted-tokens)
    /// when the Clerk instance mints JWT-template tokens.
    pub fn new(secret_key: impl Into<String>) -> Result<Self, crate::core::ClerkError> {
        Self::from_config(ClerkAuthLayerConfig::new(secret_key))
    }

    /// Build a layer from the conventional `CLERK_SECRET_KEY` environment variable.
    ///
    /// Uses the default configuration; see the type-level docs on
    /// [restricting accepted tokens](ClerkAuthLayer#restricting-accepted-tokens)
    /// when the Clerk instance mints JWT-template tokens.
    pub fn from_env() -> Result<Self, crate::core::ClerkError> {
        Self::from_config(ClerkAuthLayerConfig::from_env()?)
    }

    /// Build a layer from owned verifier configuration.
    ///
    /// Use this when an application needs to override Clerk backend settings or
    /// enable optional claim validation.
    pub fn from_config(config: ClerkAuthLayerConfig) -> Result<Self, crate::core::ClerkError> {
        let verifier = Verifier::new(config)?;
        Ok(Self {
            inner: Arc::new(Inner { verifier }),
        })
    }
}

impl<S> Layer<S> for ClerkAuthLayer {
    type Service = ClerkAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ClerkAuthService {
            inner,
            layer: self.clone(),
        }
    }
}

/// Tower service produced by [`ClerkAuthLayer`].
#[derive(Clone)]
pub struct ClerkAuthService<S> {
    inner: S,
    layer: ClerkAuthLayer,
}

impl<S: std::fmt::Debug> std::fmt::Debug for ClerkAuthService<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClerkAuthService")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl<S> Service<Request<Body>> for ClerkAuthService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + MaybeSend + 'static,
    S::Future: MaybeSend + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = ServiceFuture<S::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let layer = self.layer.clone();
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        service_future(Box::pin(async move {
            match verify_request(req, layer.inner.verifier.clone()).await {
                VerifiedRequest::Forward(req) => inner.call(req).await,
                VerifiedRequest::Unavailable(response) => Ok(response),
            }
        }))
    }
}
