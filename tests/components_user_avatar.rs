//! Behavior test for the `UserAvatar` fallback, driven through an SSR-seeded
//! provider. Pins the component wiring on top of the browser-hydrated
//! `use_user()` state.

#![cfg(feature = "server")]

#[path = "support/auth.rs"]
mod auth;

use auth::sample_auth;
use dioxus::prelude::*;
use dioxus_clerk::server::VerificationOutcome;
use dioxus_clerk::{ClerkProvider, UserAvatar};
use dioxus_fullstack_core::FullstackContext;
use http::Request;
use std::cell::RefCell;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    static RENDERED_TAGS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

#[test]
fn user_avatar_renders_fallback_without_browser_hydrated_user() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_tags();

    // An SSR-verified session carries claims but never a clerk-js `User`, so
    // the avatar must render its fallback instead of an empty `<img>`.
    let mut dom = VirtualDom::new(AvatarApp);
    dom.provide_root_context(fullstack_context(VerificationOutcome::Valid(sample_auth())));
    dom.rebuild_in_place();

    assert!(saw_tag("avatar-fallback"));
}

#[component]
fn AvatarApp() -> Element {
    rsx! {
        ClerkProvider { publishable_key: Some("pk_test_avatar".to_string()),
            UserAvatar { fallback: rsx! { RenderTag { tag: "avatar-fallback" } } }
        }
    }
}

#[component]
fn RenderTag(tag: &'static str) -> Element {
    RENDERED_TAGS.with(|tags| tags.borrow_mut().push(tag));
    rsx! {}
}

fn fullstack_context(outcome: VerificationOutcome) -> FullstackContext {
    let mut req = Request::builder().uri("/").body(()).unwrap();
    req.extensions_mut().insert(outcome);
    let (parts, _body) = req.into_parts();
    FullstackContext::new(parts)
}

fn saw_tag(tag: &'static str) -> bool {
    RENDERED_TAGS.with(|tags| tags.borrow().contains(&tag))
}

fn reset_tags() {
    RENDERED_TAGS.with(|tags| tags.borrow_mut().clear());
}
