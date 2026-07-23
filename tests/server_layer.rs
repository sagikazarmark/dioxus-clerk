#![cfg(feature = "server")]

//! Integration tests for `ClerkAuthLayer`.

#[path = "support/auth.rs"]
mod auth;

use auth::sample_auth;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use axum::{Router, routing::get};
use dioxus_clerk::server::{
    ClerkAuth, ClerkAuthLayer, ClerkAuthLayerConfig, InvalidTokenReason, VerificationOutcome,
};
use std::convert::Infallible;
use tower::{Layer, ServiceExt, service_fn};

#[tokio::test]
async fn v1_flat_organization_claims_remain_supported() {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    let (priv_pem, jwks_json) = test_keys();
    let issued_at = now() as i64;
    let not_before = issued_at;
    let expires_at = issued_at + 600;

    let claims = serde_json::json!({
        "sub": "user_2abc",
        "sid": "sess_2def",
        "org_id": "org_2ghi",
        "org_role": "org:admin",
        "org_permissions": ["org:dashboard:read", "org:teams:manage"],
        "iss": "https://test.clerk.dev",
        "exp": expires_at,
        "iat": issued_at,
        "nbf": not_before,
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("test-kid".into());
    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(priv_pem.as_bytes()).unwrap(),
    )
    .unwrap();

    let server = start_jwks_mock(&jwks_json).await;
    let layer = layer_with_jwks_base_url(format!("{}/v1", server.uri()));
    let app: Router = Router::new()
        .route(
            "/",
            get(move |req: Request<Body>| async move {
                let Some(VerificationOutcome::Valid(auth)) =
                    req.extensions().get::<VerificationOutcome>()
                else {
                    return "anon".to_string();
                };

                assert_eq!(auth.user_id, "user_2abc");
                assert_eq!(auth.session_id.as_deref(), Some("sess_2def"));
                assert_eq!(auth.org_id.as_deref(), Some("org_2ghi"));
                assert_eq!(auth.org_role.as_deref(), Some("org:admin"));
                assert_eq!(
                    auth.org_permissions,
                    vec!["org:dashboard:read", "org:teams:manage"]
                );
                assert_eq!(auth.exp, expires_at);
                assert_eq!(auth.nbf, not_before);
                assert_eq!(auth.iat, issued_at);
                assert!(req.extensions().get::<ClerkAuth>().is_none());

                "valid".to_string()
            }),
        )
        .layer(layer);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"valid");
}

#[tokio::test]
async fn v2_token_populates_active_organization_and_normalizes_role() {
    let auth = auth_from_signed_claims(claims_with([
        ("v", serde_json::json!(2)),
        (
            "o",
            serde_json::json!({
                "id": "org_2ghi",
                "slg": "acme",
                "rol": "admin"
            }),
        ),
    ]))
    .await
    .unwrap();

    assert_eq!(auth.org_id.as_deref(), Some("org_2ghi"));
    assert_eq!(auth.org_slug.as_deref(), Some("acme"));
    assert_eq!(auth.org_role.as_deref(), Some("org:admin"));
    assert!(auth.org_permissions.is_empty());
}

#[tokio::test]
async fn v2_token_does_not_double_prefix_organization_role() {
    let mut claims = v2_claims(None, None, None);
    claims["o"]["rol"] = serde_json::json!("org:admin");
    let auth = auth_from_signed_claims(claims).await.unwrap();

    assert_eq!(auth.org_role.as_deref(), Some("org:admin"));
}

#[tokio::test]
async fn v2_token_reconstructs_feature_scoped_permissions() {
    let auth = auth_from_signed_claims(claims_with([
        ("v", serde_json::json!(2)),
        ("fea", serde_json::json!("o:dashboard,o:teams")),
        (
            "o",
            serde_json::json!({
                "id": "org_2ghi",
                "rol": "admin",
                "per": "manage,read",
                "fpm": "3,2"
            }),
        ),
    ]))
    .await
    .unwrap();

    assert_eq!(
        auth.org_permissions,
        [
            "org:dashboard:manage",
            "org:dashboard:read",
            "org:teams:read"
        ]
    );
    assert!(auth.has_permission("org:dashboard:manage"));
    assert!(auth.has_permission("org:dashboard:read"));
    assert!(auth.has_permission("org:teams:read"));
    assert!(!auth.has_permission("org:teams:manage"));

    let auth_state = dioxus_clerk::core::AuthState::from(&auth);
    assert!(auth_state.has_role("org:admin"));
    assert!(auth_state.has_permission("org:dashboard:read"));

    let snapshot = dioxus_clerk::ssr::InitialAuthSnapshot::from(&auth);
    assert_eq!(snapshot.org_id.as_deref(), Some("org_2ghi"));
    assert_eq!(snapshot.org_permissions, auth.org_permissions);
}

#[tokio::test]
async fn v2_token_malformed_permission_compression_grants_nothing() {
    let cases = [
        (
            "fewer masks than features",
            Some("o:dashboard,o:teams"),
            Some("manage,read"),
            Some("3"),
        ),
        (
            "more masks than features",
            Some("o:dashboard"),
            Some("manage,read"),
            Some("3,2"),
        ),
        (
            "bit outside permission list",
            Some("o:dashboard"),
            Some("manage,read"),
            Some("7"),
        ),
        (
            "malformed mask",
            Some("o:dashboard"),
            Some("read"),
            Some("not-a-number"),
        ),
        (
            "overflowing mask",
            Some("o:dashboard"),
            Some("read"),
            Some("340282366920938463463374607431768211456"),
        ),
        ("empty feature", Some("o:"), Some("read"), Some("1")),
        (
            "empty permission",
            Some("o:dashboard"),
            Some("read,"),
            Some("2"),
        ),
        ("missing features", None, Some("read"), Some("1")),
        ("missing permissions", Some("o:dashboard"), None, Some("1")),
        (
            "missing feature permission map",
            Some("o:dashboard"),
            Some("read"),
            None,
        ),
    ];

    for (name, features, permissions, feature_permission_map) in cases {
        let auth =
            auth_from_signed_claims(v2_claims(features, permissions, feature_permission_map))
                .await
                .unwrap();

        assert_eq!(auth.org_id.as_deref(), Some("org_2ghi"), "{name}");
        assert_eq!(auth.org_role.as_deref(), Some("org:admin"), "{name}");
        assert!(auth.org_permissions.is_empty(), "{name}");
    }
}

#[tokio::test]
async fn v2_token_without_active_organization_has_no_organization_context() {
    let auth = auth_from_signed_claims(claims_with([("v", serde_json::json!(2))]))
        .await
        .unwrap();

    assert_eq!(auth.org_id, None);
    assert_eq!(auth.org_slug, None);
    assert_eq!(auth.org_role, None);
    assert!(auth.org_permissions.is_empty());
}

#[tokio::test]
async fn v2_permission_map_preserves_user_feature_alignment_and_zero_masks() {
    let auth = auth_from_signed_claims(v2_claims(
        Some("u:personal,o:dashboard,o:teams"),
        Some("manage,read"),
        Some("3,0,2"),
    ))
    .await
    .unwrap();

    assert_eq!(auth.org_permissions, ["org:teams:read"]);
}

#[tokio::test]
async fn v2_nested_organization_takes_precedence_over_flat_v1_claims() {
    let mut claims = v2_claims(Some("o:dashboard"), Some("read"), Some("1"));
    {
        let claims = claims.as_object_mut().unwrap();
        claims.insert("org_id".into(), serde_json::json!("org_legacy"));
        claims.insert("org_slug".into(), serde_json::json!("legacy"));
        claims.insert("org_role".into(), serde_json::json!("org:owner"));
        claims.insert(
            "org_permissions".into(),
            serde_json::json!(["org:legacy:all"]),
        );
    }
    let auth = auth_from_signed_claims(claims).await.unwrap();

    assert_eq!(auth.org_id.as_deref(), Some("org_2ghi"));
    assert_eq!(auth.org_slug, None);
    assert_eq!(auth.org_role.as_deref(), Some("org:admin"));
    assert_eq!(auth.org_permissions, ["org:dashboard:read"]);
    assert!(!auth.has_permission("org:legacy:all"));
}

#[tokio::test]
async fn signed_token_missing_required_claim_records_invalid_outcome() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_test_token_with_claims(claims_without(["sub"]), "test-kid", Some("JWT"));
    let app = outcome_app(layer_with_jwks_base_url(format!("{}/v1", server.uri())));

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(body_text(response).await, "invalid");
}

#[tokio::test]
async fn token_without_session_id_is_invalid_by_default() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    // A template-style token: instance-signed with a subject but no `sid`.
    let token = signed_test_token_with_claims(claims_without(["sid"]), "test-kid", Some("JWT"));
    let app = outcome_app(layer_with_jwks_base_url(format!("{}/v1", server.uri())));

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(body_text(response).await, "invalid");
}

#[tokio::test]
async fn token_without_session_id_is_valid_when_non_session_tokens_allowed() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_test_token_with_claims(claims_without(["sid"]), "test-kid", Some("JWT"));
    let config = ClerkAuthLayerConfig::new("sk_test_unused")
        .with_insecure_backend_api_base_url(format!("{}/v1", server.uri()))
        .allow_non_session_tokens();
    let app = outcome_app(layer_with_config(config));

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(body_text(response).await, "valid");
}

#[tokio::test]
async fn expired_token_records_invalid_outcome_with_expired_reason() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_test_token_with_claims(
        claims_with([
            ("iat", serde_json::json!((now() - 1_200) as i64)),
            ("nbf", serde_json::json!((now() - 1_200) as i64)),
            ("exp", serde_json::json!((now() - 600) as i64)),
        ]),
        "test-kid",
        Some("JWT"),
    );
    let layer = layer_with_jwks_base_url(format!("{}/v1", server.uri()));
    let app: Router = Router::new()
        .route(
            "/",
            get(|req: Request<Body>| async move {
                match req.extensions().get::<VerificationOutcome>() {
                    Some(VerificationOutcome::Invalid(InvalidTokenReason::Expired)) => "expired",
                    Some(VerificationOutcome::Invalid(_)) => "invalid",
                    _ => "other",
                }
            }),
        )
        .layer(layer);

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(body_text(response).await, "expired");
}

#[tokio::test]
async fn not_yet_valid_token_records_invalid_outcome_beyond_clock_skew() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_test_token_with_claims(
        claims_with([
            ("iat", serde_json::json!((now() + 600) as i64)),
            ("nbf", serde_json::json!((now() + 600) as i64)),
            ("exp", serde_json::json!((now() + 1_200) as i64)),
        ]),
        "test-kid",
        Some("JWT"),
    );
    let app = outcome_app(layer_with_jwks_base_url(format!("{}/v1", server.uri())));

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(body_text(response).await, "invalid");
}

#[tokio::test]
async fn signed_token_with_empty_subject_records_invalid_outcome() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_test_token_with_claims(
        claims_with([("sub", serde_json::json!(""))]),
        "test-kid",
        Some("JWT"),
    );
    let app = outcome_app(layer_with_jwks_base_url(format!("{}/v1", server.uri())));

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(body_text(response).await, "invalid");
}

#[tokio::test]
async fn issuer_validation_rejects_mismatch_and_accepts_configured_issuer() {
    let (_priv_pem, jwks_json) = test_keys();

    for (issuer, expected) in [
        ("https://test.clerk.dev", "valid"),
        ("https://other.clerk.dev", "invalid"),
    ] {
        let server = start_jwks_mock(&jwks_json).await;
        let config = ClerkAuthLayerConfig::new("sk_test_unused")
            .with_insecure_backend_api_base_url(format!("{}/v1", server.uri()))
            .add_issuer(issuer);
        let app = outcome_app(layer_with_config(config));

        let response = app
            .oneshot(request_with_auth(Some(&format!(
                "Bearer {}",
                signed_test_token()
            ))))
            .await
            .unwrap();

        assert_eq!(body_text(response).await, expected, "issuer: {issuer}");
    }
}

#[tokio::test]
async fn session_cookie_is_verified_through_the_layer() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_test_token();
    let app = outcome_app(layer_with_jwks_base_url(format!("{}/v1", server.uri())));

    let request = Request::builder()
        .uri("/")
        .header(header::COOKIE, format!("__session={token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(body_text(response).await, "valid");
}

#[tokio::test]
async fn unverifiable_session_cookie_fails_closed_even_after_an_invalid_cookie() {
    // `__session` is garbage (Invalid, rejected without a JWKS fetch); the
    // suffixed cookie is well-formed but the JWKS endpoint is unreachable
    // (Unavailable). Unavailable must win regardless of cookie order: the
    // request may carry a perfectly valid session that simply could not be
    // checked, so it must not proceed as anonymous.
    let token = signed_test_token();
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(layer_with_jwks_base_url("http://127.0.0.1:1/v1"));

    let request = Request::builder()
        .uri("/")
        .header(
            header::COOKIE,
            format!("__session=not-a-jwt; __session_a={token}"),
        )
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn valid_suffixed_cookie_wins_after_invalid_session_cookie() {
    // `__session` is garbage but a suffixed `__session_a` cookie carries a
    // valid token: any valid cookie must win over earlier invalid ones.
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_test_token();
    let app = outcome_app(layer_with_jwks_base_url(format!("{}/v1", server.uri())));

    let request = Request::builder()
        .uri("/")
        .header(
            header::COOKIE,
            format!("__session=not-a-jwt; __session_a={token}"),
        )
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(body_text(response).await, "valid");
}

#[tokio::test]
async fn jwks_fetch_sends_secret_key_bearer_auth() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/v1/jwks"))
        .and(wiremock::matchers::header(
            "authorization",
            "Bearer sk_test_unused",
        ))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(&jwks_json))
        .mount(&server)
        .await;
    let token = signed_test_token();
    let app = outcome_app(layer_with_jwks_base_url(format!("{}/v1", server.uri())));

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    // The mock only matches with the bearer header; without it the fetch
    // 404s and the request would fail closed as "unavailable".
    assert_eq!(body_text(response).await, "valid");
}

#[tokio::test]
async fn jwks_fetch_failure_backs_off_instead_of_refetching_per_request() {
    let server = start_jwks_mock_response(500, "upstream down").await;
    let token = signed_test_token();
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(layer_with_jwks_base_url(format!("{}/v1", server.uri())));

    let responses = futures_util::future::join_all((0..8).map(|_| {
        app.clone()
            .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
    }))
    .await;

    for response in responses {
        assert_eq!(response.unwrap().status(), StatusCode::SERVICE_UNAVAILABLE);
    }
    // The first fetch fails and arms the failure backoff; every other
    // request (queued or later) must fail unavailable without its own
    // upstream fetch.
    assert_eq!(server.received_requests().await.unwrap().len(), 1);

    // Still inside the backoff window: a fresh request must not refetch.
    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[test]
fn empty_secret_key_is_rejected_at_layer_construction() {
    let result = ClerkAuthLayer::new("");

    assert!(matches!(
        result,
        Err(dioxus_clerk::core::ClerkError::InvalidConfig(message))
            if message.contains("secret key")
    ));
}

#[tokio::test]
async fn concurrent_cold_cache_requests_share_one_jwks_fetch() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_test_token();
    let app = outcome_app(layer_with_jwks_base_url(format!("{}/v1", server.uri())));

    let responses = futures_util::future::join_all((0..8).map(|_| {
        app.clone()
            .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
    }))
    .await;

    for response in responses {
        assert_eq!(body_text(response.unwrap()).await, "valid");
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn no_auth_header_records_missing_outcome_and_passes_through() {
    let response = test_app(|req: Request<Body>| async move {
        assert!(matches!(
            req.extensions().get::<VerificationOutcome>(),
            Some(VerificationOutcome::Missing)
        ));
        assert!(req.extensions().get::<ClerkAuth>().is_none());
        Ok::<_, Infallible>(Response::new(Body::from("ok")))
    })
    .await
    .oneshot(request_with_auth(None))
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response).await, "ok");
}

#[tokio::test]
async fn invalid_bearer_token_records_invalid_outcome_and_passes_through() {
    let response = test_app(|req: Request<Body>| async move {
        assert!(matches!(
            req.extensions().get::<VerificationOutcome>(),
            Some(VerificationOutcome::Invalid(_))
        ));
        assert!(req.extensions().get::<ClerkAuth>().is_none());
        Ok::<_, Infallible>(Response::new(Body::from("ok")))
    })
    .await
    .oneshot(request_with_auth(Some("Bearer invalid-token")))
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response).await, "ok");
}

#[tokio::test]
async fn invalid_outcome_removes_stale_raw_auth_extension() {
    let mut request = request_with_auth(Some("Bearer invalid-token"));
    request.extensions_mut().insert(sample_auth());

    let response = test_app(|req: Request<Body>| async move {
        assert!(matches!(
            req.extensions().get::<VerificationOutcome>(),
            Some(VerificationOutcome::Invalid(_))
        ));
        assert!(req.extensions().get::<ClerkAuth>().is_none());
        Ok::<_, Infallible>(Response::new(Body::from("ok")))
    })
    .await
    .oneshot(request)
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn jwks_unavailable_fails_closed_with_503() {
    let token = signed_test_token();
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(layer_with_jwks_base_url("http://127.0.0.1:1/v1"));

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn malformed_jwks_fails_closed_with_503() {
    let token = signed_test_token();
    let server = start_jwks_mock_response(200, "not-json").await;
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(layer_with_jwks_base_url(format!("{}/v1", server.uri())));

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[test]
fn https_backend_api_base_url_is_required_by_default() {
    let result = ClerkAuthLayer::from_config(
        ClerkAuthLayerConfig::new("sk_test_unused")
            .with_backend_api_base_url("http://localhost:1234/v1"),
    );

    assert!(matches!(
        result,
        Err(dioxus_clerk::core::ClerkError::InvalidConfig(message))
            if message.contains("must use https")
    ));
}

#[test]
fn insecure_backend_api_base_url_opt_in_allows_http() {
    let result = ClerkAuthLayer::from_config(
        ClerkAuthLayerConfig::new("sk_test_unused")
            .with_insecure_backend_api_base_url("http://localhost:1234/v1"),
    );

    assert!(result.is_ok());
}

#[test]
fn config_debug_redacts_secret_key() {
    let config = ClerkAuthLayerConfig::new("sk_test_secret")
        .with_authorized_parties(["https://example.com"])
        .add_audience("api://default");

    let debug = format!("{config:?}");

    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("sk_test_secret"));
}

#[tokio::test]
async fn authorized_party_mismatch_records_invalid_outcome() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_test_token_with_claims(
        claims_with([("azp", serde_json::json!("https://evil.example"))]),
        "test-kid",
        Some("JWT"),
    );
    let layer = layer_with_config(
        ClerkAuthLayerConfig::new("sk_test_unused")
            .with_insecure_backend_api_base_url(format!("{}/v1", server.uri()))
            .with_authorized_parties(["https://app.example"]),
    );
    let app = outcome_app(layer);

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(body_text(response).await, "invalid");
}

#[tokio::test]
async fn audience_mismatch_records_invalid_outcome() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_test_token_with_claims(
        claims_with([("aud", serde_json::json!("api://wrong"))]),
        "test-kid",
        Some("JWT"),
    );
    let layer = layer_with_config(
        ClerkAuthLayerConfig::new("sk_test_unused")
            .with_insecure_backend_api_base_url(format!("{}/v1", server.uri()))
            .add_audience("api://expected"),
    );
    let app = outcome_app(layer);

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(body_text(response).await, "invalid");
}

#[tokio::test]
async fn token_with_nbf_inside_clock_skew_is_valid() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_test_token_with_claims(
        claims_with([("nbf", serde_json::json!(now() + 3))]),
        "test-kid",
        Some("JWT"),
    );
    let layer = layer_with_jwks_base_url(format!("{}/v1", server.uri()));
    let app = outcome_app(layer);

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(body_text(response).await, "valid");
}

#[tokio::test]
async fn token_without_jwt_header_type_is_invalid_without_fetching_jwks() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_test_token_with_claims(base_claims(), "test-kid", None);
    let layer = layer_with_jwks_base_url(format!("{}/v1", server.uri()));
    let app = outcome_app(layer);

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(body_text(response).await, "invalid");
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn token_with_non_rs256_algorithm_is_invalid_without_fetching_jwks() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let token = signed_hs256_test_token();
    let layer = layer_with_jwks_base_url(format!("{}/v1", server.uri()));
    let app = outcome_app(layer);

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(body_text(response).await, "invalid");
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn unknown_kid_does_not_refetch_before_rate_limit() {
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    let valid_token = signed_test_token();
    let unknown_kid_token =
        signed_test_token_with_claims(base_claims(), "unknown-kid", Some("JWT"));
    let layer = layer_with_jwks_base_url(format!("{}/v1", server.uri()));
    let app = outcome_app(layer);

    let response = app
        .clone()
        .oneshot(request_with_auth(Some(&format!("Bearer {valid_token}"))))
        .await
        .unwrap();
    assert_eq!(body_text(response).await, "valid");

    let response = app
        .oneshot(request_with_auth(Some(&format!(
            "Bearer {unknown_kid_token}"
        ))))
        .await
        .unwrap();
    assert_eq!(body_text(response).await, "invalid");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn jwks_redirect_response_fails_closed_without_following() {
    // The signing keys are the entire trust root: a redirecting `/jwks` must
    // never be followed to another origin. On native, redirects are refused
    // outright, so a 3xx fails closed as unavailable rather than sourcing keys
    // from the redirect target.
    let token = signed_test_token();
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/v1/jwks"))
        .respond_with(
            wiremock::ResponseTemplate::new(302)
                .insert_header("location", "https://evil.example/jwks"),
        )
        .mount(&server)
        .await;
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(layer_with_jwks_base_url(format!("{}/v1", server.uri())));

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn jwks_auth_failure_fails_closed_with_503() {
    // A 401/403 from `/jwks` (a wrong or revoked secret key) still fails closed
    // like any other unavailable verification, but travels the distinct
    // secret-key-rejected branch.
    let token = signed_test_token();
    let server = start_jwks_mock_response(403, "forbidden").await;
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(layer_with_jwks_base_url(format!("{}/v1", server.uri())));

    let response = app
        .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

// --- helpers -----------------------------------------------------------------

fn layer_with_jwks_base_url(jwks_base_url: impl Into<String>) -> ClerkAuthLayer {
    let cfg = ClerkAuthLayerConfig::new("sk_test_unused")
        .with_insecure_backend_api_base_url(jwks_base_url.into());
    layer_with_config(cfg)
}

fn layer_with_config(config: ClerkAuthLayerConfig) -> ClerkAuthLayer {
    ClerkAuthLayer::from_config(config).unwrap()
}

fn outcome_app(layer: ClerkAuthLayer) -> Router {
    Router::new()
        .route(
            "/",
            get(|req: Request<Body>| async move {
                match req.extensions().get::<VerificationOutcome>() {
                    Some(VerificationOutcome::Valid(_)) => "valid",
                    Some(VerificationOutcome::Invalid(_)) => "invalid",
                    Some(VerificationOutcome::Missing) => "missing",
                    Some(VerificationOutcome::Unavailable) => "unavailable",
                    Some(_) => "unknown",
                    None => "none",
                }
            }),
        )
        .layer(layer)
}

async fn body_text(response: Response) -> String {
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

async fn test_app<F, Fut>(
    handler: F,
) -> impl tower::Service<Request<Body>, Response = Response, Error = Infallible>
where
    F: Fn(Request<Body>) -> Fut + Clone + Send + 'static,
    Fut: std::future::Future<Output = Result<Response, Infallible>> + Send + 'static,
{
    let (_priv_pem, jwks_json) = test_keys();
    let server = start_jwks_mock(&jwks_json).await;
    layer_with_jwks_base_url(format!("{}/v1", server.uri())).layer(service_fn(handler))
}

async fn auth_from_signed_claims(claims: serde_json::Value) -> Result<ClerkAuth, String> {
    let token = signed_test_token_with_claims(claims, "test-kid", Some("JWT"));
    let response = test_app(|req: Request<Body>| async move {
        let body = match req.extensions().get::<VerificationOutcome>() {
            Some(VerificationOutcome::Valid(auth)) => serde_json::to_string(auth).unwrap(),
            outcome => format!("unexpected verification outcome: {outcome:?}"),
        };
        Ok::<_, Infallible>(Response::new(Body::from(body)))
    })
    .await
    .oneshot(request_with_auth(Some(&format!("Bearer {token}"))))
    .await
    .unwrap();
    let body = body_text(response).await;

    serde_json::from_str(&body).map_err(|_| body)
}

fn request_with_auth(auth: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().uri("/");
    if let Some(auth) = auth {
        builder = builder.header(header::AUTHORIZATION, auth);
    }
    builder.body(Body::empty()).unwrap()
}

fn signed_test_token() -> String {
    signed_test_token_with_claims(base_claims(), "test-kid", Some("JWT"))
}

fn signed_test_token_with_claims(
    claims: serde_json::Value,
    kid: &str,
    typ: Option<&str>,
) -> String {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    let (priv_pem, _jwks_json) = test_keys();

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.into());
    header.typ = typ.map(String::from);
    encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(priv_pem.as_bytes()).unwrap(),
    )
    .unwrap()
}

fn signed_hs256_test_token() -> String {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("test-kid".into());
    header.typ = Some("JWT".into());
    encode(
        &header,
        &base_claims(),
        &EncodingKey::from_secret(b"test-secret"),
    )
    .unwrap()
}

fn claims_with(
    overrides: impl IntoIterator<Item = (&'static str, serde_json::Value)>,
) -> serde_json::Value {
    let mut claims = base_claims();
    let object = claims.as_object_mut().unwrap();
    for (key, value) in overrides {
        object.insert(key.into(), value);
    }
    claims
}

fn claims_without(keys: impl IntoIterator<Item = &'static str>) -> serde_json::Value {
    let mut claims = base_claims();
    let object = claims.as_object_mut().unwrap();
    for key in keys {
        object.remove(key);
    }
    claims
}

fn v2_claims(
    features: Option<&str>,
    permissions: Option<&str>,
    feature_permission_map: Option<&str>,
) -> serde_json::Value {
    let mut claims = claims_with([
        ("v", serde_json::json!(2)),
        (
            "o",
            serde_json::json!({
                "id": "org_2ghi",
                "rol": "admin",
            }),
        ),
    ]);
    if let Some(permissions) = permissions {
        claims["o"]["per"] = serde_json::json!(permissions);
    }
    if let Some(feature_permission_map) = feature_permission_map {
        claims["o"]["fpm"] = serde_json::json!(feature_permission_map);
    }
    if let Some(features) = features {
        claims["fea"] = serde_json::json!(features);
    }
    claims
}

fn base_claims() -> serde_json::Value {
    serde_json::json!({
        "sub": "user_2abc",
        "sid": "sess_2def",
        "iss": "https://test.clerk.dev",
        "exp": (now() + 600) as i64,
        "iat": now() as i64,
        "nbf": now() as i64,
    })
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn test_keys() -> (String, String) {
    // Hard-coded RSA-2048 PEM for tests. Regenerate with
    //   bash tests/fixtures/gen_keys.sh
    let priv_pem = include_str!("fixtures/test_priv.pem").to_string();
    let jwks = include_str!("fixtures/test_jwks.json").to_string();
    (priv_pem, jwks)
}

async fn start_jwks_mock(jwks: &str) -> wiremock::MockServer {
    start_jwks_mock_response(200, jwks).await
}

async fn start_jwks_mock_response(status: u16, body: &str) -> wiremock::MockServer {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/v1/jwks"))
        .respond_with(wiremock::ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    server
}
