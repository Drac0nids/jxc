use axum::{
    Json,
    extract::{Extension, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use rust_decimal::Decimal;
use serde_json::json;

use crate::routes::common::{
    InboundBatchRequest, InboundRequest, InventoryBatchItemResultData, InventoryBatchResultData,
    InventoryResultData, ListLowStockAlertsQuery, LowStockAlertData, OutboundItemResult,
    ensure_role, generate_server_biz_no, parse_decimal, postgres_pool_or_none, product_snapshot,
    require_idempotency_key, resolve_product_id, save_idempotency_record,
    save_idempotency_record_sync, try_idempotent_replay,
};
use crate::{
    error::AppError,
    extractors::AppJson,
    middleware::AuthContext,
    models::StockLog,
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};

pub async fn list_low_stock_alerts(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(query): Query<ListLowStockAlertsQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let only_active = query.only_active.unwrap_or(true);

    let keyword = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_ascii_lowercase());

    let products = if state.repository.is_postgres() {
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

    let mut alerts = products
        .iter()
        .filter(|p| p.tenant_id == auth.tenant_id && !p.is_deleted)
        .filter_map(|p| {
            let shortage_qty = (p.min_stock_limit - p.current_stock).max(0);
            if only_active && shortage_qty == 0 {
                return None;
            }

            Some(LowStockAlertData {
                product_id: p.id,
                sku: p.sku.clone(),
                barcode: p.barcode.clone(),
                name: p.name.clone(),
                unit: p.unit.clone(),
                current_stock: p.current_stock,
                min_stock_limit: p.min_stock_limit,
                shortage_qty,
            })
        })
        .collect::<Vec<_>>();

    if let Some(ref kw) = keyword {
        alerts.retain(|a| {
            a.name.to_ascii_lowercase().contains(kw)
                || a.sku.to_ascii_lowercase().contains(kw)
                || a.barcode.to_ascii_lowercase().contains(kw)
        });
    }

    alerts.sort_by(|a, b| {
        b.shortage_qty
            .cmp(&a.shortage_qty)
            .then_with(|| a.product_id.cmp(&b.product_id))
    });

    let active_count = alerts.iter().filter(|a| a.shortage_qty > 0).count() as u64;
    let total = alerts.len() as u64;
    let start = ((page - 1) * page_size) as usize;
    let end = usize::min(start + page_size as usize, alerts.len());
    let list = if start >= alerts.len() {
        Vec::new()
    } else {
        alerts[start..end].to_vec()
    };

    let body = ApiResponse::success(
        json!({
            "list": list,
            "total": total,
            "page": page,
            "page_size": page_size,
            "active_low_stock_count": active_count
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

pub async fn inbound(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    AppJson(req): AppJson<InboundRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("入库请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/inventory/inbound", idempotency_key
    );

    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if req.qty <= 0 {
        return Err(AppError::bad_request("qty 必须大于 0").with_request_id(request_id));
    }
    let unit_cost = req
        .unit_cost
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|raw| parse_decimal(raw, "unit_cost", &request_id))
        .transpose()?;
    let biz_no = generate_server_biz_no("IN");

    let resolved_product_id = if state.repository.is_postgres() {
        if let Some(pid) = req.product_id {
            pid
        } else {
            let barcode = req
                .barcode
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .ok_or_else(|| {
                    AppError::bad_request("product_id 或 barcode 至少提供一个")
                        .with_request_id(request_id.clone())
                })?;
            state
                .repository
                .find_product_by_barcode(
                    postgres_pool_or_none(&state),
                    auth.tenant_id,
                    barcode,
                    false,
                )
                .await
                .map_err(|err| err.with_request_id(request_id.clone()))?
                .ok_or_else(|| {
                    AppError::not_found("商品不存在").with_request_id(request_id.clone())
                })?
                .id
        }
    } else {
        resolve_product_id(
            &state,
            auth.tenant_id,
            req.product_id,
            req.barcode.as_deref(),
            &request_id,
        )?
    };

    if state.repository.is_postgres() {
        let updated = state
            .repository
            .inbound(
                postgres_pool_or_none(&state),
                auth.tenant_id,
                resolved_product_id,
                req.qty,
                unit_cost,
                req.expected_version,
                &biz_no,
                auth.user_id,
            )
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;

        let data = InventoryResultData {
            biz_no: biz_no.clone(),
            product_id: updated.id,
            current_stock: updated.current_stock,
            cost_price: updated.cost_price.round_dp(4).to_string(),
            version: updated.version,
            track_batches: updated.track_batches,
        };

        let body = ApiResponse::success(data, request_id.clone());
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("入库响应序列化失败").with_request_id(request_id.clone())
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

    let product = products
        .get_mut(&resolved_product_id)
        .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;

    if product.tenant_id != auth.tenant_id || product.is_deleted {
        return Err(AppError::not_found("商品不存在").with_request_id(request_id));
    }

    if let Some(expected_version) = req.expected_version
        && expected_version != product.version
    {
        return Err(AppError::conflict(4091, "版本冲突，请刷新后重试")
            .with_data(json!({
                "resource": "product",
                "resource_id": product.id,
                "expected_version": expected_version,
                "current_version": product.version,
                "latest_snapshot": product_snapshot(product)
            }))
            .with_request_id(request_id));
    }

    let old_stock = product.current_stock;
    let old_cost = product.cost_price;
    let new_stock = old_stock + req.qty;

    let effective_unit_cost = unit_cost.or(product.last_inbound_unit_cost).unwrap_or(old_cost);
    let new_cost = if new_stock <= 0 {
        effective_unit_cost.round_dp(4)
    } else {
        let old_total = old_cost * Decimal::from(old_stock);
        let in_total = effective_unit_cost * Decimal::from(req.qty);
        ((old_total + in_total) / Decimal::from(new_stock)).round_dp(4)
    };

    product.current_stock = new_stock;
    product.cost_price = new_cost;
    product.last_inbound_unit_cost = Some(effective_unit_cost.round_dp(4));
    product.version += 1;
    let updated = product.clone();
    drop(products);

    let mut stock_logs = state
        .stock_logs
        .lock()
        .map_err(|_| AppError::internal("库存流水锁异常").with_request_id(request_id.clone()))?;
    let log_id = stock_logs.len() as i64 + 1;
    stock_logs.push(StockLog::now(
        log_id,
        auth.tenant_id,
        updated.id,
        "IN_PURCHASE",
        biz_no.clone(),
        req.qty,
        updated.current_stock,
        updated.cost_price,
        None,
        Some(effective_unit_cost),
        auth.user_id,
    ));
    drop(stock_logs);

    let data = InventoryResultData {
        biz_no,
        product_id: updated.id,
        current_stock: updated.current_stock,
        cost_price: updated.cost_price.round_dp(4).to_string(),
        version: updated.version,
        track_batches: updated.track_batches,
    };

    let body = ApiResponse::success(data, request_id.clone());
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("入库响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record_sync(&state, &scope_key, request_payload, response_body)?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn inbound_batch(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    AppJson(req): AppJson<InboundBatchRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("批量入库请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/inventory/inbound/batch", idempotency_key
    );

    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if req.items.is_empty() {
        return Err(AppError::bad_request("items 不能为空").with_request_id(request_id));
    }

    let biz_no = generate_server_biz_no("IN");

    if state.repository.is_postgres() {
        let mut repo_items = Vec::with_capacity(req.items.len());
        for item in &req.items {
            if item.qty <= 0 {
                return Err(
                    AppError::bad_request("items.qty 必须大于 0").with_request_id(request_id)
                );
            }
            let unit_cost = item
                .unit_cost
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(|raw| parse_decimal(raw, "unit_cost", &request_id))
                .transpose()?;

            let product_id = if let Some(pid) = item.product_id {
                pid
            } else {
                let barcode = item
                    .barcode
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| {
                        AppError::bad_request("items.product_id 或 items.barcode 至少提供一个")
                            .with_request_id(request_id.clone())
                    })?;
                state
                    .repository
                    .find_product_by_barcode(
                        postgres_pool_or_none(&state),
                        auth.tenant_id,
                        barcode,
                        false,
                    )
                    .await
                    .map_err(|err| err.with_request_id(request_id.clone()))?
                    .ok_or_else(|| {
                        AppError::not_found("商品不存在").with_request_id(request_id.clone())
                    })?
                    .id
            };

            repo_items.push((product_id, item.qty, unit_cost, item.expected_version));
        }

        let updated_products = state
            .repository
            .inbound_batch(
                postgres_pool_or_none(&state),
                auth.tenant_id,
                &biz_no,
                &repo_items,
                auth.user_id,
            )
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;

        let data = InventoryBatchResultData {
            biz_no: biz_no.clone(),
            items: updated_products
                .into_iter()
                .map(|p| InventoryBatchItemResultData {
                    product_id: p.id,
                    current_stock: p.current_stock,
                    cost_price: p.cost_price.round_dp(4).to_string(),
                    version: p.version,
                    track_batches: p.track_batches,
                })
                .collect(),
        };

        let body = ApiResponse::success(data, request_id.clone());
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("批量入库响应序列化失败").with_request_id(request_id.clone())
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

    let mut prepared = Vec::with_capacity(req.items.len());

    for item in &req.items {
        if item.qty <= 0 {
            return Err(AppError::bad_request("items.qty 必须大于 0").with_request_id(request_id));
        }

        let resolved_product_id = resolve_product_id(
            &state,
            auth.tenant_id,
            item.product_id,
            item.barcode.as_deref(),
            &request_id,
        )?;

        let product = products
            .get(&resolved_product_id)
            .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;

        if product.tenant_id != auth.tenant_id || product.is_deleted {
            return Err(AppError::not_found("商品不存在").with_request_id(request_id));
        }

        if let Some(ev) = item.expected_version
            && ev != product.version
        {
            return Err(AppError::conflict(4091, "版本冲突，请刷新后重试")
                .with_data(json!({
                    "resource": "product",
                    "resource_id": product.id,
                    "expected_version": ev,
                    "current_version": product.version,
                    "latest_snapshot": product_snapshot(product)
                }))
                .with_request_id(request_id));
        }

        let parsed_unit_cost = item
            .unit_cost
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|raw| parse_decimal(raw, "unit_cost", &request_id))
            .transpose()?;

        prepared.push((resolved_product_id, item.qty, parsed_unit_cost));
    }

    let mut result_items = Vec::with_capacity(prepared.len());
    let mut log_entries = Vec::with_capacity(prepared.len());
    for (product_id, qty, parsed_unit_cost) in &prepared {
        let product = products
            .get_mut(product_id)
            .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;

        let old_stock = product.current_stock;
        let old_cost = product.cost_price;
        let new_stock = old_stock + *qty;
        let effective_unit_cost = (*parsed_unit_cost)
            .or(product.last_inbound_unit_cost)
            .unwrap_or(old_cost);
        let new_cost = if new_stock <= 0 {
            effective_unit_cost.round_dp(4)
        } else {
            let old_total = old_cost * Decimal::from(old_stock);
            let in_total = effective_unit_cost * Decimal::from(*qty);
            ((old_total + in_total) / Decimal::from(new_stock)).round_dp(4)
        };

        product.current_stock = new_stock;
        product.cost_price = new_cost;
        product.last_inbound_unit_cost = Some(effective_unit_cost.round_dp(4));
        product.version += 1;

        result_items.push(InventoryBatchItemResultData {
            product_id: product.id,
            current_stock: product.current_stock,
            cost_price: product.cost_price.round_dp(4).to_string(),
            version: product.version,
            track_batches: product.track_batches,
        });

        log_entries.push((
            product.id,
            *qty,
            product.current_stock,
            product.cost_price,
            effective_unit_cost,
        ));
    }

    drop(products);

    let mut stock_logs = state
        .stock_logs
        .lock()
        .map_err(|_| AppError::internal("库存流水锁异常").with_request_id(request_id.clone()))?;
    let mut next_log_id = stock_logs.len() as i64 + 1;
    for (product_id, qty, current_stock, cost_price, inbound_unit_cost) in log_entries {
        stock_logs.push(StockLog::now(
            next_log_id,
            auth.tenant_id,
            product_id,
            "IN_PURCHASE",
            biz_no.clone(),
            qty,
            current_stock,
            cost_price,
            None,
            Some(inbound_unit_cost),
            auth.user_id,
        ));
        next_log_id += 1;
    }
    drop(stock_logs);

    let data = InventoryBatchResultData {
        biz_no,
        items: result_items,
    };
    let body = ApiResponse::success(data, request_id.clone());
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("批量入库响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record_sync(&state, &scope_key, request_payload, response_body)?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn outbound(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    AppJson(req): AppJson<crate::routes::common::OutboundRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "SALES"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("出库请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/inventory/outbound", idempotency_key
    );

    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if req.items.is_empty() {
        return Err(AppError::bad_request("items 不能为空").with_request_id(request_id));
    }
    let biz_no = generate_server_biz_no("OUT");

    let mut std_items = Vec::with_capacity(req.items.len());
    for item in &req.items {
        if item.qty <= 0 {
            return Err(AppError::bad_request("items.qty 必须大于 0").with_request_id(request_id));
        }
        let sell_price = parse_decimal(&item.sell_price, "sell_price", &request_id)?;
        std_items.push((item, sell_price));
    }

    if state.repository.is_postgres() {
        let outbound_items = std_items
            .iter()
            .map(|(item, sell_price)| {
                (
                    item.product_id,
                    item.qty,
                    item.expected_version,
                    Some(*sell_price),
                )
            })
            .collect::<Vec<_>>();

        let updated_products = state
            .repository
            .outbound(
                postgres_pool_or_none(&state),
                auth.tenant_id,
                &biz_no,
                req.expected_version,
                &outbound_items,
                state.config.allow_negative_stock,
                auth.user_id,
            )
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;

        use std::collections::HashMap;
        let product_map = updated_products
            .into_iter()
            .map(|p| (p.id, p))
            .collect::<HashMap<_, _>>();

        let mut total_amount = Decimal::ZERO;
        let mut result_items = Vec::with_capacity(req.items.len());
        for (item, sell_price) in &std_items {
            let product = product_map.get(&item.product_id).ok_or_else(|| {
                AppError::internal("出库结果与请求明细不一致").with_request_id(request_id.clone())
            })?;
            total_amount += *sell_price * Decimal::from(item.qty);
            result_items.push(OutboundItemResult {
                product_id: product.id,
                qty: item.qty,
                current_stock: product.current_stock,
                version: product.version,
            });
        }

        let body = ApiResponse::success(
            json!({
                "biz_no": biz_no,
                "items": result_items,
                "total_amount": total_amount.round_dp(4).to_string()
            }),
            request_id.clone(),
        );

        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("出库响应序列化失败").with_request_id(request_id.clone())
        })?;
        save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

        return Ok((
            StatusCode::OK,
            build_response_headers(&request_id, false),
            Json(body),
        )
            .into_response());
    }

    let mut total_amount = Decimal::ZERO;
    let mut products = state
        .products
        .lock()
        .map_err(|_| AppError::internal("商品状态锁异常").with_request_id(request_id.clone()))?;

    for (item, _) in &std_items {
        let product = products
            .get(&item.product_id)
            .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;

        if product.tenant_id != auth.tenant_id || product.is_deleted {
            return Err(AppError::not_found("商品不存在").with_request_id(request_id));
        }

        let expected_version = item.expected_version.or(req.expected_version);
        if let Some(ev) = expected_version
            && ev != product.version
        {
            return Err(AppError::conflict(4091, "版本冲突，请刷新后重试")
                .with_data(json!({
                    "resource": "product",
                    "resource_id": product.id,
                    "expected_version": ev,
                    "current_version": product.version,
                    "latest_snapshot": product_snapshot(product)
                }))
                .with_request_id(request_id));
        }

        if !state.config.allow_negative_stock && product.current_stock < item.qty {
            return Err(
                AppError::business(StatusCode::BAD_REQUEST, 4001, "库存不足")
                    .with_data(json!({
                        "failed_product_id": product.id,
                        "available_stock": product.current_stock,
                        "required_qty": item.qty
                    }))
                    .with_request_id(request_id),
            );
        }
    }

    let mut result_items = Vec::with_capacity(req.items.len());
    let mut stock_logs = state
        .stock_logs
        .lock()
        .map_err(|_| AppError::internal("库存流水锁异常").with_request_id(request_id.clone()))?;
    let start_log_id = stock_logs.len() as i64 + 1;

    for (next_log_id, (item, sell_price)) in (start_log_id..).zip(std_items.iter()) {
        let product = products
            .get_mut(&item.product_id)
            .ok_or_else(|| AppError::not_found("商品不存在").with_request_id(request_id.clone()))?;

        product.current_stock -= item.qty;
        product.version += 1;

        stock_logs.push(StockLog::now(
            next_log_id,
            auth.tenant_id,
            product.id,
            "OUT_SALE",
            biz_no.clone(),
            -item.qty,
            product.current_stock,
            product.cost_price,
            Some(*sell_price),
            None,
            auth.user_id,
        ));

        total_amount += *sell_price * Decimal::from(item.qty);
        result_items.push(OutboundItemResult {
            product_id: product.id,
            qty: item.qty,
            current_stock: product.current_stock,
            version: product.version,
        });
    }

    drop(stock_logs);
    drop(products);

    let body = ApiResponse::success(
        json!({
            "biz_no": biz_no,
            "items": result_items,
            "total_amount": total_amount.round_dp(4).to_string()
        }),
        request_id.clone(),
    );

    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("出库响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record_sync(&state, &scope_key, request_payload, response_body)?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}
