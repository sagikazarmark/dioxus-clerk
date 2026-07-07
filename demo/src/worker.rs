use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use dioxus_clerk::server::{ClerkAuthLayer, ClerkAuthLayerConfig, VerificationOutcome};
use tower_service::Service;
use worker::{event, Context, Env, Error, HttpRequest, Result};

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    if !req.uri().path().starts_with("/api/") {
        return asset_response(req, &env).await;
    }

    let mut router = router(clerk_auth_layer(&env)?);

    Ok(router.call(req).await?)
}

fn router(auth_layer: ClerkAuthLayer) -> Router {
    Router::new()
        .route("/api/get_my_counter", post(get_my_counter))
        .route("/api/whoami", get(whoami))
        .layer(auth_layer)
}

async fn asset_response(
    req: HttpRequest,
    env: &Env,
) -> Result<axum::http::Response<axum::body::Body>> {
    let response = env.assets("ASSETS")?.fetch_request(req).await?;
    let (parts, body) = response.into_parts();
    Ok(axum::http::Response::from_parts(
        parts,
        axum::body::Body::new(body),
    ))
}

fn clerk_auth_layer(env: &Env) -> Result<ClerkAuthLayer> {
    let secret_key = env.secret("CLERK_SECRET_KEY")?.to_string();
    let config = ClerkAuthLayerConfig::new(secret_key);

    ClerkAuthLayer::from_config(config).map_err(|error| Error::RustError(error.to_string()))
}

async fn get_my_counter(Extension(outcome): Extension<VerificationOutcome>) -> impl IntoResponse {
    match outcome.into_auth() {
        Some(auth) => Json(auth.user_id.bytes().map(u64::from).sum::<u64>()).into_response(),
        None => unauthenticated(),
    }
}

/// Bearer-authenticated counterpart to the native `/api/whoami` route: echoes
/// the verified user id. The client sends `Authorization: Bearer <token>` from
/// `use_auth().get_token()`, which `ClerkAuthLayer` verifies before this runs.
async fn whoami(Extension(outcome): Extension<VerificationOutcome>) -> impl IntoResponse {
    match outcome.into_auth() {
        Some(auth) => auth.user_id.into_response(),
        None => unauthenticated(),
    }
}

fn unauthenticated() -> axum::response::Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "message": "Unauthenticated",
            "code": StatusCode::UNAUTHORIZED.as_u16(),
        })),
    )
        .into_response()
}
