//! Provider runtime auth state plus typed mirrors of clerk-js User/Session shapes.

use super::{AuthState, AuthStatus};
use crate::ssr::{InitialAuthSnapshot, InitialAuthStatus};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Reactive auth state machine held by `ClerkProvider`.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AuthRuntimeState {
    status: AuthRuntimeStateKind,
    is_loaded: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(not(clerk_client), allow(dead_code))]
enum AuthRuntimeStateKind {
    /// Clerk has not resolved yet; there is no reliable auth answer.
    Loading,
    /// Auth resolved; no active session.
    SignedOut,
    /// Server verified a session during SSR, but clerk-js has not supplied
    /// full `User` and `Session` objects on the client yet.
    SignedInSnapshot {
        /// Clerk user id from verified server auth.
        user_id: String,
        /// Active session id from verified server auth, if present.
        session_id: Option<String>,
        /// Active organization id from verified server auth, if present.
        org_id: Option<String>,
        /// Active organization slug from verified server auth, if present.
        org_slug: Option<String>,
        /// Organization role from verified server auth, if present.
        org_role: Option<String>,
        /// Organization permissions from verified server auth.
        org_permissions: Vec<String>,
    },
    /// Clerk JS loaded; an active session exists with full client details.
    SignedIn {
        /// Authenticated user.
        user: User,
        /// Active session.
        session: Session,
        /// Active organization id if it came from verified server auth.
        org_id: Option<String>,
        /// Active organization slug if it came from verified server auth.
        org_slug: Option<String>,
        /// Organization role if it came from verified server auth.
        org_role: Option<String>,
        /// Organization permissions if they came from verified server auth.
        org_permissions: Vec<String>,
    },
    /// Clerk JS loaded a signed-in session that is not yet fully active: it has
    /// pending after-auth tasks (`status == "pending"`). Treated as signed-out
    /// for rendering gates (clerk-js `treatPendingAsSignedOut` default), but the
    /// session and its `current_task` are exposed for after-auth routing.
    Pending {
        /// Authenticated user.
        user: User,
        /// Pending session, carrying `current_task`/`tasks`.
        session: Session,
    },
}

/// A clerk-js observation that can transition application-visible auth state.
// One short-lived value per clerk-js event that is consumed immediately;
// boxing the signed-in payload would trade a transient stack-size difference
// for a per-event allocation.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(not(clerk_client), allow(dead_code))]
pub(crate) enum AuthObservation {
    /// Clerk has not supplied a reliable auth answer.
    Loading,
    /// Clerk resolved with no active session.
    SignedOut,
    /// Clerk resolved with full client-side user and session details.
    SignedIn {
        /// Authenticated user.
        user: User,
        /// Active session.
        session: Session,
    },
    /// Clerk resolved a signed-in session with pending after-auth tasks
    /// (`status == "pending"`). Treated as signed-out for gates, but the
    /// session's `current_task` is exposed for after-auth routing.
    Pending {
        /// Authenticated user.
        user: User,
        /// Pending session, carrying `current_task`/`tasks`.
        session: Session,
    },
}

impl Default for AuthRuntimeState {
    fn default() -> Self {
        Self::loading()
    }
}

#[cfg_attr(not(clerk_client), allow(dead_code))]
impl AuthRuntimeState {
    /// Initial unknown state before clerk-js has loaded.
    pub fn loading() -> Self {
        Self::with_status(AuthRuntimeStateKind::Loading, false)
    }

    fn with_status(status: AuthRuntimeStateKind, is_loaded: bool) -> Self {
        Self { status, is_loaded }
    }

    fn signed_out_with_loaded(is_loaded: bool) -> Self {
        Self::with_status(AuthRuntimeStateKind::SignedOut, is_loaded)
    }

    fn with_loaded(mut self, is_loaded: bool) -> Self {
        self.is_loaded = is_loaded;
        self
    }

    /// Interpret an SSR initial auth snapshot as initial client Auth state.
    ///
    /// Only a server-verified signed-out snapshot seeds a resolved signed-out
    /// state; an unverified snapshot stays loading so a render that never
    /// checked the session cannot flash signed-out UI at a signed-in user.
    /// A signed-in snapshot with a missing or empty user id is malformed and
    /// stays loading too: an empty user id cannot represent a signed-in
    /// session, matching [`AuthState::signed_in`].
    pub fn from_initial_auth_snapshot(snapshot: &InitialAuthSnapshot) -> Self {
        match snapshot.status {
            InitialAuthStatus::Unverified => return Self::loading(),
            InitialAuthStatus::SignedOut => return Self::signed_out_with_loaded(false),
            InitialAuthStatus::SignedIn => {}
        }

        let Some(user_id) = snapshot.user_id.clone().filter(|id| !id.is_empty()) else {
            return Self::loading();
        };

        Self::with_status(
            AuthRuntimeStateKind::SignedInSnapshot {
                user_id,
                session_id: snapshot.session_id.clone(),
                org_id: snapshot.org_id.clone(),
                org_slug: snapshot.org_slug.clone(),
                org_role: snapshot.org_role.clone(),
                org_permissions: snapshot.org_permissions.clone(),
            },
            false,
        )
    }

    /// Build full signed-in state from clerk-js details while preserving
    /// verified organization claims across same-session updates.
    #[cfg(test)]
    pub fn from_js_session(user: User, session: Session, previous: &Self) -> Self {
        Self::from_js_session_with_loaded(user, session, previous, true)
    }

    /// Whether verified org claims from `previous` may be carried into a fresh
    /// clerk-js observation. The org must match too: clerk-js switches the
    /// active organization (`setActive({ organization })`) without changing
    /// the session id, and stale role/permission claims must not keep
    /// satisfying org gates for the wrong organization.
    fn matches_user_session_org(
        previous_user_id: &str,
        previous_session_id: Option<&str>,
        previous_org_id: Option<&str>,
        user: &User,
        session: &Session,
    ) -> bool {
        previous_user_id == user.id
            && previous_session_id == Some(session.id.as_str())
            && previous_org_id == session.last_active_organization_id.as_deref()
    }

    fn from_js_session_with_loaded(
        user: User,
        session: Session,
        previous: &Self,
        is_loaded: bool,
    ) -> Self {
        let (org_id, org_slug, org_role, org_permissions) = match &previous.status {
            AuthRuntimeStateKind::SignedInSnapshot {
                user_id,
                session_id,
                org_id,
                org_slug,
                org_role,
                org_permissions,
            } if Self::matches_user_session_org(
                user_id,
                session_id.as_deref(),
                org_id.as_deref(),
                &user,
                &session,
            ) =>
            {
                (
                    org_id.clone(),
                    org_slug.clone(),
                    org_role.clone(),
                    org_permissions.clone(),
                )
            }
            AuthRuntimeStateKind::SignedIn {
                user: previous_user,
                session: previous_session,
                org_id,
                org_slug,
                org_role,
                org_permissions,
            } if Self::matches_user_session_org(
                &previous_user.id,
                Some(previous_session.id.as_str()),
                org_id.as_deref(),
                &user,
                &session,
            ) =>
            {
                (
                    org_id.clone(),
                    org_slug.clone(),
                    org_role.clone(),
                    org_permissions.clone(),
                )
            }
            _ => (None, None, None, vec![]),
        };
        Self::with_status(
            AuthRuntimeStateKind::SignedIn {
                user,
                session,
                org_id,
                org_slug,
                org_role,
                org_permissions,
            },
            is_loaded,
        )
    }

    /// Apply a clerk-js observation while preserving current loadedness.
    pub fn apply_observation(&self, observation: AuthObservation) -> Self {
        match observation {
            // A transient interim event must not flash loading UI at a user who
            // already has a resolved signed-in or pending session.
            AuthObservation::Loading if self.is_signed_in() || self.is_pending() => self.clone(),
            AuthObservation::Loading => {
                Self::with_status(AuthRuntimeStateKind::Loading, self.is_loaded)
            }
            AuthObservation::SignedOut => Self::signed_out_with_loaded(self.is_loaded),
            AuthObservation::SignedIn { user, session } => {
                Self::from_js_session_with_loaded(user, session, self, self.is_loaded)
            }
            AuthObservation::Pending { user, session } => Self::with_status(
                AuthRuntimeStateKind::Pending { user, session },
                self.is_loaded,
            ),
        }
    }

    /// Apply an observation produced after `Clerk.load()` has completed.
    pub fn apply_loaded_observation(&self, observation: AuthObservation) -> Self {
        self.apply_observation(observation).with_loaded(true)
    }

    /// Convert the current runtime state into app-visible auth state.
    pub fn to_state(&self) -> AuthState {
        match &self.status {
            AuthRuntimeStateKind::Loading => AuthState {
                status: AuthStatus::Loading,
                is_loaded: self.is_loaded,
                ..AuthState::signed_out()
            },
            AuthRuntimeStateKind::SignedOut => AuthState {
                status: AuthStatus::SignedOut,
                is_loaded: self.is_loaded,
                ..AuthState::signed_out()
            },
            AuthRuntimeStateKind::SignedInSnapshot {
                user_id,
                session_id,
                org_id,
                org_slug,
                org_role,
                org_permissions,
            } => AuthState {
                status: AuthStatus::SignedIn,
                is_loaded: self.is_loaded,
                user_id: Some(user_id.clone()),
                session_id: session_id.clone(),
                org_id: org_id.clone(),
                org_slug: org_slug.clone(),
                org_role: org_role.clone(),
                org_permissions: org_permissions.clone(),
            },
            AuthRuntimeStateKind::SignedIn {
                user,
                session,
                org_id,
                org_slug,
                org_role,
                org_permissions,
            } => AuthState {
                status: AuthStatus::SignedIn,
                is_loaded: self.is_loaded,
                user_id: Some(user.id.clone()),
                session_id: Some(session.id.clone()),
                org_id: org_id.clone(),
                org_slug: org_slug.clone(),
                org_role: org_role.clone(),
                org_permissions: org_permissions.clone(),
            },
            // A pending session is treated as signed-out for the app-visible
            // auth answer; the pending session itself is read through
            // `session()` / `use_session()`, not the flat auth state.
            AuthRuntimeStateKind::Pending { .. } => AuthState {
                status: AuthStatus::SignedOut,
                is_loaded: self.is_loaded,
                ..AuthState::signed_out()
            },
        }
    }

    /// Whether clerk-js has completed `Clerk.load()` for this auth state.
    pub fn is_loaded(&self) -> bool {
        self.is_loaded
    }

    /// True when the state represents either server-snapshot or full JS
    /// signed-in knowledge. A pending session is not signed-in: it is treated
    /// as signed-out until its after-auth tasks resolve.
    pub fn is_signed_in(&self) -> bool {
        matches!(
            &self.status,
            AuthRuntimeStateKind::SignedInSnapshot { .. } | AuthRuntimeStateKind::SignedIn { .. }
        )
    }

    /// True when clerk-js reports a signed-in session with pending after-auth
    /// tasks. Such a session gates as signed-out but is exposed for routing.
    pub fn is_pending(&self) -> bool {
        matches!(&self.status, AuthRuntimeStateKind::Pending { .. })
    }

    /// Apply `treat_pending_as_signed_out` to produce the effective state a
    /// reader sees, mirroring clerk-js `resolveAuthState`.
    ///
    /// With the flag on (clerk-js's default), a pending session is left as-is
    /// and every gate reads it as signed-out. With the flag off, the pending
    /// session is read as its underlying signed-in state, so `SignedIn` /
    /// `Protect` render and `use_auth().is_signed_in()` is true. Non-pending
    /// states, and the flag-on case, borrow `self` without cloning.
    ///
    /// Role/permission gates still fail closed for a resolved pending session:
    /// it carries no server-verified org claims (its org id comes only from the
    /// live clerk-js session), matching the "gates fail closed without claims"
    /// contract of [`AuthRuntimeState::allows_signed_in_gate`].
    pub fn resolve_pending(&self, treat_pending_as_signed_out: bool) -> Cow<'_, Self> {
        match &self.status {
            AuthRuntimeStateKind::Pending { user, session } if !treat_pending_as_signed_out => {
                Cow::Owned(Self::with_status(
                    AuthRuntimeStateKind::SignedIn {
                        org_id: session.last_active_organization_id.clone(),
                        org_slug: None,
                        org_role: None,
                        org_permissions: Vec::new(),
                        user: user.clone(),
                        session: session.clone(),
                    },
                    self.is_loaded,
                ))
            }
            _ => Cow::Borrowed(self),
        }
    }

    /// Whether `SignedIn` rendering should be shown for this state.
    pub fn should_render_signed_in(&self) -> bool {
        self.is_signed_in()
    }

    /// Whether `SignedOut` rendering should be shown for this state. A pending
    /// session renders signed-out, matching clerk-js's `treatPendingAsSignedOut`
    /// default.
    pub fn should_render_signed_out(&self) -> bool {
        matches!(
            &self.status,
            AuthRuntimeStateKind::SignedOut | AuthRuntimeStateKind::Pending { .. }
        )
    }

    /// Full clerk-js user details, available only after browser hydration.
    pub fn user(&self) -> Option<&User> {
        match &self.status {
            AuthRuntimeStateKind::SignedIn { user, .. }
            | AuthRuntimeStateKind::Pending { user, .. } => Some(user),
            AuthRuntimeStateKind::Loading
            | AuthRuntimeStateKind::SignedOut
            | AuthRuntimeStateKind::SignedInSnapshot { .. } => None,
        }
    }

    /// Full clerk-js session details, available only after browser hydration.
    /// This includes a pending session, so callers can read its `current_task`
    /// for after-auth routing.
    pub fn session(&self) -> Option<&Session> {
        match &self.status {
            AuthRuntimeStateKind::SignedIn { session, .. }
            | AuthRuntimeStateKind::Pending { session, .. } => Some(session),
            AuthRuntimeStateKind::Loading
            | AuthRuntimeStateKind::SignedOut
            | AuthRuntimeStateKind::SignedInSnapshot { .. } => None,
        }
    }

    /// Whether signed-in gating should render for this state and gate request.
    ///
    /// Matching Clerk React, `permission` takes precedence when both gates are
    /// requested; `role` is ignored in that case.
    pub fn allows_signed_in_gate(&self, role: Option<&str>, permission: Option<&str>) -> bool {
        if !self.should_render_signed_in() {
            return false;
        }

        let role = if permission.is_some() { None } else { role };

        if role.is_none() && permission.is_none() {
            return true;
        }

        let (org_role, org_permissions) = match &self.status {
            AuthRuntimeStateKind::SignedInSnapshot {
                org_role,
                org_permissions,
                ..
            }
            | AuthRuntimeStateKind::SignedIn {
                org_role,
                org_permissions,
                ..
            } => (org_role.as_deref(), org_permissions.as_slice()),
            // Unreachable: `should_render_signed_in()` already returned false
            // for these, but the match must stay exhaustive.
            AuthRuntimeStateKind::Loading
            | AuthRuntimeStateKind::SignedOut
            | AuthRuntimeStateKind::Pending { .. } => (None, &[][..]),
        };
        let role_allowed = match role {
            Some(required) => org_role == Some(required),
            None => true,
        };
        let permission_allowed = match permission {
            Some(required) => org_permissions.iter().any(|actual| actual == required),
            None => true,
        };

        role_allowed && permission_allowed
    }
}

/// Typed mirror of the fields this crate reads from a Clerk JS `User`. Unknown
/// fields are ignored on deserialize. Use `dioxus_clerk::use_clerk()` for
/// supported browser actions, or application JS for fields not mirrored here.
///
/// Construct values with [`User::new`] and set optional fields directly; the
/// struct is `#[non_exhaustive]` so new clerk-js fields can be mirrored
/// without a breaking release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct User {
    /// Clerk user id, e.g. `user_2abc`.
    pub id: String,
    /// First name.
    #[serde(default, alias = "firstName")]
    pub first_name: Option<String>,
    /// Last name.
    #[serde(default, alias = "lastName")]
    pub last_name: Option<String>,
    /// Primary email address, serialized string-form.
    ///
    /// Deserialization also accepts clerk-js's object form
    /// (`{ "emailAddress": "..." }`), extracting the address string.
    #[serde(
        default,
        alias = "primaryEmailAddress",
        deserialize_with = "deserialize_primary_email_address"
    )]
    pub primary_email_address: Option<String>,
    /// Avatar URL.
    #[serde(default, alias = "imageUrl")]
    pub image_url: Option<String>,
}

impl User {
    /// Creates a user with the given Clerk user id and all optional fields
    /// unset. Set remaining fields directly.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            first_name: None,
            last_name: None,
            primary_email_address: None,
            image_url: None,
        }
    }
}

/// Extract the address string from clerk-js's `primaryEmailAddress`, which is
/// either an already-flattened string or an `EmailAddress` object.
fn deserialize_primary_email_address<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum PrimaryEmailAddress {
        Address(String),
        Object(EmailAddressObject),
    }

    #[derive(Deserialize)]
    struct EmailAddressObject {
        #[serde(default, alias = "emailAddress")]
        email_address: Option<String>,
    }

    Ok(
        match Option::<PrimaryEmailAddress>::deserialize(deserializer)? {
            None => None,
            Some(PrimaryEmailAddress::Address(address)) => Some(address),
            Some(PrimaryEmailAddress::Object(object)) => object.email_address,
        },
    )
}

/// Session status as reported by clerk-js.
///
/// Serializes as the raw clerk-js status string. The enum is
/// `#[non_exhaustive]`; statuses this crate has not named yet round-trip
/// through [`SessionStatus::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
#[non_exhaustive]
pub enum SessionStatus {
    /// The session is valid and the user is signed in.
    Active,
    /// The user abandoned the auth flow before the session became active.
    Abandoned,
    /// The session ended, for example by signing out of it.
    Ended,
    /// The session expired.
    Expired,
    /// The session requires additional steps (for example MFA) before it is
    /// active.
    Pending,
    /// The session was removed.
    Removed,
    /// The session was replaced by a newer session.
    Replaced,
    /// The session was revoked.
    Revoked,
    /// A status string this crate has not named yet.
    ///
    /// Only produced by the `From<&str>`/`From<String>`/`FromStr` conversions,
    /// which canonicalize known strings to their named variants first. The
    /// payload is an [`OtherStatus`] with no public constructor, so an `Other`
    /// can never alias a named variant (e.g. hold `"active"`): reads through
    /// [`SessionStatus::as_str`] and comparisons therefore stay consistent.
    Other(OtherStatus),
}

/// A clerk-js session status string with no named [`SessionStatus`] variant.
///
/// Obtained by matching on [`SessionStatus::Other`]; read the raw string with
/// [`OtherStatus::as_str`]. It has no public constructor: values are produced
/// only by [`SessionStatus`]'s `From`/`FromStr` conversions, which canonicalize
/// known strings first, so an `OtherStatus` never holds a value a named variant
/// would represent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OtherStatus(String);

impl OtherStatus {
    /// The raw clerk-js status string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OtherStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl SessionStatus {
    /// The raw clerk-js status string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Abandoned => "abandoned",
            Self::Ended => "ended",
            Self::Expired => "expired",
            Self::Pending => "pending",
            Self::Removed => "removed",
            Self::Replaced => "replaced",
            Self::Revoked => "revoked",
            Self::Other(status) => status.as_str(),
        }
    }
}

impl SessionStatus {
    fn from_known(status: &str) -> Option<Self> {
        Some(match status {
            "active" => Self::Active,
            "abandoned" => Self::Abandoned,
            "ended" => Self::Ended,
            "expired" => Self::Expired,
            "pending" => Self::Pending,
            "removed" => Self::Removed,
            "replaced" => Self::Replaced,
            "revoked" => Self::Revoked,
            _ => return None,
        })
    }
}

impl From<&str> for SessionStatus {
    fn from(status: &str) -> Self {
        Self::from_known(status).unwrap_or_else(|| Self::Other(OtherStatus(status.to_owned())))
    }
}

impl From<String> for SessionStatus {
    fn from(status: String) -> Self {
        // Move the owned buffer into `Other` instead of re-allocating it.
        Self::from_known(&status).unwrap_or(Self::Other(OtherStatus(status)))
    }
}

impl From<SessionStatus> for String {
    fn from(status: SessionStatus) -> Self {
        status.as_str().to_owned()
    }
}

impl std::str::FromStr for SessionStatus {
    type Err = std::convert::Infallible;

    fn from_str(status: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(status))
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A clerk-js v6 post-authentication session task the user must complete
/// before the session becomes fully active, such as MFA enrollment or
/// organization selection.
///
/// The struct is `#[non_exhaustive]`; construct values with [`SessionTask::new`]
/// so new clerk-js task fields can be mirrored without a breaking release.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SessionTask {
    /// The task identifier, as reported by clerk-js.
    pub key: SessionTaskKey,
}

impl SessionTask {
    /// Creates a session task for the given key.
    pub fn new(key: impl Into<SessionTaskKey>) -> Self {
        Self { key: key.into() }
    }
}

/// A clerk-js v6 session task key.
///
/// Serializes as the raw clerk-js task-key string. The enum is
/// `#[non_exhaustive]`; keys this crate has not named yet round-trip through
/// [`SessionTaskKey::Other`], mirroring [`SessionStatus`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
#[non_exhaustive]
pub enum SessionTaskKey {
    /// The user must choose or create an organization.
    ChooseOrganization,
    /// The user must reset their password.
    ResetPassword,
    /// The user must enroll in multi-factor authentication.
    SetupMfa,
    /// A task-key string this crate has not named yet.
    ///
    /// Only produced by the `From<&str>`/`From<String>` conversions, which
    /// canonicalize known keys to their named variants first. The payload is an
    /// [`OtherTaskKey`] with no public constructor, so an `Other` can never
    /// alias a named variant.
    Other(OtherTaskKey),
}

/// A clerk-js session task-key string with no named [`SessionTaskKey`] variant.
///
/// Obtained by matching on [`SessionTaskKey::Other`]; read the raw string with
/// [`OtherTaskKey::as_str`]. It has no public constructor, so an `OtherTaskKey`
/// never holds a value a named variant would represent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OtherTaskKey(String);

impl OtherTaskKey {
    /// The raw clerk-js task-key string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OtherTaskKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl SessionTaskKey {
    /// The raw clerk-js task-key string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::ChooseOrganization => "choose-organization",
            Self::ResetPassword => "reset-password",
            Self::SetupMfa => "setup-mfa",
            Self::Other(key) => key.as_str(),
        }
    }

    fn from_known(key: &str) -> Option<Self> {
        Some(match key {
            "choose-organization" => Self::ChooseOrganization,
            "reset-password" => Self::ResetPassword,
            "setup-mfa" => Self::SetupMfa,
            _ => return None,
        })
    }
}

impl From<&str> for SessionTaskKey {
    fn from(key: &str) -> Self {
        Self::from_known(key).unwrap_or_else(|| Self::Other(OtherTaskKey(key.to_owned())))
    }
}

impl From<String> for SessionTaskKey {
    fn from(key: String) -> Self {
        // Move the owned buffer into `Other` instead of re-allocating it.
        Self::from_known(&key).unwrap_or(Self::Other(OtherTaskKey(key)))
    }
}

impl From<SessionTaskKey> for String {
    fn from(key: SessionTaskKey) -> Self {
        key.as_str().to_owned()
    }
}

impl std::str::FromStr for SessionTaskKey {
    type Err = std::convert::Infallible;

    fn from_str(key: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(key))
    }
}

impl std::fmt::Display for SessionTaskKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Typed mirror of the fields this crate reads from a Clerk JS `Session`.
///
/// Construct values with [`Session::new`] and set optional fields directly;
/// the struct is `#[non_exhaustive]` so new clerk-js fields can be mirrored
/// without a breaking release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Session {
    /// Session id, e.g. `sess_2def`.
    pub id: String,
    /// Session status, as reported by clerk-js.
    pub status: SessionStatus,
    /// Active organization id for this session, as reported by clerk-js, if
    /// any. Used to invalidate server-verified org claims when the active
    /// organization changes within a session.
    #[serde(default, alias = "lastActiveOrganizationId")]
    pub last_active_organization_id: Option<String>,
    /// Last activity (unix ms).
    #[serde(default, alias = "lastActiveAt")]
    pub last_active_at: Option<i64>,
    /// Expiry (unix ms).
    #[serde(default, alias = "expireAt")]
    pub expire_at: Option<i64>,
    /// The current pending session task, if the session is not yet fully
    /// active. A signed-in-but-pending session (`status == "pending"`) carries
    /// one; apps route on it to complete the after-auth flow.
    #[serde(default, alias = "currentTask")]
    pub current_task: Option<SessionTask>,
    /// All pending session tasks, in clerk-js order. clerk-js reports
    /// `Array<SessionTask> | null`; a `null` reads as no tasks.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub tasks: Vec<SessionTask>,
}

/// Deserialize a possibly-`null` value into its `Default`, so clerk-js's
/// `tasks: Array<SessionTask> | null` reads a `null` as an empty list rather
/// than a deserialize error.
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

impl Session {
    /// Creates a session with the given id and status and all optional fields
    /// unset. Set remaining fields directly.
    pub fn new(id: impl Into<String>, status: impl Into<SessionStatus>) -> Self {
        Self {
            id: id.into(),
            status: status.into(),
            last_active_organization_id: None,
            last_active_at: None,
            expire_at: None,
            current_task: None,
            tasks: Vec::new(),
        }
    }

    /// True when clerk-js reports the session as active.
    pub fn is_active(&self) -> bool {
        self.status == SessionStatus::Active
    }
}

#[cfg(test)]
mod tests;
