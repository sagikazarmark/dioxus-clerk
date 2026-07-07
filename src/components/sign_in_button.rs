use super::AuthButtonMode;
use crate::options::SignInOptions;
use dioxus::prelude::*;

/// Unstyled sign-in button that schedules the appropriate Clerk action.
///
/// This component renders a native `<button>` and treats `children` as button
/// contents. For design-system buttons that own their own DOM element, call
/// [`crate::use_clerk`] from the button's click handler instead.
///
/// Props cover the options Clerk's modal and redirect flows accept. Embedded
/// UI options such as `routing`/`path` apply only to the mounted [`SignIn`]
/// component, not to a button, so they are not exposed here.
///
/// [`SignIn`]: crate::SignIn
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn SignInAction() -> Element {
///     rsx! {
///         SignInButton { class: "btn btn-primary", "Sign in" }
///     }
/// }
/// ```
#[component]
pub fn SignInButton(
    /// Whether to open a modal or redirect. Defaults to redirect for Clerk React parity.
    #[props(default)]
    mode: AuthButtonMode,
    /// Full URL or path to the sign-up page used by Clerk's sign-in flow.
    #[props(into)]
    sign_up_url: Option<String>,
    /// Full URL or path to the waitlist page used by Clerk's sign-in flow.
    #[props(into)]
    waitlist_url: Option<String>,
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
    /// Raw Clerk initialValues object.
    initial_values: Option<serde_json::Value>,
    /// Whether sign-in attempts can transfer to sign-up when Clerk supports it.
    transferable: Option<bool>,
    /// Raw Clerk appearance object.
    appearance: Option<serde_json::Value>,
    /// Advanced options forwarded to Clerk, as a
    /// [`SignInOptions`](crate::SignInOptions) builder or a raw
    /// `serde_json::Value`. Explicit props win when both set the same Clerk
    /// option key.
    #[props(default = SignInOptions::from_value(serde_json::Value::Null), into)]
    options: SignInOptions,
    /// Disable the generated `<button>`.
    #[props(default)]
    disabled: bool,
    /// Attributes spread onto the generated `<button>` (`id`, `class`,
    /// `title`, `aria-*`, `r#type`, ...). `r#type` defaults to `"button"` to
    /// avoid accidental form submits.
    #[props(extends = GlobalAttributes, extends = button)]
    attributes: Vec<Attribute>,
    /// Optional click handler, called before the Clerk action is scheduled
    /// (matching Clerk React's ordering).
    #[props(default, into)]
    onclick: Callback<MouseEvent>,
    /// Optional custom button contents. Defaults to `Sign in`.
    #[props(default = rsx! { "Sign in" })]
    children: Element,
) -> Element {
    let options = options
        .maybe_sign_up_url(sign_up_url)
        .maybe_waitlist_url(waitlist_url)
        .maybe_force_redirect_url(force_redirect_url)
        .maybe_fallback_redirect_url(fallback_redirect_url)
        .maybe_sign_up_force_redirect_url(sign_up_force_redirect_url)
        .maybe_sign_up_fallback_redirect_url(sign_up_fallback_redirect_url)
        .maybe_initial_values(initial_values)
        .maybe_transferable(transferable)
        .maybe_appearance(appearance)
        .into_value();

    let clerk = crate::use_clerk();
    super::button_host::button_host(
        super::button_host::ButtonChrome {
            attributes,
            disabled,
            onclick,
            children,
        },
        move || match mode {
            AuthButtonMode::Modal => clerk.open_sign_in_with_options(options.clone()),
            AuthButtonMode::Redirect => clerk.redirect_to_sign_in_with_options(options.clone()),
        },
    )
}
