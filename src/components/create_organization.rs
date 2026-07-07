use crate::options::CreateOrganizationOptions;
use crate::options::Routing;
use dioxus::prelude::*;

/// Mounted Clerk create-organization UI.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn NewOrganizationPage() -> Element {
///     rsx! {
///         CreateOrganization { routing: Routing::Path, path: "/organizations/new" }
///     }
/// }
/// ```
#[component]
pub fn CreateOrganization(
    /// Embedded routing mode.
    #[props(into)]
    routing: Option<Routing>,
    /// Path used by embedded routing.
    #[props(into)]
    path: Option<String>,
    /// URL Clerk should use after creating an organization.
    #[props(into)]
    after_create_organization_url: Option<String>,
    /// Raw Clerk appearance object.
    appearance: Option<serde_json::Value>,
    /// Advanced options forwarded to Clerk, as a
    /// [`CreateOrganizationOptions`](crate::CreateOrganizationOptions) builder
    /// or a raw `serde_json::Value`. Explicit props win when both set the same
    /// Clerk option key.
    #[props(default = CreateOrganizationOptions::from_value(serde_json::Value::Null), into)]
    options: CreateOrganizationOptions,
    /// Optional id for the Clerk widget host `<div>` that clerk-js mounts
    /// into. May also be set through the attribute spread; this prop wins
    /// when both are present.
    #[props(into)]
    id: Option<String>,
    /// Attributes (including `class`) spread onto the Clerk widget host
    /// `<div>`.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// Rendered while the Clerk create-organization widget is mounting.
    #[props(default = rsx! {})]
    fallback: Element,
) -> Element {
    let options = options
        .maybe_routing(routing)
        .maybe_path(path)
        .maybe_after_create_organization_url(after_create_organization_url)
        .maybe_appearance(appearance)
        .into_value();

    super::widget::render(
        super::widget::Widget::CreateOrganization,
        super::widget::WidgetProps::new(options, fallback, id, attributes),
    )
}
