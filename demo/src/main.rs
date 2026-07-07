//! dioxus-clerk demo — binary entry point.
//!
//! The demo is a docs-by-example gallery: every page mounts a real feature
//! *and* renders that feature's own source (via the compile-time `code!`
//! macro), so the snippet you read is exactly the code that runs.
//!
//! The UI lives in [`app`] (router + shell), [`pages`] (one route each), and
//! [`examples`] (the small, pure components the pages both mount and quote).
//! These modules are only compiled for the `web`/`server` builds; the
//! Cloudflare Worker build ([`crate::worker`] in `lib.rs`) never renders pages.

#[cfg(any(feature = "web", feature = "server"))]
mod app;
#[cfg(any(feature = "web", feature = "server"))]
mod examples;
#[cfg(any(feature = "web", feature = "server"))]
mod pages;
#[cfg(any(feature = "web", feature = "server"))]
mod server_api;
#[cfg(any(feature = "web", feature = "server"))]
mod ui;

#[cfg(any(feature = "server", feature = "web"))]
fn main() {
    #[cfg(feature = "server")]
    {
        use dioxus::server::{axum, serve, DioxusRouterExt, ServeConfig};
        serve(|| async move {
            Ok(axum::Router::new()
                .serve_dioxus_application(ServeConfig::new(), app::App)
                // Plain (non-server-fn) API route used by the "Server & tokens"
                // page to demonstrate the Authorization: Bearer flow.
                .merge(server_api::whoami_router())
                .layer(clerk_auth_layer()))
        });
    }
    #[cfg(not(feature = "server"))]
    {
        dioxus::prelude::launch(app::App);
    }
}

#[cfg(not(any(feature = "server", feature = "web")))]
fn main() {}

#[cfg(feature = "server")]
fn clerk_auth_layer() -> dioxus_clerk::server::ClerkAuthLayer {
    let config = dioxus_clerk::server::ClerkAuthLayerConfig::from_env().expect("CLERK_SECRET_KEY");
    dioxus_clerk::server::ClerkAuthLayer::from_config(config).expect("default Clerk auth config")
}
