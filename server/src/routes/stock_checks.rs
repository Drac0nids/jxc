use axum::{
    Json,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde_json::json;
use std::collections::{HashMap, HashSet};

use crate::routes::common::{
    StockCheckConfirmRequest, StockCheckCreateRequest, append_audit_log, ensure_role,
    generate_server_biz_no, postgres_pool_or_none, product_snapshot,
    require_idempotency_key, save_idempotency_record, save_idempotency_record_sync, to_stock_check_data, to_stock_check_data_with_names,
    try_idempotent_replay,
};
use crate::{
    error::AppError,
    extractors::AppJson,
    middleware::AuthContext,
    models::{StockCheck, StockCheckItem, StockCheckStatus, StockLog},
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};

pub async fn create_stock_check(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    AppJson(req): AppJson<StockCheckCreateRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("盘点单创建请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/inventory/stock-checks", idempotency_key
    );
    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if req.items.is_empty() {
        return Err(AppError::bad_request("items 不能为空").with_request_id(request_id));
    }

    let mut uniq = HashSet::new();
    let mut item_product_ids = Vec::with_capacity(req.items.len());
    for item in &req.items {
        if !uniq.insert(item.product_id) {
            return Err(
                AppError::bad_request("items.product_id 存在重复").with_request_id(request_id)
            );
        }
        item_product_ids.push(item.product_id);
    }

    if state.repository.is_postgres() {
        let pool = &state.persistence;

        let mut items = Vec::with_capacity(item_product_ids.len());
        for product_id in item_product_ids {
            let product = state
                .repository
                .find_product_by_id(pool, auth.tenant_id, product_id, false)
                .await
                .map_err(|err| err.with_request_id(request_id.clone()))?
                .ok_or_else(|| {
                    AppError::not_found("商品不存在").with_request_id(request_id.clone())
                })?;

            items.push(StockCheckItem {
                product_id,
                book_stock: product.current_stock,
                actual_stock: None,
                delta_qty: None,
            });
        }

        let id = state
            .repository
            .next_stock_check_id(pool)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;
        let now = Utc::now().to_rfc3339();

        for _ in 0..8 {
            let biz_no = generate_server_biz_no("SC");
            if state
                .repository
                .is_stock_check_biz_no_taken(pool, auth.tenant_id, &biz_no)
                .await
                .map_err(|err| err.with_request_id(request_id.clone()))?
            {
                continue;
            }

            let check = StockCheck {
                id,
                tenant_id: auth.tenant_id,
                biz_no,
                status: StockCheckStatus::Draft,
                items: items.clone(),
                remark: req.remark.clone(),
                created_by: auth.user_id,
                counting_at: None,
                confirmed_at: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            match state.repository.create_stock_check(pool, &check).await {
                Ok(_) => {
                    let data = to_stock_check_data_with_names(
                        &state, auth.tenant_id, &request_id, &check,
                    )
                    .await
                    .map_err(|err| err.with_request_id(request_id.clone()))?;
                    let body = ApiResponse::success(data, request_id.clone());
                    let response_body = serde_json::to_value(&body).map_err(|_| {
                        AppError::internal("盘点单创建响应序列化失败")
                            .with_request_id(request_id.clone())
                    })?;
                    save_idempotency_record(&state, &scope_key, request_payload, response_body)
                        .await?;

                    return Ok((
                        StatusCode::OK,
                        build_response_headers(&request_id, false),
                        Json(body),
                    )
                    .into_response());
                }
                Err(err) => {
                    let err = err.with_request_id(request_id.clone());
                    if err.status == StatusCode::CONFLICT && err.code == 4090 {
                        continue;
                    }
                    return Err(err);
                }
            }
        }

        return Err(AppError::internal("盘点单号生成失败，请稍后重试").with_request_id(request_id));
    }

    let mut items = Vec::with_capacity(req.items.len());

    {
        let products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;

        for product_id in item_product_ids {
            let product = products.get(&product_id).ok_or_else(|| {
                AppError::not_found("商品不存在").with_request_id(request_id.clone())
            })?;
            if product.tenant_id != auth.tenant_id || product.is_deleted {
                return Err(AppError::not_found("商品不存在").with_request_id(request_id));
            }

            items.push(StockCheckItem {
                product_id,
                book_stock: product.current_stock,
                actual_stock: None,
                delta_qty: None,
            });
        }
    }

    let now = Utc::now().to_rfc3339();
    let mut next_id = state.next_stock_check_id.lock().map_err(|_| {
        AppError::internal("盘点单ID状态锁异常").with_request_id(request_id.clone())
    })?;
    *next_id += 1;
    let id = *next_id;
    drop(next_id);

    let mut checks = state
        .stock_checks
        .lock()
        .map_err(|_| AppError::internal("盘点单状态锁异常").with_request_id(request_id.clone()))?;

    let mut selected_biz_no = None;
    for _ in 0..8 {
        let candidate = generate_server_biz_no("SC");
        let taken = checks
            .values()
            .any(|c| c.tenant_id == auth.tenant_id && c.biz_no == candidate);
        if !taken {
            selected_biz_no = Some(candidate);
            break;
        }
    }

    let biz_no = selected_biz_no.ok_or_else(|| {
        AppError::internal("盘点单号生成失败，请稍后重试").with_request_id(request_id.clone())
    })?;

    let check = StockCheck {
        id,
        tenant_id: auth.tenant_id,
        biz_no,
        status: StockCheckStatus::Draft,
        items,
        remark: req.remark,
        created_by: auth.user_id,
        counting_at: None,
        confirmed_at: None,
        created_at: now.clone(),
        updated_at: now,
    };

    checks.insert(check.id, check.clone());
    drop(checks);

    // 内存模式同步构建商品名称映射（避免将 MutexGuard 跨越 await 点）
    let product_name_map: HashMap<i64, String> = {
        let products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;
        products
            .values()
            .filter(|p| p.tenant_id == auth.tenant_id)
            .map(|p| (p.id, p.name.clone()))
            .collect()
    };
    let body = ApiResponse::success(
        to_stock_check_data(&check, &product_name_map),
        request_id.clone(),
    );
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("盘点单创建响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record_sync(&state, &scope_key, request_payload, response_body)?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
    .into_response())
}

pub async fn get_stock_check(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let check = if state.repository.is_postgres() {
        state
            .repository
            .find_stock_check_by_id(&state.persistence, auth.tenant_id, id)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?
            .ok_or_else(|| {
                AppError::not_found("盘点单不存在").with_request_id(request_id.clone())
            })?
    } else {
        let checks = state.stock_checks.lock().map_err(|_| {
            AppError::internal("盘点单状态锁异常").with_request_id(request_id.clone())
        })?;
        let check = checks.get(&id).ok_or_else(|| {
            AppError::not_found("盘点单不存在").with_request_id(request_id.clone())
        })?;

        if check.tenant_id != auth.tenant_id {
            return Err(AppError::not_found("盘点单不存在").with_request_id(request_id));
        }

        check.clone()
    };

    let body = ApiResponse::success(
        to_stock_check_data_with_names(&state, auth.tenant_id, &request_id, &check)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?,
        request_id.clone(),
    );
    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
    .into_response())
}

pub async fn start_stock_check(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    AppJson(req): AppJson<crate::routes::common::OrderActionRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("盘点单开始请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/inventory/stock-checks/start", id, idempotency_key
    );
    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if state.repository.is_postgres() {
        let updated = state
            .repository
            .start_stock_check(
                &state.persistence,
                auth.tenant_id,
                id,
                req.expected_version,
                auth.user_id,
                &request_id,
            )
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;

        let data = to_stock_check_data_with_names(
            &state, auth.tenant_id, &request_id, &updated,
        )
        .await
        .map_err(|err| err.with_request_id(request_id.clone()))?;
        let body = ApiResponse::success(data, request_id.clone());
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("盘点单开始响应序列化失败").with_request_id(request_id.clone())
        })?;
        save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

        return Ok((
            StatusCode::OK,
            build_response_headers(&request_id, false),
            Json(body),
        )
        .into_response());
    }

    let existing = {
        let checks = state.stock_checks.lock().map_err(|_| {
            AppError::internal("盘点单状态锁异常").with_request_id(request_id.clone())
        })?;
        let check = checks.get(&id).ok_or_else(|| {
            AppError::not_found("盘点单不存在").with_request_id(request_id.clone())
        })?;
        if check.tenant_id != auth.tenant_id {
            return Err(AppError::not_found("盘点单不存在").with_request_id(request_id));
        }
        check.clone()
    };

    

    if existing.status != StockCheckStatus::Draft {
        return Err(AppError::conflict(4090, "仅 DRAFT 状态可开始盘点")
            .with_data(json!({ "status": existing.status.as_str() }))
            .with_request_id(request_id));
    }

    let updated = {
        let mut checks = state.stock_checks.lock().map_err(|_| {
            AppError::internal("盘点单状态锁异常").with_request_id(request_id.clone())
        })?;
        let check = checks.get_mut(&id).ok_or_else(|| {
            AppError::not_found("盘点单不存在").with_request_id(request_id.clone())
        })?;



        let now = Utc::now().to_rfc3339();
        check.status = StockCheckStatus::Counting;

        check.counting_at = Some(now.clone());
        check.updated_at = now;
        check.clone()
    };

    append_audit_log(
        &state,
        auth.tenant_id,
        auth.user_id,
        "STOCK_CHECK_START",
        "stock_check",
        updated.id.to_string(),
        serde_json::to_value(&existing).unwrap_or_else(|_| json!({})),
        serde_json::to_value(&updated).unwrap_or_else(|_| json!({})),
        &request_id,
    )?;

    let data = to_stock_check_data_with_names(&state, auth.tenant_id, &request_id, &updated)
        .await
        .map_err(|err| err.with_request_id(request_id.clone()))?;
    let body = ApiResponse::success(data, request_id.clone());
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("盘点单开始响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
    .into_response())
}

pub async fn confirm_stock_check(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    AppJson(req): AppJson<StockCheckConfirmRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("盘点单确认请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/inventory/stock-checks/confirm", id, idempotency_key
    );
    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if req.items.is_empty() {
        return Err(AppError::bad_request("items 不能为空").with_request_id(request_id));
    }

    if state.repository.is_postgres() {
        let actual_items = req
            .items
            .iter()
            .map(|item| (item.product_id, item.actual_stock))
            .collect::<Vec<_>>();

        let updated = state
            .repository
            .confirm_stock_check(
                &state.persistence,
                auth.tenant_id,
                id,
                req.expected_version,
                &actual_items,
                req.remark.clone(),
                auth.user_id,
                &request_id,
            )
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;

        let data = to_stock_check_data_with_names(
            &state, auth.tenant_id, &request_id, &updated,
        )
        .await
        .map_err(|err| err.with_request_id(request_id.clone()))?;
        let body = ApiResponse::success(data, request_id.clone());
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("盘点单确认响应序列化失败").with_request_id(request_id.clone())
        })?;
        save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

        return Ok((
            StatusCode::OK,
            build_response_headers(&request_id, false),
            Json(body),
        )
        .into_response());
    }

    let mut actual_map: HashMap<i64, i32> = HashMap::new();
    for item in &req.items {
        if item.actual_stock < 0 {
            return Err(
                AppError::bad_request("actual_stock 不能小于 0").with_request_id(request_id)
            );
        }
        if actual_map
            .insert(item.product_id, item.actual_stock)
            .is_some()
        {
            return Err(
                AppError::bad_request("items.product_id 存在重复").with_request_id(request_id)
            );
        }
    }

    let existing = {
        let checks = state.stock_checks.lock().map_err(|_| {
            AppError::internal("盘点单状态锁异常").with_request_id(request_id.clone())
        })?;
        let check = checks.get(&id).ok_or_else(|| {
            AppError::not_found("盘点单不存在").with_request_id(request_id.clone())
        })?;
        if check.tenant_id != auth.tenant_id {
            return Err(AppError::not_found("盘点单不存在").with_request_id(request_id));
        }
        check.clone()
    };

    

    if existing.status != StockCheckStatus::Counting {
        return Err(AppError::conflict(4090, "仅 COUNTING 状态可确认盘点")
            .with_data(json!({ "status": existing.status.as_str() }))
            .with_request_id(request_id));
    }

    if actual_map.len() != existing.items.len() {
        return Err(AppError::bad_request("盘点项数量不匹配")
            .with_data(json!({
                "expected_items": existing.items.len(),
                "actual_items": actual_map.len()
            }))
            .with_request_id(request_id));
    }

    let expected_set: HashSet<i64> = existing.items.iter().map(|i| i.product_id).collect();
    for product_id in actual_map.keys() {
        if !expected_set.contains(product_id) {
            return Err(AppError::bad_request("盘点商品不在盘点单中")
                .with_data(json!({ "product_id": product_id }))
                .with_request_id(request_id));
        }
    }

    let mut adjustment_map: HashMap<i64, (i32, i32)> = HashMap::new();
    {
        let mut products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;
        let mut stock_logs = state.stock_logs.lock().map_err(|_| {
            AppError::internal("库存流水锁异常").with_request_id(request_id.clone())
        })?;
        let mut next_log_id = stock_logs.len() as i64 + 1;

        for item in &existing.items {
            let product = products.get_mut(&item.product_id).ok_or_else(|| {
                AppError::not_found("商品不存在").with_request_id(request_id.clone())
            })?;
            if product.tenant_id != auth.tenant_id || product.is_deleted {
                return Err(AppError::not_found("商品不存在").with_request_id(request_id));
            }

            if product.current_stock != item.book_stock {
                return Err(
                    AppError::conflict(4091, "盘点基准库存已变化，请重新发起盘点")
                        .with_data(json!({
                            "resource": "product",
                            "resource_id": product.id,
                            "book_stock": item.book_stock,
                            "current_stock": product.current_stock,
                            "latest_snapshot": product_snapshot(product)
                        }))
                        .with_request_id(request_id),
                );
            }

            let actual_stock = actual_map.get(&item.product_id).copied().ok_or_else(|| {
                AppError::bad_request("盘点项不完整")
                    .with_data(json!({ "product_id": item.product_id }))
                    .with_request_id(request_id.clone())
            })?;

            let delta = actual_stock - item.book_stock;
            product.current_stock = actual_stock;


            if delta != 0 {
                stock_logs.push(StockLog::now(
                    next_log_id,
                    auth.tenant_id,
                    product.id,
                    "ADJ_CHECK",
                    existing.biz_no.clone(),
                    delta,
                    product.current_stock,
                    product.cost_price,
                    None,
                    None,
                    auth.user_id,
                ));
                next_log_id += 1;
            }

            adjustment_map.insert(item.product_id, (actual_stock, delta));
        }
    }

    let updated = {
        let mut checks = state.stock_checks.lock().map_err(|_| {
            AppError::internal("盘点单状态锁异常").with_request_id(request_id.clone())
        })?;
        let check = checks.get_mut(&id).ok_or_else(|| {
            AppError::not_found("盘点单不存在").with_request_id(request_id.clone())
        })?;



        for item in &mut check.items {
            if let Some((actual_stock, delta_qty)) = adjustment_map.get(&item.product_id).copied() {
                item.actual_stock = Some(actual_stock);
                item.delta_qty = Some(delta_qty);
            }
        }

        let now = Utc::now().to_rfc3339();
        check.status = StockCheckStatus::Confirmed;

        check.confirmed_at = Some(now.clone());
        check.updated_at = now;
        if let Some(remark) = req.remark.clone() {
            check.remark = Some(remark);
        }
        check.clone()
    };

    append_audit_log(
        &state,
        auth.tenant_id,
        auth.user_id,
        "STOCK_CHECK_CONFIRM",
        "stock_check",
        updated.id.to_string(),
        serde_json::to_value(&existing).unwrap_or_else(|_| json!({})),
        serde_json::to_value(&updated).unwrap_or_else(|_| json!({})),
        &request_id,
    )?;

    let data = to_stock_check_data_with_names(&state, auth.tenant_id, &request_id, &updated)
        .await
        .map_err(|err| err.with_request_id(request_id.clone()))?;
    let body = ApiResponse::success(data, request_id.clone());
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("盘点单确认响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
    .into_response())
}
