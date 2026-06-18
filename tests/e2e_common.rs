//! Shared helpers for HTTP integration tests (require DATABASE_URL + REDIS_URL).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rust_decimal::Decimal;
use tower::ServiceExt;
use uuid::Uuid;

use rsc_gateway::internal::config::Config;
use rsc_gateway::internal::state::AppState;
use rsc_gateway::internal::wallets::models::RecordDepositRequest;
use rsc_gateway::routes;

pub fn e2e_enabled() -> bool {
    std::env::var("RSC_E2E").ok().as_deref() == Some("1")
}

pub async fn setup_app() -> AppState {
    dotenvy::dotenv().ok();
    let config = Config::load().expect("load config for e2e");
    config.validate().expect("validate config for e2e");
    AppState::build(config).await.expect("build app state for e2e")
}

pub async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("parse json body")
}

pub async fn register_and_token(
    app: &axum::Router,
    email: &str,
) -> (Uuid, String) {
    let password = "TestPassword1!xy";
    let register = serde_json::json!({
        "email": email,
        "password": password,
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(register.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "register {}", email);
    let json = body_json(response).await;
    let token = json["access_token"]
        .as_str()
        .expect("access_token")
        .to_string();

    let me = authed_json(
        app,
        "GET",
        "/auth/me",
        &token,
        None::<serde_json::Value>,
    )
    .await;
    let user_id = Uuid::parse_str(me["id"].as_str().expect("user id")).expect("uuid");

    (user_id, token)
}

pub async fn authed_json(
    app: &axum::Router,
    method: &str,
    uri: &str,
    token: &str,
    body: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json");

    let request = if let Some(body) = body {
        builder = builder.header("content-type", "application/json");
        builder.body(Body::from(body.to_string())).unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };

    let response = app.clone().oneshot(request).await.unwrap();
    assert!(
        response.status().is_success(),
        "{method} {uri} failed: {}",
        response.status()
    );
    body_json(response).await
}

pub async fn fund_wallet(
    state: &AppState,
    user_id: Uuid,
    asset: &str,
    chain: &str,
    amount: Decimal,
    key: &str,
) {
    let wallet_id = state
        .wallets
        .get_default_wallet_id(user_id)
        .await
        .expect("default wallet");

    state
        .wallets
        .record_confirmed_deposit(RecordDepositRequest {
            wallet_id,
            asset: asset.to_string(),
            chain: chain.to_string(),
            tx_hash: format!("e2e:{key}"),
            confirmations: 1,
            idempotency_key: key.to_string(),
            amount,
            from_address: None,
            to_address: None,
            metadata: Some(serde_json::json!({ "e2e": true })),
        })
        .await
        .expect("fund wallet");
}

pub async fn available_balance(
    state: &AppState,
    wallet_id: Uuid,
    asset: &str,
    chain: &str,
) -> Decimal {
    state
        .wallets
        .get_wallet_balance(wallet_id, asset, chain)
        .await
        .expect("balance query")
        .map(|b| b.available)
        .unwrap_or(Decimal::ZERO)
}

pub fn build_router(state: AppState) -> axum::Router {
    routes::create_router(state)
}

pub fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@e2e.rsc.test", Uuid::new_v4().simple())
}
