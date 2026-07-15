//! Route-aware navigation for docs-by-example applications.

use dioxus::prelude::*;

/// A nav [`Link`] whose active styling is driven by the parsed route.
/// `active` can override exact equality for parent links with callback or child
/// routes.
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
    let is_active = active.unwrap_or(current == route);
    let class = if is_active {
        format!("{class} bg-primary/10 font-semibold text-primary")
    } else {
        class
    };
    rsx! {
        Link {
            to: route,
            class: "{class}",
            "aria-current": is_active.then_some("page"),
            "{label}"
        }
    }
}
