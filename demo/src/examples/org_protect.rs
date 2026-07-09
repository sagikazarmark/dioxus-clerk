use dioxus::prelude::*;
use dioxus_clerk::*;

/// `Protect` renders its children only when the signed-in user holds the given
/// organization role (or permission), checked against server-verified auth
/// claims; otherwise it renders `fallback`. It is fail-closed, so treat it as a
/// UI hint and always enforce the same rule on the server.
#[component]
pub fn OrgProtectExample() -> Element {
    rsx! {
        Protect {
            role: "org:admin",
            fallback: rsx! {
                p { class: "text-sm text-base-content/60",
                    "You are not an org:admin, so this admin-only panel stays hidden."
                }
            },
            div { class: "rounded-xl border border-success/30 bg-success/5 p-4 text-sm font-medium text-success",
                "Visible only to members with the org:admin role."
            }
        }
    }
}
