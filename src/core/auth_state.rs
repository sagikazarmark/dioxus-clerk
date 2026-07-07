//! Auth state: the app-visible authentication knowledge consumed by
//! components and hooks.

use super::claims::ClerkAuth;
use super::error::ClerkError;

/// App-visible authentication state for components and hooks.
///
/// [`status`](AuthState::status) is the single source of truth for the
/// signed-in answer; signed-in checks derive from it so contradictory states
/// cannot be represented. The fields are private so `status` and `user_id`
/// cannot be mutated out of step with each other — read them through the
/// accessors ([`status`](AuthState::status), [`user_id`](AuthState::user_id),
/// …) and build values through the constructors and `with_*` setters. SSR
/// initial state carries `crate::ssr::InitialAuthSnapshot` instead. Browser
/// loadedness is client lifecycle state, so server-rendered auth snapshots do
/// not set `is_loaded`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AuthState {
    pub(crate) status: AuthStatus,
    pub(crate) is_loaded: bool,
    pub(crate) user_id: Option<String>,
    pub(crate) session_id: Option<String>,
    // Org claims come from verified server auth. A client-side org switch
    // (`setActive`) does not update `org_id` until the server re-verifies.
    pub(crate) org_id: Option<String>,
    pub(crate) org_slug: Option<String>,
    pub(crate) org_role: Option<String>,
    pub(crate) org_permissions: Vec<String>,
}

/// Resolved authentication status for app-facing rendering decisions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AuthStatus {
    /// Clerk has not supplied a reliable auth answer yet.
    #[default]
    Loading,
    /// Clerk has resolved and no active session is known.
    SignedOut,
    /// A signed-in session is known, either from SSR initial state or clerk-js.
    SignedIn,
}

impl AuthStatus {
    /// True while auth has not resolved to signed-in or signed-out yet.
    pub fn is_loading(self) -> bool {
        matches!(self, Self::Loading)
    }

    /// True only when auth has resolved and no active session is known.
    pub fn is_signed_out(self) -> bool {
        matches!(self, Self::SignedOut)
    }

    /// True when a signed-in session is known.
    pub fn is_signed_in(self) -> bool {
        matches!(self, Self::SignedIn)
    }
}

/// Client-side rendering authorization check.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthRequirement {
    /// Require any signed-in session.
    SignedIn,
    /// Require a server-verified organization role.
    Role(String),
    /// Require a server-verified organization permission.
    Permission(String),
}

impl AuthRequirement {
    /// Require any signed-in session.
    pub fn signed_in() -> Self {
        Self::SignedIn
    }

    /// Require a server-verified organization role.
    pub fn role(role: impl Into<String>) -> Self {
        Self::Role(role.into())
    }

    /// Require a server-verified organization permission.
    pub fn permission(permission: impl Into<String>) -> Self {
        Self::Permission(permission.into())
    }
}

impl AuthState {
    /// Auth state before Clerk has supplied a reliable answer.
    pub fn loading() -> Self {
        Self {
            status: AuthStatus::Loading,
            ..Self::signed_out()
        }
    }

    /// Auth state representing a resolved signed-out session.
    ///
    /// `is_loaded` stays `false`: browser loadedness is client lifecycle
    /// state, not part of the auth answer. Set it explicitly when modeling a
    /// client where clerk-js has finished loading.
    pub fn signed_out() -> Self {
        Self {
            status: AuthStatus::SignedOut,
            is_loaded: false,
            user_id: None,
            session_id: None,
            org_id: None,
            org_slug: None,
            org_role: None,
            org_permissions: vec![],
        }
    }

    /// Auth state representing a signed-in session for the given user id.
    /// Chain the `with_*` setters to add optional session/org fields (and
    /// `is_loaded`).
    ///
    /// An empty user id cannot represent a signed-in session and produces
    /// [`AuthState::signed_out`], matching `From<&ClerkAuth>`.
    pub fn signed_in(user_id: impl Into<String>) -> Self {
        let user_id = user_id.into();
        if user_id.is_empty() {
            return Self::signed_out();
        }

        Self {
            status: AuthStatus::SignedIn,
            user_id: Some(user_id),
            ..Self::signed_out()
        }
    }

    /// Set whether clerk-js has finished loading on the client.
    #[must_use]
    pub fn with_loaded(mut self, is_loaded: bool) -> Self {
        self.is_loaded = is_loaded;
        self
    }

    /// Set the active session id.
    #[must_use]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set the active organization id.
    #[must_use]
    pub fn with_org_id(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    /// Set the active organization slug.
    #[must_use]
    pub fn with_org_slug(mut self, org_slug: impl Into<String>) -> Self {
        self.org_slug = Some(org_slug.into());
        self
    }

    /// Set the organization role from verified server auth.
    #[must_use]
    pub fn with_org_role(mut self, org_role: impl Into<String>) -> Self {
        self.org_role = Some(org_role.into());
        self
    }

    /// Set the organization permissions from verified server auth.
    #[must_use]
    pub fn with_org_permissions(
        mut self,
        org_permissions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.org_permissions = org_permissions.into_iter().map(Into::into).collect();
        self
    }

    /// Explicit auth resolution status — the single source of truth for the
    /// signed-in answer.
    pub fn status(&self) -> AuthStatus {
        self.status
    }

    /// Whether clerk-js has finished loading on the client.
    pub fn is_loaded(&self) -> bool {
        self.is_loaded
    }

    /// Clerk user id, if signed in.
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Active session id, if any.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Org id from verified server auth, if any.
    pub fn org_id(&self) -> Option<&str> {
        self.org_id.as_deref()
    }

    /// Org slug from verified server auth, if any.
    pub fn org_slug(&self) -> Option<&str> {
        self.org_slug.as_deref()
    }

    /// Org role from verified server auth, if any.
    pub fn org_role(&self) -> Option<&str> {
        self.org_role.as_deref()
    }

    /// Org permissions from verified server auth.
    pub fn org_permissions(&self) -> &[String] {
        &self.org_permissions
    }

    /// True when a signed-in session is known.
    pub fn is_signed_in(&self) -> bool {
        self.status.is_signed_in()
    }

    /// True while auth has not resolved to signed-in or signed-out yet.
    pub fn is_loading(&self) -> bool {
        self.status.is_loading()
    }

    /// True only when auth has resolved and no active session is known;
    /// `false` while auth is still loading.
    pub fn is_signed_out(&self) -> bool {
        self.status.is_signed_out()
    }

    /// Return the user id for signed-in auth state.
    ///
    /// Preserves the loading/signed-out distinction the rest of the API keeps:
    /// [`ClerkError::NotLoaded`] while auth has not resolved yet (retry or
    /// wait), [`ClerkError::Unauthenticated`] only once auth has resolved
    /// without a signed-in user (redirect to sign-in).
    pub fn require_signed_in(&self) -> Result<&str, ClerkError> {
        if self.status.is_loading() {
            return Err(ClerkError::NotLoaded);
        }
        if !self.status.is_signed_in() {
            return Err(ClerkError::Unauthenticated);
        }

        self.user_id.as_deref().ok_or(ClerkError::Unauthenticated)
    }

    /// True if the auth state includes the given server-verified org role.
    pub fn has_role(&self, role: &str) -> bool {
        self.org_role.as_deref() == Some(role)
    }

    /// True if the auth state includes the given server-verified org permission.
    pub fn has_permission(&self, permission: &str) -> bool {
        self.org_permissions.iter().any(|p| p == permission)
    }

    /// True if the auth state satisfies a rendering auth requirement.
    ///
    /// Role and permission requirements imply a signed-in session; they are
    /// never satisfied by a loading or signed-out state.
    pub fn has(&self, requirement: &AuthRequirement) -> bool {
        if !self.is_signed_in() {
            return false;
        }
        match requirement {
            AuthRequirement::SignedIn => true,
            AuthRequirement::Role(role) => self.has_role(role),
            AuthRequirement::Permission(permission) => self.has_permission(permission),
        }
    }
}

/// Converts server-verified claims into app-visible auth state.
///
/// `is_loaded` stays `false`: these claims come from the server, where
/// clerk-js does not exist, so browser loadedness cannot be inferred.
impl From<ClerkAuth> for AuthState {
    fn from(c: ClerkAuth) -> Self {
        if c.user_id.is_empty() {
            return Self::signed_out();
        }

        Self {
            status: AuthStatus::SignedIn,
            is_loaded: false,
            user_id: Some(c.user_id),
            session_id: c.session_id,
            org_id: c.org_id,
            org_slug: c.org_slug,
            org_role: c.org_role,
            org_permissions: c.org_permissions,
        }
    }
}

/// Converts server-verified claims into app-visible auth state, cloning the
/// claim fields. See `From<ClerkAuth>`.
impl From<&ClerkAuth> for AuthState {
    fn from(c: &ClerkAuth) -> Self {
        Self::from(c.clone())
    }
}
