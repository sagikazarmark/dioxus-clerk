use crate::options::SignOutOptions;
use dioxus::prelude::*;

/// Unstyled sign-out button that schedules `Clerk.signOut(...)`.
///
/// This component renders a native `<button>` and treats `children` as button
/// contents. For design-system buttons that own their own DOM element, call
/// [`crate::use_clerk`] from the button's click handler instead.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn SignOutAction() -> Element {
///     rsx! {
///         SignOutButton { class: "btn btn-ghost", redirect_url: "/", "Sign out" }
///     }
/// }
/// ```
#[component]
pub fn SignOutButton(
    /// Full URL or path to navigate to after sign-out.
    #[props(into)]
    redirect_url: Option<String>,
    /// Sign out a specific session id in multi-session applications.
    #[props(into)]
    session_id: Option<String>,
    /// Advanced options forwarded to Clerk, as a
    /// [`SignOutOptions`](crate::SignOutOptions) builder or a raw
    /// `serde_json::Value`. Explicit props win when both set the same Clerk
    /// option key.
    #[props(default = SignOutOptions::from_value(serde_json::Value::Null), into)]
    options: SignOutOptions,
    /// Disable the generated `<button>`.
    #[props(default)]
    disabled: bool,
    /// Attributes spread onto the generated `<button>` (`id`, `class`,
    /// `title`, `aria-*`, `r#type`, ...). `r#type` defaults to `"button"` to
    /// avoid accidental form submits.
    #[props(extends = GlobalAttributes, extends = button)]
    attributes: Vec<Attribute>,
    /// Optional click handler, called before the sign-out action is scheduled
    /// (matching Clerk React's ordering).
    #[props(default, into)]
    onclick: Callback<MouseEvent>,
    /// Optional custom button contents. Defaults to `Sign out`.
    #[props(default = rsx! { "Sign out" })]
    children: Element,
) -> Element {
    let options = options
        .maybe_redirect_url(redirect_url)
        .maybe_session_id(session_id)
        .into_value();

    let clerk = crate::use_clerk();
    super::button_host::button_host(
        super::button_host::ButtonChrome {
            attributes,
            disabled,
            onclick,
            children,
        },
        move || clerk.sign_out_with_options(options.clone()),
    )
}
