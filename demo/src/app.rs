//! Router, provider, and the shared shell.

use dioxus::prelude::*;
use dioxus_clerk::*;

use crate::components::{DemoFooter, DemoHeader, Sidebar, SidebarNavLink, SidebarNavSection};
use crate::pages::*;

const STYLE: Asset = asset!("/build/style.css");

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
    #[route("/privacy")]
    PrivacyPolicy {},
    #[route("/terms")]
    TermsOfService {},
}

#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: STYLE }
        Router::<Route> {}
    }
}

#[component]
fn ClerkLayout() -> Element {
    // env! requires CLERK_PUBLISHABLE_KEY at compile time. Compiled into both
    // halves of the fullstack build (server + wasm), so they must match.
    let pk: Option<String> = Some(env!("CLERK_PUBLISHABLE_KEY").to_string());
    let nav = use_navigator();
    let current = use_route::<Route>();

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
                DemoHeader {
                    home: Route::Home {},
                    mark: "dc",
                    name: "dioxus-clerk",
                    github_url: "https://github.com/sagikazarmark/dioxus-clerk",
                    actions: Some(rsx! { HeaderAuthActions {} }),
                }
                div { class: "mx-auto w-full max-w-7xl lg:flex lg:gap-8 lg:px-6",
                    Sidebar {
                        SidebarNavSection { label: "Basics",
                            SidebarNavLink { route: Route::Home {}, label: "Overview" }
                            SidebarNavLink { route: Route::Minimal {}, label: "Minimal" }
                            SidebarNavLink { route: Route::Buttons {}, label: "Buttons & modes" }
                        }
                        SidebarNavSection { label: "Components",
                            SidebarNavLink {
                                route: Route::SignInPage {},
                                label: "Sign in",
                                active: Some(nav_active(&current, &Route::SignInPage {})),
                            }
                            SidebarNavLink {
                                route: Route::SignUpPage {},
                                label: "Sign up",
                                active: Some(nav_active(&current, &Route::SignUpPage {})),
                            }
                            SidebarNavLink { route: Route::ProfilePage {}, label: "Profile & avatar" }
                            SidebarNavLink { route: Route::Organizations {}, label: "Organizations" }
                            SidebarNavLink { route: Route::WaitlistPage {}, label: "Waitlist" }
                        }
                        SidebarNavSection { label: "Advanced",
                            SidebarNavLink { route: Route::Gating {}, label: "Gating & control" }
                            SidebarNavLink { route: Route::SessionTasks {}, label: "Session tasks" }
                            SidebarNavLink { route: Route::Reverification {}, label: "Reverification" }
                            SidebarNavLink { route: Route::Hooks {}, label: "Hooks & state" }
                            SidebarNavLink { route: Route::Imperative {}, label: "Imperative actions" }
                            SidebarNavLink { route: Route::Errors {}, label: "Error handling" }
                        }
                        SidebarNavSection { label: "Server",
                            SidebarNavLink { route: Route::ServerDemo {}, label: "Server & tokens" }
                        }
                    }
                    main { id: "main-content", class: "min-w-0 flex-1 px-4 py-8 sm:px-6 lg:px-0 lg:py-12",
                        ErrorBanner {}
                        Outlet::<Route> {}
                    }
                }
                DemoFooter {
                    description: "A demo for the dioxus-clerk library.",
                    links: rsx! {
                        Link { to: Route::PrivacyPolicy {}, class: "hover:text-base-content", "Privacy" }
                        Link { to: Route::TermsOfService {}, class: "hover:text-base-content", "Terms" }
                    },
                }
            }
        }
    }
}

#[component]
fn HeaderAuthActions() -> Element {
    rsx! {
        SignedOutWhenLoaded {
            // Labeled (disabled) placeholder while clerk-js loads, so the header
            // reads as a "Sign in" button rather than a bare spinner blob.
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
