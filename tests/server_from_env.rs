//! Behavior tests for the `CLERK_SECRET_KEY` environment constructors.
//!
//! The process environment is global, so every test serializes on one lock
//! and restores the previous value before releasing it.

#![cfg(feature = "server")]

use dioxus_clerk::core::ClerkError;
use dioxus_clerk::server::{ClerkAuthLayer, ClerkAuthLayerConfig};
use std::sync::Mutex;

const ENV_KEY: &str = "CLERK_SECRET_KEY";

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_env(value: Option<&str>, test: impl FnOnce()) {
    let _guard = ENV_LOCK.lock().unwrap();
    let previous = std::env::var(ENV_KEY).ok();
    // SAFETY: all env mutation in this binary goes through `with_env`, which
    // holds `ENV_LOCK`, so no other thread reads or writes the environment
    // concurrently.
    match value {
        Some(value) => unsafe { std::env::set_var(ENV_KEY, value) },
        None => unsafe { std::env::remove_var(ENV_KEY) },
    }

    test();

    // SAFETY: same as above; `ENV_LOCK` is still held.
    match previous {
        Some(previous) => unsafe { std::env::set_var(ENV_KEY, previous) },
        None => unsafe { std::env::remove_var(ENV_KEY) },
    }
}

#[test]
fn layer_from_env_builds_with_present_secret_key() {
    with_env(Some("sk_test_from_env"), || {
        assert!(ClerkAuthLayer::from_env().is_ok());
    });
}

#[test]
fn layer_from_env_fails_with_missing_secret_key() {
    with_env(None, || {
        let error = ClerkAuthLayer::from_env().expect_err("missing env var must fail");
        assert!(
            matches!(&error, ClerkError::InvalidConfig(message) if message.contains(ENV_KEY)),
            "unexpected error: {error}"
        );
    });
}

#[test]
fn layer_from_env_fails_with_empty_secret_key() {
    with_env(Some(""), || {
        let error = ClerkAuthLayer::from_env().expect_err("empty secret key must fail");
        assert!(
            matches!(&error, ClerkError::InvalidConfig(message) if message.contains("empty")),
            "unexpected error: {error}"
        );
    });
}

#[test]
fn config_from_env_reads_present_secret_key_and_fails_when_missing() {
    with_env(Some("sk_test_config"), || {
        assert!(ClerkAuthLayerConfig::from_env().is_ok());
    });
    with_env(None, || {
        let error = ClerkAuthLayerConfig::from_env().expect_err("missing env var must fail");
        assert!(matches!(error, ClerkError::InvalidConfig(_)));
    });
}
