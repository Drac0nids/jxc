use axum::{
    Json,
    extract::{Extension, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    http::StatusCode,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::AppError,
    extractors::AppJson,
    middleware::AuthContext,
    response::{ApiResponse, build_response_headers, resolve_request_id},
    routes::common::{
        ensure_role, generate_server_biz_no, postgres_pool_or_none,
        require_idempotency_key, save_idempotency_record, try_idempotent_replay,
    },
    state::AppState,
};

// ── Request / Query types ─────────────────────────────────────────────────────

#[derive(Deserialize, serde::Serialize)]
pub struct SerialInboundRequest {
    pub product_id: i64,
    pub batch_id: Option<Uuid>,
    #[serde(default)]
    pub unit_cost: Option<String>,
    pub sns: Vec<String>,
}

#[derive(Deserialize, serde::Serialize)]
pub struct SerialOutboundRequest {
    #[serde(default)]
    pub sell_price: Option<String>,
    pub sns: Vec<String>,
}

#[derive(Deserialize)]
pub struct SerialListQuery {
    pub product_id: Option<i64>,
    pub sn: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct SerialHistoryQuery {
    pub status: Option<String>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 { 1 }
fn default_page_size() -> i64 { 20 }

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_optional_decimal(s: &Option<String>, field: &str, request_id: &str) -> Result<Option<Decimal>, AppError> {
    match s {
        None => Ok(None),
        Some(v) if v.trim().is_empty() => Ok(None),
        Some(v) => v.trim().parse::<Decimal>()
            .map(Some)
            .map_err(|_| AppError::bad_request(format!("{field} 必须是合法金额")).with_request_id(request_id.to_string())),
    }
}

// 使用与其他操作一致的带随机后缀的业务单号，避免并发碰撞
fn serial_biz_no() -> String {
    generate_server_biz_no("SN")
}

// ── serial_inbound ────────────────────────────────────────────────────────────

pub async fn serial_inbound(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    AppJson(req): AppJson<SerialInboundRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("流水码入库请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/serials/inbound", idempotency_key
    );
    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if req.sns.is_empty() {
        return Err(AppError::bad_request("sns 不能为空").with_request_id(request_id));
    }

    let unit_cost = parse_optional_decimal(&req.unit_cost, "unit_cost", &request_id)?;
    let biz_no = serial_biz_no();

    state.repository
        .serial_inbound(
            &state.persistence,
            auth.tenant_id,
            req.product_id,
            req.batch_id,
            unit_cost,
            &biz_no,
            &req.sns,
        )
        .await
        .map_err(|e| e.with_request_id(request_id.clone()))?;

    let body = ApiResponse::success(
        json!({ "biz_no": biz_no, "count": req.sns.len() }),
        request_id.clone(),
    );
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("流水码入库响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

    Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response())
}

// ── serial_outbound ───────────────────────────────────────────────────────────

pub async fn serial_outbound(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    AppJson(req): AppJson<SerialOutboundRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    // PURCHASER 也可做流水码出库（如采购退货场景）
    ensure_role(&auth.role, &["OWNER", "ADMIN", "SALES", "PURCHASER"], &request_id)?;

    let idempotency_key = require_idempotency_key(&headers, &request_id)?;
    let request_payload = serde_json::to_value(&req).map_err(|_| {
        AppError::internal("流水码出库请求序列化失败").with_request_id(request_id.clone())
    })?;
    let scope_key = format!(
        "{}:{}:{}:{}",
        auth.tenant_id, "POST", "/api/v1/serials/outbound", idempotency_key
    );
    if let Some(replayed) =
        try_idempotent_replay(&state, &scope_key, &request_payload, &request_id).await?
    {
        return Ok(replayed);
    }

    if req.sns.is_empty() {
        return Err(AppError::bad_request("sns 不能为空").with_request_id(request_id));
    }

    let sell_price = parse_optional_decimal(&req.sell_price, "sell_price", &request_id)?;
    let biz_no = serial_biz_no();

    let results = state.repository
        .serial_outbound(
            &state.persistence,
            auth.tenant_id,
            &biz_no,
            sell_price,
            &req.sns,
        )
        .await
        .map_err(|e| e.with_request_id(request_id.clone()))?;

    let list: Vec<_> = results.iter().map(|s| json!({
        "sn": s.sn,
        "product_id": s.product_id,
        "status": s.status,
        "outbound_biz_no": s.outbound_biz_no,
    })).collect();

    let body = ApiResponse::success(
        json!({ "biz_no": biz_no, "count": list.len(), "list": list }),
        request_id.clone(),
    );
    let response_body = serde_json::to_value(&body).map_err(|_| {
        AppError::internal("流水码出库响应序列化失败").with_request_id(request_id.clone())
    })?;
    save_idempotency_record(&state, &scope_key, request_payload, response_body).await?;

    Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response())
}

// ── list / find serials ───────────────────────────────────────────────────────

pub async fn query_serials(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(q): Query<SerialListQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER", "SALES"], &request_id)?;

    // 按 SN 精确查
    if let Some(sn) = &q.sn {
        let result = state.repository
            .find_serial_by_sn(&state.persistence, auth.tenant_id, sn)
            .await
            .map_err(|e| e.with_request_id(request_id.clone()))?;

        let body = ApiResponse::success(json!(result), request_id.clone());
        return Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response());
    }

    // 按商品列表
    let product_id = q.product_id.ok_or_else(|| {
        AppError::bad_request("必须提供 product_id 或 sn").with_request_id(request_id.clone())
    })?;

    let list = state.repository
        .list_serials_by_product(
            &state.persistence,
            auth.tenant_id,
            product_id,
            q.status.as_deref(),
        )
        .await
        .map_err(|e| e.with_request_id(request_id.clone()))?;

    let total = list.len() as u64;
    let body = ApiResponse::success(json!({ "list": list, "total": total }), request_id.clone());
    Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response())
}

// ── list serial history ────────────────────────────────────────────────────────

pub async fn list_serial_history(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(q): Query<SerialHistoryQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER", "SALES"], &request_id)?;

    let page = q.page.max(1);
    let page_size = q.page_size.clamp(10, 100);

    let (list, total) = state.repository
        .list_serials_history(
            &state.persistence,
            auth.tenant_id,
            q.status.as_deref(),
            page,
            page_size,
        )
        .await
        .map_err(|e| e.with_request_id(request_id.clone()))?;

    let body = ApiResponse::success(
        json!({ "list": list, "total": total, "page": page, "page_size": page_size }),
        request_id.clone(),
    );
    Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response())
}
