use super::*;
use chrono::{Duration as ChronoDuration, Utc};
use serde_json::json;

use crate::{
    models::{BarcodeLookupCache, BarcodeLookupStatus},
    routes::common::barcode_scope_key,
};

#[tokio::test]
async fn barcode_lookup_should_return_degraded_when_third_party_not_configured_and_no_cache() {
    let state = make_test_state();
    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/products/barcode-lookup?barcode=690123456789")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build barcode lookup request");

    let resp = app.oneshot(req).await.expect("send barcode lookup request");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = response_json(resp).await;
    assert_eq!(body["data"]["barcode"], "690123456789");
    assert_eq!(body["data"]["status"], "NOT_FOUND");
    assert_eq!(body["data"]["suggested_name"], Value::Null);
    assert_eq!(body["data"]["cache_hit"], false);
    assert_eq!(body["data"]["source"], "DEGRADED");
}

#[tokio::test]
async fn barcode_lookup_should_hit_valid_cache_before_third_party_fallback() {
    let state = make_test_state();
    let tenant_id = {
        let users = state.users.lock().expect("lock users");
        users.get("admin").expect("admin user should exist").tenant_id
    };

    let barcode = "690123456789";
    {
        let mut cache = state
            .barcode_lookup_cache
            .lock()
            .expect("lock barcode_lookup_cache");
        cache.insert(
            barcode_scope_key(&tenant_id, barcode),
            BarcodeLookupCache {
                tenant_id,
                barcode: barcode.to_string(),
                lookup_status: BarcodeLookupStatus::Found,
                product_name: Some("缓存商品名称".to_string()),
                raw_payload: json!({
                    "name": "缓存商品名称"
                }),
                expires_at: Utc::now() + ChronoDuration::minutes(5),
                updated_at: Utc::now(),
            },
        );
    }

    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/products/barcode-lookup?barcode={barcode}"))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build barcode lookup request");

    let resp = app.oneshot(req).await.expect("send barcode lookup request");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = response_json(resp).await;
    assert_eq!(body["data"]["barcode"], barcode);
    assert_eq!(body["data"]["status"], "FOUND");
    assert_eq!(body["data"]["suggested_name"], "缓存商品名称");
    assert_eq!(body["data"]["cache_hit"], true);
    assert_eq!(body["data"]["source"], "CACHE");
}

#[tokio::test]
async fn barcode_lookup_should_use_stale_cache_when_third_party_fails() {
    let state = make_test_state();
    let tenant_id = {
        let users = state.users.lock().expect("lock users");
        users.get("admin").expect("admin user should exist").tenant_id
    };

    let barcode = "690123456788";
    {
        let mut cache = state
            .barcode_lookup_cache
            .lock()
            .expect("lock barcode_lookup_cache");
        cache.insert(
            barcode_scope_key(&tenant_id, barcode),
            BarcodeLookupCache {
                tenant_id,
                barcode: barcode.to_string(),
                lookup_status: BarcodeLookupStatus::Found,
                product_name: Some("过期缓存商品名称".to_string()),
                raw_payload: json!({
                    "name": "过期缓存商品名称"
                }),
                expires_at: Utc::now() - ChronoDuration::minutes(1),
                updated_at: Utc::now() - ChronoDuration::minutes(2),
            },
        );
    }

    let app = build_router(state);
    let (app, token) = login_token(app).await;

    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/products/barcode-lookup?barcode={barcode}"))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .expect("build barcode lookup request");

    let resp = app.oneshot(req).await.expect("send barcode lookup request");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = response_json(resp).await;
    assert_eq!(body["data"]["barcode"], barcode);
    assert_eq!(body["data"]["status"], "FOUND");
    assert_eq!(body["data"]["suggested_name"], "过期缓存商品名称");
    assert_eq!(body["data"]["cache_hit"], true);
    assert_eq!(body["data"]["source"], "CACHE_STALE");
}