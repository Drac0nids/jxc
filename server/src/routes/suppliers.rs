use axum::{
    Json,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppError,
    middleware::AuthContext,
    models::Supplier,
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};
use crate::routes::common::{ensure_role, postgres_pool_or_none};

// ── 请求/响应结构 ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateSupplierRequest {
    pub name: String,
    pub phone: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSupplierRequest {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SupplierData {
    pub id: i64,
    pub name: String,
    pub phone: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

fn supplier_to_data(s: Supplier) -> SupplierData {
    SupplierData {
        id: s.id,
        name: s.name,
        phone: s.phone,
        notes: s.notes,
        created_at: s.created_at.to_rfc3339(),
    }
}

// ── 端点 ────────────────────────────────────────────────────────────────────────

/// GET /suppliers — 列表（名称模糊搜索）
pub async fn list_suppliers(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let pool = postgres_pool_or_none(&state);
    let keyword = params.get("q").cloned();

    let suppliers = state
        .repository
        .list_suppliers(pool, auth.tenant_id, keyword)
        .await?;

    let data: Vec<SupplierData> = suppliers.into_iter().map(supplier_to_data).collect();
    let body = ApiResponse::success(data, request_id.clone());
    Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response())
}

/// POST /suppliers — 新建
pub async fn create_supplier(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Json(req): Json<CreateSupplierRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    if req.name.trim().is_empty() {
        return Err(AppError::bad_request("供应商名称不能为空"));
    }

    let pool = postgres_pool_or_none(&state);
    let supplier = state
        .repository
        .create_supplier(pool, auth.tenant_id, req.name.trim().to_string(), req.phone, req.notes)
        .await?;

    let body = ApiResponse::success(supplier_to_data(supplier), request_id.clone());
    Ok((StatusCode::CREATED, build_response_headers(&request_id, false), Json(body)).into_response())
}

/// PUT /suppliers/:id — 更新
pub async fn update_supplier(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSupplierRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN", "PURCHASER"], &request_id)?;

    let pool = postgres_pool_or_none(&state);
    let supplier = state
        .repository
        .update_supplier(pool, auth.tenant_id, id, req.name, req.phone, req.notes)
        .await?;

    let body = ApiResponse::success(supplier_to_data(supplier), request_id.clone());
    Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response())
}

/// DELETE /suppliers/:id — 软删除
pub async fn delete_supplier(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN"], &request_id)?;

    let pool = postgres_pool_or_none(&state);
    state
        .repository
        .delete_supplier(pool, auth.tenant_id, id)
        .await?;

    let body = ApiResponse::success(serde_json::json!({"deleted": true}), request_id.clone());
    Ok((StatusCode::OK, build_response_headers(&request_id, false), Json(body)).into_response())
}
