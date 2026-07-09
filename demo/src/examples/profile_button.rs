use dioxus::prelude::*;
use dioxus_clerk::*;

/// `UserButton` is Clerk's hosted account menu: the avatar opens a dropdown with
/// account management and sign-out. `user_profile_mode` chooses whether "Manage
/// account" opens the profile in a `Modal` (shown here) or navigates to a page
/// (`UserProfileMode::Navigation`, paired with your own `UserProfile` route).
#[component]
pub fn ProfileButtonExample() -> Element {
    rsx! {
        SignedIn {
            UserButton { user_profile_mode: UserProfileMode::Modal }
        }
        SignedOut {
            p { class: "text-sm text-base-content/60", "Sign in to see the account menu." }
        }
    }
}
