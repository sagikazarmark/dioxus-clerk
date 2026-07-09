use dioxus::prelude::*;
use dioxus_clerk::*;

/// `CreateOrganization` is the standalone create-an-org form, the same step the
/// switcher and list expose, mounted on its own so you can give it a dedicated
/// route. `routing` controls how its sub-pages are addressed.
#[component]
pub fn OrgCreateExample() -> Element {
    rsx! {
        SignedIn {
            CreateOrganization { routing: Routing::Hash }
        }
        SignedOut {
            p { class: "text-sm text-base-content/60", "Sign in to create an organization." }
        }
    }
}
