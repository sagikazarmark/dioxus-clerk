//! Backend surface for the "Server & tokens" page.
//!
//! It demonstrates the two verified-request shapes Clerk supports:
//!
//! - a Dioxus **server function** authenticated by the session **cookie**, and
//! - a plain Axum route authenticated by an **`Authorization: Bearer`** token.
//!
//! The per-target `cfg` juggling lives here so the example component
//! (`examples/server_call.rs`) can present one clean call surface
//! ([`my_counter`] and [`who_am_i`]) across the native fullstack, static-web,
//! and server builds.

use dioxus::prelude::*;

/// Deterministic per-user counter computed on the server from the verified
/// Clerk user id. Called as a normal Dioxus server function: the browser sends
/// the Clerk session cookie automatically, `ClerkAuthLayer` verifies it, and
/// `current_auth()` returns the verified identity.
#[cfg(any(feature = "server", feature = "fullstack-web"))]
#[server(endpoint = "get_my_counter")]
pub async fn get_my_counter() -> Result<u64, ServerFnError> {
    use dioxus_clerk::server::current_auth;
    let auth = current_auth()?;
    Ok(auth.user_id.bytes().map(u64::from).sum())
}

/// Client-facing wrapper over the counter: calls the server function on the
/// fullstack builds, or the Worker `/api` route on the static-web build.
#[cfg(any(feature = "server", feature = "fullstack-web"))]
pub async fn my_counter() -> Result<u64, String> {
    get_my_counter().await.map_err(|error| error.to_string())
}

#[cfg(all(feature = "web", not(feature = "fullstack-web")))]
pub async fn my_counter() -> Result<u64, String> {
    fetch_u64("/api/get_my_counter").await
}

/// Call the plain `/api/whoami` route with an optional `Authorization: Bearer`
/// token and return the verified user id (or an error string). Any HTTP client
/// could make this call: that is the point of the bearer flow.
#[cfg(feature = "web")]
pub async fn who_am_i(token: Option<String>) -> Result<String, String> {
    let mut request = gloo_net::http::Request::get("/api/whoami");
    if let Some(token) = token {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let response = request.send().await.map_err(|error| error.to_string())?;
    if !response.ok() {
        return Err(format!("server returned HTTP {}", response.status()));
    }
    response.text().await.map_err(|error| error.to_string())
}

/// SSR never invokes the browser-side bearer call; this stub only exists so the
/// example component compiles for the server target.
#[cfg(all(feature = "server", not(feature = "web")))]
pub async fn who_am_i(_token: Option<String>) -> Result<String, String> {
    Err("bearer call runs in the browser".to_string())
}

#[cfg(all(feature = "web", not(feature = "fullstack-web")))]
async fn fetch_u64(url: &str) -> Result<u64, String> {
    let response = gloo_net::http::Request::post(url)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.ok() {
        return Err(format!("server returned HTTP {}", response.status()));
    }
    response
        .json::<u64>()
        .await
        .map_err(|error| error.to_string())
}

/// Router with the plain `/api/whoami` route for the native fullstack server.
/// Verifies the request through the `ClerkAuth` extractor (which reads the
/// `VerificationOutcome` that `ClerkAuthLayer` inserted) and echoes the user id.
#[cfg(feature = "server")]
pub fn whoami_router() -> dioxus::server::axum::Router {
    use dioxus::server::axum::{routing::get, Router};
    Router::new().route("/api/whoami", get(whoami_handler))
}

#[cfg(feature = "server")]
async fn whoami_handler(auth: dioxus_clerk::server::ClerkAuth) -> String {
    // On rejection, the `AuthRejection` extractor error becomes a 401/503
    // response automatically, so a successful handler always has a verified id.
    auth.user_id
}
