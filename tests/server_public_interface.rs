#![cfg(feature = "server")]

use axum::http::StatusCode;
use axum::response::IntoResponse;
use dioxus_clerk::ClerkError;
use dioxus_clerk::server::{
    AuthRejection, ClerkAuth, ClerkAuthLayer, ClerkAuthLayerConfig, VerificationOutcome,
    current_auth, current_auth_opt, current_outcome,
};

#[test]
fn server_module_exposes_server_setup_interface() {
    let _layer = ClerkAuthLayer::new("sk_test_unused").expect("default config is valid");
    let _from_env: fn() -> Result<ClerkAuthLayer, dioxus_clerk::core::ClerkError> =
        ClerkAuthLayer::from_env;
    let _config = ClerkAuthLayerConfig::new("sk_test_unused")
        .with_backend_api_base_url("https://api.clerk.com/v1")
        .with_authorized_parties(["https://example.com"])
        .add_authorized_party("https://admin.example.com")
        .add_audience("api://default")
        .add_issuer("https://example.clerk.accounts.dev");
    let _config_from_env: fn() -> Result<ClerkAuthLayerConfig, dioxus_clerk::core::ClerkError> =
        ClerkAuthLayerConfig::from_env;

    let mut auth = ClerkAuth::new("user_2abc", 9_999_999_999);
    auth.session_id = Some("sess_2def".into());
    let outcome = VerificationOutcome::Valid(auth);
    assert!(matches!(&outcome, VerificationOutcome::Valid(_)));

    let _current_auth: fn() -> Result<ClerkAuth, dioxus_clerk::core::ClerkError> = current_auth;
    let _current_auth_opt: fn() -> Result<Option<ClerkAuth>, dioxus_clerk::core::ClerkError> =
        current_auth_opt;
    let _current_outcome: fn() -> Option<VerificationOutcome> = current_outcome;
    assert_eq!(
        AuthRejection::Missing.into_response().status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        AuthRejection::Invalid.into_response().status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        AuthRejection::Unavailable.into_response().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[test]
fn clerk_errors_convert_to_server_fn_errors() {
    let error: dioxus_fullstack_core::ServerFnError = ClerkError::Unauthenticated.into();
    let status: axum::http::StatusCode = error.into();

    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}
