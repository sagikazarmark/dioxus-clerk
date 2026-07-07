//! Initial values consumed by `ClerkProvider` before it provides context.
//!
//! Path selection must stay additive under Cargo feature unification: a
//! browser-wasm build keeps the client path even when the `server` feature
//! leaks into its dependency graph. Only the explicit `worker` opt-in
//! (server-on-wasm, e.g. Cloudflare Workers) switches wasm to the server path.

use crate::ssr::ProviderStartup;

#[cfg(any(
    all(feature = "server", not(target_arch = "wasm32")),
    all(feature = "worker", target_arch = "wasm32")
))]
pub(crate) fn provider_startup(prop_publishable_key: Option<String>) -> ProviderStartup {
    crate::server::ssr::provider_startup_from_current_context(prop_publishable_key)
}

/// Native render without server verification: the seed read is `Missing`, so
/// the interpreted seed stays `loading` — not signed-out — and no
/// initial-state script is emitted. A returning signed-in user must not see a
/// signed-out flash from a render that never checked their session.
#[cfg(all(not(target_arch = "wasm32"), not(feature = "server")))]
pub(crate) fn provider_startup(prop_publishable_key: Option<String>) -> ProviderStartup {
    crate::ssr::provider_startup_from_read(
        crate::ssr::InitialStateRead::Missing,
        prop_publishable_key,
    )
}

#[cfg(clerk_client)]
pub(crate) fn provider_startup(prop_publishable_key: Option<String>) -> ProviderStartup {
    crate::ssr_document::startup(prop_publishable_key)
}
