//! Clerk-specific presentation helpers.

use dioxus::prelude::*;

/// A callout for features that require Clerk Dashboard configuration before
/// they do anything. Links straight to the relevant setting and docs.
#[component]
pub fn SetupCallout(
    #[props(into)] title: String,
    #[props(into)] dashboard_label: String,
    #[props(into)] dashboard_url: String,
    #[props(into)] docs_label: String,
    #[props(into)] docs_url: String,
    children: Element,
) -> Element {
    rsx! {
        div { class: "mt-8 rounded-2xl border border-warning/40 bg-warning/5 p-5",
            div { class: "flex items-center gap-2",
                span { class: "text-lg", "Setup" }
                p { class: "font-semibold text-base-content", "{title}" }
            }
            div { class: "mt-2 max-w-[70ch] text-sm leading-6 text-base-content/70", {children} }
            div { class: "mt-4 flex flex-wrap gap-3",
                a {
                    class: "btn btn-sm btn-outline btn-warning",
                    href: "{dashboard_url}",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    "{dashboard_label} ↗"
                }
                a {
                    class: "btn btn-sm btn-ghost",
                    href: "{docs_url}",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    "{docs_label} ↗"
                }
            }
        }
    }
}

/// Inline link to external Clerk documentation.
#[component]
pub fn DocLink(#[props(into)] href: String, children: Element) -> Element {
    rsx! {
        a {
            class: "link link-primary",
            href: "{href}",
            target: "_blank",
            rel: "noopener noreferrer",
            {children}
        }
    }
}

/// Compact centered spinner for a mounting Clerk widget.
#[component]
pub fn Spinner() -> Element {
    rsx! {
        div { class: "grid min-h-64 place-items-center",
            span { class: "loading loading-spinner loading-md text-primary" }
        }
    }
}

/// Key/value readout rendered as a definition grid.
#[component]
pub fn StateGrid(rows: Vec<(&'static str, String)>) -> Element {
    rsx! {
        dl { class: "grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 font-mono text-sm",
            for (label , value) in rows {
                dt { class: "text-base-content/55", "{label}" }
                dd { class: "break-all", "{value}" }
            }
        }
    }
}

/// Placeholder shown while a Clerk-hosted widget mounts.
#[component]
pub fn WidgetFallback(#[props(into)] label: String) -> Element {
    rsx! {
        div { class: "grid min-h-64 place-items-center rounded-2xl border border-dashed border-base-300 bg-base-100/70 px-6 text-center",
            div {
                span { class: "loading loading-spinner loading-md text-primary" }
                p { class: "mt-4 text-sm font-medium text-base-content", "{label}" }
                p { class: "mt-2 text-xs text-base-content/55", "Clerk is loading the hosted UI." }
            }
        }
    }
}

/// Full-panel spinner used by pages that wait out auth resolution before
/// deciding between signed-in and signed-out content.
#[component]
pub fn CheckingAuthPanel() -> Element {
    rsx! {
        section { class: "grid min-h-[40vh] place-items-center",
            div { class: "rounded-[2rem] border border-base-300 bg-base-100 p-8 text-center shadow-sm",
                span { class: "loading loading-spinner loading-md text-primary" }
                h1 { class: "mt-4 text-2xl font-bold tracking-tight", "Checking auth state" }
                p { class: "mt-3 max-w-[42ch] text-sm leading-6 text-base-content/65",
                    "Finalizing the Clerk session before showing signed-in or signed-out content."
                }
            }
        }
    }
}

/// Small inline "checking auth" chip for use inside signed-out fallbacks.
#[component]
pub fn CheckingChip() -> Element {
    rsx! {
        div { class: "flex w-fit items-center gap-3 rounded-2xl border border-base-300 bg-base-200/70 px-4 py-3 text-sm text-base-content/70",
            span { class: "loading loading-spinner loading-sm text-primary" }
            span { "Checking auth state" }
        }
    }
}
