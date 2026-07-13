use dioxus::prelude::*;
use dioxus_clerk::*;

use crate::components::Spinner;

/// Clerk's prebuilt sign-in form mounted directly into your route. With
/// `Routing::Path`, Clerk keeps its sub-steps (SSO, email verification) under
/// `path`, so the router must also accept `"/sign-in/:..segments"` (see the
/// `Route` enum in `app.rs`). `fallback` shows while the widget mounts.
#[component]
pub fn EmbeddedSignInExample() -> Element {
    rsx! {
        SignIn {
            routing: Routing::Path,
            path: "/sign-in",
            sign_up_url: "/sign-up",
            fallback_redirect_url: "/hooks",
            class: "mx-auto max-w-md",
            fallback: rsx! { Spinner {} },
        }
    }
}
