use crate::options::UserButtonOptions;
use crate::options::UserProfileMode;
use dioxus::prelude::*;

/// Mounted Clerk user button UI.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn HeaderAccountMenu() -> Element {
///     rsx! {
///         SignedIn {
///             UserButton { class: "inline-flex" }
///         }
///     }
/// }
/// ```
#[component]
pub fn UserButton(
    /// URL Clerk should use after sign-out.
    #[props(into)]
    after_sign_out_url: Option<String>,
    /// URL Clerk should use after switching sessions in a multi-session app.
    #[props(into)]
    after_switch_session_url: Option<String>,
    /// URL Clerk should use when adding another account.
    #[props(into)]
    sign_in_url: Option<String>,
    /// Show the user's name next to the avatar when Clerk supports it.
    show_name: Option<bool>,
    /// Whether the user button menu should open by default on first render.
    default_open: Option<bool>,
    /// How the user button opens the user profile UI.
    #[props(into)]
    user_profile_mode: Option<UserProfileMode>,
    /// User profile URL for navigation mode.
    #[props(into)]
    user_profile_url: Option<String>,
    /// Raw options forwarded to the underlying `UserProfile` component.
    user_profile_props: Option<serde_json::Value>,
    /// Raw Clerk appearance object.
    appearance: Option<serde_json::Value>,
    /// Advanced options forwarded to Clerk, as a
    /// [`UserButtonOptions`](crate::UserButtonOptions) builder or a raw
    /// `serde_json::Value`. Explicit props win when both set the same Clerk
    /// option key.
    #[props(default = UserButtonOptions::from_value(serde_json::Value::Null), into)]
    options: UserButtonOptions,
    /// Optional id for the Clerk widget host `<div>` that clerk-js mounts
    /// into. May also be set through the attribute spread; this prop wins
    /// when both are present.
    #[props(into)]
    id: Option<String>,
    /// Attributes (including `class`) spread onto the Clerk widget host
    /// `<div>`.
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// Rendered while the Clerk user button widget is mounting.
    #[props(default = rsx! {})]
    fallback: Element,
) -> Element {
    let options = options
        .maybe_after_sign_out_url(after_sign_out_url)
        .maybe_after_switch_session_url(after_switch_session_url)
        .maybe_sign_in_url(sign_in_url)
        .maybe_show_name(show_name)
        .maybe_default_open(default_open)
        .maybe_user_profile_mode(user_profile_mode)
        .maybe_user_profile_url(user_profile_url)
        .maybe_user_profile_props(user_profile_props)
        .maybe_appearance(appearance)
        .into_value();

    super::widget::render(
        super::widget::Widget::UserButton,
        super::widget::WidgetProps::new(options, fallback, id, attributes),
    )
}
