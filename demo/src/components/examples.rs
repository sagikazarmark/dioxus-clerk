//! Layout and state for documented live examples.

use dioxus::prelude::*;

/// How an [`ExampleSection`] arranges its live demo and source code.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum ExampleLayout {
    /// A tab switcher between the full-width demo and its full-width source.
    #[default]
    Tabbed,
    /// Demo and source side by side in a two-column grid on large screens.
    Columns,
}

/// The active pane of a tabbed [`ExampleSection`].
#[derive(Clone, Copy, PartialEq)]
enum SectionTab {
    Demo,
    Source,
}

/// A single documented example: a heading, a short explanation, the live
/// component, and the exact source that produced it.
#[component]
pub fn ExampleSection(
    #[props(into)] title: String,
    intro: Element,
    demo: Element,
    code: Element,
    #[props(default)] layout: ExampleLayout,
) -> Element {
    let mut tab = use_signal(|| SectionTab::Demo);

    let demo_frame = rsx! {
        div { class: "rounded-2xl border border-base-300 bg-base-200/40 p-5", {demo} }
    };
    let code_frame = rsx! {
        div { class: "overflow-x-auto rounded-2xl border border-base-300 bg-base-200/60 p-4 text-sm [&_pre]:!bg-transparent",
            {code}
        }
    };

    let body = match layout {
        ExampleLayout::Tabbed => rsx! {
            div { role: "tablist", class: "mt-6 tabs tabs-border",
                button {
                    role: "tab",
                    class: if tab() == SectionTab::Demo { "tab tab-active" } else { "tab" },
                    "aria-selected": tab() == SectionTab::Demo,
                    onclick: move |_| tab.set(SectionTab::Demo),
                    "Demo"
                }
                button {
                    role: "tab",
                    class: if tab() == SectionTab::Source { "tab tab-active" } else { "tab" },
                    "aria-selected": tab() == SectionTab::Source,
                    onclick: move |_| tab.set(SectionTab::Source),
                    "Source"
                }
            }
            // Keep both panes mounted so switching tabs does not reset live auth state.
            div { class: if tab() == SectionTab::Demo { "mt-4" } else { "mt-4 hidden" }, {demo_frame} }
            div { class: if tab() == SectionTab::Source { "mt-4" } else { "mt-4 hidden" }, {code_frame} }
        },
        ExampleLayout::Columns => rsx! {
            div { class: "mt-6 grid gap-6 xl:grid-cols-2",
                div { class: "min-w-0",
                    p { class: "mb-3 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Demo" }
                    {demo_frame}
                }
                div { class: "min-w-0",
                    p { class: "mb-3 text-xs font-semibold uppercase tracking-wider text-base-content/45", "Source" }
                    {code_frame}
                }
            }
        },
    };

    rsx! {
        section { class: "mt-10 rounded-[2rem] border border-base-300 bg-base-100 p-6 shadow-sm sm:p-8",
            h2 { class: "text-xl font-semibold tracking-tight", "{title}" }
            p { class: "mt-2 max-w-[70ch] text-sm leading-6 text-base-content/65", {intro} }
            {body}
        }
    }
}

/// A labeled pane within a [`DemoSurface`], with an optional status or action.
#[component]
pub fn DemoPane(
    #[props(into)] label: String,
    #[props(into, default)] accessory: Option<Element>,
    children: Element,
) -> Element {
    rsx! {
        div { class: "min-w-0",
            div { class: "mb-2 flex items-center justify-between gap-2",
                p { class: "text-xs font-semibold uppercase tracking-wider text-base-content/45",
                    "{label}"
                }
                if let Some(accessory) = accessory {
                    {accessory}
                }
            }
            {children}
        }
    }
}

/// Responsive two-pane surface for interactive demos.
#[component]
pub fn DemoSurface(primary: Element, secondary: Element) -> Element {
    rsx! {
        div { class: "@container",
            div { class: "grid gap-6 @3xl:grid-cols-2",
                {primary}
                {secondary}
            }
        }
    }
}
