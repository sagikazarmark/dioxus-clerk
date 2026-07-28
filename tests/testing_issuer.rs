#![cfg(all(feature = "server", feature = "testing"))]

//! Integration tests for the `testing` feature.
//!
//! These run minted tokens through the real `ClerkAuthLayer` verification path
//! rather than asserting on claim JSON, so the helpers stay honest: if the
//! verifier's expectations move, these fail.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use axum::{Router, routing::get};
use dioxus_clerk::server::{ClerkAuth, ClerkAuthLayer, ClerkAuthLayerConfig, VerificationOutcome};
use dioxus_clerk::testing::{TestClerk, TestIssuer, TestSession};
use tower::ServiceExt;

#[tokio::test]
async fn test_clerk_signs_in_a_user_in_two_calls() {
    let clerk = TestClerk::new().unwrap();

    let outcome =
        outcome_for_layer_and_cookie(clerk.layer().unwrap(), clerk.cookie("user_2abc").unwrap())
            .await;

    match outcome {
        TestOutcome::Valid(auth) => assert_eq!(auth.user_id, "user_2abc"),
        other => panic!("expected a signed-in user, got {other:?}"),
    }
}

#[tokio::test]
async fn test_clerk_accepts_a_bearer_header_too() {
    let clerk = TestClerk::new().unwrap();
    let layer = clerk.layer().unwrap();
    let app = outcome_app(layer);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::AUTHORIZATION, clerk.bearer("user_2abc").unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    match deserialize_outcome(response).await {
        TestOutcome::Valid(auth) => assert_eq!(auth.user_id, "user_2abc"),
        other => panic!("expected a signed-in user, got {other:?}"),
    }
}

#[tokio::test]
async fn test_clerk_customized_sessions_go_through_the_same_layer() {
    let clerk = TestClerk::new().unwrap();
    let session = clerk
        .session("user_2abc")
        .with_organization("org_2ghi")
        .with_organization_role("org:admin");

    let outcome =
        outcome_for_layer_and_cookie(clerk.layer().unwrap(), clerk.cookie_for(&session).unwrap())
            .await;

    match outcome {
        TestOutcome::Valid(auth) => {
            assert_eq!(auth.org_id.as_deref(), Some("org_2ghi"));
            assert_eq!(auth.org_role.as_deref(), Some("org:admin"));
        }
        other => panic!("expected a signed-in admin, got {other:?}"),
    }
}

#[tokio::test]
async fn test_clerk_rejects_a_token_it_did_not_mint() {
    let clerk = TestClerk::new().unwrap();
    let other = TestClerk::new().unwrap();

    let outcome =
        outcome_for_layer_and_cookie(clerk.layer().unwrap(), other.cookie("user_2abc").unwrap())
            .await;

    assert!(
        matches!(outcome, TestOutcome::Invalid),
        "each TestClerk has its own key: {outcome:?}"
    );
}

#[test]
fn test_clerk_can_wrap_an_existing_issuer_to_share_a_key() {
    let issuer = TestIssuer::generate().unwrap();
    let jwks = issuer.jwks_json().unwrap();

    let clerk = TestClerk::from_issuer(issuer);

    assert_eq!(clerk.jwks_json().unwrap(), jwks);
    assert_eq!(clerk.issuer().key_id(), "dioxus-clerk-test-key");
}

#[test]
fn test_clerk_config_can_be_hardened_before_building_a_layer() {
    let clerk = TestClerk::new().unwrap();
    let config = clerk
        .config()
        .unwrap()
        .add_authorized_party("https://app.example.com");

    assert!(ClerkAuthLayer::from_config(config).is_ok());
}

#[tokio::test]
async fn minted_token_verifies_against_the_issuer_jwks() {
    let issuer = TestIssuer::generate().unwrap();
    let session = TestSession::new("user_2abc").with_session_id("sess_2def");

    let auth = verify(&issuer, &session).await.expect("session is valid");

    assert_eq!(auth.user_id, "user_2abc");
    assert_eq!(auth.session_id.as_deref(), Some("sess_2def"));
    assert!(auth.exp > auth.iat);
}

#[tokio::test]
async fn session_id_defaults_to_a_value_derived_from_the_user_id() {
    let issuer = TestIssuer::generate().unwrap();

    let auth = verify(&issuer, &TestSession::new("user_2abc"))
        .await
        .expect("session is valid");

    assert_eq!(auth.session_id.as_deref(), Some("sess_user_2abc"));
}

#[tokio::test]
async fn organization_permissions_round_trip_through_the_v2_claim_encoding() {
    let issuer = TestIssuer::generate().unwrap();
    let session = TestSession::new("user_2abc")
        .with_organization("org_2ghi")
        .with_organization_slug("acme")
        .with_organization_role("org:admin")
        .with_organization_permissions([
            "org:dashboard:read",
            "org:dashboard:manage",
            "org:teams:read",
        ]);

    let auth = verify(&issuer, &session).await.expect("session is valid");

    assert_eq!(auth.org_id.as_deref(), Some("org_2ghi"));
    assert_eq!(auth.org_slug.as_deref(), Some("acme"));
    assert_eq!(auth.org_role.as_deref(), Some("org:admin"));

    // Order is by feature then permission, following the `fea`/`per` layout the
    // decoder walks, so compare as sets.
    let mut permissions = auth.org_permissions.clone();
    permissions.sort();
    assert_eq!(
        permissions,
        vec![
            "org:dashboard:manage",
            "org:dashboard:read",
            "org:teams:read"
        ]
    );
}

#[tokio::test]
async fn organization_role_without_a_prefix_is_normalized() {
    let issuer = TestIssuer::generate().unwrap();
    let session = TestSession::new("user_2abc")
        .with_organization("org_2ghi")
        .with_organization_role("admin");

    let auth = verify(&issuer, &session).await.expect("session is valid");

    assert_eq!(auth.org_role.as_deref(), Some("org:admin"));
}

#[tokio::test]
async fn v1_organization_claims_are_emitted_on_request() {
    let issuer = TestIssuer::generate().unwrap();
    let session = TestSession::new("user_2abc")
        .with_organization("org_2ghi")
        .with_organization_slug("acme")
        .with_organization_role("org:admin")
        .with_organization_permissions(["org:dashboard:read", "org:teams:manage"])
        .with_v1_organization_claims();

    let auth = verify(&issuer, &session).await.expect("session is valid");

    assert_eq!(auth.org_id.as_deref(), Some("org_2ghi"));
    assert_eq!(auth.org_slug.as_deref(), Some("acme"));
    assert_eq!(
        auth.org_permissions,
        vec!["org:dashboard:read", "org:teams:manage"]
    );
}

#[tokio::test]
async fn a_malformed_organization_permission_fails_at_signing_time() {
    let issuer = TestIssuer::generate().unwrap();
    let session = TestSession::new("user_2abc")
        .with_organization("org_2ghi")
        .with_organization_permissions(["dashboard:read"]);

    let error = issuer.sign(&session).expect_err("permission is malformed");

    assert!(
        error.to_string().contains("dashboard:read"),
        "error should name the offending permission: {error}"
    );
}

#[tokio::test]
async fn expired_sessions_are_rejected() {
    let issuer = TestIssuer::generate().unwrap();

    let outcome = outcome(&issuer, &TestSession::new("user_2abc").expired()).await;

    assert!(
        matches!(outcome, TestOutcome::Invalid),
        "expected an expired token to be invalid, got {outcome:?}"
    );
}

#[tokio::test]
async fn sessions_without_a_session_id_are_rejected_by_default() {
    let issuer = TestIssuer::generate().unwrap();
    let session = TestSession::new("user_2abc").without_session_id();

    let outcome = outcome(&issuer, &session).await;

    assert!(
        matches!(outcome, TestOutcome::Invalid),
        "expected a template-shaped token to be invalid, got {outcome:?}"
    );
}

#[tokio::test]
async fn a_token_from_another_issuer_is_rejected() {
    let issuer = TestIssuer::generate().unwrap();
    let other = TestIssuer::generate().unwrap();
    let token = other.sign(&TestSession::new("user_2abc")).unwrap();

    let outcome = outcome_for_token(&issuer, &token).await;

    assert!(
        matches!(outcome, TestOutcome::Invalid),
        "expected a foreign-signed token to be invalid, got {outcome:?}"
    );
}

#[tokio::test]
async fn an_unknown_key_id_is_rejected_rather_than_reported_unavailable() {
    let issuer = TestIssuer::generate().unwrap();
    // Same key material, different `kid`, so the signature would verify but no
    // key in the configured JWKS is selectable for the token's header.
    let renamed = TestIssuer::from_pem(&issuer.to_pem().unwrap())
        .unwrap()
        .with_key_id("some-other-kid");
    let token = renamed.sign(&TestSession::new("user_2abc")).unwrap();

    let outcome = outcome_for_token(&issuer, &token).await;

    assert!(
        matches!(outcome, TestOutcome::Invalid),
        "a static keyset cannot refresh, so an unknown kid is invalid, not unavailable: {outcome:?}"
    );
}

#[tokio::test]
async fn extra_claims_override_builder_claims() {
    let issuer = TestIssuer::generate().unwrap();
    let session = TestSession::new("user_2abc").with_claim("sub", "user_overridden");

    let auth = verify(&issuer, &session).await.expect("session is valid");

    assert_eq!(auth.user_id, "user_overridden");
}

#[tokio::test]
async fn configured_issuer_and_authorized_party_claims_are_enforced() {
    let issuer = TestIssuer::generate().unwrap();
    let config = ClerkAuthLayerConfig::new("")
        .with_static_jwks(issuer.jwks_json().unwrap())
        .add_issuer("https://test.clerk.accounts.dev")
        .add_authorized_party("https://app.example.com");

    let matching = issuer
        .sign(
            &TestSession::new("user_2abc")
                .with_issuer("https://test.clerk.accounts.dev")
                .with_authorized_party("https://app.example.com"),
        )
        .unwrap();
    let wrong_party = issuer
        .sign(
            &TestSession::new("user_2abc")
                .with_issuer("https://test.clerk.accounts.dev")
                .with_authorized_party("https://evil.example.com"),
        )
        .unwrap();

    assert!(matches!(
        outcome_for_config(config.clone(), &matching).await,
        TestOutcome::Valid(_)
    ));
    assert!(matches!(
        outcome_for_config(config, &wrong_party).await,
        TestOutcome::Invalid
    ));
}

#[test]
fn a_pem_round_trip_preserves_the_signing_key() {
    let issuer = TestIssuer::generate().unwrap();
    let reloaded = TestIssuer::from_pem(&issuer.to_pem().unwrap()).unwrap();

    assert_eq!(issuer.jwks_json().unwrap(), reloaded.jwks_json().unwrap());
    assert_eq!(issuer.key_id(), reloaded.key_id());
}

#[test]
fn a_generated_key_file_is_created_once_and_reused() {
    let directory = tempfile::tempdir().unwrap();
    // A nested path, so the helper has to create the parent directory too.
    let path = directory.path().join("keys").join("clerk_test_key.pem");

    let created = TestIssuer::from_pem_file_or_generate(&path).unwrap();
    assert!(path.exists(), "the key file should have been created");

    let reused = TestIssuer::from_pem_file_or_generate(&path).unwrap();
    assert_eq!(
        created.jwks_json().unwrap(),
        reused.jwks_json().unwrap(),
        "a second run should reuse the key on disk, not generate a new one"
    );

    let loaded = TestIssuer::from_pem_file(&path).unwrap();
    assert_eq!(created.jwks_json().unwrap(), loaded.jwks_json().unwrap());
}

#[test]
fn a_missing_key_file_reports_the_path() {
    let error = TestIssuer::from_pem_file("does/not/exist.pem").expect_err("file is missing");

    assert!(
        error.to_string().contains("does/not/exist.pem"),
        "error should name the path: {error}"
    );
}

#[test]
fn an_empty_static_jwks_is_rejected_at_construction() {
    let config = ClerkAuthLayerConfig::new("sk_test_unused").with_static_jwks(r#"{"keys":[]}"#);

    let error = ClerkAuthLayer::from_config(config).expect_err("an empty keyset verifies nothing");

    assert!(
        error.to_string().contains("no keys"),
        "error should explain the empty keyset: {error}"
    );
}

#[test]
fn a_malformed_static_jwks_is_rejected_at_construction() {
    let config = ClerkAuthLayerConfig::new("sk_test_unused").with_static_jwks("not json");

    assert!(ClerkAuthLayer::from_config(config).is_err());
}

#[test]
fn a_static_jwks_makes_the_secret_key_unnecessary() {
    let issuer = TestIssuer::generate().unwrap();
    let config = ClerkAuthLayerConfig::new("").with_static_jwks(issuer.jwks_json().unwrap());

    assert!(
        ClerkAuthLayer::from_config(config).is_ok(),
        "nothing is fetched, so there is no secret key to require"
    );
}

#[test]
fn an_empty_secret_key_is_still_rejected_when_keys_are_fetched() {
    let error = ClerkAuthLayer::from_config(ClerkAuthLayerConfig::new(""))
        .expect_err("fetching needs a secret key");

    assert!(error.to_string().contains("secret key"), "{error}");
}

#[test]
fn the_session_cookie_helper_produces_a_usable_cookie() {
    let issuer = TestIssuer::generate().unwrap();
    let session = TestSession::new("user_2abc");

    let cookie = issuer.session_cookie(&session).unwrap();

    assert!(cookie.starts_with("__session="));
}

/// `VerificationOutcome` is deliberately not serializable, so the handler
/// projects it onto this shuttle type to carry the decision out through the
/// response body.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum TestOutcome {
    Valid(ClerkAuth),
    Invalid,
    Missing,
    Other,
}

async fn verify(issuer: &TestIssuer, session: &TestSession) -> Option<ClerkAuth> {
    match outcome(issuer, session).await {
        TestOutcome::Valid(auth) => Some(auth),
        other => panic!("expected a valid session, got {other:?}"),
    }
}

async fn outcome(issuer: &TestIssuer, session: &TestSession) -> TestOutcome {
    let token = issuer.sign(session).expect("test token");
    outcome_for_token(issuer, &token).await
}

async fn outcome_for_token(issuer: &TestIssuer, token: &str) -> TestOutcome {
    let config =
        ClerkAuthLayerConfig::new("").with_static_jwks(issuer.jwks_json().expect("test JWKS"));
    outcome_for_config(config, token).await
}

/// Drives a token through the real layer and reports the outcome the layer put
/// on the request.
async fn outcome_for_config(config: ClerkAuthLayerConfig, token: &str) -> TestOutcome {
    let layer = ClerkAuthLayer::from_config(config).expect("test Clerk auth layer");
    outcome_for_layer_and_cookie(layer, format!("__session={token}")).await
}

async fn outcome_for_layer_and_cookie(layer: ClerkAuthLayer, cookie: String) -> TestOutcome {
    let response = outcome_app(layer)
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router response");

    assert_eq!(response.status(), StatusCode::OK);
    deserialize_outcome(response).await
}

/// A router whose only handler reports the outcome the layer inserted, encoded
/// into the response body.
fn outcome_app(layer: ClerkAuthLayer) -> Router {
    Router::new()
        .route(
            "/",
            get(|req: Request<Body>| async move {
                let outcome = match req.extensions().get::<VerificationOutcome>() {
                    Some(VerificationOutcome::Valid(auth)) => TestOutcome::Valid(auth.clone()),
                    Some(VerificationOutcome::Invalid(_)) => TestOutcome::Invalid,
                    Some(VerificationOutcome::Missing) => TestOutcome::Missing,
                    _ => TestOutcome::Other,
                };
                serde_json::to_string(&outcome).expect("serializable outcome")
            }),
        )
        .layer(layer)
}

async fn deserialize_outcome(response: Response) -> TestOutcome {
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body");

    serde_json::from_slice(&body).expect("outcome round-trips through JSON")
}
