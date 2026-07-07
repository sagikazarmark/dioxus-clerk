use dioxus::prelude::*;
use dioxus_clerk::*;

/// `SignInButton` / `SignUpButton` render a plain `<button>` and schedule the
/// matching Clerk action when clicked. `mode` chooses between Clerk's modal
/// flow and a redirect to the configured sign-in/sign-up URL (the default).
/// `SignOutButton` ends the session and can navigate afterwards.
#[component]
pub fn ButtonsExample() -> Element {
    rsx! {
        div { class: "flex flex-wrap items-center gap-3",
            SignInButton {
                mode: AuthButtonMode::Modal,
                class: "btn btn-primary",
                "Sign in (modal)"
            }
            SignInButton {
                mode: AuthButtonMode::Redirect,
                class: "btn btn-outline",
                "Sign in (redirect)"
            }
            SignUpButton {
                mode: AuthButtonMode::Modal,
                class: "btn btn-secondary",
                "Create account (modal)"
            }
            SignedIn {
                SignOutButton { class: "btn btn-ghost", redirect_url: "/", "Sign out" }
            }
        }
    }
}
