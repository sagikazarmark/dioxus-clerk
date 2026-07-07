use crate::context::use_clerk_context;
use dioxus::prelude::*;

/// Render children when a signed-in session is known.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn AccountNav() -> Element {
///     rsx! {
///         SignedIn { UserButton {} }
///     }
/// }
/// ```
#[component]
pub fn SignedIn(
    /// Whether a session with pending after-auth tasks is treated as signed
    /// out. clerk-js's default is `true`; set `false` to render children for a
    /// pending session (for example to host its task UI). Mirrors Clerk React's
    /// `treatPendingAsSignedOut`.
    #[props(default = true)]
    treat_pending_as_signed_out: bool,
    children: Element,
) -> Element {
    let ctx = use_clerk_context();
    let auth = ctx.auth.read();
    if auth
        .resolve_pending(treat_pending_as_signed_out)
        .should_render_signed_in()
    {
        rsx! { {children} }
    } else {
        rsx! {}
    }
}

/// Render children only after clerk-js has loaded and auth resolved to signed in.
///
/// The loading-aware counterpart to [`SignedIn`], mirroring
/// [`crate::SignedOutWhenLoaded`]. This is useful in fullstack/SSR apps that
/// render before clerk-js has finished resolving the browser session: use
/// `fallback` for the interim (for example an avatar skeleton) and render
/// [`crate::ClerkFailed`] or [`crate::use_clerk_error`] nearby for failures.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn AccountNav() -> Element {
///     rsx! {
///         SignedInWhenLoaded {
///             fallback: rsx! { span { "Checking auth..." } },
///             UserButton {}
///         }
///     }
/// }
/// ```
#[component]
pub fn SignedInWhenLoaded(
    /// Rendered while clerk-js is loading and no resolved signed-out state is known.
    #[props(default = rsx! {})]
    fallback: Element,
    /// Whether a session with pending after-auth tasks is treated as signed
    /// out. See [`SignedIn`].
    #[props(default = true)]
    treat_pending_as_signed_out: bool,
    children: Element,
) -> Element {
    let ctx = use_clerk_context();
    let auth = ctx.auth.read();
    let auth = auth.resolve_pending(treat_pending_as_signed_out);
    let load_error = ctx.load_error.read();

    if auth.is_loaded() && auth.should_render_signed_in() {
        rsx! { {children} }
    } else if !auth.is_loaded() && !auth.should_render_signed_out() && load_error.is_none() {
        fallback
    } else {
        rsx! {}
    }
}
