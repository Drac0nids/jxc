use super::*;
use chrono::{FixedOffset, Utc};
use rust_decimal::Decimal;

use crate::models::{SalesOrder, SalesOrderItem, SalesOrderStatus, StockLog};

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
        .header("X-Idempotency-Key", "idem-prod-4092-001")
        .body(Body::from(
            json!({
                "barcode": "6991234500009",
                "name": "测试商品-4092-B",
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
async fn dashboard_report_should_use_stock_log_aligned_metrics() {
    let state = make_test_state();

    let (tenant_id, operator_id) = {
        let users = state.users.lock().expect("lock users");
        let admin = users.get("admin").expect("admin exists");
        (admin.tenant_id, admin.id)
    };

    {
        let mut products = state.products.lock().expect("lock products");
        let p1 = products.get_mut(&1001).expect("sample product exists");
        p1.current_stock = 5;
        p1.min_stock_limit = 10;

        products.insert(
            1002,
            crate::models::Product {
                id: 1002,
                tenant_id,
                sku: "SP-500".to_string(),
                barcode: "690123456790".to_string(),
                name: "雪碧 500ml".to_string(),
                unit: "瓶".to_string(),
                current_stock: 20,
                cost_price: Decimal::new(150, 2),
                retail_price: Decimal::new(500, 2),
                last_inbound_unit_cost: Some(Decimal::new(450, 2)),
                min_stock_limit: 5,
                version: 1,
                is_deleted: false,
                category_id: None,
            },
        );
    }

    {
        let now = Utc::now().to_rfc3339();
        let mut sales_orders = state.sales_orders.lock().expect("lock sales_orders");
        sales_orders.clear();

        sales_orders.insert(
            7001,
            SalesOrder {
                id: 7001,
                tenant_id,
                biz_no: "SO-TEST-001".to_string(),
                customer_id: None,
                status: SalesOrderStatus::Confirmed,
                items: vec![
                    SalesOrderItem {
                        product_id: 1001,
                        qty: 3,
                        sell_price: Decimal::new(350, 2),
                        returned_qty: 1,
                        product_name_snapshot: None,
                    },
                    SalesOrderItem {
                        product_id: 1002,
                        qty: 1,
                        sell_price: Decimal::new(500, 2),
                        returned_qty: 0,
                        product_name_snapshot: None,
                    },
                ],
                remark: None,
                created_by: operator_id,
                version: 2,
                confirmed_at: Some(now.clone()),
                returned_at: None,
                voided_at: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            },
        );

        // 同日已确认但无任何 OUT_SALE 流水：不应计入今日订单数
        sales_orders.insert(
            7002,
            SalesOrder {
                id: 7002,
                tenant_id,
                biz_no: "SO-TEST-002".to_string(),
                customer_id: None,
                status: SalesOrderStatus::Confirmed,
                items: vec![SalesOrderItem {
                    product_id: 1001,
                    qty: 1,
                    sell_price: Decimal::new(350, 2),
                    returned_qty: 0,
                    product_name_snapshot: None,
                }],
                remark: None,
                created_by: operator_id,
                version: 2,
                confirmed_at: Some(now.clone()),
                returned_at: None,
                voided_at: None,
                created_at: now.clone(),
                updated_at: now,
            },
        );
    }

    {
        let mut stock_logs = state.stock_logs.lock().expect("lock stock_logs");
        stock_logs.clear();
        stock_logs.push(StockLog::now(
            1,
            tenant_id,
            1001,
            "OUT_SALE",
            "SO-TEST-001",
            -3,
            2,
            Decimal::new(210, 2),
            Some(Decimal::new(350, 2)),
            None,
            operator_id,
        ));
        stock_logs.push(StockLog::now(
            2,
            tenant_id,
            1001,
            "RETURN_SALE",
            "SO-TEST-001",
            1,
            3,
            Decimal::new(210, 2),
            Some(Decimal::new(350, 2)),
            None,
            operator_id,
        ));
        stock_logs.push(StockLog::now(
            3,
            tenant_id,
            1002,
            "OUT_SALE",
            "SO-TEST-001",
            -1,
            19,
            Decimal::new(150, 2),
            Some(Decimal::new(500, 2)),
            None,
            operator_id,
        ));

        // legacy 出库无销售单明细映射，但有 snapshot_sell_price：应计入看板统计
        stock_logs.push(StockLog::now(
            4,
            tenant_id,
            1001,
            "OUT_SALE",
            "OUT-LEGACY-001",
            -2,
            1,
            Decimal::new(210, 2),
            Some(Decimal::new(400, 2)),
            None,
            operator_id,
        ));
    }

    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/reports/dashboard")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build dashboard req");
    let resp = app.oneshot(req).await.expect("send dashboard req");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = response_json(resp).await;
    assert_eq!(body["data"]["total_sales"], "20.00");
    assert_eq!(body["data"]["total_gross_profit"], "10.10");
    assert_eq!(body["data"]["total_orders"], 2);
    assert_eq!(body["data"]["low_stock_count"], 1);
    assert_eq!(body["data"]["top_selling_item"], "可口可乐 330ml");
}

#[tokio::test]
async fn dashboard_orders_drilldown_should_align_with_dashboard_total_orders_and_support_paging() {
    let state = make_test_state();

    let (tenant_id, operator_id) = {
        let users = state.users.lock().expect("lock users");
        let admin = users.get("admin").expect("admin exists");
        (admin.tenant_id, admin.id)
    };

    {
        let now = Utc::now().to_rfc3339();
        let mut sales_orders = state.sales_orders.lock().expect("lock sales_orders");
        sales_orders.clear();

        sales_orders.insert(
            7101,
            SalesOrder {
                id: 7101,
                tenant_id,
                biz_no: "SO-DRILL-001".to_string(),
                customer_id: None,
                status: SalesOrderStatus::Confirmed,
                items: vec![SalesOrderItem {
                    product_id: 1001,
                    qty: 2,
                    sell_price: Decimal::new(350, 2),
                    returned_qty: 0,
                    product_name_snapshot: None,
                }],
                remark: None,
                created_by: operator_id,
                version: 2,
                confirmed_at: Some(now.clone()),
                returned_at: None,
                voided_at: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            },
        );

        sales_orders.insert(
            7102,
            SalesOrder {
                id: 7102,
                tenant_id,
                biz_no: "SO-DRILL-002".to_string(),
                customer_id: None,
                status: SalesOrderStatus::Confirmed,
                items: vec![SalesOrderItem {
                    product_id: 1001,
                    qty: 1,
                    sell_price: Decimal::new(350, 2),
                    returned_qty: 0,
                    product_name_snapshot: None,
                }],
                remark: None,
                created_by: operator_id,
                version: 2,
                confirmed_at: Some(now.clone()),
                returned_at: None,
                voided_at: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            },
        );

        // 存在销售单但没有可映射 OUT_SALE 流水：不计入看板订单数与下钻列表
        sales_orders.insert(
            7103,
            SalesOrder {
                id: 7103,
                tenant_id,
                biz_no: "SO-DRILL-003".to_string(),
                customer_id: None,
                status: SalesOrderStatus::Confirmed,
                items: vec![SalesOrderItem {
                    product_id: 1001,
                    qty: 1,
                    sell_price: Decimal::new(350, 2),
                    returned_qty: 0,
                    product_name_snapshot: None,
                }],
                remark: None,
                created_by: operator_id,
                version: 2,
                confirmed_at: Some(now.clone()),
                returned_at: None,
                voided_at: None,
                created_at: now.clone(),
                updated_at: now,
            },
        );
    }

    {
        let mut stock_logs = state.stock_logs.lock().expect("lock stock_logs");
        stock_logs.clear();
        stock_logs.push(StockLog::now(
            11,
            tenant_id,
            1001,
            "OUT_SALE",
            "SO-DRILL-001",
            -2,
            98,
            Decimal::new(210, 2),
            Some(Decimal::new(350, 2)),
            None,
            operator_id,
        ));
        stock_logs.push(StockLog::now(
            12,
            tenant_id,
            1001,
            "OUT_SALE",
            "SO-DRILL-002",
            -1,
            97,
            Decimal::new(210, 2),
            Some(Decimal::new(350, 2)),
            None,
            operator_id,
        ));
        stock_logs.push(StockLog::now(
            13,
            tenant_id,
            1001,
            "OUT_SALE",
            "OUT-LEGACY-DRILL",
            -5,
            92,
            Decimal::new(210, 2),
            Some(Decimal::new(400, 2)),
            None,
            operator_id,
        ));
    }

    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let dashboard_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/reports/dashboard")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build dashboard req");
    let dashboard_resp = app
        .clone()
        .oneshot(dashboard_req)
        .await
        .expect("send dashboard req");
    assert_eq!(dashboard_resp.status(), StatusCode::OK);
    let dashboard_body = response_json(dashboard_resp).await;
    assert_eq!(dashboard_body["data"]["total_orders"], 3);

    let drilldown_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/reports/dashboard/orders")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build drilldown req");
    let drilldown_resp = app
        .clone()
        .oneshot(drilldown_req)
        .await
        .expect("send drilldown req");
    assert_eq!(drilldown_resp.status(), StatusCode::OK);

    let drilldown_body = response_json(drilldown_resp).await;
    assert_eq!(drilldown_body["data"]["total"], dashboard_body["data"]["total_orders"]);

    let biz_nos = drilldown_body["data"]["list"]
        .as_array()
        .expect("drilldown list should be array")
        .iter()
        .filter_map(|item| item["biz_no"].as_str())
        .collect::<Vec<_>>();
    assert!(biz_nos.contains(&"SO-DRILL-001"));
    assert!(biz_nos.contains(&"SO-DRILL-002"));
    assert!(!biz_nos.contains(&"SO-DRILL-003"));
    assert!(biz_nos.contains(&"OUT-LEGACY-DRILL"));

    let legacy_order = drilldown_body["data"]["list"]
        .as_array()
        .expect("drilldown list should be array")
        .iter()
        .find(|item| item["biz_no"] == "OUT-LEGACY-DRILL")
        .expect("should include OUT-LEGACY-DRILL aggregate order");
    assert_eq!(legacy_order["status"], "OUTBOUND_ONLY");

    let legacy_items = legacy_order["items"]
        .as_array()
        .expect("legacy aggregate order items should be array");
    assert!(!legacy_items.is_empty());

    let legacy_item = legacy_items
        .iter()
        .find(|item| item["product_id"] == 1001)
        .expect("legacy aggregate item for product 1001 should exist");
    assert_eq!(legacy_item["product_name"], "可口可乐 330ml");
    assert_eq!(legacy_item["qty"], 5);
    assert_eq!(legacy_item["sell_price"], "4.00");
    assert_eq!(legacy_item["line_amount"], "20.00");
    assert_eq!(legacy_item["returned_qty"], 0);

    let timezone = FixedOffset::east_opt(8 * 3600).expect("timezone");
    let today = Utc::now().with_timezone(&timezone).date_naive().to_string();

    let page1_req = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/reports/dashboard/orders?start_date={today}&end_date={today}&page=1&page_size=1"
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build page1 req");
    let page1_resp = app.clone().oneshot(page1_req).await.expect("send page1 req");
    assert_eq!(page1_resp.status(), StatusCode::OK);
    let page1_body = response_json(page1_resp).await;
    assert_eq!(page1_body["data"]["total"], 3);
    assert_eq!(
        page1_body["data"]["list"]
            .as_array()
            .expect("page1 list")
            .len(),
        1
    );

    let page2_req = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/v1/reports/dashboard/orders?start_date={today}&end_date={today}&page=2&page_size=1"
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build page2 req");
    let page2_resp = app.clone().oneshot(page2_req).await.expect("send page2 req");
    assert_eq!(page2_resp.status(), StatusCode::OK);
    let page2_body = response_json(page2_resp).await;
    assert_eq!(page2_body["data"]["total"], 3);
    assert_eq!(
        page2_body["data"]["list"]
            .as_array()
            .expect("page2 list")
            .len(),
        1
    );
}
