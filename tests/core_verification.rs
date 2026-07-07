#[path = "support/auth.rs"]
mod auth;

use auth::sample_auth;
use dioxus_clerk::core::{InvalidTokenReason, VerificationOutcome};

#[test]
fn valid_outcome_exposes_auth() {
    let outcome = VerificationOutcome::Valid(sample_auth());

    let auth = outcome.auth().expect("valid outcome carries auth");

    assert_eq!(auth.user_id, "user_2abc");
    assert_eq!(auth.session_id.as_deref(), Some("sess_2def"));
}

#[test]
fn valid_outcome_converts_into_auth() {
    let outcome = VerificationOutcome::Valid(sample_auth());

    let auth = outcome.into_auth().expect("valid outcome carries auth");

    assert_eq!(auth.user_id, "user_2abc");
}

#[test]
fn anonymous_outcomes_do_not_expose_auth() {
    assert!(VerificationOutcome::Missing.auth().is_none());
    assert!(
        VerificationOutcome::Invalid(InvalidTokenReason::Other)
            .auth()
            .is_none()
    );
    assert!(VerificationOutcome::Unavailable.auth().is_none());
    assert!(VerificationOutcome::Missing.into_auth().is_none());
}
