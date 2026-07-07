use dioxus::prelude::*;
use dioxus_clerk::*;

/// Clerk's waitlist form. The widget mounts regardless of configuration, but
/// submissions only work when the Clerk instance's sign-up mode is set to
/// "Waitlist" in the Dashboard — which also disables normal sign-up on that
/// instance. See the callout on this page.
#[component]
pub fn WaitlistExample() -> Element {
    rsx! {
        Waitlist {
            class: "mx-auto max-w-md",
            fallback: rsx! {
                div { class: "grid min-h-64 place-items-center",
                    span { class: "loading loading-spinner loading-md text-primary" }
                }
            },
        }
    }
}
