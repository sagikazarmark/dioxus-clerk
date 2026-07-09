#![cfg(not(feature = "server"))]

use dioxus::prelude::*;
use dioxus_clerk::{ClerkProvider, SignedIn, SignedOut, use_auth};
use std::sync::atomic::{AtomicBool, Ordering};

static AUTH_ASSERTED: AtomicBool = AtomicBool::new(false);
static SIGNED_OUT_RENDERED: AtomicBool = AtomicBool::new(false);
static SIGNED_IN_RENDERED: AtomicBool = AtomicBool::new(false);

/// A native render without server verification never verified anything, so it
/// must seed `loading`, rendering neither `SignedIn` nor `SignedOut`, instead
/// of flashing signed-out content at returning signed-in users.
#[test]
fn clerk_provider_without_server_feature_seeds_loading_auth() {
    AUTH_ASSERTED.store(false, Ordering::SeqCst);
    SIGNED_OUT_RENDERED.store(false, Ordering::SeqCst);
    SIGNED_IN_RENDERED.store(false, Ordering::SeqCst);

    let mut dom = VirtualDom::new(App);
    dom.rebuild_in_place();

    assert!(AUTH_ASSERTED.load(Ordering::SeqCst));
    assert!(!SIGNED_OUT_RENDERED.load(Ordering::SeqCst));
    assert!(!SIGNED_IN_RENDERED.load(Ordering::SeqCst));
}

#[component]
fn App() -> Element {
    rsx! {
        ClerkProvider { publishable_key: Some("pk_test_no_server".to_string()),
            AuthProbe {}
            SignedOut { SignedOutProbe {} }
            SignedIn { SignedInProbe {} }
        }
    }
}

#[component]
fn AuthProbe() -> Element {
    let auth = use_auth();
    let state = auth.state();

    assert!(state.is_loading());
    assert!(!state.is_signed_in());
    assert!(!state.is_signed_out());
    assert!(state.user_id().is_none());
    assert!(!state.is_loaded());

    AUTH_ASSERTED.store(true, Ordering::SeqCst);
    rsx! {}
}

#[component]
fn SignedOutProbe() -> Element {
    SIGNED_OUT_RENDERED.store(true, Ordering::SeqCst);
    rsx! {}
}

#[component]
fn SignedInProbe() -> Element {
    SIGNED_IN_RENDERED.store(true, Ordering::SeqCst);
    rsx! {}
}
