use crate::options::RedirectOptions;
use dioxus::prelude::*;

/// Redirect signed-out users to Clerk's sign-in flow after clerk-js loads.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn ProtectedPage() -> Element {
///     rsx! {
///         SignedOut { RedirectToSignIn {} }
///         SignedIn { h1 { "Dashboard" } }
///     }
/// }
/// ```
#[component]
pub fn RedirectToSignIn(
    /// Always redirect here after sign-in.
    #[props(into)]
    force_redirect_url: Option<String>,
    /// Fallback redirect URL after sign-in.
    #[props(into)]
    fallback_redirect_url: Option<String>,
    /// Always redirect here after sign-up from sign-in.
    #[props(into)]
    sign_up_force_redirect_url: Option<String>,
    /// Fallback redirect URL after sign-up from sign-in.
    #[props(into)]
    sign_up_fallback_redirect_url: Option<String>,
    /// Advanced options forwarded to Clerk, as a
    /// [`RedirectOptions`](crate::RedirectOptions) builder or a raw
    /// `serde_json::Value`. Explicit props win when both set the same Clerk
    /// option key.
    #[props(default = RedirectOptions::from_value(serde_json::Value::Null), into)]
    options: RedirectOptions,
) -> Element {
    let options = options
        .maybe_force_redirect_url(force_redirect_url)
        .maybe_fallback_redirect_url(fallback_redirect_url)
        .maybe_sign_up_force_redirect_url(sign_up_force_redirect_url)
        .maybe_sign_up_fallback_redirect_url(sign_up_fallback_redirect_url)
        .into_value();

    #[cfg(clerk_client)]
    crate::lifecycle::use_redirect_to_sign_in(options.clone());
    #[cfg(not(clerk_client))]
    let _ = &options;
    rsx! {}
}

/// Redirect signed-out users to Clerk's sign-up flow after clerk-js loads.
#[component]
pub fn RedirectToSignUp(
    /// Always redirect here after sign-up.
    #[props(into)]
    force_redirect_url: Option<String>,
    /// Fallback redirect URL after sign-up.
    #[props(into)]
    fallback_redirect_url: Option<String>,
    /// Always redirect here after sign-in from sign-up.
    #[props(into)]
    sign_in_force_redirect_url: Option<String>,
    /// Fallback redirect URL after sign-in from sign-up.
    #[props(into)]
    sign_in_fallback_redirect_url: Option<String>,
    /// Advanced options forwarded to Clerk, as a
    /// [`RedirectOptions`](crate::RedirectOptions) builder or a raw
    /// `serde_json::Value`. Explicit props win when both set the same Clerk
    /// option key.
    #[props(default = RedirectOptions::from_value(serde_json::Value::Null), into)]
    options: RedirectOptions,
) -> Element {
    let options = options
        .maybe_force_redirect_url(force_redirect_url)
        .maybe_fallback_redirect_url(fallback_redirect_url)
        .maybe_sign_in_force_redirect_url(sign_in_force_redirect_url)
        .maybe_sign_in_fallback_redirect_url(sign_in_fallback_redirect_url)
        .into_value();

    #[cfg(clerk_client)]
    crate::lifecycle::use_redirect_to_sign_up(options.clone());
    #[cfg(not(clerk_client))]
    let _ = &options;
    rsx! {}
}
