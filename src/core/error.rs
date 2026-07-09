//! Unified error type for the dioxus-clerk family of crates.

use super::reverification::ReverificationLevel;
use super::verification::InvalidTokenReason;
use serde_json::Value;
use thiserror::Error;

/// Errors produced anywhere in the dioxus-clerk stack.
///
/// Variants store only owned strings (no `JsValue`, no transport-specific
/// error sources) so this crate stays target-neutral, and errors stay
/// `Clone + PartialEq + Eq` for signal storage and test assertions. This is a
/// deliberate design: causes are flattened into the message at the boundary
/// where they occur, and `Error::source()` is always `None`. Conversion impls
/// live in the consumer modules.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClerkError {
    /// Clerk-js has not finished loading yet.
    #[error("clerk has not finished loading")]
    NotLoaded,

    /// The browser Clerk lifecycle could not make progress before a deadline:
    /// a hung `Clerk.load()`, or a lifecycle that never started. Distinct from
    /// [`ClerkError::NotLoaded`], which is the transient still-loading state a
    /// caller can wait out.
    #[error("clerk lifecycle timed out: {0}")]
    Timeout(String),

    /// A browser-only Clerk action was awaited on a target where clerk-js can
    /// never load (server or native builds).
    #[error("clerk-js is not available on this build target")]
    UnsupportedTarget,

    /// The clerk-js script failed to load or did not become ready in time.
    #[error("clerk-js failed to load: {0}")]
    ScriptLoad(String),

    /// The browser is offline, so clerk-js could not fetch a fresh session
    /// token.
    ///
    /// clerk-js 6 throws `ClerkOfflineError` from `session.getToken()` in this
    /// case, where 5.x returned `null`. Surfaced as a distinct, transient
    /// variant so callers can retry or fall back to a cached token instead of
    /// treating it as a hard failure or a signed-out state.
    #[error("clerk is offline")]
    Offline,

    /// Request has no session/credentials.
    #[error("unauthenticated")]
    Unauthenticated,

    /// Session JWT is past its `exp` claim.
    #[error("session token expired")]
    TokenExpired,

    /// Server could not fetch or refresh JWKS from Clerk's Backend API.
    ///
    /// The message is intentionally coarse: this error's `Display` can reach
    /// HTTP responses. The verification layer logs the underlying cause at
    /// `warn` level via `tracing`.
    #[error("clerk jwks unavailable: {0}")]
    JwksUnavailable(String),

    /// A server context reader was called outside a server function or SSR scope.
    #[error(
        "no server context available; Clerk auth server context methods must be called inside a server function or SSR scope"
    )]
    NoServerContext,

    /// Configuration was invalid (missing key, malformed env, etc.).
    #[error("invalid clerk configuration: {0}")]
    InvalidConfig(String),

    /// A gated action needs step-up reverification: the user must re-assert a
    /// fresh authentication factor before it can proceed. Carries the required
    /// [`ReverificationLevel`] when clerk reported one.
    ///
    /// Consumed by the reverification hook to trigger a re-auth prompt and
    /// resume the action. Produced from either clerk reverification signal:
    ///
    /// - the server-side path: a gated `#[server]` action surfaces a 403
    ///   reverification *hint* (JSON), which
    ///   [`ClerkError::from_reverification_hint`] maps, recovering the level;
    /// - the client-side path: a direct clerk-js call *throws* a
    ///   `ClerkAPIResponseError` carrying the `session_reverification_required`
    ///   code, which a caller maps into this variant. The throw does not carry
    ///   the level, so that path yields `level: None`, matching
    ///   clerk-react's `useReverification`.
    #[error("reverification required")]
    NeedsReverification {
        /// The authentication-factor level the reverification requires, when
        /// clerk reported one.
        level: Option<ReverificationLevel>,
    },

    /// The user dismissed the step-up reverification prompt without completing
    /// it, so the gated action did not run. Mirrors clerk-js's
    /// `reverification_cancelled` runtime error.
    #[error("reverification cancelled")]
    ReverificationCancelled,

    /// JS interop failure (wasm-bindgen / clerk-js threw).
    #[error("clerk js error: {0}")]
    Js(String),
}

impl ClerkError {
    /// Recognize a clerk step-up reverification hint and map it to
    /// [`ClerkError::NeedsReverification`], carrying the required level.
    ///
    /// A gated `#[server]` action (or any caller reading a clerk API response as
    /// JSON) hits a 403 reverification hint of the shape emitted by clerk's
    /// `clerk_render_reverification` and recognized by `@clerk/shared`'s
    /// `isReverificationHint`:
    ///
    /// ```json
    /// { "clerk_error": {
    ///     "type": "forbidden",
    ///     "reason": "reverification-error",
    ///     "metadata": { "reverification": { "level": "second_factor" } } } }
    /// ```
    ///
    /// Returns `None` for any value that is not such a hint, so a caller can map
    /// only the reverification case and pass every other error through
    /// unchanged.
    pub fn from_reverification_hint(value: &Value) -> Option<Self> {
        let clerk_error = value.get("clerk_error")?;
        if clerk_error.get("type").and_then(Value::as_str) != Some("forbidden")
            || clerk_error.get("reason").and_then(Value::as_str) != Some("reverification-error")
        {
            return None;
        }

        let level = clerk_error
            .pointer("/metadata/reverification/level")
            .and_then(Value::as_str)
            .map(ReverificationLevel::from);

        Some(Self::NeedsReverification { level })
    }
}

/// Maps a token-verification failure reason to the error callers act on.
///
/// Only [`InvalidTokenReason::Expired`] maps to [`ClerkError::TokenExpired`];
/// other reasons (including `NotYetValid`) collapse into
/// [`ClerkError::Unauthenticated`], because expiry is the one case callers can
/// meaningfully act on (prompt a re-authentication).
impl From<InvalidTokenReason> for ClerkError {
    fn from(reason: InvalidTokenReason) -> Self {
        match reason {
            InvalidTokenReason::Expired => Self::TokenExpired,
            _ => Self::Unauthenticated,
        }
    }
}
