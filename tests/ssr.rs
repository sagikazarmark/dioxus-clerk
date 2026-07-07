#[path = "support/auth.rs"]
mod auth;

use auth::sample_auth;
use dioxus_clerk::core::{InvalidTokenReason, VerificationOutcome};
use dioxus_clerk::ssr::{InitialAuthSnapshot, InitialAuthStatus, InitialState};

#[test]
fn initial_state_from_verified_auth_preserves_initial_auth_snapshot_without_loadedness() {
    let state = InitialState::from_verified_auth(Some(&sample_auth()), Some("pk_test_state"));

    assert!(state.auth.is_signed_in());
    assert_eq!(state.auth.user_id.as_deref(), Some("user_2abc"));
    assert_eq!(state.auth.session_id.as_deref(), Some("sess_2def"));
    assert_eq!(state.auth.org_id.as_deref(), Some("org_2ghi"));
    assert_eq!(state.publishable_key.as_deref(), Some("pk_test_state"));
}

#[test]
fn initial_state_from_valid_outcome_preserves_initial_auth_snapshot() {
    let outcome = VerificationOutcome::Valid(sample_auth());
    let state = InitialState::from_outcome(Some(&outcome), Some("pk_test_state"));

    assert!(state.auth.is_signed_in());
    assert_eq!(state.auth.user_id.as_deref(), Some("user_2abc"));
    assert_eq!(state.auth.session_id.as_deref(), Some("sess_2def"));
    assert_eq!(state.auth.org_id.as_deref(), Some("org_2ghi"));
    assert_eq!(state.publishable_key.as_deref(), Some("pk_test_state"));
}

#[test]
fn initial_state_from_valid_outcome_preserves_verified_gate_claims() {
    let outcome = VerificationOutcome::Valid(sample_auth());

    let state = InitialState::from_outcome(Some(&outcome), Some("pk_test_state"));

    assert_eq!(state.auth.org_role.as_deref(), Some("admin"));
    assert_eq!(state.auth.org_permissions, vec!["org:read", "org:write"]);
}

#[test]
fn ssr_initial_state_interface_carries_valid_outcome_through_state_and_safe_html() {
    let outcome = VerificationOutcome::Valid(sample_auth());

    let state = InitialState::from_outcome(Some(&outcome), Some("pk_test_state"));
    let html = state.script_html();
    let open_end = html.find('>').expect("opening tag");
    let close_start = html.rfind("</script>").expect("closing tag");
    let body = &html[open_end + 1..close_start];
    let decoded: InitialState = serde_json::from_str(body).unwrap();

    assert!(state.auth.is_signed_in());
    assert_eq!(state.auth.user_id.as_deref(), Some("user_2abc"));
    assert_eq!(state.publishable_key.as_deref(), Some("pk_test_state"));
    assert_eq!(decoded, state);
}

#[test]
fn initial_state_from_invalid_outcome_is_signed_out_initial_auth() {
    let state = InitialState::from_outcome(
        Some(&VerificationOutcome::Invalid(InvalidTokenReason::Other)),
        Some("pk"),
    );

    assert!(!state.auth.is_signed_in());
    assert!(state.auth.user_id.is_none());
    assert_eq!(state.publishable_key.as_deref(), Some("pk"));
}

#[test]
fn initial_state_from_missing_outcome_is_signed_out() {
    let state = InitialState::from_outcome(Some(&VerificationOutcome::Missing), None);

    assert!(!state.auth.is_signed_in());
    assert!(state.publishable_key.is_none());
}

#[test]
fn initial_state_from_unavailable_outcome_is_unverified_without_gate_claims() {
    let state = InitialState::from_outcome(Some(&VerificationOutcome::Unavailable), None);

    assert!(!state.auth.is_signed_in());
    assert_eq!(state.auth.status, InitialAuthStatus::Unverified);
    assert!(state.auth.org_role.is_none());
    assert!(state.auth.org_permissions.is_empty());
}

#[test]
fn initial_state_snapshot_tolerates_missing_optional_fields_on_the_wire() {
    let decoded: InitialState =
        serde_json::from_str(r#"{"auth":{"status":"signed_in","user_id":"user_2abc"}}"#).unwrap();

    assert!(decoded.auth.is_signed_in());
    assert_eq!(decoded.auth.user_id.as_deref(), Some("user_2abc"));
    assert!(decoded.auth.session_id.is_none());
    assert!(decoded.auth.org_id.is_none());
    assert!(decoded.auth.org_permissions.is_empty());
    assert!(decoded.publishable_key.is_none());
}

#[test]
fn initial_state_script_html_round_trips_and_escapes_script_close() {
    let mut snapshot = InitialAuthSnapshot::signed_in(r#"user_</script><img src=x>"#);
    snapshot.org_role = Some("<!--<script>".into());
    let state = InitialState::new(snapshot, Some("pk_test_state"));

    let html = state.script_html();
    let open_end = html.find('>').expect("opening tag");
    let close_start = html.rfind("</script>").expect("closing tag");
    let body = &html[open_end + 1..close_start];

    assert!(!body.contains('<'));
    assert!(!body.contains("</script>"));
    assert!(!body.contains("<!--"));

    let decoded: InitialState = serde_json::from_str(body).unwrap();
    assert_eq!(decoded.publishable_key.as_deref(), Some("pk_test_state"));
    assert_eq!(
        decoded.auth.user_id.as_deref(),
        Some(r#"user_</script><img src=x>"#)
    );
    assert_eq!(decoded.auth.org_role.as_deref(), Some("<!--<script>"));
}
