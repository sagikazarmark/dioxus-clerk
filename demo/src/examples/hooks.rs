use dioxus::prelude::*;
use dioxus_clerk::*;

use crate::components::StateGrid;

/// The reactive hooks.
///
/// `use_auth` exposes resolved auth facts (ids, org role/permissions, helpers
/// like `has_role`). `use_user` and `use_session` add the full clerk-js `User`
/// and `Session` once the browser has hydrated them; `None` until then, even
/// while `use_auth` already reports signed-in from a server snapshot.
#[component]
pub fn HooksExample() -> Element {
    let auth = use_auth();
    let user = use_user();
    let session = use_session();

    let state = auth.state();
    let is_loaded = state.is_loaded();
    let status = state.status();
    let is_signed_in = state.is_signed_in();
    let user_id = state.user_id().unwrap_or_default().to_string();
    let org_role = state.org_role().unwrap_or_default().to_string();

    rsx! {
        StateGrid {
            rows: vec![
                ("is_loaded", format!("{is_loaded}")),
                ("status", format!("{status:?}")),
                ("is_signed_in", format!("{is_signed_in}")),
                ("user_id", user_id),
                ("org_role", org_role),
            ],
        }
        if let Some(user) = user.user() {
            p { class: "mt-4 text-sm",
                "use_user → "
                span { class: "font-medium",
                    "{user.first_name.clone().unwrap_or_default()} {user.last_name.clone().unwrap_or_default()}"
                }
                if let Some(email) = user.primary_email_address.as_ref() {
                    span { class: "text-base-content/60", " · {email}" }
                }
            }
        }
        if let Some(session) = session.session() {
            p { class: "mt-1 break-all font-mono text-xs text-base-content/60",
                "use_session → session_id: {session.id}"
            }
        }
    }
}
