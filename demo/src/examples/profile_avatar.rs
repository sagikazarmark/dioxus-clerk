use dioxus::prelude::*;
use dioxus_clerk::*;

/// `UserAvatar` is a lightweight `<img>` of the current user's image, handy for
/// a custom header or nav bar where you want the picture without the menu. It
/// renders `fallback` until the image is available.
#[component]
pub fn ProfileAvatarExample() -> Element {
    rsx! {
        SignedIn {
            UserAvatar {
                class: "h-12 w-12 rounded-full ring ring-base-300",
                fallback: rsx! { span { class: "h-12 w-12 rounded-full bg-base-300" } },
            }
        }
        SignedOut {
            p { class: "text-sm text-base-content/60", "Sign in to see your avatar." }
        }
    }
}
