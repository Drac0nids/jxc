use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    error::AppError,
    middleware::AuthContext,
    models::ProductBatch,
    repository::ProductBatchWithProduct,
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};
use crate::routes::common::{ensure_role, postgres_pool_or_none};

// ── 请求结构 ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateBatchRequest {
    pub product_id: i64,
    pub lot_number: Option<String>,
    pub supplier: Option<String>,
    pub inbound_at: Option<NaiveDate>,
    pub produced_at: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBatchRequest {
    pub lot_number: Option<String>,
    pub supplier: Option<String>,
    pub produced_at: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListBatchesQuery {
    pub product_id: Option<i64>,
    pub only_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ExpiringQuery {
    pub within_days: Option<i32>,
}

// ── 响应结构 ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BatchData {
    pub id: i64,
    pub product_id: i64,
    pub lot_number: String,
    pub supplier: Option<String>,
    pub inbound_at: String,
    pub produced_at: Option<String>,
    pub expires_at: Option<String>,
    pub notes: Option<String>,
    pub is_sold_out: bool,
    pub sold_out_at: Option<String>,
    pub days_until_expiry: Option<i64>,
    pub expiry_level: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ExpiringBatchData {
    pub batch: BatchData,
    pub product_name: String,
    pub product_sku: String,
}

fn batch_to_data(b: ProductBatch) -> BatchData {
    let today = chrono::Utc::now().date_naive();
    let (days_until_expiry, expiry_level) = if let Some(exp) = b.expires_at {
        let days = (exp - today).num_days();
        let level = if days < 0 { "EXPIRED" } else if days < 7 { "CRITICAL" } else if days < 15 { "WARNING" } else { "NOTICE" };
        (Some(days), Some(level.to_string()))
    } else {
        (None, None)
    };
    BatchData {
        id: b.id,
        product_id: b.product_id,
        lot_number: b.lot_number,
        supplier: b.supplier,
        inbound_at: b.inbound_at.to_string(),
        produced_at: b.produced_at.map(|d| d.to_string()),
        expires_at: b.expires_at.map(|d| d.to_string()),
        notes: b.notes,
        is_sold_out: b.is_sold_out,
        sold_out_at: b.sold_out_at.map(|d| d.to_rfc3339()),
        days_until_expiry,
        expiry_level,
        created_at: b.created_at.to_rfc3339(),
    }
}

fn expiring_to_data(e: ProductBatchWithProduct) -> ExpiringBatchData {
    ExpiringBatchData {
        batch: batch_to_data(e.batch),
        product_name: e.product_name,
        product_sku: e.product_sku,
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /batches
pub async fn create_batch(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Json(req): Json<CreateBatchRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    let pool = &state.persistence;
    let today = chrono::Utc::now().date_naive();
    let inbound_at = req.inbound_at.unwrap_or(today);

    // 若 lot_number 未填写，自动生成 YYYYMMDD-NN（NN 为同天同商品的序号，从 01 开始）
    let lot_number = match req.lot_number.filter(|s| !s.trim().is_empty()) {
        Some(v) => v,
        None => {
            // 查询今天该商品已有几个批次，序号 = count + 1
            let date_prefix = inbound_at.format("%Y%m%d").to_string();
            let existing: i64 = state.repository.count_batches_by_date(
                pool, auth.tenant_id, req.product_id, inbound_at,
            ).await.unwrap_or(0);
            format!("{}-{:02}", date_prefix, existing + 1)
        }
    };

    let batch = state.repository.create_product_batch(
        pool,
        auth.tenant_id,
        req.product_id,
        lot_number,
        req.supplier,
        inbound_at,
        req.produced_at,
        req.expires_at,
        req.notes,
    ).await?;

    let body = ApiResponse::success(batch_to_data(batch), request_id.clone());
    Ok((StatusCode::CREATED, build_response_headers(&request_id, false), Json(body)).into_response())
}

/// GET /batches
pub async fn list_batches(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(q): Query<ListBatchesQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let pool = &state.persistence;

    let batches = state.repository.list_product_batches(
        pool,
        auth.tenant_id,
        q.product_id,
        q.only_active.unwrap_or(false),
    ).await?;

    let data: Vec<BatchData> = batches.into_iter().map(batch_to_data).collect();
    let body = ApiResponse::success(data, request_id.clone());
    Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response())
}

/// GET /batches/expiring
pub async fn list_expiring_batches(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Query(q): Query<ExpiringQuery>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let pool = &state.persistence;

    let within_days = q.within_days.unwrap_or(30).clamp(1, 365);
    let items = state.repository.list_expiring_batches(
        pool,
        auth.tenant_id,
        within_days,
    ).await?;

    let data: Vec<ExpiringBatchData> = items.into_iter().map(expiring_to_data).collect();
    let body = ApiResponse::success(data, request_id.clone());
    Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response())
}

/// PUT /batches/:id
pub async fn update_batch(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<UpdateBatchRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    let pool = &state.persistence;
    let batch = state.repository.update_product_batch(
        pool,
        auth.tenant_id,
        id,
        req.lot_number,
        req.supplier,
        req.produced_at,
        req.expires_at,
        req.notes,
    ).await?;

    let body = ApiResponse::success(batch_to_data(batch), request_id.clone());
    Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response())
}

/// POST /batches/:id/sold-out
pub async fn mark_sold_out(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    let pool = &state.persistence;
    let batch = state.repository.mark_batch_sold_out(pool, auth.tenant_id, id).await?;

    let body = ApiResponse::success(batch_to_data(batch), request_id.clone());
    Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response())
}

/// DELETE /batches/:id
pub async fn delete_batch(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN"], &request_id)?;

    let pool = &state.persistence;
    state.repository.delete_product_batch(pool, auth.tenant_id, id).await?;

    let body = ApiResponse::success(json!(null), request_id.clone());
    Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response())
}
