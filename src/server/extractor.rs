//! Axum extractors for regular HTTP handlers.

use axum::extract::{FromRequestParts, OptionalFromRequestParts};
use axum::http::{StatusCode, request::Parts};
use axum::response::{IntoResponse, Response};

use crate::core::{ClerkAuth, VerificationOutcome};

/// Rejection returned when extracting [`ClerkAuth`] from an Axum request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthRejection {
    /// The request did not include valid auth credentials.
    Missing,
    /// Credentials were present but failed verification.
    Invalid,
    /// Verification infrastructure was unavailable.
    Unavailable,
}

impl AuthRejection {
    /// HTTP status this rejection maps to: `401 Unauthorized` for
    /// [`Missing`](Self::Missing)/[`Invalid`](Self::Invalid),
    /// `503 Service Unavailable` for [`Unavailable`](Self::Unavailable).
    ///
    /// Exposed so callers building a custom [`IntoResponse`] (a JSON error
    /// body, a redirect) can reuse the mapping instead of re-deriving it.
    pub fn status_code(self) -> StatusCode {
        match self {
            Self::Missing | Self::Invalid => StatusCode::UNAUTHORIZED,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// Short, non-sensitive reason phrase used as the default response body.
    pub fn message(self) -> &'static str {
        match self {
            Self::Missing => "unauthenticated",
            Self::Invalid => "invalid Clerk credentials",
            Self::Unavailable => "Clerk verification unavailable",
        }
    }
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        (self.status_code(), self.message()).into_response()
    }
}

impl<S> FromRequestParts<S> for ClerkAuth
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match parts.extensions.get::<VerificationOutcome>() {
            Some(VerificationOutcome::Valid(auth)) => Ok(auth.clone()),
            Some(VerificationOutcome::Invalid(_)) => Err(AuthRejection::Invalid),
            Some(VerificationOutcome::Unavailable) => Err(AuthRejection::Unavailable),
            Some(VerificationOutcome::Missing) => Err(AuthRejection::Missing),
            None => {
                log_missing_layer();
                Err(AuthRejection::Missing)
            }
        }
    }
}

/// Optional extraction still fails closed on `Unavailable`.
///
/// A missing or invalid credential yields `Ok(None)` (the anonymous case an
/// optional extractor exists to model), but a JWKS/verification outage yields
/// `Err(AuthRejection::Unavailable)` → 503 rather than `None`.
/// `Unavailable` must never be silently downgraded to anonymous: doing so would
/// let a Clerk outage quietly strip a signed-in user of their auth. So an
/// otherwise-public route using `Option<ClerkAuth>` will 503 during an outage.
impl<S> OptionalFromRequestParts<S> for ClerkAuth
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        match parts.extensions.get::<VerificationOutcome>() {
            Some(VerificationOutcome::Valid(auth)) => Ok(Some(auth.clone())),
            Some(VerificationOutcome::Unavailable) => Err(AuthRejection::Unavailable),
            Some(VerificationOutcome::Missing | VerificationOutcome::Invalid(_)) => Ok(None),
            None => {
                log_missing_layer();
                Ok(None)
            }
        }
    }
}

/// A request with no `VerificationOutcome` extension means no
/// [`crate::server::ClerkAuthLayer`] ran for this route. The layer inserts an
/// outcome on every request it forwards, so this almost always indicates a
/// forgotten `.layer(...)`, which would otherwise be indistinguishable from
/// "everyone is anonymous". Warn once so the misconfiguration is visible in
/// production without flooding the log on every request. Shared with the
/// server-function context readers, which face the same hazard.
pub(super) fn log_missing_layer() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static WARNED: AtomicBool = AtomicBool::new(false);

    if WARNED.swap(true, Ordering::Relaxed) {
        tracing::debug!(
            "no Clerk verification outcome on request; is ClerkAuthLayer installed on this route?"
        );
    } else {
        tracing::warn!(
            "no Clerk verification outcome on request; is ClerkAuthLayer installed on this route?"
        );
    }
}
