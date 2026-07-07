use crate::options::WaitlistOptions;
use dioxus::prelude::*;

/// Mounted Clerk waitlist UI.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn WaitlistPage() -> Element {
///     rsx! {
///         Waitlist { class: "mx-auto max-w-md" }
///     }
/// }
/// ```
#[component]
pub fn Waitlist(
    /// URL Clerk should use after joining the waitlist.
    #[props(into)]
    after_join_waitlist_url: Option<String>,
    /// Raw Clerk appearance object.
    appearance: Option<serde_json::Value>,
    /// Advanced options forwarded to Clerk, as a
    /// [`WaitlistOptions`](crate::WaitlistOptions) builder or a raw
    /// `serde_json::Value`. Explicit props win when both set the same Clerk
    /// option key.
    #[props(default = WaitlistOptions::from_value(serde_json::Value::Null), into)]
    options: WaitlistOptions,
    /// Optional id for the Clerk widget host `<div>` that clerk-js mounts
    /// into. May also be set through the attribute spread; this prop wins
    /// when both are present.
    #[props(into)]
    id: Option<String>,
    /// Attributes (including `class`) spread onto the Clerk widget host
    /// `<div>`.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// Rendered while the Clerk waitlist widget is mounting.
    #[props(default = rsx! {})]
    fallback: Element,
) -> Element {
    let options = options
        .maybe_after_join_waitlist_url(after_join_waitlist_url)
        .maybe_appearance(appearance)
        .into_value();

    super::widget::render(
        super::widget::Widget::Waitlist,
        super::widget::WidgetProps::new(options, fallback, id, attributes),
    )
}
