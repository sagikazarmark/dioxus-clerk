use dioxus::prelude::*;
use dioxus_clerk::*;

use crate::components::Spinner;

/// Clerk's waitlist form. The widget mounts regardless of configuration, but
/// submissions only work when the Clerk instance's sign-up mode is set to
/// "Waitlist" in the Dashboard, which also disables normal sign-up on that
/// instance. See the callout on this page.
#[component]
pub fn WaitlistExample() -> Element {
    rsx! {
        Waitlist {
            class: "mx-auto max-w-md",
            fallback: rsx! { Spinner {} },
        }
    }
}
