use axum::{
    Json,
    extract::{Extension, Path, Query, State},
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
    routes::common::{ensure_role, postgres_pool_or_none},
    state::AppState,
};

// ── Request / Query types ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SerialInboundRequest {
    pub product_id: i64,
    pub batch_id: Option<Uuid>,
    #[serde(default)]
    pub unit_cost: Option<String>,
    pub sns: Vec<String>,
}

#[derive(Deserialize)]
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

fn serial_biz_no() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    format!("SN-{ts}")
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

    if req.sns.is_empty() {
        return Err(AppError::bad_request("sns 不能为空").with_request_id(request_id));
    }

    let unit_cost = parse_optional_decimal(&req.unit_cost, "unit_cost", &request_id)?;
    let biz_no = serial_biz_no();

    state.repository
        .serial_inbound(
            postgres_pool_or_none(&state),
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
    ensure_role(&auth.role, &["OWNER", "ADMIN", "SALES"], &request_id)?;

    if req.sns.is_empty() {
        return Err(AppError::bad_request("sns 不能为空").with_request_id(request_id));
    }

    let sell_price = parse_optional_decimal(&req.sell_price, "sell_price", &request_id)?;
    let biz_no = serial_biz_no();

    let results = state.repository
        .serial_outbound(
            postgres_pool_or_none(&state),
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
            .find_serial_by_sn(postgres_pool_or_none(&state), auth.tenant_id, sn)
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
            postgres_pool_or_none(&state),
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
