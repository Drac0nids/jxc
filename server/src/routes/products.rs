use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{Duration as ChronoDuration, Utc};
use serde_json::{Value, json};
use std::time::Duration;

use crate::routes::common::{
    BarcodeLookupData, BarcodeLookupQuery, CreateProductRequest, DeleteProductQuery,
    ListProductsQuery, ScanProductData, ScanQuery, UpdateProductRequest, barcode_scope_key,
    ensure_role, parse_decimal, parse_required_text, postgres_pool_or_none,
    require_idempotency_key, save_idempotency_record, save_idempotency_record_sync,
    to_product_data, try_idempotent_replay,
};
use crate::{
    error::AppError,
    extractors::AppJson,
    middleware::AuthContext,
    models::{BarcodeLookupCache, BarcodeLookupStatus},
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};

fn extract_name_from_lookup_payload(payload: &Value) -> Option<String> {
    let candidate_keys = ["product_name", "name", "goods_name", "item_name", "title"];

    for key in candidate_keys {
        if let Some(name) = payload
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            return Some(name.to_string());
        }
    }

    for key in ["data", "result", "item", "product"] {
        if let Some(name) = payload.get(key).and_then(extract_name_from_lookup_payload) {
            return Some(name);
        }
    }

    if let Some(items) = payload.as_array() {
        for item in items {
            if let Some(name) = extract_name_from_lookup_payload(item) {
                return Some(name);
            }
        }
    }

    None
}

async fn lookup_name_from_third_party(
    state: &AppState,
    barcode: &str,
) -> Result<(BarcodeLookupStatus, Option<String>, Value), String> {
    let api_url = state
        .config
        .barcode_lookup_api_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "BARCODE_LOOKUP_API_URL 未配置".to_string())?;

    let timeout_ms = state.config.barcode_lookup_timeout_ms.max(100);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .map_err(|err| format!("创建第三方条码查询客户端失败: {err}"))?;

    let mut request = client.get(api_url).query(&[("barcode", barcode)]);

    if let Some(api_key) = state
        .config
        .barcode_lookup_api_key
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        request = request.header("x-api-key", api_key);
    }

    let response = request
        .send()
        .await
        .map_err(|err| format!("调用第三方条码查询失败: {err}"))?;

    if !response.status().is_success() {
        return Err(format!("第三方条码查询返回非成功状态: {}", response.status()));
    }

    let payload = response
        .json::<Value>()
        .await
        .map_err(|err| format!("解析第三方条码查询响应失败: {err}"))?;

    let suggested_name = extract_name_from_lookup_payload(&payload);
    let lookup_status = if suggested_name.is_some() {
        BarcodeLookupStatus::Found
    } else {
        BarcodeLookupStatus::NotFound
    };

    Ok((lookup_status, suggested_name, payload))
}

pub async fn barcode_lookup_product_name(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(query): Query<BarcodeLookupQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER", "SALES"], &request_id)?;

    let barcode = query.barcode.trim();
    if barcode.is_empty() {
        return Err(AppError::bad_request("barcode 不能为空").with_request_id(request_id));
    }

    let now = Utc::now();
    let cache_record = if state.repository.is_postgres() {
        state
            .repository
            .find_barcode_lookup_cache(postgres_pool_or_none(&state), auth.tenant_id, barcode)
            .await?
    } else {
        let cache = state.barcode_lookup_cache.lock().map_err(|_| {
            AppError::internal("条码查询缓存锁异常").with_request_id(request_id.clone())
        })?;
        let key = barcode_scope_key(&auth.tenant_id, barcode);
        cache.get(&key).cloned()
    };

    if let Some(cache) = cache_record.clone()
        && cache.expires_at > now
    {
        let body = ApiResponse::success(
            BarcodeLookupData {
                barcode: barcode.to_string(),
                status: cache.lookup_status.as_str().to_string(),
                suggested_name: cache.product_name.clone(),
                cache_hit: true,
                source: "CACHE".to_string(),
            },
            request_id.clone(),
        );

        return Ok((
            StatusCode::OK,
            build_response_headers(&request_id, false),
            Json(body),
        )
            .into_response());
    }

    let stale_cache = cache_record.filter(|cache| cache.expires_at <= now);

    match lookup_name_from_third_party(&state, barcode).await {
        Ok((lookup_status, suggested_name, raw_payload)) => {
            let ttl_secs = match lookup_status {
                BarcodeLookupStatus::Found => state.config.barcode_lookup_found_ttl_secs.max(1),
                BarcodeLookupStatus::NotFound => {
                    state.config.barcode_lookup_not_found_ttl_secs.max(1)
                }
            };

            let cache_entry = BarcodeLookupCache {
                tenant_id: auth.tenant_id,
                barcode: barcode.to_string(),
                lookup_status,
                product_name: suggested_name.clone(),
                raw_payload,
                expires_at: now + ChronoDuration::seconds(ttl_secs),
                updated_at: now,
            };

            if state.repository.is_postgres() {
                state
                    .repository
                    .upsert_barcode_lookup_cache(postgres_pool_or_none(&state), &cache_entry)
                    .await?;
            } else {
                let mut cache = state.barcode_lookup_cache.lock().map_err(|_| {
                    AppError::internal("条码查询缓存锁异常").with_request_id(request_id.clone())
                })?;
                let key = barcode_scope_key(&auth.tenant_id, barcode);
                cache.insert(key, cache_entry);
            }

            let body = ApiResponse::success(
                BarcodeLookupData {
                    barcode: barcode.to_string(),
                    status: lookup_status.as_str().to_string(),
                    suggested_name,
                    cache_hit: false,
                    source: "THIRD_PARTY".to_string(),
                },
                request_id.clone(),
            );

            Ok((
                StatusCode::OK,
                build_response_headers(&request_id, false),
                Json(body),
            )
                .into_response())
        }
        Err(_) => {
            if let Some(cache) = stale_cache {
                let body = ApiResponse::success(
                    BarcodeLookupData {
                        barcode: barcode.to_string(),
                        status: cache.lookup_status.as_str().to_string(),
                        suggested_name: cache.product_name,
                        cache_hit: true,
                        source: "CACHE_STALE".to_string(),
                    },
                    request_id.clone(),
                );

                return Ok((
                    StatusCode::OK,
                    build_response_headers(&request_id, false),
                    Json(body),
                )
                    .into_response());
            }

            let body = ApiResponse::success(
                BarcodeLookupData {
                    barcode: barcode.to_string(),
                    status: BarcodeLookupStatus::NotFound.as_str().to_string(),
                    suggested_name: None,
                    cache_hit: false,
                    source: "DEGRADED".to_string(),
                },
                request_id.clone(),
            );

            Ok((
                StatusCode::OK,
                build_response_headers(&request_id, false),
                Json(body),
            )
                .into_response())
        }
    }
}

pub async fn scan_product(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(query): Query<ScanQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    if query.barcode.trim().is_empty() {
        return Err(AppError::bad_request("barcode 不能为空").with_request_id(request_id));
    }

    let barcode = query.barcode.trim();
    let product = if state.repository.is_postgres() {
        state
            .repository
            .find_product_by_barcode(
                postgres_pool_or_none(&state),
                auth.tenant_id,
                barcode,
                false,
            )
            .await?
            .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?
    } else {
        let barcode_key = barcode_scope_key(&auth.tenant_id, barcode);
        let barcode_index = state.barcode_index.lock().map_err(|_| {
            AppError::internal("条码索引锁异常").with_request_id(request_id.clone())
        })?;
        let product_id = barcode_index
            .get(&barcode_key)
            .copied()
            .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;
        drop(barcode_index);

        let products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;
        let product = products
            .get(&product_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;

        if product.tenant_id != auth.tenant_id || product.is_deleted {
            return Err(AppError::not_found("商品不存在").with_request_id(request_id));
        }

        product
    };

    let hide_cost_price = auth.role.eq_ignore_ascii_case("SALES");

    let data = ScanProductData {
        id: product.id,
        sku: product.sku,
        barcode: product.barcode,
        name: product.name,
        unit: product.unit,
        current_stock: product.current_stock,
        retail_price: product.retail_price.round_dp(4).to_string(),
        cost_price: if hide_cost_price {
            None
        } else {
            Some(product.cost_price.round_dp(4).to_string())
        },
        min_stock_limit: product.min_stock_limit,
    };

    let body = ApiResponse::success(data, request_id.clone());

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn list_products(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(query): Query<ListProductsQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let keyword = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_ascii_lowercase());
    let barcode_filter = query
        .barcode
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned);

    let mut filtered = if state.repository.is_postgres() {
        state
            .repository
            .list_products_by_tenant(postgres_pool_or_none(&state), auth.tenant_id)
            .await?
    } else {
        let products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;

        products
            .values()
            .filter(|p| p.tenant_id == auth.tenant_id && !p.is_deleted)
            .cloned()
            .collect::<Vec<_>>()
    };

    if let Some(ref kw) = keyword {
        filtered.retain(|p| {
            p.name.to_ascii_lowercase().contains(kw)
                || p.sku.to_ascii_lowercase().contains(kw)
                || p.barcode.to_ascii_lowercase().contains(kw)
        });
    }

    if let Some(ref barcode) = barcode_filter {
        filtered.retain(|p| p.barcode == *barcode);
    }

    let low_stock_only = query
        .low_stock
        .as_deref()
        .map(str::trim)
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if low_stock_only {
        filtered.retain(|p| p.current_stock < p.min_stock_limit);
        // Sort by shortage (gap) descending - largest deficit first
        filtered.sort_by(|a, b| {
            let gap_a = a.min_stock_limit - a.current_stock;
            let gap_b = b.min_stock_limit - b.current_stock;
            gap_b.cmp(&gap_a).then_with(|| a.id.cmp(&b.id))
        });
    } else {
        filtered.sort_by_key(|p| p.id);
    }

    if let Some(cat_id) = query.category_id {
        filtered.retain(|p| p.category_id == Some(cat_id));
    }

    let total = filtered.len() as u64;
    let start = ((page - 1) * page_size) as usize;
    let end = usize::min(start + page_size as usize, filtered.len());

    let hide_cost_price = auth.role.eq_ignore_ascii_case("SALES");
    let list = if start >= filtered.len() {
        Vec::new()
    } else {
        filtered[start..end]
            .iter()
            .map(|p| to_product_data(p, hide_cost_price))
            .collect::<Vec<_>>()
    };

    let body = ApiResponse::success(
        json!({
            "list": list,
            "total": total,
            "page": page,
            "page_size": page_size
        }),
        request_id.clone(),
    );

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn get_product(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let product = if state.repository.is_postgres() {
        state
            .repository
            .find_product_by_id(postgres_pool_or_none(&state), auth.tenant_id, id, false)
            .await?
            .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?
    } else {
        let products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;

        let product = products
            .get(&id)
            .cloned()
            .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;

        if product.tenant_id != auth.tenant_id || product.is_deleted {
            return Err(AppError::not_found("商品不存在").with_request_id(request_id));
        }

        product
    };

    let hide_cost_price = auth.role.eq_ignore_ascii_case("SALES");
    let body = ApiResponse::success(
        to_product_data(&product, hide_cost_price),
        request_id.clone(),
    );

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn create_product(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    AppJson(req): AppJson<CreateProductRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("新建商品请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/products", idempotency_key
    );

    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    let barcode = parse_required_text(&req.barcode, "barcode", &request_id)?;
    let name = parse_required_text(&req.name, "name", &request_id)?;
    let unit = parse_required_text(&req.unit, "unit", &request_id)?;
    let retail_price = parse_decimal(&req.retail_price, "retail_price", &request_id)?;

    if let Some(raw) = &req.sku
        && raw.trim().is_empty()
    {
        return Err(AppError::bad_request("sku 不能为空").with_request_id(request_id));
    }

    let init_stock = req.init_stock.unwrap_or(0);
    if init_stock < 0 {
        return Err(AppError::bad_request("init_stock 不能小于 0").with_request_id(request_id));
    }

    let min_stock_limit = req.min_stock_limit.unwrap_or(0);
    if min_stock_limit < 0 {
        return Err(AppError::bad_request("min_stock_limit 不能小于 0").with_request_id(request_id));
    }

    let cost_price_raw = parse_required_text(&req.cost_price, "cost_price", &request_id)?;
    let cost_price = parse_decimal(&cost_price_raw, "cost_price", &request_id)?;

    if state.repository.is_postgres() {
        let sku = req
            .sku
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToOwned::to_owned);
        let pool = postgres_pool_or_none(&state);

        if let Some(existing_id) = state
            .repository
            .is_barcode_taken(pool, auth.tenant_id, &barcode, None)
            .await?
        {
            return Err(AppError::business(StatusCode::CONFLICT, 4002, "条码已存在")
                .with_data(json!({
                    "barcode": barcode,
                    "existing_product_id": existing_id
                }))
                .with_request_id(request_id));
        }

        let next_id = state.repository.next_product_id(pool).await?;
        let sku = sku.unwrap_or_else(|| format!("SKU-{next_id}"));

        if state
            .repository
            .is_sku_taken(pool, auth.tenant_id, &sku, None)
            .await?
        {
            return Err(AppError::conflict(4090, "sku 已存在")
                .with_data(json!({ "sku": sku }))
                .with_request_id(request_id));
        }

        let product = crate::models::Product {
            id: next_id,
            tenant_id: auth.tenant_id,
            sku,
            barcode,
            name,
            unit,
            current_stock: init_stock,
            cost_price: cost_price.round_dp(4),
            retail_price: retail_price.round_dp(4),
            last_inbound_unit_cost: if init_stock > 0 {
                Some(cost_price.round_dp(4))
            } else {
                None
            },
            min_stock_limit,
            is_deleted: false,
            category_id: req.category_id,
            track_batches: req.track_batches.unwrap_or(false),
        };

        state.repository.create_product(pool, &product).await?;

        let body = ApiResponse::success(to_product_data(&product, false), request_id.clone());
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("新建商品响应序列化失败").with_request_id(request_id.clone())
        })?;
        save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

        return Ok((
            StatusCode::OK,
            build_response_headers(&request_id, false),
            Json(body),
        )
            .into_response());
    }

    let mut barcode_index = state
        .barcode_index
        .lock()
        .map_err(|_| AppError::internal("条码索引锁异常").with_request_id(request_id.clone()))?;
    let barcode_key = barcode_scope_key(&auth.tenant_id, &barcode);
    if let Some(existing_id) = barcode_index.get(&barcode_key).copied() {
        return Err(AppError::business(StatusCode::CONFLICT, 4002, "条码已存在")
            .with_data(json!({
                "barcode": barcode,
                "existing_product_id": existing_id
            }))
            .with_request_id(request_id));
    }

    let mut products = state
        .products
        .lock()
        .map_err(|_| AppError::internal("商品状态锁异常").with_request_id(request_id.clone()))?;

    let mut next_product_id = state
        .next_product_id
        .lock()
        .map_err(|_| AppError::internal("商品ID状态锁异常").with_request_id(request_id.clone()))?;
    *next_product_id += 1;
    let product_id = *next_product_id;
    drop(next_product_id);

    let sku = req
        .sku
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("SKU-{product_id}"));

    if products
        .values()
        .any(|p| p.tenant_id == auth.tenant_id && p.sku.eq_ignore_ascii_case(&sku) && !p.is_deleted)
    {
        return Err(AppError::conflict(4090, "sku 已存在")
            .with_data(json!({ "sku": sku }))
            .with_request_id(request_id));
    }

    let product = crate::models::Product {
        id: product_id,
        tenant_id: auth.tenant_id,
        sku,
        barcode,
        name,
        unit,
        current_stock: init_stock,
        cost_price: cost_price.round_dp(4),
        retail_price: retail_price.round_dp(4),
        last_inbound_unit_cost: if init_stock > 0 {
            Some(cost_price.round_dp(4))
        } else {
            None
        },
        min_stock_limit,
        is_deleted: false,
        category_id: None, // 内存模式不支持分类
        track_batches: false,
    };

    products.insert(product.id, product.clone());
    barcode_index.insert(barcode_key, product.id);
    drop(products);
    drop(barcode_index);

    if product.current_stock > 0 {
        let mut stock_logs = state.stock_logs.lock().map_err(|_| {
            AppError::internal("库存流水锁异常").with_request_id(request_id.clone())
        })?;
        let log_id = stock_logs.len() as i64 + 1;
        stock_logs.push(crate::models::StockLog::now(
            log_id,
            auth.tenant_id,
            product.id,
            "INIT_PRODUCT",
            format!("INIT-PRODUCT-{}", product.id),
            product.current_stock,
            product.current_stock,
            product.cost_price,
            None,
            None,
            auth.user_id,
        ));
    }

    let body = ApiResponse::success(to_product_data(&product, false), request_id.clone());
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("新建商品响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record_sync(&state, &scope_key, request_payload, response_body)?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn update_product(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    AppJson(req): AppJson<UpdateProductRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("更新商品请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}:{}",
        auth.tenant_id, "PUT", "/api/v1/products", id, idempotency_key
    );

    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    let sku = match req.sku {
        Some(raw) => Some(parse_required_text(&raw, "sku", &request_id)?),
        None => None,
    };
    let barcode = match req.barcode {
        Some(raw) => Some(parse_required_text(&raw, "barcode", &request_id)?),
        None => None,
    };
    let name = match req.name {
        Some(raw) => Some(parse_required_text(&raw, "name", &request_id)?),
        None => None,
    };
    let unit = match req.unit {
        Some(raw) => Some(parse_required_text(&raw, "unit", &request_id)?),
        None => None,
    };
    let retail_price = match req.retail_price.as_deref() {
        Some(raw) => Some(parse_decimal(raw, "retail_price", &request_id)?),
        None => None,
    };
    let min_stock_limit = req.min_stock_limit;

    if min_stock_limit.is_none()
        && sku.is_none()
        && barcode.is_none()
        && name.is_none()
        && unit.is_none()
        && retail_price.is_none()
        && req.category_id.is_none()
        && req.track_batches.is_none()
    {
        return Err(AppError::bad_request("至少提供一个可更新字段").with_request_id(request_id));
    }

    if let Some(limit) = min_stock_limit
        && limit < 0
    {
        return Err(AppError::bad_request("min_stock_limit 不能小于 0").with_request_id(request_id));
    }

    if state.repository.is_postgres() {
        let pool = postgres_pool_or_none(&state);
        let existing = state
            .repository
            .find_product_by_id(pool, auth.tenant_id, id, false)
            .await?
            .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;



        let new_sku = sku.unwrap_or_else(|| existing.sku.clone());
        let new_barcode = barcode.unwrap_or_else(|| existing.barcode.clone());
        let new_name = name.unwrap_or_else(|| existing.name.clone());
        let new_unit = unit.unwrap_or_else(|| existing.unit.clone());
        let new_retail_price = retail_price.unwrap_or(existing.retail_price);
        let new_min_stock_limit = min_stock_limit.unwrap_or(existing.min_stock_limit);

        if state
            .repository
            .is_sku_taken(pool, auth.tenant_id, &new_sku, Some(existing.id))
            .await?
        {
            return Err(AppError::conflict(4090, "sku 已存在")
                .with_data(json!({ "sku": new_sku }))
                .with_request_id(request_id));
        }

        if let Some(existing_product_id) = state
            .repository
            .is_barcode_taken(pool, auth.tenant_id, &new_barcode, Some(existing.id))
            .await?
        {
            return Err(AppError::business(StatusCode::CONFLICT, 4002, "条码已存在")
                .with_data(json!({
                    "barcode": new_barcode,
                    "existing_product_id": existing_product_id
                }))
                .with_request_id(request_id));
        }

        let updated = crate::models::Product {
            id: existing.id,
            tenant_id: existing.tenant_id,
            sku: new_sku,
            barcode: new_barcode,
            name: new_name,
            unit: new_unit,
            current_stock: existing.current_stock,
            cost_price: existing.cost_price,
            retail_price: new_retail_price.round_dp(4),
            last_inbound_unit_cost: existing.last_inbound_unit_cost,
            min_stock_limit: new_min_stock_limit,
            is_deleted: existing.is_deleted,
            category_id: req.category_id.or(existing.category_id),
            track_batches: req.track_batches.unwrap_or(existing.track_batches),
        };

        state
            .repository
            .update_product(pool, &updated)
            .await?;

        let body = ApiResponse::success(to_product_data(&updated, false), request_id.clone());
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("更新商品响应序列化失败").with_request_id(request_id.clone())
        })?;
        save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

        return Ok((
            StatusCode::OK,
            build_response_headers(&request_id, false),
            Json(body),
        )
            .into_response());
    }

    let mut barcode_index = state
        .barcode_index
        .lock()
        .map_err(|_| AppError::internal("条码索引锁异常").with_request_id(request_id.clone()))?;
    let mut products = state
        .products
        .lock()
        .map_err(|_| AppError::internal("商品状态锁异常").with_request_id(request_id.clone()))?;

    let existing = products
        .get(&id)
        .cloned()
        .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;
    if existing.tenant_id != auth.tenant_id || existing.is_deleted {
        return Err(AppError::not_found("商品不存在").with_request_id(request_id));
    }



    let new_sku = sku.unwrap_or_else(|| existing.sku.clone());
    let new_barcode = barcode.unwrap_or_else(|| existing.barcode.clone());
    let new_name = name.unwrap_or_else(|| existing.name.clone());
    let new_unit = unit.unwrap_or_else(|| existing.unit.clone());
    let new_retail_price = retail_price.unwrap_or(existing.retail_price);
    let new_min_stock_limit = min_stock_limit.unwrap_or(existing.min_stock_limit);

    if products.values().any(|p| {
        p.id != existing.id
            && p.tenant_id == auth.tenant_id
            && p.sku.eq_ignore_ascii_case(&new_sku)
            && !p.is_deleted
    }) {
        return Err(AppError::conflict(4090, "sku 已存在")
            .with_data(json!({ "sku": new_sku }))
            .with_request_id(request_id));
    }

    if new_barcode != existing.barcode {
        let new_barcode_key = barcode_scope_key(&auth.tenant_id, &new_barcode);
        if let Some(existing_product_id) = barcode_index.get(&new_barcode_key).copied()
            && existing_product_id != existing.id
        {
            return Err(AppError::business(StatusCode::CONFLICT, 4002, "条码已存在")
                .with_data(json!({
                    "barcode": new_barcode,
                    "existing_product_id": existing_product_id
                }))
                .with_request_id(request_id));
        }
    }

    let product = products
        .get_mut(&id)
        .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;
    product.sku = new_sku;
    product.barcode = new_barcode.clone();
    product.name = new_name;
    product.unit = new_unit;
    product.retail_price = new_retail_price.round_dp(4);
    product.min_stock_limit = new_min_stock_limit;
    if let Some(tb) = req.track_batches {
        product.track_batches = tb;
    }
    let updated = product.clone();

    if new_barcode != existing.barcode {
        let old_barcode_key = barcode_scope_key(&auth.tenant_id, &existing.barcode);
        let new_barcode_key = barcode_scope_key(&auth.tenant_id, &new_barcode);
        barcode_index.remove(&old_barcode_key);
        barcode_index.insert(new_barcode_key, existing.id);
    }

    drop(products);
    drop(barcode_index);

    let body = ApiResponse::success(to_product_data(&updated, false), request_id.clone());
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("更新商品响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record_sync(&state, &scope_key, request_payload, response_body)?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn delete_product(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(query): Query<DeleteProductQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&query).map_err(|_| {
        AppError::internal("删除商品请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}:{}",
        auth.tenant_id, "DELETE", "/api/v1/products", id, idempotency_key
    );

    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if state.repository.is_postgres() {
        let pool = postgres_pool_or_none(&state);
        let existing = state
            .repository
            .find_product_by_id(pool, auth.tenant_id, id, false)
            .await?
            .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;



        if existing.current_stock > 0 {
            return Err(AppError::conflict(4090, "存在库存，无法删除商品")
                .with_data(json!({
                    "resource": "product",
                    "resource_id": existing.id,
                    "current_stock": existing.current_stock
                }))
                .with_request_id(request_id));
        }

        let mut deleted = existing.clone();
        deleted.is_deleted = true;
        state
            .repository
            .update_product(pool, &deleted)
            .await?;

        let body = ApiResponse::success(
            json!({
                "id": id,
                "deleted": true
            }),
            request_id.clone(),
        );
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("删除商品响应序列化失败").with_request_id(request_id.clone())
        })?;
        save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

        return Ok((
            StatusCode::OK,
            build_response_headers(&request_id, false),
            Json(body),
        )
            .into_response());
    }

    let mut products = state
        .products
        .lock()
        .map_err(|_| AppError::internal("商品状态锁异常").with_request_id(request_id.clone()))?;

    let existing = products
        .get(&id)
        .cloned()
        .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;
    if existing.tenant_id != auth.tenant_id || existing.is_deleted {
        return Err(AppError::not_found("商品不存在").with_request_id(request_id));
    }



    if existing.current_stock > 0 {
        return Err(AppError::conflict(4090, "存在库存，无法删除商品")
            .with_data(json!({
                "resource": "product",
                "resource_id": existing.id,
                "current_stock": existing.current_stock
            }))
            .with_request_id(request_id));
    }

    let product = products
        .get_mut(&id)
        .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;
    product.is_deleted = true;
    drop(products);

    let body = ApiResponse::success(
        json!({
            "id": id,
            "deleted": true
        }),
        request_id.clone(),
    );
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("删除商品响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record_sync(&state, &scope_key, request_payload, response_body)?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}
