use dioxus::prelude::*;
use dioxus_clerk::*;

/// The sign-up counterpart to the embedded sign-in form. It shares the same
/// route-aware provider setup; `fallback_redirect_url` sends brand-new accounts
/// to a protected page once Clerk finishes the flow.
#[component]
pub fn EmbeddedSignUpExample() -> Element {
    rsx! {
        SignUp {
            routing: Routing::Path,
            path: "/sign-up",
            sign_in_url: "/sign-in",
            fallback_redirect_url: "/hooks",
            class: "mx-auto max-w-md",
            fallback: rsx! {
                div { class: "grid min-h-64 place-items-center",
                    span { class: "loading loading-spinner loading-md text-primary" }
                }
            },
        }
    }
}
