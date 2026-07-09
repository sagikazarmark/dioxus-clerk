//! Router, provider, and the shared shell (header + grouped sidebar).

use dioxus::prelude::*;
use dioxus_clerk::*;

use crate::pages::*;

const TAILWIND_CSS: Asset = asset!("/assets/style.css");

/// Every page hangs off the one `ClerkLayout`, so there is a single
/// `ClerkProvider` for the whole app. The two `:..segments` routes let Clerk's
/// path-routed sign-in/sign-up widgets own their OAuth/SSO callback sub-paths.
#[derive(Routable, Clone, PartialEq, Debug)]
pub enum Route {
    #[layout(ClerkLayout)]
    #[route("/")]
    Home {},
    #[route("/minimal")]
    Minimal {},
    #[route("/buttons")]
    Buttons {},
    #[route("/sign-in")]
    SignInPage {},
    #[route("/sign-in/:..segments")]
    SignInCallbackPage { segments: Vec<String> },
    #[route("/sign-up")]
    SignUpPage {},
    #[route("/sign-up/:..segments")]
    SignUpCallbackPage { segments: Vec<String> },
    #[route("/profile")]
    ProfilePage {},
    #[route("/organizations")]
    Organizations {},
    #[route("/waitlist")]
    WaitlistPage {},
    #[route("/gating")]
    Gating {},
    #[route("/session-tasks")]
    SessionTasks {},
    #[route("/reverification")]
    Reverification {},
    #[route("/hooks")]
    Hooks {},
    #[route("/imperative")]
    Imperative {},
    #[route("/errors")]
    Errors {},
    #[route("/server")]
    ServerDemo {},
}

/// Grouped navigation shared by the desktop sidebar and the mobile strip.
fn nav_groups() -> Vec<(&'static str, Vec<(Route, &'static str)>)> {
    vec![
        (
            "Basics",
            vec![
                (Route::Home {}, "Overview"),
                (Route::Minimal {}, "Minimal"),
                (Route::Buttons {}, "Buttons & modes"),
            ],
        ),
        (
            "Components",
            vec![
                (Route::SignInPage {}, "Sign in"),
                (Route::SignUpPage {}, "Sign up"),
                (Route::ProfilePage {}, "Profile & avatar"),
                (Route::Organizations {}, "Organizations"),
                (Route::WaitlistPage {}, "Waitlist"),
            ],
        ),
        (
            "Advanced",
            vec![
                (Route::Gating {}, "Gating & control"),
                (Route::SessionTasks {}, "Session tasks"),
                (Route::Reverification {}, "Reverification"),
                (Route::Hooks {}, "Hooks & state"),
                (Route::Imperative {}, "Imperative actions"),
                (Route::Errors {}, "Error handling"),
            ],
        ),
        ("Server", vec![(Route::ServerDemo {}, "Server & tokens")]),
    ]
}

#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

#[component]
fn ClerkLayout() -> Element {
    // env! requires CLERK_PUBLISHABLE_KEY at compile time. Compiled into both
    // halves of the fullstack build (server + wasm), so they must match.
    let pk: Option<String> = Some(env!("CLERK_PUBLISHABLE_KEY").to_string());
    let nav = use_navigator();

    rsx! {
        ClerkProvider {
            publishable_key: pk,
            sign_in_url: "/sign-in",
            sign_up_url: "/sign-up",
            sign_in_fallback_redirect_url: "/hooks",
            sign_up_fallback_redirect_url: "/hooks",
            router_push: move |path: String| { let _ = nav.push(path); },
            router_replace: move |path: String| { let _ = nav.replace(path); },
            div { class: "min-h-screen bg-base-100 text-base-content",
                Header {}
                MobileNav {}
                div { class: "mx-auto flex w-full max-w-7xl gap-8 px-4 sm:px-6",
                    Sidebar {}
                    main { class: "min-w-0 flex-1 py-8 lg:py-12",
                        ErrorBanner {}
                        Outlet::<Route> {}
                    }
                }
            }
        }
    }
}

#[component]
fn Header() -> Element {
    rsx! {
        header { class: "sticky top-0 z-20 border-b border-base-300 bg-base-100/90 backdrop-blur",
            div { class: "mx-auto flex min-h-16 w-full max-w-7xl items-center justify-between gap-4 px-4 sm:px-6",
                Link {
                    to: Route::Home {},
                    class: "flex min-w-0 items-center gap-3 rounded-2xl p-1.5 pr-3 transition-colors hover:bg-base-200",
                    span { class: "grid h-9 w-9 shrink-0 place-items-center rounded-2xl bg-primary text-sm font-bold text-primary-content shadow-sm", "dc" }
                    span { class: "min-w-0",
                        span { class: "block truncate text-sm font-semibold tracking-tight", "dioxus-clerk" }
                        span { class: "hidden text-xs text-base-content/60 sm:block", "Docs by example" }
                    }
                }
                div { class: "flex shrink-0 items-center gap-2",
                    SignedOutWhenLoaded {
                        // Labeled (disabled) placeholder while clerk-js loads, so
                        // the header reads as a "Sign in" button rather than a
                        // bare spinner blob.
                        fallback: rsx! {
                            button { class: "btn btn-primary btn-sm shadow-sm", disabled: true,
                                span { class: "loading loading-spinner loading-xs" }
                                "Sign in"
                            }
                        },
                        SignInButton { mode: AuthButtonMode::Modal, class: "btn btn-primary btn-sm shadow-sm", "Sign in" }
                        Link { to: Route::SignUpPage {}, class: "btn btn-ghost btn-sm hidden sm:inline-flex", "Sign up" }
                    }
                    SignedIn {
                        UserButton { fallback: rsx! { span { class: "loading loading-spinner loading-sm text-primary" } } }
                        SignOutButton { class: "btn btn-ghost btn-sm", redirect_url: "/", "Sign out" }
                    }
                }
            }
        }
    }
}

/// Whether `target` should be highlighted given the `current` route. Compares
/// parsed `Route`s (not raw URLs), so Clerk's `?redirect_url=…` query string on
/// the sign-in/up pages doesn't defeat the match. The sign-in/up callback
/// sub-routes highlight their base nav entry too.
fn nav_active(current: &Route, target: &Route) -> bool {
    match target {
        Route::SignInPage {} => {
            matches!(
                current,
                Route::SignInPage {} | Route::SignInCallbackPage { .. }
            )
        }
        Route::SignUpPage {} => {
            matches!(
                current,
                Route::SignUpPage {} | Route::SignUpCallbackPage { .. }
            )
        }
        _ => current == target,
    }
}

/// A nav `Link` whose active styling is driven by the parsed route rather than
/// the raw URL (which `Link`'s built-in `active_class` keys on, and which query
/// params break).
#[component]
fn NavLink(route: Route, #[props(into)] label: String, #[props(into)] class: String) -> Element {
    let current = use_route::<Route>();
    let class = if nav_active(&current, &route) {
        format!("{class} bg-primary/10 font-semibold text-primary")
    } else {
        class
    };
    rsx! {
        Link { to: route, class: "{class}", "{label}" }
    }
}

#[component]
fn Sidebar() -> Element {
    rsx! {
        aside { class: "hidden w-56 shrink-0 lg:block",
            nav { class: "sticky top-24 space-y-6 py-8",
                for (section , items) in nav_groups() {
                    div {
                        p { class: "px-3 text-xs font-semibold uppercase tracking-wider text-base-content/45", "{section}" }
                        ul { class: "mt-2 space-y-0.5",
                            for (route , label) in items {
                                li {
                                    NavLink {
                                        route,
                                        label,
                                        class: "block rounded-lg px-3 py-1.5 text-sm text-base-content/75 transition-colors hover:bg-base-200 hover:text-base-content",
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn MobileNav() -> Element {
    rsx! {
        nav { class: "border-b border-base-300 bg-base-100 lg:hidden",
            div { class: "mx-auto flex w-full max-w-7xl gap-1 overflow-x-auto px-4 py-2 sm:px-6",
                for (_section , items) in nav_groups() {
                    for (route , label) in items {
                        NavLink {
                            route,
                            label,
                            class: "whitespace-nowrap rounded-lg px-3 py-1.5 text-sm text-base-content/75 hover:bg-base-200",
                        }
                    }
                }
            }
        }
    }
}

/// Global banner for fatal Clerk init errors. Per-page error handling is
/// demonstrated on the "Error handling" page via `use_clerk_error`.
#[component]
fn ErrorBanner() -> Element {
    let err = use_clerk_error();
    let err = err.read();
    let Some(err) = err.as_ref() else {
        return rsx! {};
    };
    rsx! {
        div { class: "alert alert-error mb-6 shadow-sm",
            span { "Auth init failed: {err}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Route;

    #[test]
    fn auth_routes_accept_clerk_callback_paths() {
        assert!("/sign-in/sso-callback".parse::<Route>().is_ok());
        assert!("/sign-up/sso-callback".parse::<Route>().is_ok());
    }
}
