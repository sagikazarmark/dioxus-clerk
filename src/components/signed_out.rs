use crate::context::use_clerk_context;
use dioxus::prelude::*;

/// Render children once auth has resolved to signed out.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn PublicNav() -> Element {
///     rsx! {
///         SignedOut { SignInButton {} }
///     }
/// }
/// ```
#[component]
pub fn SignedOut(
    /// Whether a session with pending after-auth tasks is treated as signed
    /// out. clerk-js's default is `true`; set `false` to stop rendering
    /// children for a pending session. Mirrors Clerk React's
    /// `treatPendingAsSignedOut`.
    #[props(default = true)]
    treat_pending_as_signed_out: bool,
    children: Element,
) -> Element {
    let ctx = use_clerk_context();
    let auth = ctx.auth.read();
    if auth
        .resolve_pending(treat_pending_as_signed_out)
        .should_render_signed_out()
    {
        rsx! { {children} }
    } else {
        rsx! {}
    }
}

/// Render children only after clerk-js has loaded and auth resolved to signed out.
///
/// This is useful in fullstack/SSR apps where the server can render an
/// anonymous auth snapshot before clerk-js has finished resolving the browser
/// session. Use `fallback` for the interim loading state and render
/// [`crate::ClerkFailed`] or [`crate::use_clerk_error`] nearby for failures.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn PublicNav() -> Element {
///     rsx! {
///         SignedOutWhenLoaded {
///             fallback: rsx! { span { "Checking auth..." } },
///             SignInButton {}
///         }
///     }
/// }
/// ```
#[component]
pub fn SignedOutWhenLoaded(
    /// Rendered while clerk-js is loading and no signed-in session is known.
    #[props(default = rsx! {})]
    fallback: Element,
    /// Whether a session with pending after-auth tasks is treated as signed
    /// out. See [`SignedOut`].
    #[props(default = true)]
    treat_pending_as_signed_out: bool,
    children: Element,
) -> Element {
    let ctx = use_clerk_context();
    let auth = ctx.auth.read();
    let auth = auth.resolve_pending(treat_pending_as_signed_out);
    let load_error = ctx.load_error.read();

    if auth.is_loaded() && auth.should_render_signed_out() {
        rsx! { {children} }
    } else if !auth.is_loaded() && !auth.is_signed_in() && load_error.is_none() {
        fallback
    } else {
        rsx! {}
    }
}
