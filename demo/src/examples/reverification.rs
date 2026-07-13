use dioxus::prelude::*;
use dioxus_clerk::*;

use crate::components::StatusLine;

/// Guard a sensitive action behind Clerk **step-up reverification**.
///
/// `use_reverification()` returns a handle whose `guard` runs your action and,
/// if it reports `ClerkError::NeedsReverification`, opens clerk-js's
/// reverification prompt and retries the action once the user re-authenticates.
/// A cancelled prompt surfaces as `ClerkError::ReverificationCancelled`.
///
/// In a real app the **server** decides: a `#[server]` action calls a protected
/// Clerk endpoint, and maps its 403 hint with
/// `ClerkError::from_reverification_hint`. Here the gate is simulated in the
/// browser: the first attempt of each run "needs reverification", so the
/// prompt is easy to trigger without a backend.
#[component]
pub fn ReverificationExample() -> Element {
    let reverify = use_reverification();
    let mut status = use_signal(String::new);
    // Stand-in for a server-side reverification gate: armed before each run, and
    // cleared by the first attempt so the retry after reverification succeeds.
    let mut armed = use_signal(|| true);
    // Guard against a second click while the action (and its prompt) is in flight.
    let mut running = use_signal(|| false);

    rsx! {
        SignedIn {
            button {
                class: "btn btn-primary",
                disabled: running(),
                onclick: move |_| async move {
                    running.set(true);
                    status.set("running…".to_string());
                    armed.set(true);

                    let outcome = reverify
                        .guard(|| async move {
                            let mut armed = armed;
                            if armed() {
                                armed.set(false);
                                Err(ClerkError::NeedsReverification {
                                    level: Some(ReverificationLevel::SecondFactor),
                                })
                            } else {
                                Ok("charge refunded")
                            }
                        })
                        .await;

                    match outcome {
                        Ok(result) => status.set(format!("done: {result}")),
                        Err(ClerkError::ReverificationCancelled) => {
                            status.set("cancelled: action did not run".to_string())
                        }
                        Err(error) => status.set(format!("failed: {error}")),
                    }

                    running.set(false);
                },
                "Run sensitive action"
            }
            StatusLine { status: status() }
        }
        SignedOut {
            p { class: "text-sm text-base-content/60",
                "Sign in to try a reverification-gated action."
            }
        }
    }
}
