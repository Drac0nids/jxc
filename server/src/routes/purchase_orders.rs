use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{FixedOffset, Utc};
use rust_decimal::Decimal;
use serde_json::json;

use crate::routes::common::{
    DashboardOrdersQuery, OrderActionRequest, PurchaseOrderCreateRequest, append_audit_log,
    ensure_role, is_report_date_in_range,
    generate_server_biz_no, parse_decimal, postgres_pool_or_none, purchase_order_snapshot,
    parse_report_date, require_idempotency_key, save_idempotency_record,
    save_idempotency_record_sync, to_purchase_order_data,
    to_purchase_order_data_with_product_names, try_idempotent_replay,
};
use crate::{
    error::AppError,
    extractors::AppJson,
    middleware::AuthContext,
    models::{PurchaseOrder, PurchaseOrderItem, PurchaseOrderStatus, StockLog},
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};

pub async fn create_purchase_order(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    AppJson(req): AppJson<PurchaseOrderCreateRequest>,
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
            product_name_snapshot: None,
        });
    }

    if state.repository.is_postgres() {
        let pool = postgres_pool_or_none(&state);

        for item in &mut items {
            let product = state
                .repository
                .find_product_by_id(pool, auth.tenant_id, item.product_id, false)
                .await
                .map_err(|err| err.with_request_id(request_id.clone()))?
                .ok_or_else(|| {
                    AppError::not_found("商品不存在").with_request_id(request_id.clone())
                })?;
            item.product_name_snapshot = Some(product.name);
        }

        let id = state
            .repository
            .next_purchase_order_id(pool)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;
        let now = Utc::now().to_rfc3339();

        for _ in 0..8 {
            let biz_no = generate_server_biz_no("PO");
            if state
                .repository
                .is_purchase_order_biz_no_taken(pool, auth.tenant_id, &biz_no)
                .await
                .map_err(|err| err.with_request_id(request_id.clone()))?
            {
                continue;
            }

            let order = PurchaseOrder {
                id,
                tenant_id: auth.tenant_id,
                biz_no,
                supplier_id: req.supplier_id,
                status: PurchaseOrderStatus::Draft,
                items: items.clone(),
                remark: req.remark.clone(),
                created_by: auth.user_id,
                version: 1,
                confirmed_at: None,
                voided_at: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            match state.repository.create_purchase_order(pool, &order).await {
                Ok(_) => {
                    let body = ApiResponse::success(
                        to_purchase_order_data_with_product_names(
                            &state,
                            auth.tenant_id,
                            &request_id,
                            &order,
                        )
                        .await?,
                        request_id.clone(),
                    );
                    let response_body = serde_json::to_value(&body).map_err(|_| {
                        AppError::internal("采购单创建响应序列化失败")
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

        return Err(AppError::internal("采购单号生成失败，请稍后重试").with_request_id(request_id));
    }

    {
        let products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;
        for item in &mut items {
            let product = products.get(&item.product_id).ok_or_else(|| {
                AppError::not_found("商品不存在").with_request_id(request_id.clone())
            })?;
            if product.tenant_id != auth.tenant_id || product.is_deleted {
                return Err(AppError::not_found("商品不存在").with_request_id(request_id));
            }
            item.product_name_snapshot = Some(product.name.clone());
        }
    }

    let order = {
        let now = Utc::now().to_rfc3339();
        let id = {
            let mut next_id = state.next_purchase_order_id.lock().map_err(|_| {
                AppError::internal("采购单ID状态锁异常").with_request_id(request_id.clone())
            })?;
            *next_id += 1;
            *next_id
        };

        let mut orders = state.purchase_orders.lock().map_err(|_| {
            AppError::internal("采购单状态锁异常").with_request_id(request_id.clone())
        })?;
        let mut selected_biz_no = None;
        for _ in 0..8 {
            let candidate = generate_server_biz_no("PO");
            let taken = orders
                .values()
                .any(|o| o.tenant_id == auth.tenant_id && o.biz_no == candidate);
            if !taken {
                selected_biz_no = Some(candidate);
                break;
            }
        }

        let biz_no = selected_biz_no.ok_or_else(|| {
            AppError::internal("采购单号生成失败，请稍后重试").with_request_id(request_id.clone())
        })?;

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

        orders.insert(order.id, order.clone());
        order
    };

    let body = ApiResponse::success(
        to_purchase_order_data_with_product_names(&state, auth.tenant_id, &request_id, &order)
            .await?,
        request_id.clone(),
    );
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

pub async fn list_purchase_orders(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(query): Query<DashboardOrdersQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "PURCHASER"], &request_id)?;

    let timezone = FixedOffset::east_opt(8 * 3600)
        .ok_or_else(|| AppError::internal("时区配置异常").with_request_id(request_id.clone()))?;
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

    let today = Utc::now().with_timezone(&timezone).date_naive();
    let (start_date, end_date) = match (
        query
            .start_date
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty()),
        query
            .end_date
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty()),
    ) {
        (None, None) => (today, today),
        (Some(start), Some(end)) => (
            parse_report_date(start, "start_date", &request_id)?,
            parse_report_date(end, "end_date", &request_id)?,
        ),
        _ => {
            return Err(AppError::bad_request("start_date 与 end_date 需同时提供")
                .with_request_id(request_id));
        }
    };

    if start_date > end_date {
        return Err(
            AppError::bad_request("start_date 不能晚于 end_date").with_request_id(request_id)
        );
    }

    let mut orders: Vec<PurchaseOrder> = if state.repository.is_postgres() {
        state
            .repository
            .list_purchase_orders_by_tenant(postgres_pool_or_none(&state), auth.tenant_id)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?
    } else {
        let orders = state.purchase_orders.lock().map_err(|_| {
            AppError::internal("采购单状态锁异常").with_request_id(request_id.clone())
        })?;

        orders
            .values()
            .filter(|order| order.tenant_id == auth.tenant_id)
            .cloned()
            .collect()
    };

    orders.retain(|order| {
        is_report_date_in_range(
            &order.created_at,
            start_date,
            end_date,
            &timezone,
            &request_id,
        )
        .unwrap_or(false)
    });
    orders.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| b.id.cmp(&a.id))
    });

    let product_name_map = crate::routes::common::load_product_name_map(&state, auth.tenant_id, &request_id)
        .await?;
    let total = orders.len() as u64;
    let start = ((page - 1) * page_size) as usize;
    let end = usize::min(start + page_size as usize, orders.len());
    let paged = if start >= orders.len() {
        Vec::new()
    } else {
        orders[start..end].to_vec()
    };

    let list = paged
        .iter()
        .map(|order| to_purchase_order_data(order, &product_name_map))
        .collect::<Vec<_>>();

    let body = ApiResponse::success(
        json!({
            "start_date": start_date.to_string(),
            "end_date": end_date.to_string(),
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

    let body = ApiResponse::success(
        to_purchase_order_data_with_product_names(&state, auth.tenant_id, &request_id, &order)
            .await?,
        request_id.clone(),
    );
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
    AppJson(req): AppJson<OrderActionRequest>,
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

        let body = ApiResponse::success(
            to_purchase_order_data_with_product_names(&state, auth.tenant_id, &request_id, &updated)
                .await?,
            request_id.clone(),
        );
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
                None,
                Some(item.unit_cost),
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

    let body = ApiResponse::success(
        to_purchase_order_data_with_product_names(&state, auth.tenant_id, &request_id, &updated)
            .await?,
        request_id.clone(),
    );
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
    AppJson(req): AppJson<OrderActionRequest>,
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

        let body = ApiResponse::success(
            to_purchase_order_data_with_product_names(&state, auth.tenant_id, &request_id, &updated)
                .await?,
            request_id.clone(),
        );
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
                None,
                None,
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

    let body = ApiResponse::success(
        to_purchase_order_data_with_product_names(&state, auth.tenant_id, &request_id, &updated)
            .await?,
        request_id.clone(),
    );
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
