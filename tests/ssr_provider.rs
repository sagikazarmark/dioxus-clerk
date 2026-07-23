#![cfg(feature = "server")]

#[path = "support/auth.rs"]
mod auth;

use auth::sample_auth;
use dioxus::prelude::*;
use dioxus_clerk::core::InvalidTokenReason;
use dioxus_clerk::server::VerificationOutcome;
use dioxus_clerk::{ClerkProvider, Protect, SignedIn, SignedOut, use_auth};
use dioxus_fullstack_core::FullstackContext;
use http::Request;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

static TEST_LOCK: Mutex<()> = Mutex::new(());
static AUTH_ASSERTED: AtomicBool = AtomicBool::new(false);
static ANONYMOUS_AUTH_ASSERTED: AtomicBool = AtomicBool::new(false);
static MISSING_OUTCOME_AUTH_ASSERTED: AtomicBool = AtomicBool::new(false);
static SIGNED_IN_RENDERED: AtomicBool = AtomicBool::new(false);
static SIGNED_OUT_RENDERED: AtomicBool = AtomicBool::new(false);
static PROTECT_RENDERED: AtomicBool = AtomicBool::new(false);
static ROLE_PROTECT_RENDERED: AtomicBool = AtomicBool::new(false);
static PERMISSION_PROTECT_RENDERED: AtomicBool = AtomicBool::new(false);
static DENIED_PROTECT_RENDERED: AtomicBool = AtomicBool::new(false);

#[test]
fn clerk_provider_seeds_server_auth_context_from_fullstack_auth() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_flags();

    let mut dom = VirtualDom::new(App);
    dom.provide_root_context(fullstack_context_with_auth());

    dom.rebuild_in_place();

    assert!(AUTH_ASSERTED.load(Ordering::SeqCst));
    assert!(SIGNED_IN_RENDERED.load(Ordering::SeqCst));
    assert!(!SIGNED_OUT_RENDERED.load(Ordering::SeqCst));
    assert!(PROTECT_RENDERED.load(Ordering::SeqCst));
    assert!(ROLE_PROTECT_RENDERED.load(Ordering::SeqCst));
    assert!(PERMISSION_PROTECT_RENDERED.load(Ordering::SeqCst));
    assert!(!DENIED_PROTECT_RENDERED.load(Ordering::SeqCst));
}

#[test]
fn clerk_provider_ignores_stale_raw_auth_when_outcome_is_invalid() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_flags();

    let mut dom = VirtualDom::new(AnonymousApp);
    dom.provide_root_context(fullstack_context_with_invalid_outcome_and_stale_auth());

    dom.rebuild_in_place();

    assert!(ANONYMOUS_AUTH_ASSERTED.load(Ordering::SeqCst));
    assert!(!SIGNED_IN_RENDERED.load(Ordering::SeqCst));
    assert!(SIGNED_OUT_RENDERED.load(Ordering::SeqCst));
    assert!(!PROTECT_RENDERED.load(Ordering::SeqCst));
}

#[test]
fn clerk_provider_treats_missing_outcome_as_anonymous() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_flags();

    let mut dom = VirtualDom::new(MissingOutcomeApp);
    dom.provide_root_context(fullstack_context_with_missing_outcome());

    dom.rebuild_in_place();

    assert!(MISSING_OUTCOME_AUTH_ASSERTED.load(Ordering::SeqCst));
    assert!(!SIGNED_IN_RENDERED.load(Ordering::SeqCst));
    assert!(SIGNED_OUT_RENDERED.load(Ordering::SeqCst));
    assert!(!PROTECT_RENDERED.load(Ordering::SeqCst));
}

#[component]
fn App() -> Element {
    rsx! {
        ClerkProvider { publishable_key: Some("pk_test_ssr".to_string()),
            AuthProbe {}
            SignedIn { SignedInProbe {} }
            SignedOut { SignedOutProbe {} }
            Protect { ProtectProbe {} }
            Protect { role: Some("org:admin".to_string()), RoleProtectProbe {} }
            Protect { permission: Some("org:dashboard:read".to_string()), PermissionProtectProbe {} }
            Protect { role: Some("org:member".to_string()), DeniedProtectProbe {} }
        }
    }
}

#[component]
fn AnonymousApp() -> Element {
    rsx! {
        ClerkProvider { publishable_key: Some("pk_test_ssr".to_string()),
            AnonymousAuthProbe {}
            SignedIn { SignedInProbe {} }
            SignedOut { SignedOutProbe {} }
            Protect { ProtectProbe {} }
        }
    }
}

#[component]
fn MissingOutcomeApp() -> Element {
    rsx! {
        ClerkProvider { publishable_key: Some("pk_test_ssr".to_string()),
            MissingOutcomeAuthProbe {}
            SignedIn { SignedInProbe {} }
            SignedOut { SignedOutProbe {} }
            Protect { ProtectProbe {} }
        }
    }
}

#[component]
fn AuthProbe() -> Element {
    let auth = use_auth();
    let state = auth.state();

    assert!(state.is_signed_in());
    assert_eq!(state.user_id(), Some("user_2abc"));
    assert_eq!(state.session_id(), Some("sess_2def"));
    assert_eq!(state.org_id(), Some("org_2ghi"));
    assert!(!state.is_loaded());

    AUTH_ASSERTED.store(true, Ordering::SeqCst);

    rsx! {}
}

#[component]
fn AnonymousAuthProbe() -> Element {
    let auth = use_auth();
    let state = auth.state();

    assert!(!state.is_signed_in());
    assert!(state.user_id().is_none());
    assert!(!state.is_loaded());

    ANONYMOUS_AUTH_ASSERTED.store(true, Ordering::SeqCst);

    rsx! {}
}

#[component]
fn MissingOutcomeAuthProbe() -> Element {
    let auth = use_auth();
    let state = auth.state();

    assert!(!state.is_signed_in());
    assert!(state.user_id().is_none());
    assert!(!state.is_loaded());

    MISSING_OUTCOME_AUTH_ASSERTED.store(true, Ordering::SeqCst);

    rsx! {}
}

#[component]
fn SignedInProbe() -> Element {
    SIGNED_IN_RENDERED.store(true, Ordering::SeqCst);
    rsx! {}
}

#[component]
fn SignedOutProbe() -> Element {
    SIGNED_OUT_RENDERED.store(true, Ordering::SeqCst);
    rsx! {}
}

#[component]
fn ProtectProbe() -> Element {
    PROTECT_RENDERED.store(true, Ordering::SeqCst);
    rsx! {}
}

#[component]
fn RoleProtectProbe() -> Element {
    ROLE_PROTECT_RENDERED.store(true, Ordering::SeqCst);
    rsx! {}
}

#[component]
fn PermissionProtectProbe() -> Element {
    PERMISSION_PROTECT_RENDERED.store(true, Ordering::SeqCst);
    rsx! {}
}

#[component]
fn DeniedProtectProbe() -> Element {
    DENIED_PROTECT_RENDERED.store(true, Ordering::SeqCst);
    rsx! {}
}

fn fullstack_context_with_auth() -> FullstackContext {
    let mut req = Request::builder().uri("/").body(()).unwrap();
    req.extensions_mut()
        .insert(VerificationOutcome::Valid(sample_auth()));
    let (parts, _body) = req.into_parts();

    FullstackContext::new(parts)
}

fn fullstack_context_with_invalid_outcome_and_stale_auth() -> FullstackContext {
    let mut req = Request::builder().uri("/").body(()).unwrap();
    req.extensions_mut().insert(sample_auth());
    req.extensions_mut()
        .insert(VerificationOutcome::Invalid(InvalidTokenReason::Other));
    let (parts, _body) = req.into_parts();

    FullstackContext::new(parts)
}

fn fullstack_context_with_missing_outcome() -> FullstackContext {
    let mut req = Request::builder().uri("/").body(()).unwrap();
    req.extensions_mut().insert(VerificationOutcome::Missing);
    let (parts, _body) = req.into_parts();

    FullstackContext::new(parts)
}

fn reset_flags() {
    AUTH_ASSERTED.store(false, Ordering::SeqCst);
    ANONYMOUS_AUTH_ASSERTED.store(false, Ordering::SeqCst);
    MISSING_OUTCOME_AUTH_ASSERTED.store(false, Ordering::SeqCst);
    SIGNED_IN_RENDERED.store(false, Ordering::SeqCst);
    SIGNED_OUT_RENDERED.store(false, Ordering::SeqCst);
    PROTECT_RENDERED.store(false, Ordering::SeqCst);
    ROLE_PROTECT_RENDERED.store(false, Ordering::SeqCst);
    PERMISSION_PROTECT_RENDERED.store(false, Ordering::SeqCst);
    DENIED_PROTECT_RENDERED.store(false, Ordering::SeqCst);
}
