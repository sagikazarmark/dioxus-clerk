use crate::options::OrganizationListOptions;
use dioxus::prelude::*;

/// Mounted Clerk organization list UI.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn OrganizationsPage() -> Element {
///     rsx! {
///         OrganizationList { class: "mx-auto max-w-lg" }
///     }
/// }
/// ```
#[component]
pub fn OrganizationList(
    /// URL Clerk should use after creating an organization.
    #[props(into)]
    after_create_organization_url: Option<String>,
    /// URL Clerk should use after selecting an organization.
    #[props(into)]
    after_select_organization_url: Option<String>,
    /// Raw Clerk appearance object.
    appearance: Option<serde_json::Value>,
    /// Advanced options forwarded to Clerk, as an
    /// [`OrganizationListOptions`](crate::OrganizationListOptions) builder or a
    /// raw `serde_json::Value`. Explicit props win when both set the same Clerk
    /// option key.
    #[props(default = OrganizationListOptions::from_value(serde_json::Value::Null), into)]
    options: OrganizationListOptions,
    /// Optional id for the Clerk widget host `<div>` that clerk-js mounts
    /// into. May also be set through the attribute spread; this prop wins
    /// when both are present.
    #[props(into)]
    id: Option<String>,
    /// Attributes (including `class`) spread onto the Clerk widget host
    /// `<div>`.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// Rendered while the Clerk organization list widget is mounting.
    #[props(default = rsx! {})]
    fallback: Element,
) -> Element {
    let options = options
        .maybe_after_create_organization_url(after_create_organization_url)
        .maybe_after_select_organization_url(after_select_organization_url)
        .maybe_appearance(appearance)
        .into_value();

    super::widget::render(
        super::widget::Widget::OrganizationList,
        super::widget::WidgetProps::new(options, fallback, id, attributes),
    )
}
