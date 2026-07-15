//! Reusable presentation components for docs-by-example applications.
//!
//! This module deliberately has no dependency on the application or the
//! library being demonstrated, so it can move into a shared crate later.

use dioxus::prelude::*;

/// An external action rendered by a [`DocsCallout`].
#[derive(Clone, PartialEq)]
pub struct ExternalAction {
    label: String,
    href: String,
}

impl ExternalAction {
    pub fn new(label: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: href.into(),
        }
    }
}

/// Read-only source panel shared by examples that show input next to controls
/// or runtime state.
#[component]
pub fn SourcePanel(#[props(into)] source: String) -> Element {
    rsx! {
        pre { class: "overflow-x-auto rounded-xl border border-base-300 bg-base-100 p-3 font-mono text-xs",
            "{source}"
        }
    }
}

/// A callout pointing at the doc that owns a feature, plus optional extra notes.
#[component]
pub fn DocsCallout(
    #[props(into)] title: String,
    #[props(default)] action: Option<ExternalAction>,
    children: Element,
) -> Element {
    rsx! {
        div { class: "mt-8 rounded-2xl border border-info/40 bg-info/5 p-5",
            div { class: "flex items-center gap-2",
                span { class: "text-sm font-semibold uppercase tracking-wider text-info", "Docs" }
                p { class: "font-semibold text-base-content", "{title}" }
            }
            div { class: "mt-2 max-w-[70ch] text-sm leading-6 text-base-content/70", {children} }
            if let Some(action) = action {
                div { class: "mt-4",
                    a {
                        class: "btn btn-sm btn-outline btn-info",
                        href: "{action.href}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "{action.label} ↗"
                    }
                }
            }
        }
    }
}

/// Muted one-line status/result readout for interactive examples. Renders
/// nothing while `status` is empty so callers can mount it unconditionally.
#[component]
pub fn StatusLine(#[props(into)] status: String) -> Element {
    if status.is_empty() {
        return rsx! {};
    }
    rsx! {
        p {
            role: "status",
            class: "mt-3 rounded-lg bg-base-100 px-3 py-2 text-sm text-base-content/75",
            "{status}"
        }
    }
}

/// Small status chip used next to previews and state panels.
#[component]
pub fn StatusChip(#[props(into)] label: String) -> Element {
    rsx! {
        span { class: "badge badge-ghost badge-sm font-mono", "{label}" }
    }
}
