//! Code presentation for docs-by-example applications.

use dioxus::prelude::*;
use dioxus_code::{CodeTheme, Theme};

/// Theme for every on-page code snippet. Defined once so all snippets match and
/// the palette is trivial to swap. `system()` supplies the initial media-query
/// preference; the demo shell overrides its CSS variables for an explicit theme
/// choice. Pair with the compile-time `code!` macro, so the highlighted snippet
/// shown is exactly the code that runs.
pub fn snippet_theme() -> CodeTheme {
    CodeTheme::system(Theme::GITHUB_LIGHT, Theme::TOKYO_NIGHT)
}

/// Inline monospace styling for an API name or identifier mentioned in prose
/// (e.g. `InlineCode { "use_example" }`). Keeps the styling in one place
/// so every reference reads the same.
#[component]
pub fn InlineCode(children: Element) -> Element {
    rsx! {
        code { class: "rounded bg-base-200 px-1.5 py-0.5 font-mono text-[0.85em] text-base-content/80",
            {children}
        }
    }
}
