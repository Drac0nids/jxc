use super::*;

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
                "cost_price": "6.60",
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
                "cost_price": "6.60",
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
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-update-second").await;
}

#[tokio::test]
async fn purchase_order_create_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload1 = json!({
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
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-po-create-second").await;
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
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-inbound-second").await;
}

#[tokio::test]
async fn inbound_batch_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload1 = json!({
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
        .header("X-Idempotency-Key", "idem-conflict-inbound-batch-001")
        .header("x-request-id", "req-idem-conflict-inbound-batch-first")
        .body(Body::from(payload1))
        .expect("build req1");

    let resp1 = app.clone().oneshot(req1).await.expect("send req1");
    assert_eq!(resp1.status(), StatusCode::OK);

    let payload2 = json!({
        "items": [
            {
                "product_id": 1001,
                "qty": 3,
                "unit_cost": "2.20",
                "expected_version": 1
            }
        ]
    })
    .to_string();

    let req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound/batch")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("X-Idempotency-Key", "idem-conflict-inbound-batch-001")
        .header("x-request-id", "req-idem-conflict-inbound-batch-second")
        .body(Body::from(payload2))
        .expect("build req2");

    let resp2 = app.oneshot(req2).await.expect("send req2");
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-inbound-batch-second")
        .await;
}

#[tokio::test]
async fn outbound_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload1 = json!({
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
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-outbound-second").await;
}

#[tokio::test]
async fn create_sales_order_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload1 = json!({
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
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-so-create-second").await;
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
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-delete-second").await;
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
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-po-void-second").await;
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
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-so-void-second").await;
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
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-so-return-second").await;
}

#[tokio::test]
async fn create_stock_check_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id()
 {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let payload1 = json!({
        "items": [{
            "product_id": 1001
        }],
        "remark": "冲突A"
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
        "items": [{
            "product_id": 1001
        }],
        "remark": "冲突B"
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
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-sc-create-second").await;
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
    assert_idempotency_payload_conflict_response(resp2, "req-idem-conflict-sc-start-second").await;
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
