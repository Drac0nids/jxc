use super::*;

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
        "cost_price": "6.60",
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
