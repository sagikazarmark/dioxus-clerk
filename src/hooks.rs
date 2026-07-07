//! Reactive accessors for descendants of `ClerkProvider`.

use crate::context::{ClerkContext, use_clerk_context};
use crate::core::{AuthRequirement, AuthState, AuthStatus, ClerkError, Session, User};
use dioxus::prelude::*;

fn auth_state_signal(
    ctx: ClerkContext,
    treat_pending_as_signed_out: bool,
) -> ReadSignal<AuthState> {
    // `treat_pending_as_signed_out` is a plain value, not a signal, so track it
    // with `use_reactive` — otherwise a caller passing a changing flag would
    // keep reading the first render's value.
    use_memo(use_reactive(
        &treat_pending_as_signed_out,
        move |treat_pending_as_signed_out| {
            ctx.auth
                .read()
                .resolve_pending(treat_pending_as_signed_out)
                .to_state()
        },
    ))
    .into()
}

/// Reactive auth hook result returned by [`use_auth`].
#[derive(Clone, Copy)]
pub struct UseAuth {
    inner: ReadSignal<AuthState>,
    ctx: ClerkContext,
}

impl std::fmt::Debug for UseAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UseAuth")
            .field("state", &*self.inner.peek())
            .finish_non_exhaustive()
    }
}

impl UseAuth {
    /// Return the current app-visible auth state.
    pub fn state(&self) -> AuthState {
        self.inner.read().clone()
    }

    /// Return the underlying read-only Dioxus signal for advanced composition.
    pub fn signal(&self) -> ReadSignal<AuthState> {
        self.inner
    }

    /// Return the explicit auth resolution status.
    pub fn status(&self) -> AuthStatus {
        self.inner.read().status
    }

    /// True while auth has not resolved to signed-in or signed-out yet.
    pub fn is_loading(&self) -> bool {
        self.status().is_loading()
    }

    /// True only when auth has resolved and no active session is known.
    pub fn is_signed_out(&self) -> bool {
        self.status().is_signed_out()
    }

    /// Whether clerk-js has finished loading on the client.
    pub fn is_loaded(&self) -> bool {
        self.inner.read().is_loaded
    }

    /// Whether a session is active.
    pub fn is_signed_in(&self) -> bool {
        self.inner.read().is_signed_in()
    }

    /// Clerk user id, if signed in.
    pub fn user_id(&self) -> Option<String> {
        self.inner.read().user_id.clone()
    }

    /// Return the user id, or an error when auth is not signed in.
    ///
    /// Returns [`ClerkError::NotLoaded`] while auth is still resolving and
    /// [`ClerkError::Unauthenticated`] once it has resolved without a
    /// signed-in user, so callers can wait out the former and redirect on the
    /// latter without flashing sign-in UI at a signed-in user.
    pub fn require_signed_in(&self) -> Result<String, ClerkError> {
        self.inner.read().require_signed_in().map(ToOwned::to_owned)
    }

    /// Get the active session token from clerk-js.
    ///
    /// This mirrors Clerk React's `useAuth().getToken()` convenience. The call
    /// waits for the browser Clerk lifecycle to load before reading clerk-js.
    ///
    /// Returns `Err(`[`ClerkError::Offline`]`)` when the browser is offline
    /// (clerk-js 6 throws `ClerkOfflineError` here) — a transient condition
    /// callers can retry rather than treat as signed-out.
    pub async fn get_token(&self) -> Result<Option<String>, ClerkError> {
        self.get_token_with_options(serde_json::Value::Null).await
    }

    /// Get the active session token with Clerk `getToken(...)` options.
    pub async fn get_token_with_options(
        &self,
        options: impl Into<serde_json::Value>,
    ) -> Result<Option<String>, ClerkError> {
        ClerkActions { ctx: self.ctx }
            .get_token_with_options(options)
            .await
    }

    /// Active session id, if signed in and available.
    pub fn session_id(&self) -> Option<String> {
        self.inner.read().session_id.clone()
    }

    /// Active organization id, if any.
    pub fn org_id(&self) -> Option<String> {
        self.inner.read().org_id.clone()
    }

    /// Active organization slug, if any.
    pub fn org_slug(&self) -> Option<String> {
        self.inner.read().org_slug.clone()
    }

    /// Organization role from verified server auth, if any.
    pub fn org_role(&self) -> Option<String> {
        self.inner.read().org_role.clone()
    }

    /// Organization permissions from verified server auth.
    pub fn org_permissions(&self) -> Vec<String> {
        self.inner.read().org_permissions.clone()
    }

    /// True if the auth state includes the given server-verified org role.
    pub fn has_role(&self, role: &str) -> bool {
        self.inner.read().has_role(role)
    }

    /// True if the auth state includes the given server-verified org permission.
    pub fn has_permission(&self, permission: &str) -> bool {
        self.inner.read().has_permission(permission)
    }

    /// True if the auth state satisfies a rendering auth requirement.
    pub fn has(&self, requirement: &AuthRequirement) -> bool {
        self.inner.read().has(requirement)
    }

    /// Sign out after Clerk lifecycle loadedness.
    ///
    /// This mirrors Clerk React's `useAuth().signOut()` convenience. Failures
    /// from the scheduled browser action are surfaced through
    /// [`use_clerk_error`]. Use [`UseAuth::try_sign_out`] when the caller needs
    /// to await completion or handle errors locally.
    pub fn sign_out(&self) {
        self.sign_out_with_options(serde_json::Value::Null);
    }

    /// Sign out with Clerk `signOut(...)` options after Clerk lifecycle loadedness.
    pub fn sign_out_with_options(&self, options: impl Into<serde_json::Value>) {
        ClerkActions { ctx: self.ctx }.sign_out_with_options(options);
    }

    /// Sign out and return any lifecycle or clerk-js error.
    pub async fn try_sign_out(&self) -> Result<(), ClerkError> {
        self.try_sign_out_with_options(serde_json::Value::Null)
            .await
    }

    /// Sign out with Clerk `signOut(...)` options and return any error.
    pub async fn try_sign_out_with_options(
        &self,
        options: impl Into<serde_json::Value>,
    ) -> Result<(), ClerkError> {
        ClerkActions { ctx: self.ctx }
            .try_sign_out_with_options(options)
            .await
    }
}

/// Options for [`use_auth_with_options`], mirroring the option object Clerk
/// React's `useAuth(...)` accepts.
///
/// Construct with [`UseAuthOptions::new`] and chain setters; the struct is
/// `#[non_exhaustive]` so new options can be added without a breaking release.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct UseAuthOptions {
    treat_pending_as_signed_out: bool,
}

impl Default for UseAuthOptions {
    fn default() -> Self {
        // clerk-js treats pending sessions as signed out by default; matching
        // that here keeps `use_auth()` consistent with the control components.
        Self {
            treat_pending_as_signed_out: true,
        }
    }
}

impl UseAuthOptions {
    /// Options with clerk-js defaults (`treat_pending_as_signed_out = true`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether a session with pending after-auth tasks is treated as signed
    /// out. `true` (the default) matches clerk-js; `false` reads a pending
    /// session as signed in.
    pub fn treat_pending_as_signed_out(mut self, value: bool) -> Self {
        self.treat_pending_as_signed_out = value;
        self
    }
}

/// Read the current app-visible auth state.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn AccountActions() -> Element {
///     let auth = use_auth();
///
///     rsx! {
///         if auth.is_signed_in() {
///             button { onclick: move |_| auth.sign_out(), "Sign out" }
///         }
///     }
/// }
/// ```
pub fn use_auth() -> UseAuth {
    use_auth_with_options(UseAuthOptions::new())
}

/// Read the current app-visible auth state with explicit [`UseAuthOptions`].
///
/// Mirrors Clerk React's `useAuth({ treatPendingAsSignedOut })`: passing
/// `UseAuthOptions::new().treat_pending_as_signed_out(false)` makes
/// `is_signed_in()`, `status()`, and `has(...)` read a pending session as
/// signed in.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn PendingAware() -> Element {
///     let auth = use_auth_with_options(
///         UseAuthOptions::new().treat_pending_as_signed_out(false),
///     );
///
///     rsx! {
///         if auth.is_signed_in() {
///             p { "Signed in (pending tasks count as signed in here)." }
///         }
///     }
/// }
/// ```
pub fn use_auth_with_options(options: UseAuthOptions) -> UseAuth {
    let ctx = use_clerk_context();
    UseAuth {
        inner: auth_state_signal(ctx, options.treat_pending_as_signed_out),
        ctx,
    }
}

/// Generate the auth-resolution predicate set for resource state structs, so
/// the semantics of loading/signed-out/signed-in are stated once for every
/// state shape that carries an [`AuthStatus`].
macro_rules! status_predicates {
    ($($ty:ident),* $(,)?) => {$(
        impl $ty {
            /// The explicit auth resolution status.
            pub fn status(&self) -> AuthStatus {
                self.status
            }

            /// Whether clerk-js has finished loading on the client.
            pub fn is_loaded(&self) -> bool {
                self.is_loaded
            }

            /// True while auth has not resolved to signed-in or signed-out yet.
            pub fn is_loading(&self) -> bool {
                self.status.is_loading()
            }

            /// True only when auth has resolved and no active session is known.
            pub fn is_signed_out(&self) -> bool {
                self.status.is_signed_out()
            }

            /// Whether a session is active.
            pub fn is_signed_in(&self) -> bool {
                self.status.is_signed_in()
            }
        }
    )*};
}

/// Generate the shared accessor set for reactive hook results wrapping a
/// resource state signal. Resource-specific accessors (`user()`, `session()`)
/// stay hand-written next to each wrapper.
macro_rules! hook_state_accessors {
    ($($wrapper:ident => $state:ident, $doc:literal;)*) => {$(
        impl $wrapper {
            #[doc = concat!("Return the current ", $doc, " state.")]
            pub fn state(&self) -> $state {
                self.inner.read().clone()
            }

            /// Return the underlying read-only Dioxus signal for advanced composition.
            pub fn signal(&self) -> ReadSignal<$state> {
                self.inner
            }

            /// Return the explicit auth resolution status.
            pub fn status(&self) -> AuthStatus {
                self.inner.read().status
            }

            /// True while auth has not resolved to signed-in or signed-out yet.
            pub fn is_loading(&self) -> bool {
                self.status().is_loading()
            }

            /// True only when auth has resolved and no active session is known.
            pub fn is_signed_out(&self) -> bool {
                self.status().is_signed_out()
            }

            /// Whether clerk-js has finished loading on the client.
            pub fn is_loaded(&self) -> bool {
                self.inner.read().is_loaded
            }

            /// Whether a session is active.
            pub fn is_signed_in(&self) -> bool {
                self.inner.read().is_signed_in()
            }
        }

        impl std::fmt::Debug for $wrapper {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!($wrapper))
                    .field("state", &*self.inner.peek())
                    .finish_non_exhaustive()
            }
        }
    )*};
}

/// Generate a memoized resource state signal projecting Auth state plus one
/// resource read out of the provider-owned runtime state.
macro_rules! resource_state_signal {
    ($($name:ident, $state:ident { $field:ident };)*) => {$(
        fn $name() -> ReadSignal<$state> {
            let ctx = use_clerk_context();
            use_memo(move || {
                let state = ctx.auth.read();
                let auth = state.to_state();
                $state {
                    status: auth.status,
                    is_loaded: state.is_loaded(),
                    $field: state.$field().cloned(),
                }
            })
            .into()
        }
    )*};
}

/// Current-user state that distinguishes loading, signed-out,
/// and signed-in-without-full-browser-user states.
///
/// Fields are read through accessors ([`status`](UserState::status),
/// [`is_loaded`](UserState::is_loaded), [`user`](UserState::user)), mirroring
/// [`AuthState`] so every read-model snapshot in the
/// crate shares one convention.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct UserState {
    pub(crate) status: AuthStatus,
    pub(crate) is_loaded: bool,
    pub(crate) user: Option<User>,
}

impl UserState {
    /// Full clerk-js user details, available after browser hydration.
    pub fn user(&self) -> Option<&User> {
        self.user.as_ref()
    }
}

status_predicates!(UserState, SessionState);

resource_state_signal! {
    user_state_signal, UserState { user };
    session_state_signal, SessionState { session };
}

hook_state_accessors! {
    UseUser => UserState, "user";
    UseSession => SessionState, "session";
}

/// Reactive user hook result returned by [`use_user`].
#[derive(Clone, Copy)]
pub struct UseUser {
    inner: ReadSignal<UserState>,
}

impl UseUser {
    /// Full clerk-js user details, available after browser hydration.
    pub fn user(&self) -> Option<User> {
        self.inner.read().user.clone()
    }
}

/// Read the current user together with loadedness and signed-in status.
///
/// This mirrors Clerk React's `useUser()` shape more closely than a bare
/// `Option<User>` and should be preferred in app code.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn Greeting() -> Element {
///     let user = use_user();
///
///     if !user.is_loaded() {
///         return rsx! { "Loading..." };
///     }
///
///     rsx! {
///         if let Some(user) = user.user() {
///             p { "Hello {user.id}" }
///         }
///     }
/// }
/// ```
pub fn use_user() -> UseUser {
    UseUser {
        inner: user_state_signal(),
    }
}

/// Current-session state that distinguishes loading,
/// signed-out, and signed-in-without-full-browser-session states.
///
/// Fields are read through accessors ([`status`](SessionState::status),
/// [`is_loaded`](SessionState::is_loaded), [`session`](SessionState::session)),
/// mirroring [`AuthState`] so every read-model snapshot
/// in the crate shares one convention.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SessionState {
    pub(crate) status: AuthStatus,
    pub(crate) is_loaded: bool,
    pub(crate) session: Option<Session>,
}

impl SessionState {
    /// Full clerk-js session details, available after browser hydration.
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }
}

/// Reactive session hook result returned by [`use_session`].
#[derive(Clone, Copy)]
pub struct UseSession {
    inner: ReadSignal<SessionState>,
}

impl UseSession {
    /// Full clerk-js session details, available after browser hydration.
    pub fn session(&self) -> Option<Session> {
        self.inner.read().session.clone()
    }
}

/// Read the current session together with loadedness and signed-in status.
///
/// This mirrors Clerk React's `useSession()` shape more closely than a bare
/// `Option<Session>` and should be preferred in app code.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn SessionId() -> Element {
///     let session = use_session();
///
///     rsx! {
///         if let Some(session) = session.session() {
///             code { "{session.id}" }
///         }
///     }
/// }
/// ```
pub fn use_session() -> UseSession {
    UseSession {
        inner: session_state_signal(),
    }
}

/// Read the latest Clerk error, if initialization failed, a scheduled
/// browser action failed, or startup found a non-fatal configuration
/// problem (e.g. an SSR seed publishable-key mismatch). A fatal
/// initialization failure wins over the recoverable kinds.
pub fn use_clerk_error() -> ReadSignal<Option<ClerkError>> {
    let ctx = use_clerk_context();
    use_memo(move || ctx.current_error()).into()
}

/// Return a callback that clears the latest recoverable error (a
/// scheduled-action failure or a non-fatal startup configuration warning).
///
/// Fatal initialization failures are terminal for the provider instance —
/// nothing retries the load — so clearing them would only convert a visible
/// error into a silent forever-loading state. They are therefore not cleared
/// by this callback.
pub fn use_clear_clerk_error() -> Callback<()> {
    let ctx = use_clerk_context();
    use_callback(move |()| {
        let mut action_error = ctx.action_error;
        action_error.set(None);
    })
}

/// Lifecycle-aware browser actions for the current clerk-js singleton.
///
/// Actions scheduled through this type wait for Clerk lifecycle loadedness
/// before touching clerk-js.
///
/// Each operation comes in two forms. The plain method (e.g. `sign_out`) is
/// fire-and-forget: it schedules the action and returns immediately, so it
/// drops into an event handler, and any failure is surfaced through
/// [`use_clerk_error`]. The `try_`-prefixed method (e.g. `try_sign_out`) is
/// awaited and hands the [`ClerkError`] back to the caller, for when it needs
/// to sequence work or handle the error locally. Each also has a
/// `_with_options` variant taking Clerk options.
#[derive(Clone, Copy)]
pub struct ClerkActions {
    ctx: ClerkContext,
}

impl std::fmt::Debug for ClerkActions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClerkActions").finish_non_exhaustive()
    }
}

/// Generate the four public entry points for one Clerk operation that takes
/// options: fire-and-forget, fire-and-forget with options, awaited, and
/// awaited with options. Every body lands on the same two Clerk action
/// dispatch functions; the operation-to-JS mapping lives once in
/// `ClerkBridge::run`.
macro_rules! clerk_option_actions {
    ($($doc:literal:
        $name:ident, $with_options:ident,
        $try_name:ident, $try_with_options:ident => $op:ident;)*) => {$(
        #[doc = concat!($doc, " after Clerk lifecycle loadedness.")]
        pub fn $name(&self) {
            self.$with_options(serde_json::Value::Null);
        }

        #[doc = concat!($doc, " with options after Clerk lifecycle loadedness.")]
        pub fn $with_options(&self, options: impl Into<serde_json::Value>) {
            crate::actions::schedule(
                self.ctx,
                crate::actions::ClerkOperation::$op(options.into()),
            );
        }

        #[doc = concat!($doc, " and return any lifecycle or clerk-js error.")]
        pub async fn $try_name(&self) -> Result<(), ClerkError> {
            self.$try_with_options(serde_json::Value::Null).await
        }

        #[doc = concat!($doc, " with options and return any error.")]
        pub async fn $try_with_options(
            &self,
            options: impl Into<serde_json::Value>,
        ) -> Result<(), ClerkError> {
            crate::actions::try_run(
                self.ctx,
                crate::actions::ClerkOperation::$op(options.into()),
            )
            .await
        }
    )*};
}

/// Generate the two public entry points for one option-less Clerk operation.
macro_rules! clerk_plain_actions {
    ($($doc:literal: $name:ident, $try_name:ident => $op:ident;)*) => {$(
        #[doc = concat!($doc, " after Clerk lifecycle loadedness.")]
        pub fn $name(&self) {
            crate::actions::schedule(self.ctx, crate::actions::ClerkOperation::$op);
        }

        #[doc = concat!($doc, " and return any lifecycle or clerk-js error.")]
        pub async fn $try_name(&self) -> Result<(), ClerkError> {
            crate::actions::try_run(self.ctx, crate::actions::ClerkOperation::$op).await
        }
    )*};
}

impl ClerkActions {
    clerk_option_actions! {
        "Open the Clerk sign-in modal":
            open_sign_in, open_sign_in_with_options,
            try_open_sign_in, try_open_sign_in_with_options => OpenSignIn;
        "Open the Clerk sign-up modal":
            open_sign_up, open_sign_up_with_options,
            try_open_sign_up, try_open_sign_up_with_options => OpenSignUp;
        "Open the Clerk user-profile modal":
            open_user_profile, open_user_profile_with_options,
            try_open_user_profile, try_open_user_profile_with_options => OpenUserProfile;
        "Sign out":
            sign_out, sign_out_with_options,
            try_sign_out, try_sign_out_with_options => SignOut;
        "Redirect to Clerk sign-in":
            redirect_to_sign_in, redirect_to_sign_in_with_options,
            try_redirect_to_sign_in, try_redirect_to_sign_in_with_options => RedirectToSignIn;
        "Redirect to Clerk sign-up":
            redirect_to_sign_up, redirect_to_sign_up_with_options,
            try_redirect_to_sign_up, try_redirect_to_sign_up_with_options => RedirectToSignUp;
    }

    clerk_plain_actions! {
        "Close the Clerk sign-in modal": close_sign_in, try_close_sign_in => CloseSignIn;
        "Close the Clerk sign-up modal": close_sign_up, try_close_sign_up => CloseSignUp;
        "Close the Clerk user-profile modal":
            close_user_profile, try_close_user_profile => CloseUserProfile;
    }

    /// Get the active session token from clerk-js.
    pub async fn get_token(&self) -> Result<Option<String>, ClerkError> {
        self.get_token_with_options(serde_json::Value::Null).await
    }

    /// Get the active session token with Clerk `getToken(...)` options.
    pub async fn get_token_with_options(
        &self,
        options: impl Into<serde_json::Value>,
    ) -> Result<Option<String>, ClerkError> {
        #[cfg(clerk_client)]
        {
            let options = options.into();
            crate::lifecycle::run_async_bridge_action_after_loaded(
                self.ctx,
                move |bridge| async move { bridge.get_token(&options).await },
            )
            .await
        }
        #[cfg(not(clerk_client))]
        {
            let _ = options;
            Err(ClerkError::UnsupportedTarget)
        }
    }
}

/// Library-level browser actions for descendants of `ClerkProvider`.
///
/// # Example
///
/// Use this for design-system buttons that own their own DOM element:
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn CustomSignInButton() -> Element {
///     let clerk = use_clerk();
///
///     rsx! {
///         button {
///             class: "btn btn-primary",
///             onclick: move |_| clerk.open_sign_in(),
///             "Sign in"
///         }
///     }
/// }
/// ```
pub fn use_clerk() -> ClerkActions {
    ClerkActions {
        ctx: use_clerk_context(),
    }
}

#[cfg(test)]
mod tests {
    use super::UseAuthOptions;

    #[test]
    fn use_auth_options_default_treats_pending_as_signed_out() {
        // clerk-js's default is `true`; Rust's `bool::default()` is `false`, so
        // the default must be set explicitly or parity silently breaks.
        assert_eq!(
            UseAuthOptions::new(),
            UseAuthOptions::new().treat_pending_as_signed_out(true)
        );
        assert_ne!(
            UseAuthOptions::new(),
            UseAuthOptions::new().treat_pending_as_signed_out(false)
        );
    }
}
