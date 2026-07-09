use dioxus::prelude::*;
use dioxus_clerk::*;

use crate::ui::Spinner;

/// Account UI for a signed-in user. `UserAvatar` is a lightweight `<img>` of
/// the current user's image (handy for custom headers). `UserButton` is the
/// hosted account menu; `user_profile_mode` picks whether it opens the profile
/// in a modal or navigates to a page. `UserProfile` mounts the full
/// account-management UI inline.
#[component]
pub fn ProfileExample() -> Element {
    rsx! {
        SignedIn {
            div { class: "flex items-center gap-4",
                UserAvatar {
                    class: "h-10 w-10 rounded-full ring ring-base-300",
                    fallback: rsx! { span { class: "h-10 w-10 rounded-full bg-base-300" } },
                }
                UserButton { user_profile_mode: UserProfileMode::Modal }
            }
            UserProfile {
                routing: Routing::Hash,
                class: "mt-6",
                fallback: rsx! { Spinner {} },
            }
        }
        SignedOut {
            p { class: "text-sm text-base-content/60", "Sign in to see your profile UI." }
        }
    }
}
