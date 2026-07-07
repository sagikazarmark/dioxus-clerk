//! Advanced shared auth types, errors, and verification outcomes.

mod auth_state;
mod claims;
mod error;
mod reverification;
mod state;
mod verification;

pub use auth_state::{AuthRequirement, AuthState, AuthStatus};
pub use claims::ClerkAuth;
pub use error::ClerkError;
pub use reverification::{OtherReverificationLevel, ReverificationLevel};
#[cfg(clerk_client)]
pub(crate) use state::AuthObservation;
pub(crate) use state::AuthRuntimeState;
pub use state::{
    OtherStatus, OtherTaskKey, Session, SessionStatus, SessionTask, SessionTaskKey, User,
};
pub use verification::{InvalidTokenReason, VerificationOutcome};
