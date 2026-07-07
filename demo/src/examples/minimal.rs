use dioxus::prelude::*;
use dioxus_clerk::*;

/// The smallest useful auth surface: a sign-in affordance when signed out, and
/// account controls when signed in. `ClerkProvider` lives higher up in the app
/// (see `app.rs`), so components here just read its state.
#[component]
pub fn MinimalExample() -> Element {
    rsx! {
        SignedOut {
            SignInButton { class: "btn btn-primary", "Sign in" }
        }
        SignedIn {
            div { class: "flex items-center gap-3",
                UserButton {}
                SignOutButton { class: "btn btn-ghost", redirect_url: "/", "Sign out" }
            }
        }
    }
}
