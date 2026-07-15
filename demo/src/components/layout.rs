//! Page and sidebar layout for docs-by-example applications.

use dioxus::prelude::*;

use super::NavLink;

const THEME_INIT_SCRIPT: &str = r#"
(() => {
  const root = document.documentElement;
  const storageKey = "demo-theme";
  let theme = null;

  try {
    theme = window.localStorage.getItem(storageKey);
  } catch {}

  if (theme !== "light" && theme !== "dark") {
    theme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }

  root.dataset.theme = theme;

  const syncToggle = () => {
    const toggle = document.querySelector("[data-theme-toggle]");
    if (!toggle) return;

    const dark = root.dataset.theme === "dark";
    const label = dark ? "Switch to light theme" : "Switch to dark theme";
    toggle.setAttribute("aria-pressed", String(dark));
    toggle.setAttribute("aria-label", label);
    toggle.setAttribute("title", label);
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", syncToggle, { once: true });
  } else {
    window.requestAnimationFrame(syncToggle);
  }
})();
"#;

const THEME_TOGGLE_SCRIPT: &str = r#"
const root = document.documentElement;
const theme = root.dataset.theme === "dark" ? "light" : "dark";
root.dataset.theme = theme;

try {
  window.localStorage.setItem("demo-theme", theme);
} catch {}

const toggle = document.querySelector("[data-theme-toggle]");
if (toggle) {
  const dark = theme === "dark";
  const label = dark ? "Switch to light theme" : "Switch to dark theme";
  toggle.setAttribute("aria-pressed", String(dark));
  toggle.setAttribute("aria-label", label);
  toggle.setAttribute("title", label);
}
"#;

/// Shared header chrome with application identity supplied by the caller.
#[component]
pub fn DemoHeader<R>(
    home: R,
    #[props(into)] mark: String,
    #[props(into)] name: String,
    #[props(into)] github_url: String,
    #[props(default)] actions: Option<Element>,
) -> Element
where
    R: Routable + PartialEq,
{
    rsx! {
        a {
            class: "sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-lg focus:bg-base-100 focus:px-4 focus:py-2 focus:font-semibold focus:shadow",
            href: "#main-content",
            "Skip to main content"
        }
        header { class: "sticky top-0 z-20 border-b border-base-300 bg-base-100/90 backdrop-blur",
            div { class: "mx-auto flex min-h-16 w-full max-w-7xl items-center justify-between gap-4 px-4 sm:px-6",
                Link {
                    to: home,
                    class: "flex min-w-0 items-center gap-3 rounded-2xl p-1.5 pr-3 transition-colors hover:bg-base-200",
                    span { class: "grid h-9 w-9 shrink-0 place-items-center rounded-2xl bg-primary text-sm font-bold text-primary-content shadow-sm",
                        "{mark}"
                    }
                    span { class: "min-w-0",
                        span { class: "block truncate text-sm font-semibold tracking-tight", "{name}" }
                    }
                }
                div { class: "flex shrink-0 items-center gap-2",
                    if let Some(actions) = actions {
                        {actions}
                    }
                    a {
                        class: "btn btn-ghost btn-sm btn-circle",
                        href: "{github_url}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "aria-label": "View {name} on GitHub",
                        title: "View on GitHub",
                        svg {
                            view_box: "0 0 24 24",
                            width: "20",
                            height: "20",
                            fill: "currentColor",
                            "aria-hidden": "true",
                            path { d: "M12 .5C5.37.5 0 5.78 0 12.29c0 5.2 3.44 9.6 8.2 11.16.6.1.82-.25.82-.56v-2c-3.34.72-4.04-1.6-4.04-1.6-.55-1.36-1.33-1.72-1.33-1.72-1.09-.73.08-.72.08-.72 1.2.08 1.84 1.22 1.84 1.22 1.07 1.8 2.8 1.28 3.49.98.1-.77.42-1.28.76-1.58-2.67-.3-5.47-1.31-5.47-5.84 0-1.29.47-2.34 1.24-3.17-.13-.3-.54-1.52.12-3.16 0 0 1.01-.32 3.3 1.21a11.6 11.6 0 0 1 6 0c2.28-1.53 3.29-1.21 3.29-1.21.66 1.64.25 2.86.12 3.16.77.83 1.24 1.88 1.24 3.17 0 4.54-2.81 5.53-5.49 5.83.43.36.81 1.09.81 2.2v3.26c0 .31.21.67.82.56A11.8 11.8 0 0 0 24 12.29C24 5.78 18.63.5 12 .5z" }
                        }
                    }
                    ThemeSwitcher {}
                }
            }
        }
    }
}

#[component]
fn ThemeSwitcher() -> Element {
    rsx! {
        document::Script { "{THEME_INIT_SCRIPT}" }
        button {
            class: "btn btn-ghost btn-sm btn-circle",
            r#type: "button",
            "data-theme-toggle": "",
            title: "Toggle light and dark theme",
            aria_label: "Toggle light and dark theme",
            onclick: move |_| {
                let _ = document::eval(THEME_TOGGLE_SCRIPT);
            },
            svg {
                class: "theme-toggle-light size-5",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                "stroke-width": "2",
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                "aria-hidden": "true",
                circle { cx: "12", cy: "12", r: "4" }
                path { d: "M12 2v2M12 20v2M4.93 4.93l1.42 1.42M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" }
            }
            svg {
                class: "theme-toggle-dark size-5",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                "stroke-width": "2",
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                "aria-hidden": "true",
                path { d: "M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79Z" }
            }
        }
    }
}

/// Shared footer chrome with application copy and links supplied by the caller.
#[component]
pub fn DemoFooter(#[props(into)] description: String, links: Element) -> Element {
    rsx! {
        footer { class: "mt-8 border-t border-base-300",
            div { class: "mx-auto flex w-full max-w-7xl flex-col items-center justify-between gap-3 px-4 py-6 text-sm text-base-content/55 sm:flex-row sm:px-6",
                span { "{description}" }
                div { class: "flex items-center gap-4", {links} }
            }
        }
    }
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

/// Navigation composed from one or more [`SidebarNavSection`] children.
///
/// The sections form a horizontal strip on small screens and a sticky sidebar
/// on large screens.
#[component]
pub fn Sidebar(children: Element) -> Element {
    rsx! {
        aside { class: "border-b border-base-300 bg-base-100 lg:w-60 lg:shrink-0 lg:border-b-0",
            nav {
                "aria-label": "Demo navigation",
                class: "flex gap-1 overflow-x-auto px-4 py-2 sm:px-6 lg:sticky lg:top-24 lg:max-h-[calc(100vh-7rem)] lg:block lg:space-y-6 lg:overflow-y-auto lg:px-0 lg:py-8 lg:pr-2",
                {children}
            }
        }
    }
}

/// A labeled group of links within a [`Sidebar`].
#[component]
pub fn SidebarNavSection(#[props(into)] label: String, children: Element) -> Element {
    rsx! {
        div { class: "shrink-0 lg:block",
            p { class: "hidden px-3 text-xs font-semibold uppercase tracking-wider text-base-content/45 lg:block",
                "{label}"
            }
            ul { class: "flex gap-1 lg:mt-2 lg:block lg:space-y-0.5", {children} }
        }
    }
}

/// Sidebar presentation around a route-aware [`NavLink`], including its
/// optional active-state override.
#[component]
pub fn SidebarNavLink<R>(
    route: R,
    #[props(into)] label: String,
    #[props(default)] active: Option<bool>,
) -> Element
where
    R: Routable + PartialEq,
{
    rsx! {
        li {
            NavLink {
                route,
                label,
                active,
                class: "block whitespace-nowrap rounded-lg px-3 py-1.5 text-sm text-base-content/75 transition-colors hover:bg-base-200 hover:text-base-content lg:whitespace-normal",
            }
        }
    }
}
