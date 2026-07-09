use dioxus::prelude::*;
use dioxus_clerk::*;

/// `OrganizationList` is the embedded pick-or-create surface: it lists every
/// organization the user belongs to and offers a create action. Unlike the
/// switcher it is a full-width management view rather than a dropdown.
#[component]
pub fn OrgListExample() -> Element {
    rsx! {
        SignedIn {
            OrganizationList {}
        }
        SignedOut {
            p { class: "text-sm text-base-content/60", "Sign in to list organizations." }
        }
    }
}
