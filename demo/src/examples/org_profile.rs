use dioxus::prelude::*;
use dioxus_clerk::*;

/// `OrganizationProfile` mounts the full management UI for the active
/// organization, members, invitations, roles, and settings, embedded inline.
/// It needs an active organization; without one the widget shows an empty
/// state. `routing` controls how its sub-pages are addressed.
#[component]
pub fn OrgProfileExample() -> Element {
    rsx! {
        SignedIn {
            OrganizationProfile { routing: Routing::Hash }
        }
        SignedOut {
            p { class: "text-sm text-base-content/60", "Sign in to manage an organization." }
        }
    }
}
