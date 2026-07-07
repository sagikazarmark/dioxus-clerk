//! Server-side credential verification outcome shared across crates.

use super::ClerkAuth;

/// Result of checking request credentials before handlers or server functions run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "a verification outcome carries the auth decision; dropping it silently skips the check"]
#[non_exhaustive]
pub enum VerificationOutcome {
    /// No bearer credentials or session cookie were present on the request.
    Missing,
    /// Credentials were present and verified.
    Valid(ClerkAuth),
    /// Credentials were present but invalid.
    Invalid(InvalidTokenReason),
    /// Verification infrastructure was unavailable. Server middleware fails closed.
    Unavailable,
}

/// Why a presented token failed verification.
///
/// Only reasons an application can meaningfully act on are distinguished;
/// everything else (malformed token, bad signature, claim mismatch) is
/// [`InvalidTokenReason::Other`] so failure details never leak to clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InvalidTokenReason {
    /// The token is past its `exp` claim; the client should refresh its session.
    Expired,
    /// The token's `nbf`/`iat` claim is in the future beyond the accepted clock skew.
    NotYetValid,
    /// Any other verification failure.
    Other,
}

impl VerificationOutcome {
    /// Returns verified auth claims only for a valid outcome.
    #[must_use]
    pub fn auth(&self) -> Option<&ClerkAuth> {
        match self {
            Self::Valid(auth) => Some(auth),
            _ => None,
        }
    }

    /// Consumes the outcome, returning verified auth only for a valid outcome.
    #[must_use]
    pub fn into_auth(self) -> Option<ClerkAuth> {
        match self {
            Self::Valid(auth) => Some(auth),
            _ => None,
        }
    }
}
