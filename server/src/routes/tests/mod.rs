use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::response::Response;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::util::ServiceExt;

use crate::{app::build_router, config::AppConfig, state::AppState};

mod auth_request_id;
mod idempotency_missing_key;
mod idempotency_payload_conflict;
mod idempotency_replay;
mod business_flow;
mod barcode_lookup;
mod json_extractor;
mod user_role_management;

fn make_test_state() -> AppState {
    AppState::new(AppConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        jwt_secret: "test-secret".to_string(),
        access_token_exp_secs: 7200,
        refresh_token_exp_secs: 604800,
        allow_negative_stock: false,
        storage_backend: crate::config::StorageBackend::Memory,
        database_url: None,
        redis_url: None,
        postgres_max_connections: 10,
        barcode_lookup_api_url: None,
        barcode_lookup_api_key: None,
        barcode_lookup_timeout_ms: 3000,
        barcode_lookup_found_ttl_secs: 2_592_000,
        barcode_lookup_not_found_ttl_secs: 86_400,
    })
}

async fn login_token(app: axum::Router) -> (axum::Router, String) {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "username": "admin",
                "password": "admin123"
            })
            .to_string(),
        ))
        .expect("build login request");

    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("login request should succeed");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("read login body")
        .to_bytes();
    let v: Value = serde_json::from_slice(&bytes).expect("login json");
    let token = v["data"]["access_token"]
        .as_str()
        .expect("token should exist")
        .to_string();
    (app, token)
}

async fn login_tokens(app: axum::Router) -> (axum::Router, String, String) {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "username": "admin",
                "password": "admin123"
            })
            .to_string(),
        ))
        .expect("build login request");

    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("login request should succeed");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("read login body")
        .to_bytes();
    let v: Value = serde_json::from_slice(&bytes).expect("login json");
    let access_token = v["data"]["access_token"]
        .as_str()
        .expect("access token should exist")
        .to_string();
    let refresh_token = v["data"]["refresh_token"]
        .as_str()
        .expect("refresh token should exist")
        .to_string();

    (app, access_token, refresh_token)
}

async fn response_json(resp: Response) -> Value {
    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("json parse")
}

async fn response_text(resp: Response) -> String {
    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("utf8 body")
}

async fn assert_missing_idempotency_key_response(resp: Response, expected_request_id: &str) {
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let replay_header = resp
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay_header, "false");

    let header_request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(header_request_id, expected_request_id);

    let body = response_json(resp).await;
    assert_eq!(body["code"], 4000);
    assert_eq!(body["request_id"], expected_request_id);
}

async fn assert_missing_idempotency_key_response_with_generated_request_id(resp: Response) {
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let replay_header = resp
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay_header, "false");

    let header_request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert!(header_request_id.starts_with("req_"));

    let body = response_json(resp).await;
    assert_eq!(body["code"], 4000);

    let body_request_id = body["request_id"]
        .as_str()
        .expect("request_id should exist in body");
    assert_eq!(body_request_id, header_request_id);
}

async fn assert_idempotency_payload_conflict_response(resp: Response, expected_request_id: &str) {
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let replay_header = resp
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay_header, "false");

    let header_request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(header_request_id, expected_request_id);

    let body = response_json(resp).await;
    assert_eq!(body["code"], 4092);
    assert_eq!(body["request_id"], expected_request_id);
}
