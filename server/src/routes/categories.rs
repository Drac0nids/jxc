use axum::{
    Json,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    error::AppError,
    middleware::AuthContext,
    models::Category,
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};
use crate::routes::common::{ensure_role, postgres_pool_or_none};

// ── 请求/响应结构 ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub parent_id: Option<i64>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct CategoryNodeData {
    pub id: i64,
    pub name: String,
    pub level: i16,
    pub sort_order: i32,
    pub children: Vec<CategoryNodeData>,
}

// ── 树构建辅助 ─────────────────────────────────────────────────────────────────

fn build_tree(mut flat: Vec<Category>) -> Vec<CategoryNodeData> {
    // 按 level、sort_order、id 排序保证稳定
    flat.sort_by(|a, b| {
        a.level
            .cmp(&b.level)
            .then_with(|| a.sort_order.cmp(&b.sort_order))
            .then_with(|| a.id.cmp(&b.id))
    });

    fn build_node(
        cat: Category,
        children_map: &mut std::collections::HashMap<i64, Vec<Category>>,
    ) -> CategoryNodeData {
        let mut children_raw = children_map.remove(&cat.id).unwrap_or_default();
        children_raw.sort_by(|a, b| {
            a.sort_order
                .cmp(&b.sort_order)
                .then_with(|| a.id.cmp(&b.id))
        });
        CategoryNodeData {
            id: cat.id,
            name: cat.name,
            level: cat.level,
            sort_order: cat.sort_order,
            children: children_raw
                .into_iter()
                .map(|c| build_node(c, children_map))
                .collect(),
        }
    }

    let mut children_map: std::collections::HashMap<i64, Vec<Category>> =
        std::collections::HashMap::new();
    let mut roots: Vec<Category> = Vec::new();

    for cat in flat {
        if let Some(pid) = cat.parent_id {
            children_map.entry(pid).or_default().push(cat);
        } else {
            roots.push(cat);
        }
    }

    roots.into_iter().map(|r| build_node(r, &mut children_map)).collect()
}

fn category_to_node(cat: Category) -> CategoryNodeData {
    CategoryNodeData {
        id: cat.id,
        name: cat.name,
        level: cat.level,
        sort_order: cat.sort_order,
        children: vec![],
    }
}

// ── 路由处理函数 ───────────────────────────────────────────────────────────────

/// GET /api/v1/categories/tree — 获取当前租户完整分类树
pub async fn list_category_tree(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let pool = postgres_pool_or_none(&state);

    let categories = state
        .repository
        .list_categories_by_tenant(pool, auth.tenant_id)
        .await?;

    let tree = build_tree(categories);
    let body = ApiResponse::success(tree, request_id.clone());

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

/// POST /api/v1/categories — 创建分类（OWNER 专属）
pub async fn create_category(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Json(req): Json<CreateCategoryRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER"], &request_id)?;

    // 验证名称不为空
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("name 不能为空").with_request_id(request_id));
    }

    // 仅在 Postgres 模式下支持
    if !state.repository.is_postgres() {
        return Err(AppError::bad_request(
            "分类管理仅在数据库模式下支持",
        )
        .with_request_id(request_id));
    }

    let pool = postgres_pool_or_none(&state);
    let sort_order = req.sort_order.unwrap_or(0);

    // 计算层级（从父分类推导）
    let level: i16 = if let Some(parent_id) = req.parent_id {
        let parent = state
            .repository
            .find_category_by_id(pool, auth.tenant_id, parent_id)
            .await?
            .ok_or_else(|| {
                AppError::not_found("父分类不存在").with_request_id(request_id.clone())
            })?;

        if parent.is_deleted {
            return Err(
                AppError::bad_request("父分类已删除").with_request_id(request_id)
            );
        }

        let child_level = parent.level + 1;
        if child_level > 3 {
            return Err(AppError::bad_request("分类层级最多 3 层").with_request_id(request_id));
        }
        child_level
    } else {
        1 // 根节点（大类）
    };

    let new_cat = Category {
        id: 0, // BIGSERIAL，数据库自动生成
        tenant_id: auth.tenant_id,
        parent_id: req.parent_id,
        name,
        level,
        sort_order,
        is_deleted: false,
    };

    let created = state.repository.create_category(pool, &new_cat).await?;

    let body = ApiResponse::success(category_to_node(created), request_id.clone());
    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

/// PUT /api/v1/categories/:id — 更新分类名/排序（OWNER 专属）
pub async fn update_category(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<UpdateCategoryRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER"], &request_id)?;

    if !state.repository.is_postgres() {
        return Err(AppError::bad_request(
            "分类管理仅在数据库模式下支持",
        )
        .with_request_id(request_id));
    }

    let pool = postgres_pool_or_none(&state);

    // 读取现有分类
    let existing = state
        .repository
        .find_category_by_id(pool, auth.tenant_id, id)
        .await?
        .ok_or_else(|| AppError::not_found("分类不存在").with_request_id(request_id.clone()))?;

    if existing.is_deleted {
        return Err(AppError::not_found("分类不存在").with_request_id(request_id));
    }

    let new_name = req
        .name
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or(existing.name.clone());

    let new_sort_order = req.sort_order.unwrap_or(existing.sort_order);

    let updated = state
        .repository
        .update_category(pool, auth.tenant_id, id, &new_name, new_sort_order)
        .await?;

    let body = ApiResponse::success(category_to_node(updated), request_id.clone());
    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

/// DELETE /api/v1/categories/:id — 删除分类（OWNER 专属，有子节点或在用商品时拒绝）
pub async fn delete_category(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER"], &request_id)?;

    if !state.repository.is_postgres() {
        return Err(AppError::bad_request(
            "分类管理仅在数据库模式下支持",
        )
        .with_request_id(request_id));
    }

    let pool = postgres_pool_or_none(&state);
    state
        .repository
        .delete_category(pool, auth.tenant_id, id)
        .await?;

    let body = ApiResponse::success(json!(null), request_id.clone());
    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}
