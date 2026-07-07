#![cfg(feature = "server")]

//! Integration tests for Axum extraction and the `current_auth` context reader.
//!
//! The context reader reads from `FullstackContext`, not directly from the live
//! axum `Request<Body>::extensions()`. The bridge is one-way: at the start
//! of a server-function request, dioxus-server splits the request into
//! `Parts`, constructs a `FullstackContext::new(parts)`, then runs the
//! handler future inside `ctx.scope(...)` so the task-local
//! `FULLSTACK_CONTEXT` is set. `ctx.extension::<T>()` then reads from
//! `parts.extensions` — which is exactly where `ClerkAuthLayer` deposits
//! the auth.
//!
//! The plan's original test design wires a plain axum handler directly,
//! which would never see a `FullstackContext` (no scope set), so
//! `current_auth()` would return `Err(InvalidConfig(...))`. We
//! reproduce dioxus-server's bridge here with a tiny tower middleware
//! that wraps the inner handler call in `FullstackContext::new(parts)
//! .scope(...)`. This faithfully exercises the production code path
//! without pulling in the full dioxus-server runtime.

#[path = "support/auth.rs"]
mod auth;

use auth::sample_auth;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Router, routing::get};
use dioxus_clerk::core::{ClerkError, InvalidTokenReason};
use dioxus_clerk::server::{
    ClerkAuth, VerificationOutcome, current_auth, current_auth_opt, current_outcome,
};
use dioxus_fullstack_core::FullstackContext;
use tower::ServiceExt;

/// Mirror of what dioxus-server does for every server-function request:
/// split the incoming request into `Parts`, build a `FullstackContext`,
/// and run the rest of the call inside `ctx.scope(...)` so the
/// task-local `FULLSTACK_CONTEXT` is set for the handler body.
async fn fullstack_scope(req: Request<Body>, next: Next) -> Response {
    let (parts, body) = req.into_parts();
    let ctx = FullstackContext::new(parts.clone());
    let req = Request::from_parts(parts, body);
    ctx.scope(next.run(req)).await
}

async fn insert_valid_outcome(mut req: Request<Body>, next: Next) -> Response {
    req.extensions_mut()
        .insert(VerificationOutcome::Valid(sample_auth()));
    next.run(req).await
}

#[tokio::test]
async fn clerk_auth_extracts_valid_outcome_for_axum_handlers() {
    async fn handler(auth: ClerkAuth) -> impl IntoResponse {
        auth.user_id
    }

    let app: Router = Router::new()
        .route("/", get(handler))
        .layer(axum::middleware::from_fn(insert_valid_outcome));

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"user_2abc");
}

#[tokio::test]
async fn clerk_auth_rejects_missing_outcome_for_axum_handlers() {
    async fn handler(_auth: ClerkAuth) -> impl IntoResponse {
        "authed"
    }

    let app: Router = Router::new().route("/", get(handler));

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"unauthenticated");
}

#[tokio::test]
async fn optional_clerk_auth_extracts_none_for_public_axum_handlers() {
    async fn handler(auth: Option<ClerkAuth>) -> impl IntoResponse {
        auth.map(|auth| auth.user_id)
            .unwrap_or_else(|| "none".into())
    }

    let app: Router = Router::new().route("/", get(handler));

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"none");
}

#[tokio::test]
async fn optional_clerk_auth_extracts_valid_outcome_for_axum_handlers() {
    async fn handler(auth: Option<ClerkAuth>) -> impl IntoResponse {
        auth.map(|auth| auth.user_id)
            .unwrap_or_else(|| "none".into())
    }

    let app: Router = Router::new()
        .route("/", get(handler))
        .layer(axum::middleware::from_fn(insert_valid_outcome));

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"user_2abc");
}

#[tokio::test]
async fn optional_clerk_auth_extracts_none_for_invalid_outcome() {
    async fn insert_invalid_outcome(mut req: Request<Body>, next: Next) -> Response {
        req.extensions_mut()
            .insert(VerificationOutcome::Invalid(InvalidTokenReason::Other));
        next.run(req).await
    }

    async fn handler(auth: Option<ClerkAuth>) -> impl IntoResponse {
        auth.map(|auth| auth.user_id)
            .unwrap_or_else(|| "none".into())
    }

    let app: Router = Router::new()
        .route("/", get(handler))
        .layer(axum::middleware::from_fn(insert_invalid_outcome));

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"none");
}

#[tokio::test]
async fn optional_clerk_auth_rejects_unavailable_outcome() {
    async fn insert_unavailable_outcome(mut req: Request<Body>, next: Next) -> Response {
        req.extensions_mut()
            .insert(VerificationOutcome::Unavailable);
        next.run(req).await
    }

    async fn handler(_auth: Option<ClerkAuth>) -> impl IntoResponse {
        "ok"
    }

    let app: Router = Router::new()
        .route("/", get(handler))
        .layer(axum::middleware::from_fn(insert_unavailable_outcome));

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"Clerk verification unavailable");
}

#[tokio::test]
async fn current_auth_reads_valid_outcome_set_by_layer() {
    async fn handler() -> impl IntoResponse {
        let auth = current_auth().expect("auth extension should be present");
        auth.user_id
    }

    // Layers run on the request in the OPPOSITE order they're declared
    // (axum wraps services bottom-up). We want the auth-inserter layer
    // to run FIRST on the request (so the extension is present when
    // `FullstackContext::new(parts)` snapshots them), then the scope
    // layer to wrap the handler call. That means the auth-inserter is
    // declared LAST.
    let app: Router = Router::new()
        .route("/", get(handler))
        .layer(axum::middleware::from_fn(fullstack_scope))
        .layer(axum::middleware::from_fn(insert_valid_outcome));

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"user_2abc");
}

#[tokio::test]
async fn crate_root_current_auth_reads_valid_outcome() {
    let cx = server_context_with_extensions();
    cx.parts_mut()
        .extensions
        .insert(VerificationOutcome::Valid(sample_auth()));

    let auth = cx.scope(async { current_auth() }).await.unwrap();

    assert_eq!(auth.user_id, "user_2abc");
}

#[tokio::test]
async fn current_auth_returns_unauthenticated_when_no_extension() {
    async fn handler() -> impl IntoResponse {
        match current_auth() {
            Ok(_) => (StatusCode::OK, "authed"),
            Err(ClerkError::Unauthenticated) => (StatusCode::OK, "unauthenticated"),
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }

    // FullstackContext IS set up (server function was invoked), but no
    // ClerkAuth extension was inserted by any upstream layer.
    let app: Router = Router::new()
        .route("/", get(handler))
        .layer(axum::middleware::from_fn(fullstack_scope));

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"unauthenticated");
}

#[tokio::test]
async fn current_auth_opt_returns_some_when_valid_outcome_present() {
    async fn handler() -> impl IntoResponse {
        let opt = current_auth_opt().expect("FullstackContext should be present in scope");
        match opt {
            Some(auth) => auth.user_id,
            None => "none".to_string(),
        }
    }

    let app: Router = Router::new()
        .route("/", get(handler))
        .layer(axum::middleware::from_fn(fullstack_scope))
        .layer(axum::middleware::from_fn(insert_valid_outcome));

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"user_2abc");
}

#[tokio::test]
async fn context_reader_returns_current_outcome() {
    let cx = server_context_with_extensions();
    cx.parts_mut()
        .extensions
        .insert(VerificationOutcome::Valid(sample_auth()));

    let outcome = cx.scope(async { current_outcome() }).await;

    assert!(
        matches!(outcome, Some(VerificationOutcome::Valid(auth)) if auth.user_id == "user_2abc")
    );
}

#[tokio::test]
async fn context_reader_current_auth_opt_returns_none_for_missing_outcome() {
    let cx = server_context_with_extensions();
    cx.parts_mut()
        .extensions
        .insert(VerificationOutcome::Missing);

    let auth = cx.scope(async { current_auth_opt() }).await.unwrap();

    assert!(auth.is_none());
}

#[tokio::test]
async fn context_reader_current_auth_opt_errors_for_unavailable_outcome() {
    let cx = server_context_with_extensions();
    cx.parts_mut()
        .extensions
        .insert(VerificationOutcome::Unavailable);

    let err = cx.scope(async { current_auth_opt() }).await.unwrap_err();

    assert!(matches!(err, ClerkError::JwksUnavailable(_)));
}

#[tokio::test]
async fn context_reader_current_auth_returns_jwks_unavailable_for_unavailable_outcome() {
    let cx = server_context_with_extensions();
    cx.parts_mut()
        .extensions
        .insert(VerificationOutcome::Unavailable);

    let err = cx.scope(async { current_auth() }).await.unwrap_err();

    assert!(matches!(err, ClerkError::JwksUnavailable(_)));
}

#[tokio::test]
async fn context_reader_current_auth_returns_token_expired_for_expired_token() {
    let cx = server_context_with_extensions();
    cx.parts_mut()
        .extensions
        .insert(VerificationOutcome::Invalid(InvalidTokenReason::Expired));

    let err = cx.scope(async { current_auth() }).await.unwrap_err();

    assert!(matches!(err, ClerkError::TokenExpired));
}

#[tokio::test]
async fn context_reader_current_auth_returns_unauthenticated_for_missing_outcome() {
    let cx = server_context_with_extensions();
    cx.parts_mut()
        .extensions
        .insert(VerificationOutcome::Missing);

    let err = cx.scope(async { current_auth() }).await.unwrap_err();

    assert!(matches!(err, ClerkError::Unauthenticated));
}

#[tokio::test]
async fn current_auth_opt_returns_none_when_extension_absent() {
    async fn handler() -> impl IntoResponse {
        let opt = current_auth_opt().expect("FullstackContext should be present in scope");
        match opt {
            Some(_) => "some",
            None => "none",
        }
    }

    let app: Router = Router::new()
        .route("/", get(handler))
        .layer(axum::middleware::from_fn(fullstack_scope));

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"none");
}

#[tokio::test]
async fn current_auth_errors_outside_fullstack_scope() {
    // No FullstackContext on the task-local — this is the "called outside
    // a server function" case. `current_auth` should return
    // `Err(NoServerContext)` and `current_auth_opt` likewise.
    let from_err = current_auth().expect_err("must error");
    assert!(
        matches!(from_err, ClerkError::NoServerContext),
        "expected NoServerContext, got {from_err:?}"
    );

    let opt_err = current_auth_opt().expect_err("must error");
    assert!(
        matches!(opt_err, ClerkError::NoServerContext),
        "expected NoServerContext, got {opt_err:?}"
    );
}

#[tokio::test]
async fn current_auth_opt_returns_none_for_invalid_outcome() {
    let cx = server_context_with_extensions();
    cx.parts_mut()
        .extensions
        .insert(VerificationOutcome::Invalid(InvalidTokenReason::Other));

    let auth = cx.scope(async { current_auth_opt() }).await.unwrap();

    assert!(auth.is_none());
}

#[tokio::test]
async fn current_auth_returns_unauthenticated_for_invalid_outcome() {
    let cx = server_context_with_extensions();
    cx.parts_mut()
        .extensions
        .insert(VerificationOutcome::Invalid(InvalidTokenReason::Other));

    let err = cx.scope(async { current_auth() }).await.unwrap_err();

    assert!(matches!(err, ClerkError::Unauthenticated));
}

#[tokio::test]
async fn current_auth_opt_returns_none_when_invalid_outcome_and_auth_extension_exist() {
    let cx = server_context_with_extensions();
    {
        let mut parts = cx.parts_mut();
        parts
            .extensions
            .insert(VerificationOutcome::Invalid(InvalidTokenReason::Other));
        parts.extensions.insert(sample_auth());
    }

    let auth = cx.scope(async { current_auth_opt() }).await.unwrap();

    assert!(auth.is_none());
}

#[tokio::test]
async fn current_auth_returns_unauthenticated_when_invalid_outcome_and_auth_extension_exist() {
    let cx = server_context_with_extensions();
    {
        let mut parts = cx.parts_mut();
        parts
            .extensions
            .insert(VerificationOutcome::Invalid(InvalidTokenReason::Other));
        parts.extensions.insert(sample_auth());
    }

    let err = cx.scope(async { current_auth() }).await.unwrap_err();

    assert!(matches!(err, ClerkError::Unauthenticated));
}

#[tokio::test]
async fn current_auth_opt_returns_none_when_only_stale_auth_extension_exists() {
    let cx = server_context_with_extensions();
    cx.parts_mut().extensions.insert(sample_auth());

    let auth = cx.scope(async { current_auth_opt() }).await.unwrap();

    assert!(auth.is_none());
}

#[tokio::test]
async fn current_auth_returns_unauthenticated_when_only_stale_auth_extension_exists() {
    let cx = server_context_with_extensions();
    cx.parts_mut().extensions.insert(sample_auth());

    let err = cx.scope(async { current_auth() }).await.unwrap_err();

    assert!(matches!(err, ClerkError::Unauthenticated));
}

fn server_context_with_extensions() -> FullstackContext {
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let (parts, _body) = req.into_parts();
    FullstackContext::new(parts)
}
