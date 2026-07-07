use dioxus::prelude::*;
use dioxus_clerk::*;

/// Surface and clear Clerk errors.
///
/// `use_clerk_error` reports the latest init failure, failed scheduled action,
/// or non-fatal config warning (e.g. an SSR seed key mismatch).
/// `use_clear_clerk_error` clears the *recoverable* kinds; fatal init failures
/// stay put, since nothing retries them and clearing would only hide the cause.
#[component]
pub fn ErrorsExample() -> Element {
    let error = use_clerk_error();
    let clear = use_clear_clerk_error();

    let error = error.read();
    if let Some(error) = error.as_ref() {
        rsx! {
            div { class: "alert alert-error",
                span { "{error}" }
                button {
                    class: "btn btn-sm",
                    onclick: move |_| clear.call(()),
                    "Dismiss"
                }
            }
        }
    } else {
        rsx! {
            p { class: "text-sm text-base-content/60",
                "No Clerk error right now. Errors from failed actions or init would appear here."
            }
        }
    }
}
