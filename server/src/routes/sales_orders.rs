use axum::{
    Json,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde_json::json;

use crate::routes::common::{
    OrderActionRequest, SalesOrderCreateRequest, SalesOrderReturnRequest, append_audit_log,
    ensure_role, generate_server_biz_no, parse_decimal, postgres_pool_or_none,
    require_idempotency_key, save_idempotency_record,
    save_idempotency_record_sync, to_sales_order_data_with_product_names, try_idempotent_replay,
};
use crate::{
    error::AppError,
    extractors::AppJson,
    middleware::AuthContext,
    models::{SalesOrderItem, SalesOrderStatus, StockLog},
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};

pub async fn create_sales_order(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    AppJson(req): AppJson<SalesOrderCreateRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "SALES"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("销售单创建请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/sales-orders", idempotency_key
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
        let sell_price = parse_decimal(&item.sell_price, "sell_price", &request_id)?;
        items.push(SalesOrderItem {
            product_id: item.product_id,
            qty: item.qty,
            sell_price,
            returned_qty: 0,
            product_name_snapshot: None,
        });
    }

    if state.repository.is_postgres() {
        let pool = &state.persistence;

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
            .next_sales_order_id(pool)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;
        let now = Utc::now().to_rfc3339();

        for _ in 0..8 {
            let biz_no = generate_server_biz_no("SO");
            if state
                .repository
                .is_sales_order_biz_no_taken(pool, auth.tenant_id, &biz_no)
                .await
                .map_err(|err| err.with_request_id(request_id.clone()))?
            {
                continue;
            }

            let order = crate::models::SalesOrder {
                id,
                tenant_id: auth.tenant_id,
                biz_no,
                customer_id: req.customer_id,
                status: SalesOrderStatus::Draft,
                items: items.clone(),
                remark: req.remark.clone(),
                created_by: auth.user_id,
                confirmed_at: None,
                returned_at: None,
                voided_at: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            match state.repository.create_sales_order(pool, &order).await {
                Ok(_) => {
                    let body = ApiResponse::success(
                        to_sales_order_data_with_product_names(
                            &state,
                            auth.tenant_id,
                            &request_id,
                            &order,
                        )
                        .await?,
                        request_id.clone(),
                    );
                    let response_body = serde_json::to_value(&body).map_err(|_| {
                        AppError::internal("销售单创建响应序列化失败")
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

        return Err(AppError::internal("销售单号生成失败，请稍后重试").with_request_id(request_id));
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
            let mut next_id = state.next_sales_order_id.lock().map_err(|_| {
                AppError::internal("销售单ID状态锁异常").with_request_id(request_id.clone())
            })?;
            *next_id += 1;
            *next_id
        };

        let mut orders = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;
        let mut selected_biz_no = None;
        for _ in 0..8 {
            let candidate = generate_server_biz_no("SO");
            let taken = orders
                .values()
                .any(|o| o.tenant_id == auth.tenant_id && o.biz_no == candidate);
            if !taken {
                selected_biz_no = Some(candidate);
                break;
            }
        }

        let biz_no = selected_biz_no.ok_or_else(|| {
            AppError::internal("销售单号生成失败，请稍后重试").with_request_id(request_id.clone())
        })?;

        let order = crate::models::SalesOrder {
            id,
            tenant_id: auth.tenant_id,
            biz_no,
            customer_id: req.customer_id,
            status: SalesOrderStatus::Draft,
            items,
            remark: req.remark,
            created_by: auth.user_id,
            confirmed_at: None,
            returned_at: None,
            voided_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        orders.insert(order.id, order.clone());
        order
    };

    let body = ApiResponse::success(
        to_sales_order_data_with_product_names(&state, auth.tenant_id, &request_id, &order)
            .await?,
        request_id.clone(),
    );
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("销售单创建响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record_sync(&state, &scope_key, request_payload, response_body)?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn get_sales_order(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let order = if state.repository.is_postgres() {
        state
            .repository
            .find_sales_order_by_id(&state.persistence, auth.tenant_id, id)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?
            .ok_or_else(|| {
                AppError::not_found("销售单不存在").with_request_id(request_id.clone())
            })?
    } else {
        let orders = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;
        let order = orders.get(&id).ok_or_else(|| {
            AppError::not_found("销售单不存在").with_request_id(request_id.clone())
        })?;
        if order.tenant_id != auth.tenant_id {
            return Err(AppError::not_found("销售单不存在").with_request_id(request_id));
        }
        order.clone()
    };

    let body = ApiResponse::success(
        to_sales_order_data_with_product_names(&state, auth.tenant_id, &request_id, &order)
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

pub async fn confirm_sales_order(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    AppJson(req): AppJson<OrderActionRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "SALES"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("销售单确认请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/sales-orders/confirm", id, idempotency_key
    );
    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if state.repository.is_postgres() {
        if auth.role.eq_ignore_ascii_case("SALES") {
            let order = state
                .repository
                .find_sales_order_by_id(&state.persistence, auth.tenant_id, id)
                .await
                .map_err(|err| err.with_request_id(request_id.clone()))?
                .ok_or_else(|| {
                    AppError::not_found("销售单不存在").with_request_id(request_id.clone())
                })?;
            if order.created_by != auth.user_id {
                return Err(
                    AppError::forbidden("仅可确认本人草稿销售单").with_request_id(request_id)
                );
            }
        }

        let updated = state
            .repository
            .confirm_sales_order(
                &state.persistence,
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
            to_sales_order_data_with_product_names(&state, auth.tenant_id, &request_id, &updated)
                .await?,
            request_id.clone(),
        );
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("销售单确认响应序列化失败").with_request_id(request_id.clone())
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
        let orders = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;
        let order = orders.get(&id).ok_or_else(|| {
            AppError::not_found("销售单不存在").with_request_id(request_id.clone())
        })?;
        if order.tenant_id != auth.tenant_id {
            return Err(AppError::not_found("销售单不存在").with_request_id(request_id));
        }
        if auth.role.eq_ignore_ascii_case("SALES") && order.created_by != auth.user_id {
            return Err(AppError::forbidden("仅可确认本人草稿销售单").with_request_id(request_id));
        }
        order.clone()
    };

    
    if existing.status != SalesOrderStatus::Draft {
        return Err(AppError::conflict(4090, "仅 DRAFT 状态可确认销售单")
            .with_data(json!({ "status": existing.status.as_str() }))
            .with_request_id(request_id));
    }

    {
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

        let mut stock_logs = state.stock_logs.lock().map_err(|_| {
            AppError::internal("库存流水锁异常").with_request_id(request_id.clone())
        })?;
        let start_log_id = stock_logs.len() as i64 + 1;
        for (next_log_id, item) in (start_log_id..).zip(existing.items.iter()) {
            let product = products.get_mut(&item.product_id).ok_or_else(|| {
                AppError::not_found("商品不存在").with_request_id(request_id.clone())
            })?;
            product.current_stock -= item.qty;

            stock_logs.push(StockLog::now(
                next_log_id,
                auth.tenant_id,
                product.id,
                "OUT_SALE",
                existing.biz_no.clone(),
                -item.qty,
                product.current_stock,
                product.cost_price,
                Some(item.sell_price),
                None,
                auth.user_id,
            ));
        }
    }

    let updated = {
        let mut orders = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;
        let order = orders.get_mut(&id).ok_or_else(|| {
            AppError::not_found("销售单不存在").with_request_id(request_id.clone())
        })?;



        let now = Utc::now().to_rfc3339();
        order.status = SalesOrderStatus::Confirmed;

        order.confirmed_at = Some(now.clone());
        order.updated_at = now;
        order.clone()
    };

    append_audit_log(
        &state,
        auth.tenant_id,
        auth.user_id,
        "SALES_ORDER_CONFIRM",
        "sales_order",
        updated.id.to_string(),
        serde_json::to_value(&existing).unwrap_or_else(|_| json!({})),
        serde_json::to_value(&updated).unwrap_or_else(|_| json!({})),
        &request_id,
    )?;

    let body = ApiResponse::success(
        to_sales_order_data_with_product_names(&state, auth.tenant_id, &request_id, &updated)
            .await?,
        request_id.clone(),
    );
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("销售单确认响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record_sync(&state, &scope_key, request_payload, response_body)?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn void_sales_order(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    AppJson(req): AppJson<OrderActionRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "SALES"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("销售单作废请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/sales-orders/void", id, idempotency_key
    );
    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if state.repository.is_postgres() {
        if auth.role.eq_ignore_ascii_case("SALES") {
            let order = state
                .repository
                .find_sales_order_by_id(&state.persistence, auth.tenant_id, id)
                .await
                .map_err(|err| err.with_request_id(request_id.clone()))?
                .ok_or_else(|| {
                    AppError::not_found("销售单不存在").with_request_id(request_id.clone())
                })?;
            if order.created_by != auth.user_id {
                return Err(
                    AppError::forbidden("仅可作废本人草稿销售单").with_request_id(request_id)
                );
            }
        }

        let updated = state
            .repository
            .void_sales_order(
                &state.persistence,
                auth.tenant_id,
                id,
                req.expected_version,
                auth.user_id,
                &request_id,
            )
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;

        let body = ApiResponse::success(
            to_sales_order_data_with_product_names(&state, auth.tenant_id, &request_id, &updated)
                .await?,
            request_id.clone(),
        );
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("销售单作废响应序列化失败").with_request_id(request_id.clone())
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
        let orders = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;
        let order = orders.get(&id).ok_or_else(|| {
            AppError::not_found("销售单不存在").with_request_id(request_id.clone())
        })?;
        if order.tenant_id != auth.tenant_id {
            return Err(AppError::not_found("销售单不存在").with_request_id(request_id));
        }
        if auth.role.eq_ignore_ascii_case("SALES") && order.created_by != auth.user_id {
            return Err(AppError::forbidden("仅可作废本人草稿销售单").with_request_id(request_id));
        }
        order.clone()
    };

    

    if existing.status != SalesOrderStatus::Draft {
        return Err(AppError::conflict(4090, "仅 DRAFT 状态可作废销售单")
            .with_data(json!({ "status": existing.status.as_str() }))
            .with_request_id(request_id));
    }

    let updated = {
        let mut orders = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;
        let order = orders.get_mut(&id).ok_or_else(|| {
            AppError::not_found("销售单不存在").with_request_id(request_id.clone())
        })?;



        let now = Utc::now().to_rfc3339();
        order.status = SalesOrderStatus::Voided;

        order.voided_at = Some(now.clone());
        order.updated_at = now;
        order.clone()
    };

    append_audit_log(
        &state,
        auth.tenant_id,
        auth.user_id,
        "SALES_ORDER_VOID",
        "sales_order",
        updated.id.to_string(),
        serde_json::to_value(&existing).unwrap_or_else(|_| json!({})),
        serde_json::to_value(&updated).unwrap_or_else(|_| json!({})),
        &request_id,
    )?;

    let body = ApiResponse::success(
        to_sales_order_data_with_product_names(&state, auth.tenant_id, &request_id, &updated)
            .await?,
        request_id.clone(),
    );
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("销售单作废响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

pub async fn return_sales_order(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    AppJson(req): AppJson<SalesOrderReturnRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "SALES"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("销售退货请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/sales-orders/return", id, idempotency_key
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
        if auth.role.eq_ignore_ascii_case("SALES") {
            let order = state
                .repository
                .find_sales_order_by_id(&state.persistence, auth.tenant_id, id)
                .await
                .map_err(|err| err.with_request_id(request_id.clone()))?
                .ok_or_else(|| {
                    AppError::not_found("销售单不存在").with_request_id(request_id.clone())
                })?;
            if order.created_by != auth.user_id {
                return Err(AppError::forbidden("仅可退货本人销售单").with_request_id(request_id));
            }
        }

        let return_items = req
            .items
            .iter()
            .map(|item| (item.product_id, item.qty))
            .collect::<Vec<_>>();

        let updated = state
            .repository
            .return_sales_order(
                &state.persistence,
                auth.tenant_id,
                id,
                req.expected_version,
                &return_items,
                req.remark.clone(),
                auth.user_id,
                &request_id,
            )
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;

        let body = ApiResponse::success(
            to_sales_order_data_with_product_names(&state, auth.tenant_id, &request_id, &updated)
                .await?,
            request_id.clone(),
        );
        let response_body = serde_json::to_value(&body).map_err(|_| {
            AppError::internal("销售退货响应序列化失败").with_request_id(request_id.clone())
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
        let orders = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;
        let order = orders.get(&id).ok_or_else(|| {
            AppError::not_found("销售单不存在").with_request_id(request_id.clone())
        })?;
        if order.tenant_id != auth.tenant_id {
            return Err(AppError::not_found("销售单不存在").with_request_id(request_id));
        }
        if auth.role.eq_ignore_ascii_case("SALES") && order.created_by != auth.user_id {
            return Err(AppError::forbidden("仅可退货本人销售单").with_request_id(request_id));
        }
        order.clone()
    };

    

    match existing.status {
        SalesOrderStatus::Confirmed | SalesOrderStatus::ReturnedPartial => {}
        _ => {
            return Err(AppError::conflict(4090, "当前状态不允许退货")
                .with_data(json!({ "status": existing.status.as_str() }))
                .with_request_id(request_id));
        }
    }

    let mut pending_returns = Vec::with_capacity(req.items.len());
    for item in &req.items {
        if item.qty <= 0 {
            return Err(AppError::bad_request("items.qty 必须大于 0").with_request_id(request_id));
        }
        let order_item = existing
            .items
            .iter()
            .find(|oi| oi.product_id == item.product_id)
            .ok_or_else(|| {
                AppError::bad_request("退货商品不在销售单中").with_data(json!({
                    "product_id": item.product_id
                }))
            })
            .map_err(|e| e.with_request_id(request_id.clone()))?;
        let remain = order_item.qty - order_item.returned_qty;
        if item.qty > remain {
            return Err(AppError::conflict(4090, "退货数量超过可退数量")
                .with_data(json!({
                    "product_id": item.product_id,
                    "remain_qty": remain,
                    "request_qty": item.qty
                }))
                .with_request_id(request_id));
        }
        pending_returns.push((item.product_id, item.qty, order_item.sell_price));
    }

    {
        let mut products = state.products.lock().map_err(|_| {
            AppError::internal("商品状态锁异常").with_request_id(request_id.clone())
        })?;
        let mut stock_logs = state.stock_logs.lock().map_err(|_| {
            AppError::internal("库存流水锁异常").with_request_id(request_id.clone())
        })?;
        let start_log_id = stock_logs.len() as i64 + 1;

        for (next_log_id, (product_id, qty, sell_price)) in
            (start_log_id..).zip(pending_returns.iter())
        {
            let product = products.get_mut(product_id).ok_or_else(|| {
                AppError::not_found("商品不存在").with_request_id(request_id.clone())
            })?;
            if product.tenant_id != auth.tenant_id || product.is_deleted {
                return Err(AppError::not_found("商品不存在").with_request_id(request_id));
            }
            product.current_stock += *qty;

            stock_logs.push(StockLog::now(
                next_log_id,
                auth.tenant_id,
                product.id,
                "RETURN_SALE",
                existing.biz_no.clone(),
                *qty,
                product.current_stock,
                product.cost_price,
                Some(*sell_price),
                None,
                auth.user_id,
            ));
        }
    }

    let updated = {
        let mut orders = state.sales_orders.lock().map_err(|_| {
            AppError::internal("销售单状态锁异常").with_request_id(request_id.clone())
        })?;
        let order = orders.get_mut(&id).ok_or_else(|| {
            AppError::not_found("销售单不存在").with_request_id(request_id.clone())
        })?;



        for (product_id, qty, _) in &pending_returns {
            if let Some(item) = order.items.iter_mut().find(|i| i.product_id == *product_id) {
                item.returned_qty += *qty;
            }
        }

        let is_full = order.items.iter().all(|i| i.returned_qty >= i.qty);
        order.status = if is_full {
            SalesOrderStatus::ReturnedFull
        } else {
            SalesOrderStatus::ReturnedPartial
        };

        if let Some(remark) = req.remark.clone() {
            order.remark = Some(remark);
        }
        let now = Utc::now().to_rfc3339();
        order.returned_at = Some(now.clone());
        order.updated_at = now;
        order.clone()
    };

    append_audit_log(
        &state,
        auth.tenant_id,
        auth.user_id,
        "SALES_ORDER_RETURN",
        "sales_order",
        updated.id.to_string(),
        serde_json::to_value(&existing).unwrap_or_else(|_| json!({})),
        serde_json::to_value(&updated).unwrap_or_else(|_| json!({})),
        &request_id,
    )?;

    let body = ApiResponse::success(
        to_sales_order_data_with_product_names(&state, auth.tenant_id, &request_id, &updated)
            .await?,
        request_id.clone(),
    );
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("销售退货响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}
