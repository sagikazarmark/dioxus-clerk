use dioxus::prelude::*;
use dioxus_clerk::*;
use dioxus_code::{code, Code};

use crate::app::Route;
use crate::examples::buttons::ButtonsExample;
use crate::examples::minimal::MinimalExample;
use crate::ui::{snippet_theme, DocLink, ExampleSection, InlineCode, PageHeader};

#[component]
pub fn Home() -> Element {
    let user = use_user();
    let state = user.state();

    let greeting = if !state.is_loaded() {
        rsx! {
            div { class: "mt-7",
                span { class: "loading loading-spinner loading-sm text-primary" }
            }
        }
    } else if let Some(user) = state.user() {
        let id = user.id.clone();
        rsx! {
            p { class: "mt-5 max-w-[68ch] text-lg leading-8 text-base-content/70",
                "Signed in as "
                code { class: "rounded bg-base-200 px-1.5 py-0.5 text-sm", "{id}" }
                ". Browse the examples in the sidebar: each one mounts a live feature next to the exact source that renders it."
            }
        }
    } else {
        rsx! {
            p { class: "mt-5 max-w-[68ch] text-lg leading-8 text-base-content/70",
                "A live gallery of every dioxus-clerk feature. Each page mounts a real component next to the exact source that produced it, so the snippet you read is the code that runs."
            }
            div { class: "mt-7 flex flex-wrap items-center gap-3",
                Link { to: Route::Minimal {}, class: "btn btn-primary btn-lg shadow-sm", "Start with Minimal" }
                Link { to: Route::ServerDemo {}, class: "btn btn-outline btn-lg", "Jump to server auth" }
            }
        }
    };

    rsx! {
        section {
            p { class: "text-sm font-semibold uppercase tracking-[0.18em] text-primary", "dioxus-clerk demo" }
            h1 { class: "mt-4 text-4xl font-bold tracking-tight text-balance sm:text-5xl", "Clerk auth for Dioxus, feature by feature." }
            {greeting}
            div { class: "mt-12 grid gap-4 sm:grid-cols-2 lg:grid-cols-3",
                HomeCard { to: Route::Minimal {}, badge: "basics", title: "Minimal", body: "Provider, auth buttons, and signed-in/out gates." }
                HomeCard { to: Route::SignInPage {}, badge: "components", title: "Embedded widgets", body: "Mount Clerk's hosted sign-in/up forms in your routes." }
                HomeCard { to: Route::Organizations {}, badge: "components", title: "Organizations", body: "Switcher, list, create, and role-gated rendering." }
                HomeCard { to: Route::Gating {}, badge: "advanced", title: "Gating & control", body: "Conditional rendering from auth and load state." }
                HomeCard { to: Route::Imperative {}, badge: "advanced", title: "Imperative actions", body: "Drive Clerk from your own buttons and handlers." }
                HomeCard { to: Route::ServerDemo {}, badge: "server", title: "Server & tokens", body: "Cookie-verified server fns and bearer tokens." }
            }
            p { class: "mt-10 text-sm text-base-content/60",
                "New to Clerk components? See the "
                DocLink { href: "https://clerk.com/docs/components/overview", "components overview" }
                " in the Clerk docs."
            }
        }
    }
}

#[component]
fn HomeCard(to: Route, badge: String, title: String, body: String) -> Element {
    rsx! {
        Link {
            to,
            class: "group rounded-[1.5rem] border border-base-300 bg-base-100 p-5 shadow-sm transition hover:-translate-y-0.5 hover:shadow-xl hover:shadow-primary/10",
            span { class: "badge badge-outline badge-primary", "{badge}" }
            h2 { class: "mt-4 text-lg font-semibold", "{title}" }
            p { class: "mt-2 text-sm leading-6 text-base-content/65", "{body}" }
        }
    }
}

#[component]
pub fn Minimal() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Basics",
            title: "Minimal auth",
            intro: "The smallest useful surface: mount the provider once (in the layout), show a sign-in action when signed out, and swap in account controls when Clerk has a session.",
        }
        ExampleSection {
            title: "Sign-in action and account controls",
            intro: rsx! {
                InlineCode { "SignedOut" }
                " and "
                InlineCode { "SignedIn" }
                " pick a branch from resolved auth state. The buttons render plain "
                InlineCode { "<button>" }
                " elements and schedule the matching Clerk action."
            },
            demo: rsx! { MinimalExample {} },
            code: rsx! { Code { src: code!("src/examples/minimal.rs"), theme: snippet_theme() } },
        }
    }
}

#[component]
pub fn Buttons() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Basics",
            title: "Buttons & modes",
            intro: "SignInButton and SignUpButton schedule Clerk's auth flows; mode chooses between an on-page modal and a redirect to your configured sign-in/sign-up URL.",
        }
        ExampleSection {
            title: "Modal vs. redirect",
            intro: rsx! {
                InlineCode { "AuthButtonMode::Modal" }
                " opens Clerk's flow in place; "
                InlineCode { "AuthButtonMode::Redirect" }
                " (the default) navigates to the configured URL. "
                InlineCode { "SignOutButton" }
                " ends the session."
            },
            demo: rsx! { ButtonsExample {} },
            code: rsx! { Code { src: code!("src/examples/buttons.rs"), theme: snippet_theme() } },
        }
        p { class: "mt-6 text-sm text-base-content/60",
            "For a design-system button that owns its own element, call use_clerk() from an onclick instead; see the "
            Link { to: Route::Imperative {}, class: "link link-primary", "Imperative actions" }
            " page."
        }
    }
}
