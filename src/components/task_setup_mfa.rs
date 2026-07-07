use crate::options::TaskSetupMFAOptions;
use dioxus::prelude::*;

/// Mounted Clerk task setup-MFA UI, for the clerk-js v6 after-auth
/// `setup-mfa` session task.
///
/// Render this when the current session has a pending `setup-mfa`
/// [`current_task`](crate::Session::current_task); clerk-js drives the MFA
/// enrollment flow and navigates to `redirect_url_complete` once every pending
/// task resolves.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn MfaSetupPage() -> Element {
///     rsx! {
///         TaskSetupMFA {
///             redirect_url_complete: "/",
///             class: "mx-auto max-w-md",
///         }
///     }
/// }
/// ```
#[component]
pub fn TaskSetupMFA(
    /// Full URL or path Clerk navigates to after all pending session tasks
    /// resolve. clerk-js requires this to complete the flow.
    #[props(into)]
    redirect_url_complete: Option<String>,
    /// Raw Clerk appearance object.
    appearance: Option<serde_json::Value>,
    /// Advanced options forwarded to Clerk, as a
    /// [`TaskSetupMFAOptions`](crate::TaskSetupMFAOptions) builder or a raw
    /// `serde_json::Value`. Explicit props win when both set the same Clerk
    /// option key.
    #[props(default = TaskSetupMFAOptions::from_value(serde_json::Value::Null), into)]
    options: TaskSetupMFAOptions,
    /// Optional id for the Clerk widget host `<div>` that clerk-js mounts
    /// into. May also be set through the attribute spread; this prop wins
    /// when both are present.
    #[props(into)]
    id: Option<String>,
    /// Attributes (including `class`) spread onto the Clerk widget host
    /// `<div>`.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// Rendered while the Clerk task setup-MFA widget is mounting.
    #[props(default = rsx! {})]
    fallback: Element,
) -> Element {
    let options = options
        .maybe_redirect_url_complete(redirect_url_complete)
        .maybe_appearance(appearance)
        .into_value();

    super::widget::render(
        super::widget::Widget::TaskSetupMfa,
        super::widget::WidgetProps::new(options, fallback, id, attributes),
    )
}
