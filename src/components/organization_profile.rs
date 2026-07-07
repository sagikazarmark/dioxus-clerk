use crate::options::OrganizationProfileOptions;
use crate::options::Routing;
use dioxus::prelude::*;

/// Mounted Clerk organization profile UI.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn OrganizationSettingsPage() -> Element {
///     rsx! {
///         OrganizationProfile { routing: Routing::Path, path: "/organization" }
///     }
/// }
/// ```
#[component]
pub fn OrganizationProfile(
    /// Embedded routing mode.
    #[props(into)]
    routing: Option<Routing>,
    /// Path used by embedded routing.
    #[props(into)]
    path: Option<String>,
    /// Raw Clerk appearance object.
    appearance: Option<serde_json::Value>,
    /// Advanced options forwarded to Clerk, as an
    /// [`OrganizationProfileOptions`](crate::OrganizationProfileOptions)
    /// builder or a raw `serde_json::Value`. Explicit props win when both set
    /// the same Clerk option key.
    #[props(default = OrganizationProfileOptions::from_value(serde_json::Value::Null), into)]
    options: OrganizationProfileOptions,
    /// Optional id for the Clerk widget host `<div>` that clerk-js mounts
    /// into. May also be set through the attribute spread; this prop wins
    /// when both are present.
    #[props(into)]
    id: Option<String>,
    /// Attributes (including `class`) spread onto the Clerk widget host
    /// `<div>`.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// Rendered while the Clerk organization profile widget is mounting.
    #[props(default = rsx! {})]
    fallback: Element,
) -> Element {
    let options = options
        .maybe_routing(routing)
        .maybe_path(path)
        .maybe_appearance(appearance)
        .into_value();

    super::widget::render(
        super::widget::Widget::OrganizationProfile,
        super::widget::WidgetProps::new(options, fallback, id, attributes),
    )
}
