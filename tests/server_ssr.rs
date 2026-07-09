#![cfg(feature = "server")]

#[path = "support/auth.rs"]
mod auth;

use auth::sample_auth;
use axum::http::Request;
use dioxus_clerk::core::InvalidTokenReason;
use dioxus_clerk::server::{
    ClerkAuth, VerificationOutcome,
    ssr::{
        initial_state, initial_state_from_current_context, initial_state_script,
        initial_state_script_from_current_context,
    },
};
use dioxus_clerk::ssr::InitialAuthStatus;
use dioxus_fullstack_core::FullstackContext;

/// Returns the JSON body of the `<script id="__clerk_initial_state" type="application/json">…</script>`
/// fragment, i.e. everything between the opening tag and the trailing `</script>`.
/// The trailing `</script>` is the legitimate closing tag and is excluded so the
/// remaining slice can be checked for `</script>` injection inside the payload.
fn body(html: &str) -> &str {
    let open_end = html.find('>').expect("opening tag");
    let close_start = html.rfind("</script>").expect("closing tag");
    &html[open_end + 1..close_start]
}

#[test]
fn ssr_script_is_valid_html_and_contains_user_id() {
    let outcome = VerificationOutcome::Valid(sample_auth());
    let html = initial_state_script(Some(&outcome), Some("pk_test_xxx"));
    assert!(html.contains(r#"id="__clerk_initial_state""#));
    assert!(html.contains(r#"type="application/json""#));
    assert!(html.contains("user_2abc"));
    assert!(html.contains("org_2ghi"));
    assert!(html.contains("pk_test_xxx"));
    // No HTML injection: only the legitimate trailing `</script>` is present.
    assert!(!body(&html).contains("</script>"));
}

#[test]
fn ssr_script_without_outcome_is_unverified() {
    let html = initial_state_script(None, Some("pk_test_xxx"));
    assert!(html.contains(r#""status":"unverified""#));
    assert!(html.contains("pk_test_xxx"));
}

#[test]
fn ssr_script_for_invalid_outcome_is_signed_out() {
    let html = initial_state_script(
        Some(&VerificationOutcome::Invalid(InvalidTokenReason::Other)),
        Some("pk_test_xxx"),
    );
    assert!(html.contains(r#""status":"signed_out""#));
    assert!(!html.contains("user_2abc"));
    assert!(html.contains("pk_test_xxx"));
}

#[test]
fn ssr_initial_state_from_explicit_outcome_preserves_valid_auth() {
    let outcome = VerificationOutcome::Valid(sample_auth());

    let state = initial_state(Some(&outcome), Some("pk_test_xxx"));

    assert!(state.auth.is_signed_in());
    assert_eq!(state.auth.user_id.as_deref(), Some("user_2abc"));
    assert_eq!(state.publishable_key.as_deref(), Some("pk_test_xxx"));
}

#[test]
fn ssr_initial_state_preserves_verified_gate_claims() {
    let outcome = VerificationOutcome::Valid(sample_auth());

    let state = initial_state(Some(&outcome), Some("pk_test_xxx"));

    assert_eq!(state.auth.org_role.as_deref(), Some("admin"));
    assert_eq!(state.auth.org_permissions, vec!["org:read", "org:write"]);
}

#[tokio::test]
async fn ssr_initial_state_from_current_context_uses_valid_outcome() {
    let cx = fullstack_context_with_outcome(VerificationOutcome::Valid(sample_auth()));

    let state = cx
        .scope(async { initial_state_from_current_context(Some("pk_test_xxx")) })
        .await;

    assert!(state.auth.is_signed_in());
    assert_eq!(state.auth.user_id.as_deref(), Some("user_2abc"));
    assert_eq!(state.publishable_key.as_deref(), Some("pk_test_xxx"));
}

#[tokio::test]
async fn ssr_initial_state_from_current_context_maps_outcomes_to_three_state_seed() {
    let cases = [
        (
            VerificationOutcome::Missing,
            InitialAuthStatus::SignedOut,
            None,
        ),
        (
            VerificationOutcome::Invalid(InvalidTokenReason::Other),
            InitialAuthStatus::SignedOut,
            None,
        ),
        (
            VerificationOutcome::Unavailable,
            InitialAuthStatus::Unverified,
            None,
        ),
        (
            VerificationOutcome::Valid(sample_auth()),
            InitialAuthStatus::SignedIn,
            Some("user_2abc"),
        ),
    ];

    for (outcome, expected_status, expected_user_id) in cases {
        let cx = fullstack_context_with_outcome(outcome);

        let state = cx
            .scope(async { initial_state_from_current_context(Some("pk_test_xxx")) })
            .await;

        assert_eq!(state.auth.status, expected_status);
        assert_eq!(state.auth.user_id.as_deref(), expected_user_id);
        assert_eq!(state.publishable_key.as_deref(), Some("pk_test_xxx"));
    }
}

#[tokio::test]
async fn ssr_script_from_current_context_treats_invalid_as_signed_out() {
    let cx =
        fullstack_context_with_outcome(VerificationOutcome::Invalid(InvalidTokenReason::Other));

    let html = cx
        .scope(async { initial_state_script_from_current_context(Some("pk_test_xxx")) })
        .await;

    assert!(html.contains(r#""status":"signed_out""#));
    assert!(!html.contains("user_2abc"));
    assert!(html.contains("pk_test_xxx"));
}

#[test]
fn ssr_script_from_current_context_without_context_is_unverified() {
    let html = initial_state_script_from_current_context(Some("pk_test_xxx"));

    assert!(html.contains(r#""status":"unverified""#));
    assert!(html.contains("pk_test_xxx"));
}

#[test]
fn ssr_script_escapes_script_close_in_user_id() {
    let auth = ClerkAuth::new(r#"u_</script><img src=x>"#, 9_999_999_999);
    let outcome = VerificationOutcome::Valid(auth);
    let html = initial_state_script(Some(&outcome), None);
    // No raw `<` may appear in the JSON body: only the legitimate closing
    // tag is allowed. This blocks both `</script>` breakout and
    // `<!--<script>` script-data double-escaping.
    assert!(!body(&html).contains('<'));
    // Sanity: the canonical defense rewrites `<` to the `<` JSON escape.
    assert!(body(&html).contains("\\u003c/script>"));
}

fn fullstack_context_with_outcome(outcome: VerificationOutcome) -> FullstackContext {
    let mut req = Request::builder().uri("/").body(()).unwrap();
    req.extensions_mut().insert(outcome);
    let (parts, _body) = req.into_parts();

    FullstackContext::new(parts)
}
