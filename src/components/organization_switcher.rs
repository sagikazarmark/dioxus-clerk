use crate::options::OrganizationSwitcherOptions;
use dioxus::prelude::*;

/// Mounted Clerk organization switcher UI.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn HeaderOrganizationMenu() -> Element {
///     rsx! {
///         OrganizationSwitcher { class: "inline-flex" }
///     }
/// }
/// ```
#[component]
pub fn OrganizationSwitcher(
    /// URL Clerk should use when creating an organization from the switcher.
    #[props(into)]
    create_organization_url: Option<String>,
    /// URL Clerk should use after creating an organization.
    #[props(into)]
    after_create_organization_url: Option<String>,
    /// URL Clerk should use for organization profile navigation.
    #[props(into)]
    organization_profile_url: Option<String>,
    /// Raw Clerk appearance object.
    appearance: Option<serde_json::Value>,
    /// Advanced options forwarded to Clerk, as an
    /// [`OrganizationSwitcherOptions`](crate::OrganizationSwitcherOptions)
    /// builder or a raw `serde_json::Value`. Explicit props win when both set
    /// the same Clerk option key.
    #[props(default = OrganizationSwitcherOptions::from_value(serde_json::Value::Null), into)]
    options: OrganizationSwitcherOptions,
    /// Optional id for the Clerk widget host `<div>` that clerk-js mounts
    /// into. May also be set through the attribute spread; this prop wins
    /// when both are present.
    #[props(into)]
    id: Option<String>,
    /// Attributes (including `class`) spread onto the Clerk widget host
    /// `<div>`.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// Rendered while the Clerk organization switcher widget is mounting.
    #[props(default = rsx! {})]
    fallback: Element,
) -> Element {
    let options = options
        .maybe_create_organization_url(create_organization_url)
        .maybe_after_create_organization_url(after_create_organization_url)
        .maybe_organization_profile_url(organization_profile_url)
        .maybe_appearance(appearance)
        .into_value();

    super::widget::render(
        super::widget::Widget::OrganizationSwitcher,
        super::widget::WidgetProps::new(options, fallback, id, attributes),
    )
}
