use dioxus::prelude::*;
use dioxus_clerk::*;
use dioxus_code::{code, Code};

use crate::examples::errors::ErrorsExample;
use crate::examples::gating::GatingExample;
use crate::examples::hooks::HooksExample;
use crate::examples::imperative::ImperativeExample;
use crate::examples::reverification::ReverificationExample;
use crate::examples::session_tasks::SessionTasksExample;
use crate::ui::{snippet_theme, CheckingAuthPanel, DocLink, ExampleSection, InlineCode, PageHeader};

#[component]
pub fn Gating() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Advanced",
            title: "Gating & control",
            intro: "Pick what renders from auth state and the clerk-js load lifecycle. Auth gates branch on the session; control components branch on loadedness; Protect branches on org role.",
        }
        ExampleSection {
            title: "Gates, control components, Protect, and redirects",
            intro: rsx! {
                "The "
                InlineCode { "*WhenLoaded" }
                " gates render a fallback until clerk-js loads, preventing a signed-out flash during hydration. The redirect components navigate away, so here they are behind buttons."
            },
            demo: rsx! { GatingExample {} },
            code: rsx! { Code { src: code!("src/examples/gating.rs"), theme: snippet_theme() } },
        }
    }
}

#[component]
pub fn SessionTasks() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Advanced",
            title: "Session tasks",
            intro: "clerk-js v6 after-auth tasks (forced MFA, org selection) leave a session pending until they resolve. A pending session gates as signed-out, but its current_task is exposed for routing.",
        }
        ExampleSection {
            title: "treat_pending_as_signed_out, current_task, and <TaskSetupMFA>",
            intro: rsx! {
                "Opt a subtree in to pending sessions with "
                InlineCode { "treat_pending_as_signed_out: false" }
                " (on "
                InlineCode { "SignedIn" }
                "/"
                InlineCode { "SignedOut" }
                "/"
                InlineCode { "Protect" }
                ", or "
                InlineCode { "use_auth" }
                "), read the task from "
                InlineCode { "use_session().session().current_task" }
                ", and mount "
                InlineCode { "TaskSetupMFA" }
                ", or pass "
                InlineCode { "task_urls" }
                " to "
                InlineCode { "ClerkProvider" }
                " to let clerk-js route pending users automatically."
            },
            demo: rsx! { SessionTasksExample {} },
            code: rsx! { Code { src: code!("src/examples/session_tasks.rs"), theme: snippet_theme() } },
        }
        p { class: "mt-6 text-sm text-base-content/60",
            "Clerk docs: "
            DocLink { href: "https://clerk.com/docs/guides/configure/session-tasks", "Session tasks" }
            "."
        }
    }
}

#[component]
pub fn Reverification() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Advanced",
            title: "Step-up reverification",
            intro: "clerk-js v6 can require a fresh authentication factor before a sensitive action runs. use_reverification wraps the action: on a needs-reverification result it prompts, then resumes on success.",
        }
        ExampleSection {
            title: "use_reverification().guard(...)",
            intro: rsx! {
                InlineCode { "guard" }
                " runs the action; if it returns "
                InlineCode { "ClerkError::NeedsReverification" }
                ", clerk-js opens the reverification prompt and the action retries on success (or returns "
                InlineCode { "ReverificationCancelled" }
                "). A real server maps its 403 hint with "
                InlineCode { "ClerkError::from_reverification_hint" }
                "."
            },
            demo: rsx! { ReverificationExample {} },
            code: rsx! { Code { src: code!("src/examples/reverification.rs"), theme: snippet_theme() } },
        }
        p { class: "mt-6 text-sm text-base-content/60",
            "Clerk docs: "
            DocLink { href: "https://clerk.com/docs/guides/secure/reverification", "Reverification" }
            "."
        }
    }
}

#[component]
pub fn Hooks() -> Element {
    rsx! {
        SignedOutWhenLoaded {
            fallback: rsx! { CheckingAuthPanel {} },
            RedirectToSignIn {}
            div { class: "alert alert-info shadow-sm", "Redirecting to sign in…" }
        }
        SignedIn {
            PageHeader {
                eyebrow: "Advanced · protected",
                title: "Hooks & auth state",
                intro: "This page renders only after the signed-in gate passes. The panel reads live state through use_auth, use_user, and use_session.",
            }
            ExampleSection {
                title: "use_auth / use_user / use_session",
                intro: rsx! {
                    InlineCode { "use_auth" }
                    " resolves auth facts immediately; "
                    InlineCode { "use_user" }
                    " and "
                    InlineCode { "use_session" }
                    " fill in the full clerk-js User and Session once the browser hydrates."
                },
                demo: rsx! { HooksExample {} },
                code: rsx! { Code { src: code!("src/examples/hooks.rs"), theme: snippet_theme() } },
            }
        }
    }
}

#[component]
pub fn Imperative() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Advanced",
            title: "Imperative actions",
            intro: "Drive Clerk from your own elements. use_clerk() gives fire-and-forget actions; use_auth() adds awaited try_* variants that return the error inline.",
        }
        ExampleSection {
            title: "use_clerk() and try_* variants",
            intro: rsx! {
                "Fire-and-forget actions wait for clerk-js to load and surface failures through "
                InlineCode { "use_clerk_error" }
                ". The awaited "
                InlineCode { "try_sign_out()" }
                " hands the outcome back to the caller."
            },
            demo: rsx! { ImperativeExample {} },
            code: rsx! { Code { src: code!("src/examples/imperative.rs"), theme: snippet_theme() } },
        }
    }
}

#[component]
pub fn Errors() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Advanced",
            title: "Error handling",
            intro: "Read and clear Clerk errors. Fatal init failures are terminal; recoverable ones (failed actions, config warnings) can be cleared.",
        }
        ExampleSection {
            title: "use_clerk_error and use_clear_clerk_error",
            intro: rsx! {
                "There is usually no error to show. Trigger one by, say, opening a modal before clerk-js is configured, and it will appear here with a dismiss button."
            },
            demo: rsx! { ErrorsExample {} },
            code: rsx! { Code { src: code!("src/examples/errors.rs"), theme: snippet_theme() } },
        }
        p { class: "mt-6 text-sm text-base-content/60",
            "Clerk error surfaces are described in the "
            DocLink { href: "https://clerk.com/docs/components/overview", "components overview" }
            "."
        }
    }
}
