//! Route-aware navigation for docs-by-example applications.

use dioxus::prelude::*;

/// A nav [`Link`] whose active styling is driven by the parsed route.
#[component]
pub fn NavLink<R>(
    route: R,
    #[props(into)] label: String,
    #[props(into, default)] class: String,
    #[props(default)] active: Option<bool>,
) -> Element
where
    R: Routable + PartialEq,
{
    let current = use_route::<R>();
    let class = if active.unwrap_or(current == route) {
        format!("{class} bg-primary/10 font-semibold text-primary")
    } else {
        class
    };
    rsx! {
        Link { to: route, class: "{class}", "{label}" }
    }
}
