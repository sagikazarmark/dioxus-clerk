//! Shared presentation helpers used across the demo pages.
//!
//! Nothing here touches Clerk: it is pure layout chrome so the `pages` and
//! `examples` modules can stay focused on the library being demonstrated.

use dioxus::prelude::*;
use dioxus_code::{CodeTheme, Theme};

/// Theme for every on-page code snippet. Defined once so all snippets match and
/// the palette is trivial to swap. `system()` follows the viewer's light/dark
/// preference via CSS media queries.
pub fn snippet_theme() -> CodeTheme {
    CodeTheme::system(Theme::GITHUB_LIGHT, Theme::TOKYO_NIGHT)
}

/// Consistent page heading: a small colored eyebrow, a title, and a lead
/// paragraph.
#[component]
pub fn PageHeader(
    #[props(into)] eyebrow: String,
    #[props(into)] title: String,
    #[props(into)] intro: String,
) -> Element {
    rsx! {
        header { class: "max-w-3xl",
            p { class: "text-sm font-semibold uppercase tracking-[0.18em] text-primary", "{eyebrow}" }
            h1 { class: "mt-3 text-4xl font-bold tracking-tight text-balance", "{title}" }
            p { class: "mt-4 text-lg leading-8 text-base-content/70", "{intro}" }
        }
    }
}

/// Inline monospace styling for a component or API name mentioned in prose
/// (e.g. `InlineCode { "OrganizationSwitcher" }`). Keeps the styling in one
/// place so every reference reads the same.
#[component]
pub fn InlineCode(children: Element) -> Element {
    rsx! {
        code { class: "rounded bg-base-200 px-1.5 py-0.5 font-mono text-[0.85em] text-base-content/80",
            {children}
        }
    }
}

/// A single documented example: a heading, a short explanation, the live
/// component, and the exact source that produced it.
///
/// `demo` is the live render; `code` is expected to be a `dioxus_code::Code`
/// block (this component supplies the surrounding card chrome). `intro` is an
/// `Element` so it can carry inline [`InlineCode`] and links.
///
/// By default the live demo and its source sit side by side. Set `stacked` for
/// embedded surfaces (a full `UserProfile`, `SignIn`, etc.) whose natural width
/// overflows a half-width column: the source then drops below a full-width live
/// demo instead of fighting it for horizontal space.
#[component]
pub fn ExampleSection(
    #[props(into)] title: String,
    intro: Element,
    demo: Element,
    code: Element,
    #[props(default = false)] stacked: bool,
) -> Element {
    // Stacked lays the demo and source out in a single full-width column;
    // side-by-side keeps them in a two-column grid on large screens.
    let layout_class = if stacked {
        "mt-6 grid grid-cols-1 gap-6"
    } else {
        "mt-6 grid gap-6 lg:grid-cols-2"
    };
    // A full-width embedded widget can still exceed a narrow viewport, so let
    // the live box scroll on its own rather than push the page. Triggers keep a
    // clip-free box so their dropdowns are never cut off.
    let live_class = if stacked {
        "flex flex-col items-center overflow-x-auto rounded-2xl border border-base-300 bg-base-200/40 p-5"
    } else {
        "flex flex-col items-center rounded-2xl border border-base-300 bg-base-200/40 p-5"
    };
    rsx! {
        section { class: "mt-10 rounded-[2rem] border border-base-300 bg-base-100 p-6 shadow-sm sm:p-8",
            h2 { class: "text-xl font-semibold tracking-tight", "{title}" }
            p { class: "mt-2 max-w-[70ch] text-sm leading-6 text-base-content/65", {intro} }
            div { class: "{layout_class}",
                // Live column.
                div {
                    p { class: "mb-3 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Live" }
                    div { class: "{live_class}", {demo} }
                }
                // Source column.
                div {
                    p { class: "mb-3 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Source" }
                    div { class: "overflow-x-auto rounded-2xl border border-base-300 bg-base-200/60 p-4 text-sm [&_pre]:!bg-transparent",
                        {code}
                    }
                }
            }
        }
    }
}

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
                span { class: "text-lg", "⚙️" }
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

/// Compact centered spinner for a mounting Clerk widget. Pass it as a widget's
/// `fallback` so the example bodies don't each repeat the same markup.
#[component]
pub fn Spinner() -> Element {
    rsx! {
        div { class: "grid min-h-64 place-items-center",
            span { class: "loading loading-spinner loading-md text-primary" }
        }
    }
}

/// Muted one-line status/result readout for the imperative examples. Renders
/// nothing while `status` is empty, so callers can mount it unconditionally and
/// keep the interesting logic (the action, not the display) in view.
#[component]
pub fn StatusLine(status: ReadSignal<String>) -> Element {
    if status.read().is_empty() {
        return rsx! {};
    }
    rsx! {
        p { class: "mt-3 text-sm text-base-content/70", "{status}" }
    }
}

/// Key/value readout rendered as a definition grid. Lets state-inspection
/// examples list the fields they read without repeating the grid markup.
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
