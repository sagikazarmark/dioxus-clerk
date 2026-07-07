use crate::options::Routing;
use crate::options::SignInOptions;
use dioxus::prelude::*;

/// Mounted Clerk sign-in UI.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn SignInPage() -> Element {
///     rsx! {
///         SignIn {
///             routing: Routing::Path,
///             path: "/sign-in",
///             class: "mx-auto max-w-md",
///             fallback: rsx! { div { "Loading sign in..." } },
///         }
///     }
/// }
/// ```
#[component]
pub fn SignIn(
    /// Embedded routing mode.
    #[props(into)]
    routing: Option<Routing>,
    /// Path used by embedded routing.
    #[props(into)]
    path: Option<String>,
    /// Full URL or path to the sign-up page linked from the sign-in UI.
    #[props(into)]
    sign_up_url: Option<String>,
    /// Full URL or path to the waitlist page linked from the sign-in UI.
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
    /// Optional id for the Clerk widget host `<div>` that clerk-js mounts
    /// into. May also be set through the attribute spread; this prop wins
    /// when both are present.
    #[props(into)]
    id: Option<String>,
    /// Attributes (including `class`) spread onto the Clerk widget host
    /// `<div>`.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// Rendered while the Clerk sign-in widget is mounting.
    #[props(default = rsx! {})]
    fallback: Element,
) -> Element {
    let options = options
        .maybe_routing(routing)
        .maybe_path(path)
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

    super::widget::render(
        super::widget::Widget::SignIn,
        super::widget::WidgetProps::new(options, fallback, id, attributes),
    )
}
