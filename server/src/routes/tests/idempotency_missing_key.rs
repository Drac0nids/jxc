use super::*;

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
                "cost_price": "6.60",
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
                "cost_price": "6.60",
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
