use dioxus::prelude::*;
use dioxus_clerk::*;

/// `OrganizationSwitcher` is the active-organization menu: it shows the current
/// org, lets the user switch between the ones they belong to, and can create or
/// manage one. It is a compact trigger, so it sits well inline in a header.
#[component]
pub fn OrgSwitcherExample() -> Element {
    rsx! {
        SignedIn {
            OrganizationSwitcher {}
        }
        SignedOut {
            p { class: "text-sm text-base-content/60", "Sign in to switch organizations." }
        }
    }
}
