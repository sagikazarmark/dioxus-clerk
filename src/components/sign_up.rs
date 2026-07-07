use crate::options::Routing;
use crate::options::SignUpOptions;
use dioxus::prelude::*;

/// Mounted Clerk sign-up UI.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn SignUpPage() -> Element {
///     rsx! {
///         SignUp {
///             routing: Routing::Path,
///             path: "/sign-up",
///             class: "mx-auto max-w-md",
///         }
///     }
/// }
/// ```
#[component]
pub fn SignUp(
    /// Embedded routing mode.
    #[props(into)]
    routing: Option<Routing>,
    /// Path used by embedded routing.
    #[props(into)]
    path: Option<String>,
    /// Full URL or path to the sign-in page linked from the sign-up UI.
    #[props(into)]
    sign_in_url: Option<String>,
    /// Full URL or path to the waitlist page linked from the sign-up UI.
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
    /// Optional id for the Clerk widget host `<div>` that clerk-js mounts
    /// into. May also be set through the attribute spread; this prop wins
    /// when both are present.
    #[props(into)]
    id: Option<String>,
    /// Attributes (including `class`) spread onto the Clerk widget host
    /// `<div>`.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// Rendered while the Clerk sign-up widget is mounting.
    #[props(default = rsx! {})]
    fallback: Element,
) -> Element {
    let options = options
        .maybe_routing(routing)
        .maybe_path(path)
        .maybe_sign_in_url(sign_in_url)
        .maybe_waitlist_url(waitlist_url)
        .maybe_force_redirect_url(force_redirect_url)
        .maybe_fallback_redirect_url(fallback_redirect_url)
        .maybe_sign_in_force_redirect_url(sign_in_force_redirect_url)
        .maybe_sign_in_fallback_redirect_url(sign_in_fallback_redirect_url)
        .maybe_initial_values(initial_values)
        .maybe_appearance(appearance)
        .into_value();

    super::widget::render(
        super::widget::Widget::SignUp,
        super::widget::WidgetProps::new(options, fallback, id, attributes),
    )
}
