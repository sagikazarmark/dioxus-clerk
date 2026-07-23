//! Verified Clerk session claims.

use serde::{Deserialize, Serialize};

/// Verified claims from a Clerk session JWT.
///
/// Mirrors the Clerk session JWT fields needed by server functions and client
/// SSR initial state rendering. Organization claims are flattened so downstream code can
/// read `org_id`/`org_role`/`org_permissions` directly.
///
/// With the `server` feature enabled, this can be used directly as an Axum
/// extractor. Use `Option<ClerkAuth>` for handlers that allow anonymous users.
///
/// Construct values with [`ClerkAuth::new`] (which takes the required `exp`)
/// and set optional fields directly; the struct is `#[non_exhaustive]` so new
/// Clerk claims can be mirrored without a breaking release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClerkAuth {
    /// Clerk user id (subject).
    pub user_id: String,
    /// Active session id.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Active organization id (when the session is org-scoped).
    #[serde(default)]
    pub org_id: Option<String>,
    /// Active organization slug, if Clerk provided one.
    #[serde(default)]
    pub org_slug: Option<String>,
    /// Organization role in Clerk's `org:<role>` form (e.g. `"org:admin"`).
    #[serde(default)]
    pub org_role: Option<String>,
    /// Custom organization permissions in Clerk's
    /// `org:<feature>:<permission>` form (e.g. `"org:invoices:create"`).
    #[serde(default)]
    pub org_permissions: Vec<String>,
    /// Expiry (unix seconds).
    pub exp: i64,
    /// Not-before (unix seconds).
    pub nbf: i64,
    /// Issued-at (unix seconds).
    pub iat: i64,
}

impl ClerkAuth {
    /// Creates claims for the given Clerk user id and expiry (`exp`, unix
    /// seconds), with all optional claims unset. Set remaining fields directly.
    ///
    /// `exp` is required rather than defaulted because a zeroed expiry is
    /// semantically already-expired: taking it here keeps a custom verifier
    /// (one producing `VerificationOutcome::Valid`) from silently minting an
    /// expired-looking auth. `nbf`/`iat` default to `0` (valid-from-epoch,
    /// which is harmless as carried metadata); set them to the verified
    /// token's real timestamps when they matter.
    pub fn new(user_id: impl Into<String>, exp: i64) -> Self {
        Self {
            user_id: user_id.into(),
            session_id: None,
            org_id: None,
            org_slug: None,
            org_role: None,
            org_permissions: vec![],
            exp,
            nbf: 0,
            iat: 0,
        }
    }

    /// True if the claims include the given permission.
    pub fn has_permission(&self, permission: &str) -> bool {
        self.org_permissions.iter().any(|p| p == permission)
    }

    /// True if the claims indicate the given org role.
    pub fn has_role(&self, role: &str) -> bool {
        self.org_role.as_deref() == Some(role)
    }

    /// True if these verified claims satisfy an authorization requirement.
    ///
    /// Mirrors [`crate::AuthState::has`] so server handlers holding a verified
    /// [`ClerkAuth`] gate on the same
    /// [`AuthRequirement`](crate::AuthRequirement) vocabulary as
    /// client-side rendering. Verified claims always represent a signed-in
    /// user, so [`AuthRequirement::SignedIn`](crate::AuthRequirement::SignedIn)
    /// is always satisfied.
    pub fn has(&self, requirement: &super::AuthRequirement) -> bool {
        use super::AuthRequirement;
        match requirement {
            AuthRequirement::SignedIn => true,
            AuthRequirement::Role(role) => self.has_role(role),
            AuthRequirement::Permission(permission) => self.has_permission(permission),
        }
    }
}
