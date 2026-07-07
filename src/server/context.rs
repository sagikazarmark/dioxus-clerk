//! Server verification outcome reader for Dioxus fullstack task context.
//!
//! Inside a Dioxus server function, call [`current_auth`],
//! [`current_auth_opt`], or [`current_outcome`] to read the Server verification
//! outcome placed on the request by [`crate::server::ClerkAuthLayer`].
//! `ClerkError` converts into Dioxus' `ServerFnError`, so server functions can
//! use `current_auth()?` directly when returning `Result<_, ServerFnError>`.

use crate::core::{ClerkAuth, ClerkError, VerificationOutcome};
use axum::http::StatusCode;
use dioxus_fullstack_core::{FullstackContext, HttpError, ServerFnError};

/// Returns the current Server verification outcome, if a `FullstackContext`
/// exists and carries one.
pub fn current_outcome() -> Option<VerificationOutcome> {
    FullstackContext::current().and_then(|ctx| ctx.extension::<VerificationOutcome>())
}

/// Returns the verification outcome carried by the current request context,
/// or an error when no `FullstackContext` exists at all.
fn required_context_outcome() -> Result<Option<VerificationOutcome>, ClerkError> {
    let ctx = FullstackContext::current().ok_or(ClerkError::NoServerContext)?;
    let outcome = ctx.extension::<VerificationOutcome>();
    if outcome.is_none() {
        // A context without an outcome means no ClerkAuthLayer ran for this
        // request — otherwise indistinguishable from "everyone is anonymous".
        super::extractor::log_missing_layer();
    }
    Ok(outcome)
}

/// Returns verified auth from the current context, or `None` for anonymous,
/// invalid, or absent outcomes.
///
/// This is the optional counterpart to [`current_auth`]: an anonymous or
/// invalid request yields `Ok(None)` here rather than an error, so a handler
/// that serves both signed-in and anonymous callers can branch on the `Option`.
/// Both readers still fail closed on an unavailable verifier.
///
/// Errors with [`ClerkError::NoServerContext`] outside a server function or
/// SSR scope, and with [`ClerkError::JwksUnavailable`] when verification
/// infrastructure was unavailable.
pub fn current_auth_opt() -> Result<Option<ClerkAuth>, ClerkError> {
    match required_context_outcome()? {
        Some(VerificationOutcome::Unavailable) => Err(jwks_unavailable()),
        Some(VerificationOutcome::Valid(auth)) => Ok(Some(auth)),
        _ => Ok(None),
    }
}

/// Returns verified auth from the current context.
///
/// Errors with [`ClerkError::Unauthenticated`] for anonymous or invalid
/// credentials, [`ClerkError::TokenExpired`] when the presented token was past
/// its expiry, [`ClerkError::JwksUnavailable`] when verification
/// infrastructure was unavailable, and [`ClerkError::NoServerContext`] outside
/// a server function or SSR scope.
///
/// Only `Expired` maps to [`ClerkError::TokenExpired`]; other invalid-token
/// reasons (including `NotYetValid`) collapse into
/// [`ClerkError::Unauthenticated`], because expiry is the one case callers
/// can meaningfully act on (prompt a re-authentication).
pub fn current_auth() -> Result<ClerkAuth, ClerkError> {
    match required_context_outcome()? {
        Some(VerificationOutcome::Valid(auth)) => Ok(auth),
        Some(VerificationOutcome::Invalid(reason)) => Err(reason.into()),
        Some(VerificationOutcome::Unavailable) => Err(jwks_unavailable()),
        _ => Err(ClerkError::Unauthenticated),
    }
}

/// The outcome extension does not carry failure details; the verification
/// layer logs the underlying cause where the fetch fails.
fn jwks_unavailable() -> ClerkError {
    ClerkError::JwksUnavailable(
        "verification infrastructure was unavailable for this request".into(),
    )
}

impl From<ClerkError> for ServerFnError {
    fn from(value: ClerkError) -> Self {
        let status = clerk_error_status(&value);
        HttpError::new(status, value.to_string()).into()
    }
}

fn clerk_error_status(error: &ClerkError) -> StatusCode {
    match error {
        ClerkError::Unauthenticated | ClerkError::TokenExpired => StatusCode::UNAUTHORIZED,
        ClerkError::JwksUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
