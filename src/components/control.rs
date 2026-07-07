use crate::context::use_clerk_context;
use dioxus::prelude::*;

/// Renders children while clerk-js is still loading and no initialization
/// error has been reported.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn AuthStatus() -> Element {
///     rsx! {
///         ClerkLoading { p { "Loading auth..." } }
///         ClerkLoaded { p { "Auth ready." } }
///         ClerkFailed { p { "Auth failed to initialize." } }
///     }
/// }
/// ```
#[component]
pub fn ClerkLoading(children: Element) -> Element {
    let ctx = use_clerk_context();
    let auth = ctx.auth.read();
    let load_error = ctx.load_error.read();

    if !auth.is_loaded() && load_error.is_none() {
        rsx! { {children} }
    } else {
        rsx! {}
    }
}

/// Renders children once clerk-js has loaded.
///
/// Gates only on loadedness: transient scheduled-action failures (and
/// non-fatal configuration warnings) never unmount a loaded subtree. Use
/// [`ClerkFailed`] to surface errors.
#[component]
pub fn ClerkLoaded(children: Element) -> Element {
    let ctx = use_clerk_context();
    let auth = ctx.auth.read();

    if auth.is_loaded() {
        rsx! { {children} }
    } else {
        rsx! {}
    }
}

/// Renders children when Clerk initialization has failed and cannot recover
/// (missing publishable key, clerk-js script failure, `Clerk.load()`
/// rejection). Read [`crate::use_clerk_error`] to display the concrete error.
///
/// Scheduled-action failures (e.g. a rejected `open_sign_in()`) and non-fatal
/// configuration warnings (e.g. an SSR seed publishable-key mismatch) do not
/// render this component; read [`crate::use_clerk_error`] to handle those
/// locally.
#[component]
pub fn ClerkFailed(children: Element) -> Element {
    let ctx = use_clerk_context();
    let load_error = ctx.load_error.read();

    if load_error.is_some() {
        rsx! { {children} }
    } else {
        rsx! {}
    }
}
