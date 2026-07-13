use dioxus::prelude::*;
use dioxus_clerk::*;

use crate::components::Spinner;

/// clerk-js v6 after-auth *session tasks* (forced MFA enrollment, organization
/// selection, …) must be completed before a session is fully active. Such a
/// session is *pending*: by default it gates as signed-out, so `SignedIn` /
/// `Protect` stay hidden, matching clerk-js's `treatPendingAsSignedOut`.
///
/// To render UI for a pending user, opt a subtree in with
/// `treat_pending_as_signed_out: false`. Inside it, read the pending task from
/// `use_session().session().current_task` and mount `TaskSetupMFA` for the
/// `setup-mfa` task; clerk-js runs the enrollment UI and navigates to
/// `redirect_url_complete` once every task resolves.
///
/// Alternatively, pass `task_urls` to `ClerkProvider` (via `ClerkOptions`) to
/// let clerk-js route pending users to your own URLs automatically.
#[component]
pub fn SessionTasksExample() -> Element {
    rsx! {
        // Both gates opt in to pending sessions (treat_pending_as_signed_out:
        // false), so a pending user is treated as signed-in here rather than
        // matching SignedOut. Without the flag, SignedIn would hide a pending
        // user and SignedOut would show for them.
        SignedOut { treat_pending_as_signed_out: false,
            p { class: "text-sm text-base-content/60", "Signed out: no active or pending session." }
        }
        SignedIn { treat_pending_as_signed_out: false,
            PendingTask {}
        }
    }
}

#[component]
fn PendingTask() -> Element {
    let session = use_session();
    let current_task = session.session().and_then(|session| session.current_task);

    rsx! {
        match current_task.as_ref().map(|task| &task.key) {
            Some(SessionTaskKey::SetupMfa) => rsx! {
                p { class: "mb-3 text-sm", "Pending task: finish MFA enrollment." }
                TaskSetupMFA {
                    redirect_url_complete: "/hooks",
                    class: "mx-auto max-w-md",
                    fallback: rsx! { Spinner {} },
                }
            },
            Some(key) => rsx! {
                p { class: "text-sm", "Pending task: {key}, route the user here to complete it." }
            },
            None => rsx! {
                p { class: "text-sm text-base-content/60",
                    "Signed in with no pending task. Sign in as a user with forced MFA "
                    "enrollment (or another after-auth task) to see the flow."
                }
            },
        }
    }
}
