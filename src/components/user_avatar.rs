use dioxus::prelude::*;

/// Simple avatar image for the current browser-hydrated user.
///
/// Renders `fallback` until clerk-js has supplied a user image URL.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn HeaderAvatar() -> Element {
///     rsx! {
///         SignedIn {
///             UserAvatar {
///                 class: "h-8 w-8 rounded-full",
///                 fallback: rsx! { span { class: "h-8 w-8 rounded-full bg-base-300" } },
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn UserAvatar(
    /// Optional image alt text. Defaults to `User avatar`.
    #[props(default = String::from("User avatar"), into)]
    alt: String,
    /// Attributes (including `id` and `class`) spread onto the generated
    /// `<img>` (for example `loading`, `width`, or `data-*`).
    #[props(extends = GlobalAttributes, extends = img)]
    attributes: Vec<Attribute>,
    /// Rendered until a browser-hydrated user image URL is available.
    #[props(default = rsx! {})]
    fallback: Element,
) -> Element {
    let user = crate::use_user();
    let Some(image_url) = user.user().and_then(|user| user.image_url) else {
        return fallback;
    };

    rsx! {
        img { src: "{image_url}", alt: "{alt}", ..attributes }
    }
}
