use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::response::Response;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::util::ServiceExt;

use crate::{app::build_router, config::AppConfig, state::AppState};

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

async fn assert_idempotency_payload_conflict_response(
    resp: Response,
    expected_request_id: &str,
) {
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

#[tokio::test]
async fn login_success_returns_200_and_token() {
    let state = make_test_state();
    let app = build_router(state);

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
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    let body: Value = serde_json::from_slice(&bytes).expect("json parse");

    assert_eq!(body["code"], 200);
    assert!(body["data"]["access_token"].as_str().is_some());
    assert!(body["request_id"].as_str().is_some());
}

#[tokio::test]
async fn register_success_returns_owner_tokens_and_tenant_id() {
    let state = make_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "tenant_name": "测试租户A",
                "username": "owner_a",
                "name": "租户A管理员",
                "password": "ownerA123"
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.clone().oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 200);
    assert_eq!(body["data"]["registered"], true);
    assert_eq!(body["data"]["tenant_name"], "测试租户A");
    assert!(body["data"]["tenant_id"].as_str().is_some());
    assert!(body["data"]["access_token"].as_str().is_some());
    assert!(body["data"]["refresh_token"].as_str().is_some());
    assert_eq!(body["data"]["user_info"]["role"], "OWNER");

    let login_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "username": "owner_a",
                "password": "ownerA123"
            })
            .to_string(),
        ))
        .expect("build login request");

    let login_resp = app.oneshot(login_req).await.expect("send login request");
    assert_eq!(login_resp.status(), StatusCode::OK);
    let login_body = response_json(login_resp).await;
    assert_eq!(login_body["code"], 200);
    assert_eq!(login_body["data"]["user_info"]["role"], "OWNER");
}

#[tokio::test]
async fn register_with_empty_required_fields_returns_4000() {
    let state = make_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "tenant_name": "",
                "username": "   ",
                "name": "",
                "password": ""
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4000);
}

#[tokio::test]
async fn register_with_duplicate_username_returns_4090() {
    let state = make_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "tenant_name": "测试租户B",
                "username": "admin",
                "name": "租户B管理员",
                "password": "ownerB123"
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4090);
    assert_eq!(body["data"]["username"], "admin");
}

#[tokio::test]
async fn refresh_token_success_returns_200_and_access_token() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, _access_token, refresh_token) = login_tokens(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/refresh")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "refresh_token": refresh_token
            })
            .to_string(),
        ))
        .expect("build refresh request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 200);
    assert!(body["data"]["access_token"].as_str().is_some());
    assert_eq!(body["data"]["user_info"]["role"], "OWNER");
}

#[tokio::test]
async fn refresh_with_expired_refresh_token_returns_4010() {
    let state = make_test_state();

    let (tenant_id, user_id) = {
        let users = state.users.lock().expect("lock users");
        let admin = users.get("admin").expect("admin exists");
        (admin.tenant_id, admin.id)
    };

    let expired_refresh_token = crate::middleware::generate_token(
        &state.config.jwt_secret,
        tenant_id,
        user_id,
        "OWNER",
        "refresh",
        -1,
    )
    .expect("generate expired refresh token");

    let app = build_router(state);
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/refresh")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "refresh_token": expired_refresh_token
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn refresh_with_wrong_jwt_signature_returns_4010() {
    let state = make_test_state();

    let (tenant_id, user_id) = {
        let users = state.users.lock().expect("lock users");
        let admin = users.get("admin").expect("admin exists");
        (admin.tenant_id, admin.id)
    };

    let wrong_signature_refresh_token = crate::middleware::generate_token(
        "another-secret",
        tenant_id,
        user_id,
        "OWNER",
        "refresh",
        state.config.refresh_token_exp_secs,
    )
    .expect("generate wrong signature refresh token");

    let app = build_router(state);
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/refresh")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "refresh_token": wrong_signature_refresh_token
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn logout_with_expired_access_token_returns_4010() {
    let state = make_test_state();

    let (tenant_id, user_id) = {
        let users = state.users.lock().expect("lock users");
        let admin = users.get("admin").expect("admin exists");
        (admin.tenant_id, admin.id)
    };

    let expired_access_token = crate::middleware::generate_token(
        &state.config.jwt_secret,
        tenant_id,
        user_id,
        "OWNER",
        "access",
        -1,
    )
    .expect("generate expired access token");

    let app = build_router(state);
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/logout")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", expired_access_token),
        )
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn login_with_unknown_user_returns_4010() {
    let state = make_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "username": "not-exists",
                "password": "whatever"
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn refresh_with_invalid_jwt_returns_4010() {
    let state = make_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/refresh")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "refresh_token": "not-a-jwt"
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn protected_route_with_wrong_jwt_signature_returns_4010() {
    let state = make_test_state();

    let (tenant_id, user_id) = {
        let users = state.users.lock().expect("lock users");
        let admin = users.get("admin").expect("admin exists");
        (admin.tenant_id, admin.id)
    };

    let wrong_signature_token = crate::middleware::generate_token(
        "another-secret",
        tenant_id,
        user_id,
        "OWNER",
        "access",
        state.config.access_token_exp_secs,
    )
    .expect("generate wrong signature token");

    let app = build_router(state);
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products?page=1&page_size=20")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", wrong_signature_token),
        )
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn protected_route_with_expired_access_token_returns_4010() {
    let state = make_test_state();

    let (tenant_id, user_id) = {
        let users = state.users.lock().expect("lock users");
        let admin = users.get("admin").expect("admin exists");
        (admin.tenant_id, admin.id)
    };

    let expired_token = crate::middleware::generate_token(
        &state.config.jwt_secret,
        tenant_id,
        user_id,
        "OWNER",
        "access",
        -1,
    )
    .expect("generate expired token");

    let app = build_router(state);
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products?page=1&page_size=20")
        .header(header::AUTHORIZATION, format!("Bearer {}", expired_token))
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn login_with_wrong_password_returns_4010() {
    let state = make_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "username": "admin",
                "password": "wrong-password"
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn refresh_with_empty_token_returns_4000() {
    let state = make_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/refresh")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "refresh_token": "   "
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4000);
}

#[tokio::test]
async fn protected_route_with_malformed_authorization_returns_4010() {
    let state = make_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products?page=1&page_size=20")
        .header(header::AUTHORIZATION, "Token abc")
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn login_should_echo_request_id_header() {
    let state = make_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-request-id", "req-login-echo-001")
        .body(Body::from(
            json!({
                "username": "admin",
                "password": "admin123"
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::OK);

    let header_request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(header_request_id, "req-login-echo-001");

    let body = response_json(resp).await;
    assert_eq!(body["request_id"], "req-login-echo-001");
}

#[tokio::test]
async fn protected_route_401_should_echo_request_id_header() {
    let state = make_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products?page=1&page_size=20")
        .header("x-request-id", "req-protected-401-001")
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let header_request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(header_request_id, "req-protected-401-001");

    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
    assert_eq!(body["request_id"], "req-protected-401-001");
}

#[tokio::test]
async fn login_without_request_id_should_generate_header_and_body_request_id() {
    let state = make_test_state();
    let app = build_router(state);

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
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::OK);

    let header_request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert!(header_request_id.starts_with("req_"));

    let body = response_json(resp).await;
    let body_request_id = body["request_id"]
        .as_str()
        .expect("request_id should exist in body");
    assert_eq!(body_request_id, header_request_id);
}

#[tokio::test]
async fn protected_route_401_without_request_id_should_generate_header_and_body_request_id() {
    let state = make_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products?page=1&page_size=20")
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let header_request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert!(header_request_id.starts_with("req_"));

    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
    let body_request_id = body["request_id"]
        .as_str()
        .expect("request_id should exist in body");
    assert_eq!(body_request_id, header_request_id);
}

#[tokio::test]
async fn idempotent_replay_without_request_id_should_keep_first_generated_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "barcode": "6991234500199",
        "name": "测试商品-自动request-id回放",
        "unit": "件",
        "retail_price": "9.90",
        "wholesale_price": "8.80",
        "init_stock": 0,
        "min_stock_limit": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/products")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-auto-rid-001")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let first_request_id = resp1
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert!(first_request_id.starts_with("req_"));

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/products")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-auto-rid-001")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);

    let replay_header = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay_header, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, first_request_id);

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], replay_request_id);
}

#[tokio::test]
async fn update_product_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/products/1001")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-update-missing-idem-001")
        .body(Body::from(
            json!({
                "name": "测试商品-更新缺幂等键"
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let header_request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(header_request_id, "req-update-missing-idem-001");

    let body = response_json(resp).await;
    assert_eq!(body["code"], 4000);
    assert_eq!(body["request_id"], "req-update-missing-idem-001");
}

#[tokio::test]
async fn purchase_order_create_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/purchase-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-po-create-missing-idem-001")
        .body(Body::from(
            json!({
                "biz_no": "PO-MISSING-IDEM-001",
                "items": [{
                    "product_id": 1001,
                    "qty": 1,
                    "unit_cost": "2.20"
                }]
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let header_request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(header_request_id, "req-po-create-missing-idem-001");

    let body = response_json(resp).await;
    assert_eq!(body["code"], 4000);
    assert_eq!(body["request_id"], "req-po-create-missing-idem-001");
}

#[tokio::test]
async fn inbound_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-inbound-missing-idem-001")
        .body(Body::from(
            json!({
                "product_id": 1001,
                "qty": 1,
                "unit_cost": "2.20",
                "biz_no": "IN-MISSING-IDEM-001"
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let header_request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(header_request_id, "req-inbound-missing-idem-001");

    let body = response_json(resp).await;
    assert_eq!(body["code"], 4000);
    assert_eq!(body["request_id"], "req-inbound-missing-idem-001");
}

#[tokio::test]
async fn outbound_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/outbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-outbound-missing-idem-001")
        .body(Body::from(
            json!({
                "biz_no": "OUT-MISSING-IDEM-001",
                "items": [{
                    "product_id": 1001,
                    "qty": 1,
                    "sell_price": "3.50"
                }]
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let header_request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(header_request_id, "req-outbound-missing-idem-001");

    let body = response_json(resp).await;
    assert_eq!(body["code"], 4000);
    assert_eq!(body["request_id"], "req-outbound-missing-idem-001");
}

#[tokio::test]
async fn create_product_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/products")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-missing-idem-001")
        .body(Body::from(
            json!({
                "barcode": "6991234500099",
                "name": "测试商品-缺幂等键",
                "unit": "件",
                "retail_price": "9.90",
                "wholesale_price": "8.80",
                "init_stock": 0,
                "min_stock_limit": 1
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let header_request_id = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(header_request_id, "req-missing-idem-001");

    let body = response_json(resp).await;
    assert_eq!(body["code"], 4000);
    assert_eq!(body["request_id"], "req-missing-idem-001");
}

#[tokio::test]
async fn update_product_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/products/1001")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "name": "测试商品-更新缺幂等键-自动request-id"
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn purchase_order_create_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/purchase-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "biz_no": "PO-MISSING-IDEM-AUTO-RID-001",
                "items": [{
                    "product_id": 1001,
                    "qty": 1,
                    "unit_cost": "2.20"
                }]
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn inbound_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "product_id": 1001,
                "qty": 1,
                "unit_cost": "2.20",
                "biz_no": "IN-MISSING-IDEM-AUTO-RID-001"
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn outbound_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/outbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "biz_no": "OUT-MISSING-IDEM-AUTO-RID-001",
                "items": [{
                    "product_id": 1001,
                    "qty": 1,
                    "sell_price": "3.50"
                }]
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn create_product_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/products")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "barcode": "6991234500199",
                "name": "测试商品-缺幂等键-自动request-id",
                "unit": "件",
                "retail_price": "9.90",
                "wholesale_price": "8.80",
                "init_stock": 0,
                "min_stock_limit": 1
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn delete_product_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/products/1001?expected_version=1")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-delete-missing-idem-001")
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response(resp, "req-delete-missing-idem-001").await;
}

#[tokio::test]
async fn purchase_order_confirm_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/purchase-orders/3001/confirm")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-po-confirm-missing-idem-001")
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response(resp, "req-po-confirm-missing-idem-001").await;
}

#[tokio::test]
async fn purchase_order_void_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/purchase-orders/3001/void")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-po-void-missing-idem-001")
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response(resp, "req-po-void-missing-idem-001").await;
}

#[tokio::test]
async fn create_sales_order_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-so-create-missing-idem-001")
        .body(Body::from(
            json!({
                "biz_no": "SO-MISSING-IDEM-001",
                "items": [{
                    "product_id": 1001,
                    "qty": 1,
                    "sell_price": "3.50"
                }]
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response(resp, "req-so-create-missing-idem-001").await;
}

#[tokio::test]
async fn sales_order_confirm_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders/4001/confirm")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-so-confirm-missing-idem-001")
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response(resp, "req-so-confirm-missing-idem-001").await;
}

#[tokio::test]
async fn sales_order_void_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders/4001/void")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-so-void-missing-idem-001")
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response(resp, "req-so-void-missing-idem-001").await;
}

#[tokio::test]
async fn sales_order_return_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders/4001/return")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-so-return-missing-idem-001")
        .body(Body::from(
            json!({
                "expected_version": 1,
                "items": [{
                    "product_id": 1001,
                    "qty": 1
                }]
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response(resp, "req-so-return-missing-idem-001").await;
}

#[tokio::test]
async fn create_stock_check_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/stock-checks")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-sc-create-missing-idem-001")
        .body(Body::from(
            json!({
                "biz_no": "SC-MISSING-IDEM-001",
                "items": [{
                    "product_id": 1001
                }]
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response(resp, "req-sc-create-missing-idem-001").await;
}

#[tokio::test]
async fn stock_check_start_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/stock-checks/5001/start")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-sc-start-missing-idem-001")
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response(resp, "req-sc-start-missing-idem-001").await;
}

#[tokio::test]
async fn stock_check_confirm_without_idempotency_key_returns_4000_and_echo_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/stock-checks/5001/confirm")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", "req-sc-confirm-missing-idem-001")
        .body(Body::from(
            json!({
                "expected_version": 2,
                "items": [{
                    "product_id": 1001,
                    "actual_stock": 100
                }]
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response(resp, "req-sc-confirm-missing-idem-001").await;
}

#[tokio::test]
async fn delete_product_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/products/1001?expected_version=1")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn purchase_order_confirm_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/purchase-orders/3001/confirm")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn purchase_order_void_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/purchase-orders/3001/void")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn create_sales_order_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "biz_no": "SO-MISSING-IDEM-AUTO-RID-001",
                "items": [{
                    "product_id": 1001,
                    "qty": 1,
                    "sell_price": "3.50"
                }]
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn sales_order_confirm_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders/4001/confirm")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn sales_order_void_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders/4001/void")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn sales_order_return_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders/4001/return")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "expected_version": 1,
                "items": [{
                    "product_id": 1001,
                    "qty": 1
                }]
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn create_stock_check_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/stock-checks")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "biz_no": "SC-MISSING-IDEM-AUTO-RID-001",
                "items": [{
                    "product_id": 1001
                }]
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn stock_check_start_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/stock-checks/5001/start")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn stock_check_confirm_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/stock-checks/5001/confirm")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "expected_version": 2,
                "items": [{
                    "product_id": 1001,
                    "actual_stock": 100
                }]
            })
            .to_string(),
        ))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_missing_idempotency_key_response_with_generated_request_id(resp).await;
}

#[tokio::test]
async fn idempotency_payload_conflict_should_echo_current_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/products")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-rid-001")
        .header("x-request-id", "req-idem-conflict-first")
        .body(Body::from(
            json!({
                "barcode": "6991234500101",
                "name": "测试商品-冲突A",
                "unit": "件",
                "retail_price": "9.90",
                "wholesale_price": "8.80",
                "init_stock": 0,
                "min_stock_limit": 1
            })
            .to_string(),
        ))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/products")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-rid-001")
        .header("x-request-id", "req-idem-conflict-second")
        .body(Body::from(
            json!({
                "barcode": "6991234500101",
                "name": "测试商品-冲突B",
                "unit": "件",
                "retail_price": "9.90",
                "wholesale_price": "8.80",
                "init_stock": 0,
                "min_stock_limit": 1
            })
            .to_string(),
        ))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-second").await;
}

#[tokio::test]
async fn update_product_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload1 = json!({
        "name": "可口可乐 330ml-幂等冲突A",
        "expected_version": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/products/1001")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-update-001")
        .header("x-request-id", "req-idem-conflict-update-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "name": "可口可乐 330ml-幂等冲突B",
        "expected_version": 1
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/products/1001")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-update-001")
        .header("x-request-id", "req-idem-conflict-update-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-update-second")
        .await;
}

#[tokio::test]
async fn purchase_order_create_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload1 = json!({
        "biz_no": "PO-CONFLICT-001",
        "items": [{
            "product_id": 1001,
            "qty": 1,
            "unit_cost": "2.20"
        }]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/purchase-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-po-create-001")
        .header("x-request-id", "req-idem-conflict-po-create-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "biz_no": "PO-CONFLICT-001",
        "items": [{
            "product_id": 1001,
            "qty": 2,
            "unit_cost": "2.20"
        }]
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/purchase-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-po-create-001")
        .header("x-request-id", "req-idem-conflict-po-create-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-po-create-second")
        .await;
}

#[tokio::test]
async fn inbound_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload1 = json!({
        "product_id": 1001,
        "qty": 1,
        "unit_cost": "2.20",
        "expected_version": 1,
        "biz_no": "IN-CONFLICT-001"
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-inbound-001")
        .header("x-request-id", "req-idem-conflict-inbound-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "product_id": 1001,
        "qty": 2,
        "unit_cost": "2.20",
        "expected_version": 1,
        "biz_no": "IN-CONFLICT-001"
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-inbound-001")
        .header("x-request-id", "req-idem-conflict-inbound-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-inbound-second")
        .await;
}

#[tokio::test]
async fn outbound_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
{
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload1 = json!({
        "biz_no": "OUT-CONFLICT-001",
        "expected_version": 1,
        "items": [{
            "product_id": 1001,
            "qty": 1,
            "sell_price": "3.50"
        }]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/outbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-outbound-001")
        .header("x-request-id", "req-idem-conflict-outbound-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "biz_no": "OUT-CONFLICT-001",
        "expected_version": 1,
        "items": [{
            "product_id": 1001,
            "qty": 2,
            "sell_price": "3.50"
        }]
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/outbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-outbound-001")
        .header("x-request-id", "req-idem-conflict-outbound-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-outbound-second")
        .await;
}

#[tokio::test]
async fn create_sales_order_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload1 = json!({
        "biz_no": "SO-CONFLICT-001",
        "items": [{
            "product_id": 1001,
            "qty": 1,
            "sell_price": "3.50"
        }]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-so-create-001")
        .header("x-request-id", "req-idem-conflict-so-create-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "biz_no": "SO-CONFLICT-001",
        "items": [{
            "product_id": 1001,
            "qty": 2,
            "sell_price": "3.50"
        }]
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-so-create-001")
        .header("x-request-id", "req-idem-conflict-so-create-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-so-create-second")
        .await;
}

#[tokio::test]
async fn delete_product_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/products")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-conflict-delete-create-product-001",
                )
                .body(Body::from(
                    json!({
                        "barcode": "6991234500301",
                        "name": "测试商品-删除冲突",
                        "unit": "件",
                        "retail_price": "9.90",
                        "wholesale_price": "8.80",
                        "init_stock": 0,
                        "min_stock_limit": 1
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let product_id = create_body["data"]["id"].as_i64().expect("product id");

    let delete_uri_payload_a = format!("/api/v1/products/{product_id}?expected_version=1");

    let req1 = Request::builder()
        .method(Method::DELETE)
        .uri(&delete_uri_payload_a)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-delete-001")
        .header("x-request-id", "req-idem-conflict-delete-first")
        .body(Body::empty())
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let delete_uri_payload_b = format!("/api/v1/products/{product_id}?expected_version=2");

    let req2 = Request::builder()
        .method(Method::DELETE)
        .uri(&delete_uri_payload_b)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-delete-001")
        .header("x-request-id", "req-idem-conflict-delete-second")
        .body(Body::empty())
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-delete-second")
        .await;
}

#[tokio::test]
async fn purchase_order_confirm_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/purchase-orders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-conflict-po-confirm-create-order-001",
                )
                .body(Body::from(
                    json!({
                        "biz_no": "PO-CONFLICT-CONFIRM-001",
                        "items": [{
                            "product_id": 1001,
                            "qty": 2,
                            "unit_cost": "2.30"
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"]
        .as_i64()
        .expect("purchase order id");

    let confirm_uri = format!("/api/v1/purchase-orders/{order_id}/confirm");

    let payload1 = json!({
        "expected_version": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&confirm_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-po-confirm-001")
        .header("x-request-id", "req-idem-conflict-po-confirm-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "expected_version": 2
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&confirm_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-po-confirm-001")
        .header("x-request-id", "req-idem-conflict-po-confirm-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-po-confirm-second")
        .await;
}

#[tokio::test]
async fn purchase_order_void_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/purchase-orders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-conflict-po-void-create-order-001",
                )
                .body(Body::from(
                    json!({
                        "biz_no": "PO-CONFLICT-VOID-001",
                        "items": [{
                            "product_id": 1001,
                            "qty": 2,
                            "unit_cost": "2.30"
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"]
        .as_i64()
        .expect("purchase order id");

    let void_uri = format!("/api/v1/purchase-orders/{order_id}/void");

    let payload1 = json!({
        "expected_version": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&void_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-po-void-001")
        .header("x-request-id", "req-idem-conflict-po-void-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "expected_version": 2
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&void_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-po-void-001")
        .header("x-request-id", "req-idem-conflict-po-void-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-po-void-second")
        .await;
}

#[tokio::test]
async fn sales_order_confirm_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/sales-orders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-conflict-so-confirm-create-order-001",
                )
                .body(Body::from(
                    json!({
                        "biz_no": "SO-CONFLICT-CONFIRM-001",
                        "items": [{
                            "product_id": 1001,
                            "qty": 2,
                            "sell_price": "3.50"
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"].as_i64().expect("sales order id");

    let confirm_uri = format!("/api/v1/sales-orders/{order_id}/confirm");

    let payload1 = json!({
        "expected_version": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&confirm_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-so-confirm-001")
        .header("x-request-id", "req-idem-conflict-so-confirm-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "expected_version": 2
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&confirm_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-so-confirm-001")
        .header("x-request-id", "req-idem-conflict-so-confirm-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-so-confirm-second")
        .await;
}

#[tokio::test]
async fn sales_order_void_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/sales-orders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-conflict-so-void-create-order-001",
                )
                .body(Body::from(
                    json!({
                        "biz_no": "SO-CONFLICT-VOID-001",
                        "items": [{
                            "product_id": 1001,
                            "qty": 2,
                            "sell_price": "3.50"
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"].as_i64().expect("sales order id");

    let void_uri = format!("/api/v1/sales-orders/{order_id}/void");

    let payload1 = json!({
        "expected_version": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&void_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-so-void-001")
        .header("x-request-id", "req-idem-conflict-so-void-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "expected_version": 2
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&void_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-so-void-001")
        .header("x-request-id", "req-idem-conflict-so-void-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-so-void-second")
        .await;
}

#[tokio::test]
async fn sales_order_return_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/sales-orders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-conflict-so-return-create-order-001",
                )
                .body(Body::from(
                    json!({
                        "biz_no": "SO-CONFLICT-RETURN-001",
                        "items": [{
                            "product_id": 1001,
                            "qty": 2,
                            "sell_price": "3.50"
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"].as_i64().expect("sales order id");

    let confirm_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/sales-orders/{order_id}/confirm"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-conflict-so-return-confirm-order-001",
                )
                .body(Body::from(
                    json!({
                        "expected_version": 1
                    })
                    .to_string(),
                ))
                .expect("build confirm request"),
        )
        .await
        .expect("send confirm request");
    assert_eq!(confirm_resp.status(), StatusCode::OK);

    let return_uri = format!("/api/v1/sales-orders/{order_id}/return");

    let payload1 = json!({
        "expected_version": 2,
        "items": [{
            "product_id": 1001,
            "qty": 1
        }]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&return_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-so-return-001")
        .header("x-request-id", "req-idem-conflict-so-return-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "expected_version": 2,
        "items": [{
            "product_id": 1001,
            "qty": 2
        }]
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&return_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-so-return-001")
        .header("x-request-id", "req-idem-conflict-so-return-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-so-return-second")
        .await;
}

#[tokio::test]
async fn create_stock_check_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload1 = json!({
        "biz_no": "SC-CONFLICT-001",
        "items": [{
            "product_id": 1001
        }]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/stock-checks")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-sc-create-001")
        .header("x-request-id", "req-idem-conflict-sc-create-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "biz_no": "SC-CONFLICT-002",
        "items": [{
            "product_id": 1001
        }]
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/stock-checks")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-sc-create-001")
        .header("x-request-id", "req-idem-conflict-sc-create-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-sc-create-second")
        .await;
}

#[tokio::test]
async fn stock_check_start_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/inventory/stock-checks")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-conflict-sc-start-create-stock-check-001",
                )
                .body(Body::from(
                    json!({
                        "biz_no": "SC-CONFLICT-START-001",
                        "items": [{
                            "product_id": 1001
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let stock_check_id = create_body["data"]["id"].as_i64().expect("stock check id");

    let start_uri = format!("/api/v1/inventory/stock-checks/{stock_check_id}/start");

    let payload1 = json!({
        "expected_version": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&start_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-sc-start-001")
        .header("x-request-id", "req-idem-conflict-sc-start-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "expected_version": 2
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&start_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-sc-start-001")
        .header("x-request-id", "req-idem-conflict-sc-start-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-sc-start-second")
        .await;
}

#[tokio::test]
async fn stock_check_confirm_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/inventory/stock-checks")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-conflict-sc-confirm-create-stock-check-001",
                )
                .body(Body::from(
                    json!({
                        "biz_no": "SC-CONFLICT-CONFIRM-001",
                        "items": [{
                            "product_id": 1001
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let stock_check_id = create_body["data"]["id"].as_i64().expect("stock check id");

    let start_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/inventory/stock-checks/{stock_check_id}/start"
                ))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-conflict-sc-confirm-start-stock-check-001",
                )
                .body(Body::from(
                    json!({
                        "expected_version": 1
                    })
                    .to_string(),
                ))
                .expect("build start request"),
        )
        .await
        .expect("send start request");
    assert_eq!(start_resp.status(), StatusCode::OK);

    let confirm_uri = format!("/api/v1/inventory/stock-checks/{stock_check_id}/confirm");

    let payload1 = json!({
        "expected_version": 2,
        "items": [{
            "product_id": 1001,
            "actual_stock": 103
        }]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&confirm_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-sc-confirm-001")
        .header("x-request-id", "req-idem-conflict-sc-confirm-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "expected_version": 2,
        "items": [{
            "product_id": 1001,
            "actual_stock": 104
        }]
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&confirm_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-sc-confirm-001")
        .header("x-request-id", "req-idem-conflict-sc-confirm-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-sc-confirm-second")
        .await;
}

#[tokio::test]
async fn idempotent_replay_should_keep_first_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "barcode": "6991234500011",
        "name": "测试商品-幂等request-id",
        "unit": "件",
        "retail_price": "9.90",
        "wholesale_price": "8.80",
        "init_stock": 0,
        "min_stock_limit": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/products")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-request-id-001")
        .header("x-request-id", "req-idem-first-001")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let body1 = response_json(resp1).await;
    assert_eq!(body1["request_id"], "req-idem-first-001");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/products")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-request-id-001")
        .header("x-request-id", "req-idem-second-001")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);

    let replay_header = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay_header, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-idem-first-001");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-idem-first-001");
}

#[tokio::test]
async fn refresh_endpoint_with_access_token_returns_401() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, access_token, _refresh_token) = login_tokens(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/refresh")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "refresh_token": access_token
            })
            .to_string(),
        ))
        .expect("build refresh request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn protected_route_with_refresh_token_returns_401() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, _access_token, refresh_token) = login_tokens(app).await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products?page=1&page_size=20")
        .header(header::AUTHORIZATION, format!("Bearer {}", refresh_token))
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn logout_with_access_token_returns_200_and_user_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, access_token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/logout")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .body(Body::empty())
        .expect("build logout request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 200);
    assert_eq!(body["data"]["logged_out"], true);
    assert!(body["data"]["user_id"].as_str().is_some());
}

#[tokio::test]
async fn logout_with_refresh_token_returns_401() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, _access_token, refresh_token) = login_tokens(app).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/logout")
        .header(header::AUTHORIZATION, format!("Bearer {}", refresh_token))
        .body(Body::empty())
        .expect("build logout request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn protected_route_without_token_returns_401() {
    let state = make_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products?page=1&page_size=20")
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    let body: Value = serde_json::from_slice(&bytes).expect("json parse");
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn create_product_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "barcode": "6991234500001",
        "name": "测试商品-幂等",
        "unit": "件",
        "retail_price": "9.90",
        "wholesale_price": "8.80",
        "init_stock": 0,
        "min_stock_limit": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/products")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-prod-001")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/products")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-prod-001")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");
}

#[tokio::test]
async fn update_product_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "name": "可口可乐 330ml-幂等更新",
        "expected_version": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/products/1001")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-prod-update-replay-001")
        .header("x-request-id", "req-prod-update-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/products/1001")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-prod-update-replay-001")
        .header("x-request-id", "req-prod-update-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-prod-update-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-prod-update-replay-first");
}

#[tokio::test]
async fn purchase_order_create_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "biz_no": "PO-IDEM-REPLAY-001",
        "items": [{
            "product_id": 1001,
            "qty": 2,
            "unit_cost": "2.10"
        }]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/purchase-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-po-create-replay-001")
        .header("x-request-id", "req-po-create-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/purchase-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-po-create-replay-001")
        .header("x-request-id", "req-po-create-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-po-create-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-po-create-replay-first");
}

#[tokio::test]
async fn inbound_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "product_id": 1001,
        "qty": 3,
        "unit_cost": "2.20",
        "expected_version": 1,
        "biz_no": "IN-IDEM-REPLAY-001"
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-inbound-replay-001")
        .header("x-request-id", "req-inbound-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-inbound-replay-001")
        .header("x-request-id", "req-inbound-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-inbound-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-inbound-replay-first");
}

#[tokio::test]
async fn outbound_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "biz_no": "OUT-IDEM-REPLAY-001",
        "expected_version": 1,
        "items": [{
            "product_id": 1001,
            "qty": 1,
            "sell_price": "3.50"
        }]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/outbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-outbound-replay-001")
        .header("x-request-id", "req-outbound-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/outbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-outbound-replay-001")
        .header("x-request-id", "req-outbound-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-outbound-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-outbound-replay-first");
}

#[tokio::test]
async fn delete_product_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/products")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-test-delete-replay-create-product-001",
                )
                .body(Body::from(
                    json!({
                        "barcode": "6991234500201",
                        "name": "测试商品-删除幂等回放",
                        "unit": "件",
                        "retail_price": "9.90",
                        "wholesale_price": "8.80",
                        "init_stock": 0,
                        "min_stock_limit": 1
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let product_id = create_body["data"]["id"].as_i64().expect("product id");

    let delete_uri = format!("/api/v1/products/{product_id}?expected_version=1");

    let req1 = Request::builder()
        .method(Method::DELETE)
        .uri(&delete_uri)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-delete-replay-001")
        .header("x-request-id", "req-delete-replay-first")
        .body(Body::empty())
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::DELETE)
        .uri(&delete_uri)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-delete-replay-001")
        .header("x-request-id", "req-delete-replay-second")
        .body(Body::empty())
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-delete-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-delete-replay-first");
}

#[tokio::test]
async fn purchase_order_confirm_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/purchase-orders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-test-po-confirm-replay-create-001",
                )
                .body(Body::from(
                    json!({
                        "biz_no": "PO-CONFIRM-REPLAY-001",
                        "items": [{
                            "product_id": 1001,
                            "qty": 2,
                            "unit_cost": "2.30"
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"]
        .as_i64()
        .expect("purchase order id");

    let confirm_uri = format!("/api/v1/purchase-orders/{order_id}/confirm");
    let payload = json!({
        "expected_version": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&confirm_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-po-confirm-replay-001")
        .header("x-request-id", "req-po-confirm-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&confirm_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-po-confirm-replay-001")
        .header("x-request-id", "req-po-confirm-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-po-confirm-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-po-confirm-replay-first");
}

#[tokio::test]
async fn purchase_order_void_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/purchase-orders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header("X-Idempotency-Key", "idem-test-po-void-replay-create-001")
                .body(Body::from(
                    json!({
                        "biz_no": "PO-VOID-REPLAY-001",
                        "items": [{
                            "product_id": 1001,
                            "qty": 2,
                            "unit_cost": "2.30"
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"]
        .as_i64()
        .expect("purchase order id");

    let void_uri = format!("/api/v1/purchase-orders/{order_id}/void");
    let payload = json!({
        "expected_version": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&void_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-po-void-replay-001")
        .header("x-request-id", "req-po-void-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&void_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-po-void-replay-001")
        .header("x-request-id", "req-po-void-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-po-void-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-po-void-replay-first");
}

#[tokio::test]
async fn create_sales_order_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "biz_no": "SO-CREATE-REPLAY-001",
        "items": [{
            "product_id": 1001,
            "qty": 2,
            "sell_price": "3.50"
        }]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-so-create-replay-001")
        .header("x-request-id", "req-so-create-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-so-create-replay-001")
        .header("x-request-id", "req-so-create-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-so-create-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-so-create-replay-first");
}

#[tokio::test]
async fn sales_order_confirm_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/sales-orders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-test-so-confirm-replay-create-001",
                )
                .body(Body::from(
                    json!({
                        "biz_no": "SO-CONFIRM-REPLAY-001",
                        "items": [{
                            "product_id": 1001,
                            "qty": 2,
                            "sell_price": "3.50"
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"].as_i64().expect("sales order id");

    let confirm_uri = format!("/api/v1/sales-orders/{order_id}/confirm");
    let payload = json!({
        "expected_version": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&confirm_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-so-confirm-replay-001")
        .header("x-request-id", "req-so-confirm-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&confirm_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-so-confirm-replay-001")
        .header("x-request-id", "req-so-confirm-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-so-confirm-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-so-confirm-replay-first");
}

#[tokio::test]
async fn sales_order_void_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/sales-orders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header("X-Idempotency-Key", "idem-test-so-void-replay-create-001")
                .body(Body::from(
                    json!({
                        "biz_no": "SO-VOID-REPLAY-001",
                        "items": [{
                            "product_id": 1001,
                            "qty": 2,
                            "sell_price": "3.50"
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"].as_i64().expect("sales order id");

    let void_uri = format!("/api/v1/sales-orders/{order_id}/void");
    let payload = json!({
        "expected_version": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&void_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-so-void-replay-001")
        .header("x-request-id", "req-so-void-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&void_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-so-void-replay-001")
        .header("x-request-id", "req-so-void-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-so-void-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-so-void-replay-first");
}

#[tokio::test]
async fn sales_order_return_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/sales-orders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header("X-Idempotency-Key", "idem-test-so-return-replay-create-001")
                .body(Body::from(
                    json!({
                        "biz_no": "SO-RETURN-REPLAY-001",
                        "items": [{
                            "product_id": 1001,
                            "qty": 2,
                            "sell_price": "3.50"
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"].as_i64().expect("sales order id");

    let confirm_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/sales-orders/{order_id}/confirm"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-test-so-return-replay-confirm-001",
                )
                .body(Body::from(
                    json!({
                        "expected_version": 1
                    })
                    .to_string(),
                ))
                .expect("build confirm request"),
        )
        .await
        .expect("send confirm request");
    assert_eq!(confirm_resp.status(), StatusCode::OK);

    let return_uri = format!("/api/v1/sales-orders/{order_id}/return");
    let payload = json!({
        "expected_version": 2,
        "items": [{
            "product_id": 1001,
            "qty": 1
        }]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&return_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-so-return-replay-001")
        .header("x-request-id", "req-so-return-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&return_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-so-return-replay-001")
        .header("x-request-id", "req-so-return-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-so-return-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-so-return-replay-first");
}

#[tokio::test]
async fn create_stock_check_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "biz_no": "SC-CREATE-REPLAY-001",
        "items": [{
            "product_id": 1001
        }]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/stock-checks")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-sc-create-replay-001")
        .header("x-request-id", "req-sc-create-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/stock-checks")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-sc-create-replay-001")
        .header("x-request-id", "req-sc-create-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-sc-create-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-sc-create-replay-first");
}

#[tokio::test]
async fn stock_check_start_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/inventory/stock-checks")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header("X-Idempotency-Key", "idem-test-sc-start-replay-create-001")
                .body(Body::from(
                    json!({
                        "biz_no": "SC-START-REPLAY-001",
                        "items": [{
                            "product_id": 1001
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let stock_check_id = create_body["data"]["id"].as_i64().expect("stock check id");

    let start_uri = format!("/api/v1/inventory/stock-checks/{stock_check_id}/start");
    let payload = json!({
        "expected_version": 1
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&start_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-sc-start-replay-001")
        .header("x-request-id", "req-sc-start-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&start_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-sc-start-replay-001")
        .header("x-request-id", "req-sc-start-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-sc-start-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-sc-start-replay-first");
}

#[tokio::test]
async fn stock_check_confirm_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/inventory/stock-checks")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(
                    "X-Idempotency-Key",
                    "idem-test-sc-confirm-replay-create-001",
                )
                .body(Body::from(
                    json!({
                        "biz_no": "SC-CONFIRM-REPLAY-001",
                        "items": [{
                            "product_id": 1001
                        }]
                    })
                    .to_string(),
                ))
                .expect("build create request"),
        )
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let stock_check_id = create_body["data"]["id"].as_i64().expect("stock check id");

    let start_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/inventory/stock-checks/{stock_check_id}/start"
                ))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header("X-Idempotency-Key", "idem-test-sc-confirm-replay-start-001")
                .body(Body::from(
                    json!({
                        "expected_version": 1
                    })
                    .to_string(),
                ))
                .expect("build start request"),
        )
        .await
        .expect("send start request");
    assert_eq!(start_resp.status(), StatusCode::OK);

    let confirm_uri = format!("/api/v1/inventory/stock-checks/{stock_check_id}/confirm");
    let payload = json!({
        "expected_version": 2,
        "items": [{
            "product_id": 1001,
            "actual_stock": 103
        }]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri(&confirm_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-sc-confirm-replay-001")
        .header("x-request-id", "req-sc-confirm-replay-first")
        .body(Body::from(payload.clone()))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);
    let replay1 = resp1
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay1, "false");

    let req2 = Request::builder()
        .method(Method::POST)
        .uri(&confirm_uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-sc-confirm-replay-001")
        .header("x-request-id", "req-sc-confirm-replay-second")
        .body(Body::from(payload))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let replay2 = resp2
        .headers()
        .get("x-idempotent-replay")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("false");
    assert_eq!(replay2, "true");

    let replay_request_id = resp2
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .expect("x-request-id should exist")
        .to_string();
    assert_eq!(replay_request_id, "req-sc-confirm-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-sc-confirm-replay-first");
}

#[tokio::test]
async fn sales_role_cannot_export_sales_report() {
    let state = make_test_state();

    // mutate state before building router: create a SALES user under admin's tenant
    {
        let users_arc = state.users.clone();
        let mut users = users_arc.lock().expect("lock users");
        let owner = users.get("admin").expect("admin exists").clone();
        users.insert(
            "sales_test".to_string(),
            crate::models::User {
                id: uuid::Uuid::new_v4(),
                tenant_id: owner.tenant_id,
                username: "sales_test".to_string(),
                name: "销售测试".to_string(),
                role: crate::models::UserRole::Sales,
                password_hash: crate::models::hash_password("sales123"),
            },
        );
    }

    let app = build_router(state);

    let req_sales_login = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "username": "sales_test", "password": "sales123" }).to_string(),
        ))
        .expect("build sales login req");
    let resp_sales_login = app
        .clone()
        .oneshot(req_sales_login)
        .await
        .expect("sales login ok");
    assert_eq!(resp_sales_login.status(), StatusCode::OK);

    let bytes = resp_sales_login
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    let v: Value = serde_json::from_slice(&bytes).expect("json");
    let sales_token = v["data"]["access_token"]
        .as_str()
        .expect("sales token")
        .to_string();

    let req_export = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/reports/sales/export?group_by=product&format=xlsx")
        .header(header::AUTHORIZATION, format!("Bearer {}", sales_token))
        .body(Body::empty())
        .expect("build export req");

    let resp_export = app.oneshot(req_export).await.expect("send export req");
    assert_eq!(resp_export.status(), StatusCode::FORBIDDEN);

    let body_bytes = resp_export
        .into_body()
        .collect()
        .await
        .expect("read export body")
        .to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).expect("json parse");
    assert_eq!(body["code"], 4030);
}

#[tokio::test]
async fn purchase_order_state_machine_should_confirm_and_void_with_stock_roundtrip() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/purchase-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-po-create-flow-001")
        .body(Body::from(
            json!({
                "biz_no": "PO-TEST-FLOW-001",
                "items": [{
                    "product_id": 1001,
                    "qty": 5,
                    "unit_cost": "2.50"
                }]
            })
            .to_string(),
        ))
        .expect("build create purchase request");
    let create_resp = app
        .clone()
        .oneshot(create_req)
        .await
        .expect("send create purchase request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"]
        .as_i64()
        .expect("purchase order id");
    assert_eq!(create_body["data"]["status"], "DRAFT");
    assert_eq!(create_body["data"]["version"], 1);

    let confirm_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/purchase-orders/{order_id}/confirm"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-po-confirm-flow-001")
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build confirm purchase request");
    let confirm_resp = app
        .clone()
        .oneshot(confirm_req)
        .await
        .expect("send confirm purchase request");
    assert_eq!(confirm_resp.status(), StatusCode::OK);
    let confirm_body = response_json(confirm_resp).await;
    assert_eq!(confirm_body["data"]["status"], "CONFIRMED");
    assert_eq!(confirm_body["data"]["version"], 2);

    let scan_after_confirm_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products/scan?barcode=690123456789")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build scan request");
    let scan_after_confirm_resp = app
        .clone()
        .oneshot(scan_after_confirm_req)
        .await
        .expect("send scan request");
    assert_eq!(scan_after_confirm_resp.status(), StatusCode::OK);
    let scan_after_confirm_body = response_json(scan_after_confirm_resp).await;
    assert_eq!(scan_after_confirm_body["data"]["current_stock"], 105);

    let void_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/purchase-orders/{order_id}/void"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-po-void-flow-001")
        .body(Body::from(
            json!({
                "expected_version": 2
            })
            .to_string(),
        ))
        .expect("build void purchase request");
    let void_resp = app
        .clone()
        .oneshot(void_req)
        .await
        .expect("send void purchase request");
    assert_eq!(void_resp.status(), StatusCode::OK);
    let void_body = response_json(void_resp).await;
    assert_eq!(void_body["data"]["status"], "VOIDED");
    assert_eq!(void_body["data"]["version"], 3);

    let scan_after_void_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products/scan?barcode=690123456789")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build scan request");
    let scan_after_void_resp = app
        .oneshot(scan_after_void_req)
        .await
        .expect("send scan request");
    assert_eq!(scan_after_void_resp.status(), StatusCode::OK);
    let scan_after_void_body = response_json(scan_after_void_resp).await;
    assert_eq!(scan_after_void_body["data"]["current_stock"], 100);
}

#[tokio::test]
async fn sales_order_state_machine_should_reach_returned_full_and_restore_stock() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-so-create-flow-001")
        .body(Body::from(
            json!({
                "biz_no": "SO-TEST-FLOW-001",
                "items": [{
                    "product_id": 1001,
                    "qty": 3,
                    "sell_price": "3.50"
                }]
            })
            .to_string(),
        ))
        .expect("build create sales request");
    let create_resp = app
        .clone()
        .oneshot(create_req)
        .await
        .expect("send create sales request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"].as_i64().expect("sales order id");
    assert_eq!(create_body["data"]["status"], "DRAFT");

    let confirm_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/sales-orders/{order_id}/confirm"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-so-confirm-flow-001")
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build confirm sales request");
    let confirm_resp = app
        .clone()
        .oneshot(confirm_req)
        .await
        .expect("send confirm sales request");
    assert_eq!(confirm_resp.status(), StatusCode::OK);
    let confirm_body = response_json(confirm_resp).await;
    assert_eq!(confirm_body["data"]["status"], "CONFIRMED");
    assert_eq!(confirm_body["data"]["version"], 2);

    let return_partial_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/sales-orders/{order_id}/return"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-so-return-partial-flow-001")
        .body(Body::from(
            json!({
                "expected_version": 2,
                "items": [{
                    "product_id": 1001,
                    "qty": 1
                }]
            })
            .to_string(),
        ))
        .expect("build partial return request");
    let return_partial_resp = app
        .clone()
        .oneshot(return_partial_req)
        .await
        .expect("send partial return request");
    assert_eq!(return_partial_resp.status(), StatusCode::OK);
    let return_partial_body = response_json(return_partial_resp).await;
    assert_eq!(return_partial_body["data"]["status"], "RETURNED_PARTIAL");
    assert_eq!(return_partial_body["data"]["version"], 3);
    assert_eq!(return_partial_body["data"]["items"][0]["returned_qty"], 1);

    let return_full_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/sales-orders/{order_id}/return"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-so-return-full-flow-001")
        .body(Body::from(
            json!({
                "expected_version": 3,
                "items": [{
                    "product_id": 1001,
                    "qty": 2
                }]
            })
            .to_string(),
        ))
        .expect("build full return request");
    let return_full_resp = app
        .clone()
        .oneshot(return_full_req)
        .await
        .expect("send full return request");
    assert_eq!(return_full_resp.status(), StatusCode::OK);
    let return_full_body = response_json(return_full_resp).await;
    assert_eq!(return_full_body["data"]["status"], "RETURNED_FULL");
    assert_eq!(return_full_body["data"]["version"], 4);
    assert_eq!(return_full_body["data"]["items"][0]["returned_qty"], 3);

    let scan_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products/scan?barcode=690123456789")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build scan request");
    let scan_resp = app.oneshot(scan_req).await.expect("send scan request");
    assert_eq!(scan_resp.status(), StatusCode::OK);
    let scan_body = response_json(scan_resp).await;
    assert_eq!(scan_body["data"]["current_stock"], 100);
}

#[tokio::test]
async fn stock_check_state_machine_should_move_to_confirmed_and_apply_delta() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/stock-checks")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-sc-create-flow-001")
        .body(Body::from(
            json!({
                "biz_no": "SC-TEST-FLOW-001",
                "items": [{ "product_id": 1001 }]
            })
            .to_string(),
        ))
        .expect("build create stock check request");
    let create_resp = app
        .clone()
        .oneshot(create_req)
        .await
        .expect("send create stock check request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let check_id = create_body["data"]["id"].as_i64().expect("stock check id");
    assert_eq!(create_body["data"]["status"], "DRAFT");
    assert_eq!(create_body["data"]["version"], 1);

    let start_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/inventory/stock-checks/{check_id}/start"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-sc-start-flow-001")
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build start stock check request");
    let start_resp = app
        .clone()
        .oneshot(start_req)
        .await
        .expect("send start stock check request");
    assert_eq!(start_resp.status(), StatusCode::OK);
    let start_body = response_json(start_resp).await;
    assert_eq!(start_body["data"]["status"], "COUNTING");
    assert_eq!(start_body["data"]["version"], 2);

    let confirm_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/inventory/stock-checks/{check_id}/confirm"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-sc-confirm-flow-001")
        .body(Body::from(
            json!({
                "expected_version": 2,
                "items": [{
                    "product_id": 1001,
                    "actual_stock": 103
                }]
            })
            .to_string(),
        ))
        .expect("build confirm stock check request");
    let confirm_resp = app
        .clone()
        .oneshot(confirm_req)
        .await
        .expect("send confirm stock check request");
    assert_eq!(confirm_resp.status(), StatusCode::OK);
    let confirm_body = response_json(confirm_resp).await;
    assert_eq!(confirm_body["data"]["status"], "CONFIRMED");
    assert_eq!(confirm_body["data"]["version"], 3);
    assert_eq!(confirm_body["data"]["items"][0]["delta_qty"], 3);

    let scan_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products/scan?barcode=690123456789")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build scan request");
    let scan_resp = app.oneshot(scan_req).await.expect("send scan request");
    assert_eq!(scan_resp.status(), StatusCode::OK);
    let scan_body = response_json(scan_resp).await;
    assert_eq!(scan_body["data"]["current_stock"], 103);
}

#[tokio::test]
async fn expected_version_conflict_should_return_4091_with_latest_snapshot() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/purchase-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-po-create-4091-001")
        .body(Body::from(
            json!({
                "biz_no": "PO-TEST-4091-001",
                "items": [{
                    "product_id": 1001,
                    "qty": 1,
                    "unit_cost": "2.20"
                }]
            })
            .to_string(),
        ))
        .expect("build create request");
    let create_resp = app
        .clone()
        .oneshot(create_req)
        .await
        .expect("send create request");
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body = response_json(create_resp).await;
    let order_id = create_body["data"]["id"]
        .as_i64()
        .expect("purchase order id");

    let conflict_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/purchase-orders/{order_id}/confirm"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-po-confirm-4091-001")
        .body(Body::from(
            json!({
                "expected_version": 999
            })
            .to_string(),
        ))
        .expect("build conflict request");
    let conflict_resp = app
        .oneshot(conflict_req)
        .await
        .expect("send conflict request");
    assert_eq!(conflict_resp.status(), StatusCode::CONFLICT);
    let conflict_body = response_json(conflict_resp).await;
    assert_eq!(conflict_body["code"], 4091);
    assert_eq!(conflict_body["data"]["resource"], "purchase_order");
    assert_eq!(conflict_body["data"]["current_version"], 1);
    assert!(conflict_body["data"]["latest_snapshot"].is_object());
}

#[tokio::test]
async fn idempotency_payload_conflict_should_return_4092() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/products")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-prod-4092-001")
        .body(Body::from(
            json!({
                "barcode": "6991234500009",
                "name": "测试商品-4092-A",
                "unit": "件",
                "retail_price": "9.90",
                "wholesale_price": "8.80",
                "init_stock": 0,
                "min_stock_limit": 1
            })
            .to_string(),
        ))
        .expect("build req1");
    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/products")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-prod-4092-001")
        .body(Body::from(
            json!({
                "barcode": "6991234500009",
                "name": "测试商品-4092-B",
                "unit": "件",
                "retail_price": "9.90",
                "wholesale_price": "8.80",
                "init_stock": 0,
                "min_stock_limit": 1
            })
            .to_string(),
        ))
        .expect("build req2");
    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_eq!(resp2.status(), StatusCode::CONFLICT);
    let body = response_json(resp2).await;
    assert_eq!(body["code"], 4092);
}

#[tokio::test]
async fn sales_report_export_csv_should_match_sales_report_summary_and_detail() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let create_sales_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/sales-orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-export-sales-create-001")
        .body(Body::from(
            json!({
                "biz_no": "SO-EXPORT-CHECK-001",
                "items": [{
                    "product_id": 1001,
                    "qty": 3,
                    "sell_price": "3.50"
                }]
            })
            .to_string(),
        ))
        .expect("build create sales request");
    let create_sales_resp = app
        .clone()
        .oneshot(create_sales_req)
        .await
        .expect("send create sales request");
    assert_eq!(create_sales_resp.status(), StatusCode::OK);
    let create_sales_body = response_json(create_sales_resp).await;
    let order_id = create_sales_body["data"]["id"]
        .as_i64()
        .expect("sales order id");

    let confirm_sales_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/sales-orders/{order_id}/confirm"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-export-sales-confirm-001")
        .body(Body::from(
            json!({
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build confirm sales request");
    let confirm_sales_resp = app
        .clone()
        .oneshot(confirm_sales_req)
        .await
        .expect("send confirm sales request");
    assert_eq!(confirm_sales_resp.status(), StatusCode::OK);

    let return_sales_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/sales-orders/{order_id}/return"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-export-sales-return-001")
        .body(Body::from(
            json!({
                "expected_version": 2,
                "items": [{
                    "product_id": 1001,
                    "qty": 1
                }]
            })
            .to_string(),
        ))
        .expect("build return sales request");
    let return_sales_resp = app
        .clone()
        .oneshot(return_sales_req)
        .await
        .expect("send return sales request");
    assert_eq!(return_sales_resp.status(), StatusCode::OK);

    let report_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/reports/sales?group_by=product&page=1&page_size=20")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build report request");
    let report_resp = app
        .clone()
        .oneshot(report_req)
        .await
        .expect("send report request");
    assert_eq!(report_resp.status(), StatusCode::OK);
    let report_body = response_json(report_resp).await;

    let first_row = &report_body["data"]["list"][0];
    let row_tail = format!(
        ",{},{},{},{}",
        first_row["total_qty"].as_i64().expect("row qty"),
        first_row["total_sales"].as_str().expect("row sales"),
        first_row["total_cost"].as_str().expect("row cost"),
        first_row["gross_profit"]
            .as_str()
            .expect("row gross profit")
    );

    let summary = &report_body["data"]["summary"];
    let expected_summary_line = format!(
        "SUMMARY,,{},{},{},{}",
        summary["total_qty"].as_i64().expect("summary qty"),
        summary["total_sales"].as_str().expect("summary sales"),
        summary["total_cost"].as_str().expect("summary cost"),
        summary["total_gross_profit"]
            .as_str()
            .expect("summary gross profit")
    );

    let export_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/reports/sales/export?group_by=product&format=csv")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build export request");
    let export_resp = app.oneshot(export_req).await.expect("send export request");
    assert_eq!(export_resp.status(), StatusCode::OK);
    let content_type = export_resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(content_type.contains("text/csv"));

    let csv_text = response_text(export_resp).await;
    let csv_text = csv_text.trim_start_matches('\u{feff}');
    let detail_line = csv_text
        .lines()
        .find(|line| line.starts_with("1001,"))
        .expect("product line should exist in csv");
    assert!(detail_line.contains(&row_tail));

    let summary_line = csv_text
        .lines()
        .find(|line| line.starts_with("SUMMARY,,"))
        .expect("summary line should exist in csv");
    assert_eq!(summary_line, expected_summary_line);
}

#[tokio::test]
async fn refresh_token_for_nonexistent_user_returns_401() {
    let state = make_test_state();

    let (tenant_id, ghost_user_id) = {
        let users = state.users.lock().expect("lock users");
        let admin = users.get("admin").expect("admin exists");
        (admin.tenant_id, uuid::Uuid::new_v4())
    };

    let ghost_refresh_token = crate::middleware::generate_token(
        &state.config.jwt_secret,
        tenant_id,
        ghost_user_id,
        "OWNER",
        "refresh",
        state.config.refresh_token_exp_secs,
    )
    .expect("generate ghost refresh token");

    let app = build_router(state);
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/refresh")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "refresh_token": ghost_refresh_token }).to_string(),
        ))
        .expect("build refresh request");

    let resp = app.oneshot(req).await.expect("send refresh request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn refresh_token_should_return_current_user_role() {
    let state = make_test_state();

    let (tenant_id, sales_user_id) = {
        let users_arc = state.users.clone();
        let mut users = users_arc.lock().expect("lock users");
        let owner = users.get("admin").expect("admin exists").clone();
        let sales_user_id = uuid::Uuid::new_v4();
        users.insert(
            "sales_refresh".to_string(),
            crate::models::User {
                id: sales_user_id,
                tenant_id: owner.tenant_id,
                username: "sales_refresh".to_string(),
                name: "销售刷新".to_string(),
                role: crate::models::UserRole::Sales,
                password_hash: crate::models::hash_password("sales123"),
            },
        );
        (owner.tenant_id, sales_user_id)
    };

    let forged_refresh_token = crate::middleware::generate_token(
        &state.config.jwt_secret,
        tenant_id,
        sales_user_id,
        "OWNER",
        "refresh",
        state.config.refresh_token_exp_secs,
    )
    .expect("generate forged refresh token");

    let app = build_router(state);

    let refresh_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/refresh")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "refresh_token": forged_refresh_token }).to_string(),
        ))
        .expect("build refresh request");
    let refresh_resp = app
        .clone()
        .oneshot(refresh_req)
        .await
        .expect("send refresh request");
    assert_eq!(refresh_resp.status(), StatusCode::OK);
    let refresh_body = response_json(refresh_resp).await;

    assert_eq!(refresh_body["data"]["user_info"]["role"], "SALES");

    let new_access_token = refresh_body["data"]["access_token"]
        .as_str()
        .expect("new access token")
        .to_string();

    let export_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/reports/sales/export?group_by=product&format=csv")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", new_access_token),
        )
        .body(Body::empty())
        .expect("build export request");
    let export_resp = app.oneshot(export_req).await.expect("send export request");
    assert_eq!(export_resp.status(), StatusCode::FORBIDDEN);
    let export_body = response_json(export_resp).await;
    assert_eq!(export_body["code"], 4030);
}

#[tokio::test]
async fn protected_route_with_nonexistent_user_token_returns_401() {
    let state = make_test_state();

    let tenant_id = {
        let users = state.users.lock().expect("lock users");
        users.get("admin").expect("admin exists").tenant_id
    };

    let forged_token = crate::middleware::generate_token(
        &state.config.jwt_secret,
        tenant_id,
        uuid::Uuid::new_v4(),
        "OWNER",
        "access",
        state.config.access_token_exp_secs,
    )
    .expect("generate token");

    let app = build_router(state);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products?page=1&page_size=20")
        .header(header::AUTHORIZATION, format!("Bearer {}", forged_token))
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4010);
}

#[tokio::test]
async fn auth_middleware_should_use_current_user_role_instead_of_token_role() {
    let state = make_test_state();

    let (tenant_id, sales_user_id) = {
        let users_arc = state.users.clone();
        let mut users = users_arc.lock().expect("lock users");
        let owner = users.get("admin").expect("admin exists").clone();
        let sales_user_id = uuid::Uuid::new_v4();
        users.insert(
            "sales_shadow".to_string(),
            crate::models::User {
                id: sales_user_id,
                tenant_id: owner.tenant_id,
                username: "sales_shadow".to_string(),
                name: "销售影子".to_string(),
                role: crate::models::UserRole::Sales,
                password_hash: crate::models::hash_password("sales123"),
            },
        );
        (owner.tenant_id, sales_user_id)
    };

    let elevated_token = crate::middleware::generate_token(
        &state.config.jwt_secret,
        tenant_id,
        sales_user_id,
        "OWNER",
        "access",
        state.config.access_token_exp_secs,
    )
    .expect("generate token");

    let app = build_router(state);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/reports/sales/export?group_by=product&format=csv")
        .header(header::AUTHORIZATION, format!("Bearer {}", elevated_token))
        .body(Body::empty())
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4030);
}

#[tokio::test]
async fn user_management_owner_can_list_and_create_users() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    // 1. List users
    let req = Request::builder()
        .uri("/api/v1/users")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 200);
    assert!(body["data"]["list"].as_array().unwrap().len() >= 1);

    // 2. Create user
    let payload = json!({
        "username": "new_user",
        "name": "新员工",
        "role": "SALES",
        "password": "password123"
    }).to_string();

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/users")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-user-create-001")
        .body(Body::from(payload))
        .expect("build req");

    let resp = app.oneshot(req).await.expect("send req");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 200);
    assert_eq!(body["data"]["username"], "new_user");
}

#[tokio::test]
async fn user_management_non_owner_cannot_list_users() {
    let state = make_test_state();
    
    // Setup a SALES user
    let tenant_id = {
        let users = state.users.lock().unwrap();
        users.get("admin").unwrap().tenant_id
    };
    let sales_user = crate::models::User {
        id: uuid::Uuid::new_v4(),
        tenant_id,
        username: "sales_test".to_string(),
        name: "销售员".to_string(),
        role: crate::models::UserRole::Sales,
        password_hash: crate::models::hash_password("sales123"),
    };
    state.users.lock().unwrap().insert(sales_user.username.clone(), sales_user);

    let app = build_router(state);
    
    // Login as SALES
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"username":"sales_test","password":"sales123"}).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = response_json(resp).await;
    let token = body["data"]["access_token"].as_str().unwrap().to_string();

    // Try to list users
    let req = Request::builder()
        .uri("/api/v1/users")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4030);
}

#[tokio::test]
async fn user_management_create_user_with_duplicate_username_returns_4090() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "username": "admin", // Duplicate
        "name": "另一个管理员",
        "role": "OWNER",
        "password": "password123"
    }).to_string();

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/users")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-user-create-dup")
        .body(Body::from(payload))
        .expect("build req");

    let resp = app.oneshot(req).await.expect("send req");
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body = response_json(resp).await;
    assert_eq!(body["code"], 4090);
}
