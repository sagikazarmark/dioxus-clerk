//! Shared auth fixtures for integration tests, including canonical cases used
//! across architecture slices.

#![allow(dead_code)]

use dioxus_clerk::core::{AuthState, ClerkAuth, VerificationOutcome};
use dioxus_clerk::ssr::{InitialAuthSnapshot, InitialState};

pub fn sample_auth() -> ClerkAuth {
    let mut auth = ClerkAuth::new("user_2abc", 9_999_999_999);
    auth.session_id = Some("sess_2def".into());
    auth.org_id = Some("org_2ghi".into());
    auth.org_slug = Some("acme".into());
    auth.org_role = Some("org:admin".into());
    auth.org_permissions = vec![
        "org:dashboard:manage".into(),
        "org:dashboard:read".into(),
        "org:teams:read".into(),
    ];
    auth
}

pub fn minimal_auth() -> ClerkAuth {
    ClerkAuth::new("user_minimal", 9_999_999_999)
}

pub fn valid_outcome() -> VerificationOutcome {
    VerificationOutcome::Valid(sample_auth())
}

pub fn signed_in_state() -> AuthState {
    AuthState::from(&sample_auth())
}

pub fn signed_out_initial_state(publishable_key: Option<&str>) -> InitialState {
    InitialState::new(InitialAuthSnapshot::signed_out(), publishable_key)
}

pub fn signed_in_initial_state(publishable_key: Option<&str>) -> InitialState {
    InitialState::new(InitialAuthSnapshot::from(&sample_auth()), publishable_key)
}
