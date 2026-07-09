use dioxus::prelude::*;
use dioxus_clerk::*;
use dioxus_code::{code, Code};

use crate::examples::server_call::ServerCallExample;
use crate::ui::{snippet_theme, CheckingAuthPanel, DocLink, ExampleSection, InlineCode, PageHeader};

#[component]
pub fn ServerDemo() -> Element {
    rsx! {
        SignedOutWhenLoaded {
            fallback: rsx! { CheckingAuthPanel {} },
            PageHeader {
                eyebrow: "Server",
                title: "Server & tokens",
                intro: "Sign in to call a verified backend two ways: a cookie-authenticated Dioxus server function, and a plain API route reached with an Authorization: Bearer token.",
            }
            div { class: "mt-8",
                SignInButton { class: "btn btn-primary btn-lg shadow-sm", "Sign in to continue" }
            }
        }
        SignedIn {
            PageHeader {
                eyebrow: "Server",
                title: "Server & tokens",
                intro: "Both buttons hit a backend that only answers verified requests. The Axum ClerkAuthLayer accepts either the session cookie or a bearer token; the counter uses the cookie, whoami uses the token.",
            }
            ExampleSection {
                title: "Cookie-verified server fn and bearer token call",
                intro: rsx! {
                    "The first button calls a Dioxus "
                    InlineCode { "#[server]" }
                    " function (the cookie rides along automatically). The second fetches a session token with "
                    InlineCode { "get_token()" }
                    " and sends it as "
                    InlineCode { "Authorization: Bearer" }
                    " to a plain "
                    InlineCode { "/api/whoami" }
                    " route."
                },
                demo: rsx! { ServerCallExample {} },
                code: rsx! { Code { src: code!("src/examples/server_call.rs"), theme: snippet_theme() } },
            }
            section { class: "mt-10 rounded-[2rem] border border-base-300 bg-base-100 p-6 shadow-sm sm:p-8",
                h2 { class: "text-xl font-semibold tracking-tight", "The backend" }
                p { class: "mt-2 max-w-[70ch] text-sm leading-6 text-base-content/65",
                    "The server function reads current_auth(); the /api/whoami route uses the ClerkAuth extractor. Both are verified by ClerkAuthLayer. The per-target cfg lives here so the client example stays clean."
                }
                div { class: "mt-6 overflow-x-auto rounded-2xl border border-base-300 bg-base-200/60 p-4 text-sm [&_pre]:!bg-transparent",
                    Code { src: code!("src/server_api.rs"), theme: snippet_theme() }
                }
                p { class: "mt-4 text-sm text-base-content/60",
                    "Clerk docs: "
                    DocLink { href: "https://clerk.com/docs/backend-requests/resources/session-tokens", "session tokens" }
                    "."
                }
            }
        }
    }
}
