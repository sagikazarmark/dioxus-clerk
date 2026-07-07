use dioxus::prelude::*;
use dioxus_clerk::*;

/// Two ways to reach a verified backend.
///
/// 1. A Dioxus **server function** (`my_counter`): the browser sends the Clerk
///    session cookie automatically; on the server, `ClerkAuthLayer` verifies it
///    and the handler reads `current_auth()`.
/// 2. A **plain API route** (`/api/whoami`) reached with an
///    `Authorization: Bearer <token>` header: `use_auth().get_token()` fetches a
///    short-lived session token, which any HTTP client (not just Dioxus) can
///    send. Use this to authenticate a third-party or non-Dioxus backend.
///
/// The transport `cfg` for each build target lives in `server_api.rs` so this
/// component stays a clean example.
#[component]
pub fn ServerCallExample() -> Element {
    let auth = use_auth();
    let mut counter = use_signal(|| None::<u64>);
    let mut counter_error = use_signal(String::new);
    let mut whoami = use_signal(String::new);

    rsx! {
        div { class: "flex flex-wrap gap-3",
            button {
                class: "btn btn-primary",
                onclick: move |_| async move {
                    match crate::server_api::my_counter().await {
                        Ok(value) => {
                            counter.set(Some(value));
                            counter_error.set(String::new());
                        }
                        Err(error) => counter_error.set(error),
                    }
                },
                "Load counter (cookie auth)"
            }
            button {
                class: "btn btn-outline",
                onclick: move |_| async move {
                    let token = auth.get_token().await.ok().flatten();
                    match crate::server_api::who_am_i(token).await {
                        Ok(user_id) => whoami.set(user_id),
                        Err(error) => whoami.set(error),
                    }
                },
                "Call /api/whoami (bearer)"
            }
        }
        if let Some(value) = counter() {
            p { class: "mt-4 font-mono text-3xl font-bold tracking-tight", "{value}" }
        }
        if !counter_error.read().is_empty() {
            p { class: "mt-2 text-sm text-error", "counter → {counter_error}" }
        }
        if !whoami.read().is_empty() {
            p { class: "mt-2 break-all font-mono text-sm text-base-content/70", "whoami → {whoami}" }
        }
    }
}
