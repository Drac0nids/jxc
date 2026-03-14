use super::*;

async fn assert_standard_json_parse_error(
    resp: Response,
    expected_status: StatusCode,
    expected_request_id: &str,
) {
    assert_eq!(resp.status(), expected_status);

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
    assert!(body["message"]
        .as_str()
        .unwrap_or_default()
        .contains("请求体解析失败"));
}

#[tokio::test]
async fn inbound_malformed_json_should_return_standard_json_error() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let request_id = "req-inbound-json-syntax-001";
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", request_id)
        .body(Body::from(
            r#"{"product_id":1001,"qty":1,"unit_cost":"2.20","expected_version":1"#,
        ))
        .expect("build malformed json request");

    let resp = app.oneshot(req).await.expect("send malformed json request");
    assert_standard_json_parse_error(resp, StatusCode::BAD_REQUEST, request_id).await;
}

#[tokio::test]
async fn inbound_json_field_type_error_should_return_standard_json_error() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let request_id = "req-inbound-json-data-001";
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", request_id)
        .body(Body::from(
            json!({
                "product_id": 1001,
                "qty": "1",
                "unit_cost": "2.20",
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build json data error request");

    let resp = app
        .oneshot(req)
        .await
        .expect("send json data error request");
    assert_standard_json_parse_error(resp, StatusCode::UNPROCESSABLE_ENTITY, request_id).await;
}

#[tokio::test]
async fn inbound_content_type_mismatch_should_return_standard_json_error() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let request_id = "req-inbound-content-type-001";
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/inventory/inbound")
        .header(header::CONTENT_TYPE, "text/plain")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("x-request-id", request_id)
        .body(Body::from(
            json!({
                "product_id": 1001,
                "qty": 1,
                "unit_cost": "2.20",
                "expected_version": 1
            })
            .to_string(),
        ))
        .expect("build content-type mismatch request");

    let resp = app
        .oneshot(req)
        .await
        .expect("send content-type mismatch request");
    assert_standard_json_parse_error(resp, StatusCode::UNSUPPORTED_MEDIA_TYPE, request_id).await;
}
