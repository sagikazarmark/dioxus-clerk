use dioxus::prelude::*;
use dioxus_clerk::*;

use crate::components::Spinner;

/// `UserProfile` mounts the full account-management UI inline, the same surface
/// `UserButton`'s modal shows, but embedded in your own page. Use it when you
/// want a dedicated `/account` route instead of a popup. `routing` controls how
/// its sub-pages are addressed; `Hash` keeps them in the URL fragment.
#[component]
pub fn ProfileEmbeddedExample() -> Element {
    rsx! {
        SignedIn {
            UserProfile {
                routing: Routing::Hash,
                fallback: rsx! { Spinner {} },
            }
        }
        SignedOut {
            p { class: "text-sm text-base-content/60", "Sign in to see the embedded profile." }
        }
    }
}
