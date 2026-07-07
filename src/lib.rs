//! Clerk authentication for [Dioxus] 0.7: components, hooks, server-function
//! context readers, and SSR initial auth state for web (WASM) and fullstack
//! (Axum) apps.
//!
//! # Quick start (web-only SPA)
//!
//! Mount [`ClerkProvider`] at the app root, then gate content with [`SignedIn`]
//! / [`SignedOut`] and drop in Clerk's prebuilt widgets:
//!
//! ```rust,ignore
//! use dioxus::prelude::*;
//! use dioxus_clerk::*;
//!
//! fn App() -> Element {
//!     rsx! {
//!         ClerkProvider { publishable_key: "pk_test_...",
//!             SignedOut { SignInButton { class: "btn" } }
//!             SignedIn { UserButton {} }
//!         }
//!     }
//! }
//! ```
//!
//! # Fullstack (Axum)
//!
//! Enable the `server` feature on the native build to verify Clerk sessions in
//! `#[server]` functions with `current_auth()`, installed by the
//! `ClerkAuthLayer` tower middleware. SSR initial auth state lets the client
//! hydrate without a flash of unauthenticated content.
//!
//! ```rust,ignore
//! #[server]
//! async fn whoami() -> Result<String, ServerFnError> {
//!     use dioxus_clerk::server::current_auth;
//!     Ok(current_auth()?.user_id)
//! }
//! ```
//!
//! # Reactive hooks
//!
//! [`use_auth`], [`use_user`], and [`use_session`] expose reactive auth state to
//! any descendant of [`ClerkProvider`]; [`use_clerk`] returns a lifecycle-aware
//! action facade for imperative flows.
//!
//! # Feature flags
//!
//! | Feature | Default | Enables |
//! | --- | --- | --- |
//! | *(none)* | ✅ | Client components, hooks, guards, Clerk widgets, and SSR consumption. |
//! | `server` | | Axum middleware, extractors, `#[server]` context readers, and SSR initial-state helpers. Enable on the native server build only. |
//! | `worker` | | `server` plus `Send`-wrapped middleware futures for single-threaded Cloudflare Workers. |
//!
//! [Dioxus]: https://dioxuslabs.com

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// docs.rs passes --cfg docsrs (see [package.metadata.docs.rs]) and builds on
// nightly, where doc_cfg renders feature-requirement badges.
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod components;
pub mod core;
pub mod hooks;
pub mod options;
pub mod prelude;
pub mod ssr;

#[cfg(feature = "server")]
#[cfg_attr(docsrs, doc(cfg(feature = "server")))]
pub mod server;

// Curated crate-root re-exports. These lists are explicit (rather than glob
// re-exporting the modules) so the stable surface is reviewable and a new
// `pub` item in a module does not silently land at the crate root. Advanced
// types (`ClerkAuth`, `VerificationOutcome`, `InvalidTokenReason`) stay under
// `crate::core`; `crate::prelude` re-exports the everyday subset.
pub use components::{
    AuthButtonMode, ClerkFailed, ClerkLoaded, ClerkLoading, ClerkProvider, CreateOrganization,
    OrganizationList, OrganizationProfile, OrganizationSwitcher, Protect, RedirectToSignIn,
    RedirectToSignUp, SignIn, SignInButton, SignOutButton, SignUp, SignUpButton, SignedIn,
    SignedInWhenLoaded, SignedOut, SignedOutWhenLoaded, TaskSetupMFA, UserAvatar, UserButton,
    UserProfile, Waitlist,
};
pub use core::{
    AuthRequirement, AuthState, AuthStatus, ClerkError, OtherReverificationLevel, OtherStatus,
    OtherTaskKey, ReverificationLevel, Session, SessionStatus, SessionTask, SessionTaskKey, User,
};
pub use hooks::{
    ClerkActions, SessionState, UseAuth, UseAuthOptions, UseSession, UseUser, UserState, use_auth,
    use_auth_with_options, use_clear_clerk_error, use_clerk, use_clerk_error, use_session,
    use_user,
};
pub use options::{
    ClerkOptions, CreateOrganizationOptions, GetTokenOptions, JsonOptions, OrganizationListOptions,
    OrganizationProfileOptions, OrganizationSwitcherOptions, RedirectOptions, Routing,
    SignInOptions, SignOutOptions, SignUpOptions, TaskSetupMFAOptions, UserButtonOptions,
    UserProfileMode, UserProfileOptions, WaitlistOptions,
};
pub use reverification::{UseReverification, use_reverification};

/// Re-export of the exact [`serde_json`] this crate builds against.
///
/// [`serde_json::Value`] appears in the public prop surface of the Clerk widget
/// components (the `options`/`appearance`/`localization`/… escape hatches) and
/// in the `impl Into<serde_json::Value>` option arguments on the hooks, so
/// `serde_json` is part of this crate's semver contract. Build your option
/// values through this re-export to stay in lockstep with the version the
/// components deserialize with.
pub use serde_json;

// Client-side browser modules. `clerk_client` (emitted by build.rs) means
// wasm32 without the `worker` feature. The gate excludes `worker` rather than
// `server` so features stay additive: a browser-wasm build keeps the client
// path even when the `server` feature unifies in; only the explicit `worker`
// opt-in (server-on-wasm) drops it.
/// Reset page-scoped `Clerk.load()` state between wasm integration tests.
///
/// Hidden test-support hook: a mock whose load promise never settles (e.g. a
/// pending-load fixture) leaves the in-flight flag set, which would make the
/// next test's provider block in the load-in-flight wait loop. Real clerk-js
/// always settles, so this never matters outside tests.
///
/// Not part of the public API. The `__` prefix and `#[doc(hidden)]` mark it as
/// an internal hook the crate's own (external) wasm integration tests reach;
/// it is excluded from this crate's semver guarantees and may change or be
/// removed at any time.
#[doc(hidden)]
#[cfg(clerk_client)]
pub fn __reset_load_state() {
    crate::bridge::reset_load_state();
}

mod actions;
#[cfg(clerk_client)]
mod bindings;
#[cfg(clerk_client)]
mod bridge;
mod context;
#[cfg(clerk_client)]
mod handle;
#[cfg(clerk_client)]
mod lifecycle;
#[cfg(clerk_client)]
mod loader;
mod reverification;
// Pure publishable-key decoding for the client loader. Compiled under `test`
// too so its logic is covered by host `cargo test`, not only the CI-only wasm
// suite.
#[cfg(any(clerk_client, test))]
mod publishable_key;
#[cfg(clerk_client)]
mod ssr_document;
mod startup;
