use dioxus::prelude::*;
use dioxus_clerk::*;

/// Clerk's organization widgets plus role-gated rendering.
///
/// `OrganizationSwitcher` is the active-org menu; `OrganizationList` lets a
/// user pick or create one; `CreateOrganization` is the standalone create form.
/// `Protect` renders its children only when the signed-in user holds the given
/// org role (or permission), verified against server-verified auth claims;
/// otherwise it renders `fallback`. Fail-closed: enforce on the server too.
#[component]
pub fn OrganizationsExample() -> Element {
    rsx! {
        SignedIn {
            div { class: "flex flex-col gap-6",
                OrganizationSwitcher {}
                OrganizationList {}
                CreateOrganization { routing: Routing::Hash }
                OrganizationProfile { routing: Routing::Hash }
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
        SignedOut {
            p { class: "text-sm text-base-content/60", "Sign in to manage organizations." }
        }
    }
}
