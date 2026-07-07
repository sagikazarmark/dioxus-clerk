use dioxus::prelude::*;
use dioxus_clerk::*;

/// Conditional-rendering primitives.
///
/// - Auth gates (`SignedIn` / `SignedOut`) pick a branch from resolved auth.
/// - `*WhenLoaded` variants render a `fallback` until clerk-js has loaded,
///   avoiding a signed-out flash during hydration.
/// - Control components (`ClerkLoading` / `ClerkLoaded` / `ClerkFailed`) gate
///   on the clerk-js load lifecycle itself.
/// - `Protect` gates on org role/permission (here, plain: signed-in only).
/// - The redirect components navigate away, so they sit behind buttons.
#[component]
pub fn GatingExample() -> Element {
    let mut go_sign_in = use_signal(|| false);
    let mut go_sign_up = use_signal(|| false);

    rsx! {
        div { class: "flex flex-wrap gap-2",
            ClerkLoading { span { class: "badge badge-warning", "clerk-js loading" } }
            ClerkLoaded { span { class: "badge badge-success", "clerk-js loaded" } }
            ClerkFailed { span { class: "badge badge-error", "clerk-js failed" } }
        }

        div { class: "mt-4 space-y-1 text-sm",
            SignedIn { p { "SignedIn — a session is known." } }
            SignedOut { p { "SignedOut — auth resolved with no session." } }
            SignedInWhenLoaded {
                fallback: rsx! { p { class: "text-base-content/50", "waiting for clerk-js…" } },
                p { "SignedInWhenLoaded — session confirmed after load." }
            }
            Protect {
                fallback: rsx! { p { class: "text-base-content/50", "Protect — hidden while signed out." } },
                p { "Protect — visible while signed in." }
            }
        }

        div { class: "mt-4 flex flex-wrap gap-3",
            button {
                class: "btn btn-sm btn-outline",
                onclick: move |_| go_sign_in.set(true),
                "RedirectToSignIn"
            }
            button {
                class: "btn btn-sm btn-outline",
                onclick: move |_| go_sign_up.set(true),
                "RedirectToSignUp"
            }
        }
        if go_sign_in() {
            RedirectToSignIn {}
        }
        if go_sign_up() {
            RedirectToSignUp {}
        }
    }
}
