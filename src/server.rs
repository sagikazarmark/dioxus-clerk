//! Server-side glue for Clerk in Dioxus fullstack apps.

mod claims;
mod config;
mod context;
mod extractor;
mod layer;
pub mod ssr;
mod verification;

pub use crate::core::{ClerkAuth, InvalidTokenReason, VerificationOutcome};
pub use config::ClerkAuthLayerConfig;
pub use context::{current_auth, current_auth_opt, current_outcome};
pub use extractor::AuthRejection;
pub use layer::{ClerkAuthLayer, ClerkAuthService};
