use super::*;

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
async fn inbound_batch_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "items": [
            {
                "product_id": 1001,
                "qty": 2,
                "unit_cost": "2.20",
                "expected_version": 1
            }
        ]
    })
    .to_string();

    let req1 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound/batch")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-inbound-batch-replay-001")
        .header("x-request-id", "req-inbound-batch-replay-first")
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
        .uri("/api/v1/inventory/inbound/batch")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-inbound-batch-replay-001")
        .header("x-request-id", "req-inbound-batch-replay-second")
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
    assert_eq!(replay_request_id, "req-inbound-batch-replay-first");

    let body2 = response_json(resp2).await;
    assert_eq!(body2["request_id"], "req-inbound-batch-replay-first");
}

#[tokio::test]
async fn inbound_without_unit_cost_should_fallback_to_last_inbound_unit_cost() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "product_id": 1001,
        "qty": 3,
        "expected_version": 1,
    })
    .to_string();

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-inbound-fallback-missing-unit-cost")
        .body(Body::from(payload))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["data"]["product_id"], 1001);
    assert_eq!(body["data"]["current_stock"], 103);
    assert_eq!(body["data"]["cost_price"], "2.1320");
}

#[tokio::test]
async fn inbound_with_empty_unit_cost_should_fallback_to_last_inbound_unit_cost() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
        "product_id": 1001,
        "qty": 3,
        "unit_cost": "",
        "expected_version": 1,
    })
    .to_string();

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-test-inbound-fallback-empty-unit-cost")
        .body(Body::from(payload))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("send request");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["data"]["product_id"], 1001);
    assert_eq!(body["data"]["current_stock"], 103);
    assert_eq!(body["data"]["cost_price"], "2.1320");
}

#[tokio::test]
async fn outbound_idempotent_replay_header_should_be_true_on_second_call() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload = json!({
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
                        "cost_price": "6.60",
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
