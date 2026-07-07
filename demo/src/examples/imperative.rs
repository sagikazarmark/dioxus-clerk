use dioxus::prelude::*;
use dioxus_clerk::*;

/// Drive Clerk from your own elements.
///
/// `use_clerk()` returns fire-and-forget actions (open/close modals, redirect,
/// sign out) that wait for clerk-js to load and surface failures through
/// `use_clerk_error`. `use_auth()` additionally offers awaited `try_*` variants
/// that hand you the error directly — use those when you need to react to the
/// outcome inline.
#[component]
pub fn ImperativeExample() -> Element {
    let clerk = use_clerk();
    let auth = use_auth();
    let mut status = use_signal(String::new);

    rsx! {
        div { class: "flex flex-wrap gap-3",
            SignedOut {
                button {
                    class: "btn btn-primary",
                    onclick: move |_| clerk.open_sign_in(),
                    "open_sign_in()"
                }
            }
            SignedIn {
                button {
                    class: "btn btn-outline",
                    onclick: move |_| clerk.open_user_profile(),
                    "open_user_profile()"
                }
                button {
                    class: "btn btn-ghost",
                    onclick: move |_| async move {
                        match auth.try_sign_out().await {
                            Ok(()) => status.set("signed out".to_string()),
                            Err(error) => status.set(format!("sign-out failed: {error}")),
                        }
                    },
                    "try_sign_out().await"
                }
            }
        }
        if !status.read().is_empty() {
            p { class: "mt-3 text-sm text-base-content/70", "{status}" }
        }
    }
}
