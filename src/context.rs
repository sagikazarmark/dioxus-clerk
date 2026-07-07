//! Private context object provided to descendants by `ClerkProvider`.

use crate::actions::ClerkOperation;
use crate::core::{AuthRuntimeState, ClerkError};
use dioxus::prelude::*;

const MISSING_PROVIDER_MESSAGE: &str = concat!(
    "dioxus-clerk API used outside <ClerkProvider>; ",
    "mount ClerkProvider at the app root or route-layout root before using ",
    "dioxus-clerk hooks or components",
);

/// Reactive auth context. Read by hooks and render Modules.
#[derive(Clone, Copy)]
pub(crate) struct ClerkContext {
    /// Current auth state. Initialized from SSR initial state on first render
    /// (server) or from the document reader (wasm), then updated by
    /// clerk-js listeners on the wasm side.
    pub(crate) auth: Signal<AuthRuntimeState>,
    /// Surfaces fatal failures from the wasm-side clerk-js init flow — cases
    /// where loading can no longer succeed:
    /// - no publishable key available from props or SSR seed
    /// - the clerk-js script failing to load or timing out
    /// - `Clerk.load()` promise rejection (bad publishable key, dashboard
    ///   origin not whitelisted, network, etc.)
    ///
    /// `ClerkFailed` renders on this signal, the action scheduler drops
    /// queued operations on it, and awaited actions fail fast on it — so
    /// only errors that make loading impossible belong here. Stays `None`
    /// on the SSR path and through the happy-path init.
    pub(crate) load_error: Signal<Option<ClerkError>>,
    /// Surfaces recoverable problems: failures from scheduled browser actions
    /// (fire-and-forget `open_sign_in()`-style calls and widget mounts) and
    /// non-fatal startup configuration warnings (malformed SSR seed,
    /// publishable-key mismatch). Kept separate from `load_error` so none of
    /// these unmount `ClerkLoaded` subtrees or stall the action pipeline.
    pub(crate) action_error: Signal<Option<ClerkError>>,
    /// Fire-and-forget Clerk operations queued until the Clerk lifecycle
    /// reports loaded. One queue per provider, drained in request order by
    /// the Clerk action dispatch scheduler, so operations from every hook
    /// share a single FIFO order.
    pub(crate) pending: Signal<Vec<ClerkOperation>>,
}

impl ClerkContext {
    /// The current error to surface to apps: an init/config failure wins over
    /// a transient action failure.
    pub(crate) fn current_error(&self) -> Option<ClerkError> {
        self.load_error
            .read()
            .clone()
            .or_else(|| self.action_error.read().clone())
    }
}

pub(crate) fn use_clerk_context() -> ClerkContext {
    try_use_context::<ClerkContext>().expect(MISSING_PROVIDER_MESSAGE)
}
