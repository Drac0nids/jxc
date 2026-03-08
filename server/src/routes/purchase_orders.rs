use axum::{
    Json,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;
use chrono::{Utc};
use rust_decimal::{Decimal};

use crate::{
    error::AppError,
    middleware::{AuthContext},
    models::{PurchaseOrder, PurchaseOrderItem, PurchaseOrderStatus, StockLog},
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};
use crate::routes::common::{
    postgres_pool_or_none, ensure_role, parse_decimal, parse_required_text,
    require_idempotency_key, try_idempotent_replay, save_idempotency_record, save_idempotency_record_sync,
    to_purchase_order_data, purchase_order_snapshot, append_audit_log,
    PurchaseOrderCreateRequest, OrderActionRequest
};

pub async fn create_purchase_order(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Json(req): Json<PurchaseOrderCreateRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("采购单创建请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/purchase-orders", idempotency_key
    );
    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    let biz_no = parse_required_text(&req.biz_no, "biz_no", &request_id)?;
    if req.items.is_empty() {
        return Err(AppError::bad_request("items 不能为空").with_request_id(request_id));
    }

    let mut items = Vec::with_capacity(req.items.len());
    for item in &req.items {
        if item.qty <= 0 {
            return Err(AppError::bad_request("items.qty 必须大于 0").with_request_id(request_id));
        }
        let unit_cost = parse_decimal(&item.unit_cost, "unit_cost", &request_id)?;
        items.push(PurchaseOrderItem {
            product_id: item.product_id,
            qty: item.qty,
            unit_cost,
        });
    }

    if state.repository.is_postgres() {
        let pool = postgres_pool_or_none(&state);

        for item in &items {
            state
                .repository
                .find_product_by_id(pool, auth.tenant_id, item.product_id, false)
                .await
                .map_err(|err| err.with_request_id(request_id.clone()))?
                .ok_or_else(|| {
                    AppError::not_found("商品不存在").with_request_id(request_id.clone())
                })?;
        }

        if state
            .repository
            .is_purchase_order_biz_no_taken(pool, auth.tenant_id, &biz_no)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?
        {
            return Err(AppError::conflict(4090, "采购单号已存在")
                .with_data(json!({ "biz_no": biz_no }))
                .with_request_id(request_id));
        }

        let id = state
            .repository
            .next_purchase_order_id(pool).await
            .map_err(|err| err.with_request_id(request_id.clone()))?;
        let now = Utc::now().to_rfc3339();

        let order = PurchaseOrder {
            id,
            tenant_id: auth.tenant_id,
            biz_no,
            supplier_id: req.supplier_id,
            status: PurchaseOrderStatus::Draft,
            items,
            remark: req.remark,
            created_by: auth.user_id,
            version: 1,
            confirmed_at: None,
            voided_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        state
            .repository
            .create_purchase_order(pool, &order)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;

        let body = ApiResponse::success(to_purchase_order_data(&order), request_id.clone());
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("采购单创建响应序列化失败").with_request_id(request_id.clone())
        })?;
        save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

        return Ok((
            StatusCode::OK,
            build_response_headers(&request_id, false),
            Json(body),
        )
            .into_response());
    }

    {
        let products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;
        for item in &items {
            let product = products.get(&item.product_id).ok_or_else(|| {
                AppError::not_found("商品不存在").with_request_id(request_id.clone())
            })?;
            if product.tenant_id != auth.tenant_id || product.is_deleted {
                return Err(AppError::not_found("商品不存在").with_request_id(request_id));
            }
        }
    }

    let now = Utc::now().to_rfc3339();
    let mut next_id = state.next_purchase_order_id.lock().map_err(|_| {
        AppError::internal("采购单ID状态锁异常").with_request_id(request_id.clone())
    })?;
    *next_id += 1;
    let id = *next_id;
    drop(next_id);

    let order = PurchaseOrder {
        id,
        tenant_id: auth.tenant_id,
        biz_no,
        supplier_id: req.supplier_id,
        status: PurchaseOrderStatus::Draft,
        items,
        remark: req.remark,
        created_by: auth.user_id,
        version: 1,
        confirmed_at: None,
        voided_at: None,
        created_at: now.clone(),
        updated_at: now,
    };

    let mut orders = state
        .purchase_orders
        .lock()
        .map_err(|_| AppError::internal("采购单状态锁异常").with_request_id(request_id.clone()))?;
    if orders
        .values()
        .any(|o| o.tenant_id == auth.tenant_id && o.biz_no == order.biz_no)
    {
        return Err(AppError::conflict(4090, "采购单号已存在")
            .with_data(json!({ "biz_no": order.biz_no }))
            .with_request_id(request_id));
    }
    orders.insert(order.id, order.clone());
    drop(orders);

    let body = ApiResponse::success(to_purchase_order_data(&order), request_id.clone());
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("采购单创建响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record_sync(&state, &scope_key, request_payload, response_body)?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn get_purchase_order(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let order = if state.repository.is_postgres() {
        state
            .repository
            .find_purchase_order_by_id(postgres_pool_or_none(&state), auth.tenant_id, id)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?
            .ok_or_else(|| {
                AppError::not_found("采购单不存在").with_request_id(request_id.clone())
            })?
    } else {
        let orders = state.purchase_orders.lock().map_err(|_| {
            AppError::internal("采购单状态锁异常").with_request_id(request_id.clone())
        })?;
        let order = orders.get(&id).ok_or_else(|| {
            AppError::not_found("采购单不存在").with_request_id(request_id.clone())
        })?;
        if order.tenant_id != auth.tenant_id {
            return Err(AppError::not_found("采购单不存在").with_request_id(request_id));
        }
        order.clone()
    };

    let body = ApiResponse::success(to_purchase_order_data(&order), request_id.clone());
    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn confirm_purchase_order(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<OrderActionRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("采购单确认请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/purchase-orders/confirm", id, idempotency_key
    );
    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if state.repository.is_postgres() {
        let updated = state
            .repository
            .confirm_purchase_order(
                postgres_pool_or_none(&state),
                auth.tenant_id,
                id,
                req.expected_version,
                auth.user_id,
                &request_id,
            )
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;

        let body = ApiResponse::success(to_purchase_order_data(&updated), request_id.clone());
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("采购单确认响应序列化失败").with_request_id(request_id.clone())
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
        let orders = state.purchase_orders.lock().map_err(|_| {
            AppError::internal("采购单状态锁异常").with_request_id(request_id.clone())
        })?;
        let order = orders.get(&id).ok_or_else(|| {
            AppError::not_found("采购单不存在").with_request_id(request_id.clone())
        })?;
        if order.tenant_id != auth.tenant_id {
            return Err(AppError::not_found("采购单不存在").with_request_id(request_id));
        }
        order.clone()
    };

    if let Some(expected_version) = req.expected_version
        && expected_version != existing.version
    {
        return Err(AppError::conflict(4091, "版本冲突，请刷新后重试")
            .with_data(json!({
                "resource": "purchase_order",
                "resource_id": existing.id,
                "expected_version": expected_version,
                "current_version": existing.version,
                "latest_snapshot": purchase_order_snapshot(&existing)
            }))
            .with_request_id(request_id));
    }
    if existing.status != PurchaseOrderStatus::Draft {
        return Err(AppError::conflict(4090, "仅 DRAFT 状态可确认采购单")
            .with_data(json!({ "status": existing.status.as_str() }))
            .with_request_id(request_id));
    }

    {
        let mut products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;
        let mut stock_logs = state.stock_logs.lock().map_err(|_| {
            AppError::internal("库存流水锁异常").with_request_id(request_id.clone())
        })?;
        let start_log_id = stock_logs.len() as i64 + 1;

        for (next_log_id, item) in (start_log_id..).zip(existing.items.iter()) {
            let product = products.get_mut(&item.product_id).ok_or_else(|| {
                AppError::not_found("商品不存在").with_request_id(request_id.clone())
            })?;
            if product.tenant_id != auth.tenant_id || product.is_deleted {
                return Err(AppError::not_found("商品不存在").with_request_id(request_id));
            }

            let old_stock = product.current_stock;
            let old_cost = product.cost_price;
            let new_stock = old_stock + item.qty;
            let new_cost = if new_stock <= 0 {
                item.unit_cost.round_dp(4)
            } else {
                let old_total = old_cost * Decimal::from(old_stock);
                let in_total = item.unit_cost * Decimal::from(item.qty);
                ((old_total + in_total) / Decimal::from(new_stock)).round_dp(4)
            };

            product.current_stock = new_stock;
            product.cost_price = new_cost;
            product.version += 1;

            stock_logs.push(StockLog::now(
                next_log_id,
                auth.tenant_id,
                product.id,
                "IN_PURCHASE",
                existing.biz_no.clone(),
                item.qty,
                product.current_stock,
                product.cost_price,
                auth.user_id,
            ));
        }
    }

    let updated = {
        let mut orders = state.purchase_orders.lock().map_err(|_| {
            AppError::internal("采购单状态锁异常").with_request_id(request_id.clone())
        })?;
        let order = orders.get_mut(&id).ok_or_else(|| {
            AppError::not_found("采购单不存在").with_request_id(request_id.clone())
        })?;

        if order.version != existing.version {
            return Err(AppError::conflict(4091, "版本冲突，请刷新后重试")
                .with_data(json!({
                    "resource": "purchase_order",
                    "resource_id": order.id,
                    "expected_version": existing.version,
                    "current_version": order.version,
                    "latest_snapshot": purchase_order_snapshot(order)
                }))
                .with_request_id(request_id));
        }

        let now = Utc::now().to_rfc3339();
        order.status = PurchaseOrderStatus::Confirmed;
        order.version += 1;
        order.confirmed_at = Some(now.clone());
        order.updated_at = now;
        order.clone()
    };

    append_audit_log(
        &state,
        auth.tenant_id,
        auth.user_id,
        "PURCHASE_ORDER_CONFIRM",
        "purchase_order",
        updated.id.to_string(),
        serde_json::to_value(&existing).unwrap_or_else(|_| json!({})),
        serde_json::to_value(&updated).unwrap_or_else(|_| json!({})),
        &request_id,
    )?;

    let body = ApiResponse::success(to_purchase_order_data(&updated), request_id.clone());
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("采购单确认响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record_sync(&state, &scope_key, request_payload, response_body)?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn void_purchase_order(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<OrderActionRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("采购单作废请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/purchase-orders/void", id, idempotency_key
    );
    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if state.repository.is_postgres() {
        let updated = state
            .repository
            .void_purchase_order(
                postgres_pool_or_none(&state),
                auth.tenant_id,
                id,
                req.expected_version,
                state.config.allow_negative_stock,
                auth.user_id,
                &request_id,
            )
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;

        let body = ApiResponse::success(to_purchase_order_data(&updated), request_id.clone());
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("采购单作废响应序列化失败").with_request_id(request_id.clone())
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
        let orders = state.purchase_orders.lock().map_err(|_| {
            AppError::internal("采购单状态锁异常").with_request_id(request_id.clone())
        })?;
        let order = orders.get(&id).ok_or_else(|| {
            AppError::not_found("采购单不存在").with_request_id(request_id.clone())
        })?;
        if order.tenant_id != auth.tenant_id {
            return Err(AppError::not_found("采购单不存在").with_request_id(request_id));
        }
        order.clone()
    };

    if let Some(expected_version) = req.expected_version
        && expected_version != existing.version
    {
        return Err(AppError::conflict(4091, "版本冲突，请刷新后重试")
            .with_data(json!({
                "resource": "purchase_order",
                "resource_id": existing.id,
                "expected_version": expected_version,
                "current_version": existing.version,
                "latest_snapshot": purchase_order_snapshot(&existing)
            }))
            .with_request_id(request_id));
    }
    if existing.status == PurchaseOrderStatus::Voided {
        return Err(AppError::conflict(4090, "采购单已作废").with_request_id(request_id));
    }

    if existing.status == PurchaseOrderStatus::Confirmed {
        let mut products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;

        for item in &existing.items {
            let product = products.get(&item.product_id).ok_or_else(|| {
                AppError::not_found("商品不存在").with_request_id(request_id.clone())
            })?;
            if product.tenant_id != auth.tenant_id || product.is_deleted {
                return Err(AppError::not_found("商品不存在").with_request_id(request_id));
            }
            if !state.config.allow_negative_stock && product.current_stock < item.qty {
                return Err(AppError::business(
                    StatusCode::BAD_REQUEST,
                    4001,
                    "库存不足，无法执行反向作废",
                )
                .with_data(json!({
                    "failed_product_id": product.id,
                    "available_stock": product.current_stock,
                    "required_qty": item.qty
                }))
                .with_request_id(request_id));
            }
        }

        let mut stock_logs = state.stock_logs.lock().map_err(|_| {
            AppError::internal("库存流水锁异常").with_request_id(request_id.clone())
        })?;
        let start_log_id = stock_logs.len() as i64 + 1;

        for (next_log_id, item) in (start_log_id..).zip(existing.items.iter()) {
            let product = products.get_mut(&item.product_id).ok_or_else(|| {
                AppError::not_found("商品不存在").with_request_id(request_id.clone())
            })?;
            product.current_stock -= item.qty;
            product.version += 1;

            stock_logs.push(StockLog::now(
                next_log_id,
                auth.tenant_id,
                product.id,
                "VOID_PURCHASE",
                existing.biz_no.clone(),
                -item.qty,
                product.current_stock,
                product.cost_price,
                auth.user_id,
            ));
        }
    }

    let updated = {
        let mut orders = state.purchase_orders.lock().map_err(|_| {
            AppError::internal("采购单状态锁异常").with_request_id(request_id.clone())
        })?;
        let order = orders.get_mut(&id).ok_or_else(|| {
            AppError::not_found("采购单不存在").with_request_id(request_id.clone())
        })?;

        if order.version != existing.version {
            return Err(AppError::conflict(4091, "版本冲突，请刷新后重试")
                .with_data(json!({
                    "resource": "purchase_order",
                    "resource_id": order.id,
                    "expected_version": existing.version,
                    "current_version": order.version,
                    "latest_snapshot": purchase_order_snapshot(order)
                }))
                .with_request_id(request_id));
        }

        let now = Utc::now().to_rfc3339();
        order.status = PurchaseOrderStatus::Voided;
        order.version += 1;
        order.voided_at = Some(now.clone());
        order.updated_at = now;
        order.clone()
    };

    append_audit_log(
        &state,
        auth.tenant_id,
        auth.user_id,
        "PURCHASE_ORDER_VOID",
        "purchase_order",
        updated.id.to_string(),
        serde_json::to_value(&existing).unwrap_or_else(|_| json!({})),
        serde_json::to_value(&updated).unwrap_or_else(|_| json!({})),
        &request_id,
    )?;

    let body = ApiResponse::success(to_purchase_order_data(&updated), request_id.clone());
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("采购单作废响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record_sync(&state, &scope_key, request_payload, response_body)?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}
