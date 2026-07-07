use super::AuthButtonMode;
use crate::options::SignUpOptions;
use dioxus::prelude::*;

/// Unstyled sign-up button that schedules the appropriate Clerk action.
///
/// This component renders a native `<button>` and treats `children` as button
/// contents. For design-system buttons that own their own DOM element, call
/// [`crate::use_clerk`] from the button's click handler instead.
///
/// Props cover the options Clerk's modal and redirect flows accept. Embedded
/// UI options such as `routing`/`path` apply only to the mounted [`SignUp`]
/// component, not to a button, so they are not exposed here.
///
/// [`SignUp`]: crate::SignUp
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn SignUpAction() -> Element {
///     rsx! {
///         SignUpButton { class: "btn btn-secondary", "Create account" }
///     }
/// }
/// ```
#[component]
pub fn SignUpButton(
    /// Whether to open a modal or redirect. Defaults to redirect for Clerk React parity.
    #[props(default)]
    mode: AuthButtonMode,
    /// Full URL or path to the sign-in page used by Clerk's sign-up flow.
    #[props(into)]
    sign_in_url: Option<String>,
    /// Full URL or path to the waitlist page used by Clerk's sign-up flow.
    #[props(into)]
    waitlist_url: Option<String>,
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
    /// Raw Clerk initialValues object.
    initial_values: Option<serde_json::Value>,
    /// Raw Clerk appearance object.
    appearance: Option<serde_json::Value>,
    /// Advanced options forwarded to Clerk, as a
    /// [`SignUpOptions`](crate::SignUpOptions) builder or a raw
    /// `serde_json::Value`. Explicit props win when both set the same Clerk
    /// option key.
    #[props(default = SignUpOptions::from_value(serde_json::Value::Null), into)]
    options: SignUpOptions,
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
    /// Optional custom button contents. Defaults to `Sign up`.
    #[props(default = rsx! { "Sign up" })]
    children: Element,
) -> Element {
    let options = options
        .maybe_sign_in_url(sign_in_url)
        .maybe_waitlist_url(waitlist_url)
        .maybe_force_redirect_url(force_redirect_url)
        .maybe_fallback_redirect_url(fallback_redirect_url)
        .maybe_sign_in_force_redirect_url(sign_in_force_redirect_url)
        .maybe_sign_in_fallback_redirect_url(sign_in_fallback_redirect_url)
        .maybe_initial_values(initial_values)
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
            AuthButtonMode::Modal => clerk.open_sign_up_with_options(options.clone()),
            AuthButtonMode::Redirect => clerk.redirect_to_sign_up_with_options(options.clone()),
        },
    )
}
