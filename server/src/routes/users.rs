use axum::{
    Json,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;
use uuid::Uuid;

use crate::routes::common::{
    CreateUserRequest, ResetUserPasswordRequest, UpdateUserRoleRequest, ensure_role,
    parse_required_text, parse_user_role_input, postgres_pool_or_none, to_user_data,
};
use crate::{
    error::AppError,
    extractors::AppJson,
    middleware::AuthContext,
    models::{User, UserRole, hash_password},
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};

// ── 权限辅助 ──────────────────────────────────────────────────────────────────

/// 检查操作者是否有权操作目标角色：操作者 rank 必须 > 目标 rank
fn can_operator_manage_role(operator_role: &str, target_role: &UserRole) -> bool {
    let operator = match operator_role.to_uppercase().as_str() {
        "OWNER" => UserRole::Owner,
        "ADMIN" => UserRole::Admin,
        "PURCHASER" => UserRole::Purchaser,
        "SALES" => UserRole::Sales,
        _ => return false,
    };
    operator.rank() > target_role.rank()
}

/// 同上，但目标角色是字符串（用于判断新角色是否可分配）
fn can_operator_assign_role(operator_role: &str, target_role_str: &str) -> bool {
    let target = match target_role_str.to_uppercase().as_str() {
        "OWNER" => UserRole::Owner,
        "ADMIN" => UserRole::Admin,
        "PURCHASER" => UserRole::Purchaser,
        "SALES" => UserRole::Sales,
        _ => return false,
    };
    can_operator_manage_role(operator_role, &target)
}

// ── list_users ────────────────────────────────────────────────────────────────

pub async fn list_users(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN"], &request_id)?;

    let users = if state.repository.is_postgres() {
        state
            .repository
            .list_users_by_tenant(postgres_pool_or_none(&state), auth.tenant_id)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?
    } else {
        let users = state.users.lock().map_err(|_| {
            AppError::internal("用户状态锁异常").with_request_id(request_id.clone())
        })?;

        users
            .values()
            .filter(|user| user.tenant_id == auth.tenant_id)
            .cloned()
            .collect::<Vec<_>>()
    };

    // ADMIN 只能看到 rank 低于自己的用户（不展示其他 ADMIN 之上的 OWNER）
    // 为了方便人员管理，ADMIN 可以看到所有人，但操作权限在后续各接口中限制
    let mut list = users.iter().map(to_user_data).collect::<Vec<_>>();
    list.sort_by(|a, b| a.username.cmp(&b.username));
    let total = list.len() as u64;

    let body = ApiResponse::success(
        json!({
            "list": list,
            "total": total
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

// ── create_user ───────────────────────────────────────────────────────────────

pub async fn create_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    AppJson(req): AppJson<CreateUserRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN"], &request_id)?;

    let username = parse_required_text(&req.username, "username", &request_id)?;
    let name = parse_required_text(&req.name, "name", &request_id)?;
    let password = parse_required_text(&req.password, "password", &request_id)?;
    let role = parse_user_role_input(&req.role, &request_id)?;

    // 只有 OWNER 可以创建 ADMIN 或 OWNER 角色
    if !can_operator_manage_role(&auth.role, &role) {
        return Err(AppError::forbidden(format!(
            "当前角色「{}」无权创建「{}」角色的用户",
            auth.role,
            role.as_str()
        ))
        .with_request_id(request_id));
    }

    // OWNER 唯一性：不允许再创建另一个 OWNER
    if role == UserRole::Owner {
        let owner_count = count_owners_in_tenant(&state, auth.tenant_id, &request_id).await?;
        if owner_count > 0 {
            return Err(AppError::conflict(4091, "每个租户只能有一个老板（OWNER）")
                .with_request_id(request_id));
        }
    }

    let password_hash = hash_password(&password);
    let user = User {
        id: Uuid::new_v4(),
        tenant_id: auth.tenant_id,
        username: username.clone(),
        name,
        role,
        password_hash,
    };

    if state.repository.is_postgres() {
        state
            .repository
            .create_user(postgres_pool_or_none(&state), &user)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;
    } else {
        let mut users = state.users.lock().map_err(|_| {
            AppError::internal("用户状态锁异常").with_request_id(request_id.clone())
        })?;

        if users.contains_key(&username) {
            return Err(AppError::conflict(4090, "用户名已存在")
                .with_data(json!({ "username": username }))
                .with_request_id(request_id));
        }

        users.insert(username, user.clone());
    }

    let body = ApiResponse::success(to_user_data(&user), request_id.clone());

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

// ── update_user_role ──────────────────────────────────────────────────────────

pub async fn update_user_role(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    AppJson(req): AppJson<UpdateUserRoleRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN"], &request_id)?;

    let user_id = Uuid::parse_str(id.trim())
        .map_err(|_| AppError::bad_request("id 格式错误，必须为 UUID"))
        .map_err(|err| err.with_request_id(request_id.clone()))?;

    // 禁止修改自己的角色
    if auth.user_id == user_id {
        return Err(AppError::forbidden("不能修改自己的角色")
            .with_request_id(request_id));
    }

    let new_role = parse_user_role_input(&req.role, &request_id)?;

    // 查目标用户的当前角色，确认操作者有权管理目标用户
    let target_user = get_user_by_id(&state, auth.tenant_id, user_id, &request_id).await?;
    if !can_operator_manage_role(&auth.role, &target_user.role) {
        return Err(AppError::forbidden(format!(
            "无权修改「{}」角色用户的权限",
            target_user.role.as_str()
        ))
        .with_request_id(request_id));
    }

    // 操作者也必须有权分配新角色
    if !can_operator_assign_role(&auth.role, new_role.as_str()) {
        return Err(AppError::forbidden(format!(
            "当前角色「{}」无权将用户设置为「{}」",
            auth.role,
            new_role.as_str()
        ))
        .with_request_id(request_id));
    }

    // OWNER 唯一性：若新角色是 OWNER，拒绝
    if new_role == UserRole::Owner {
        return Err(AppError::conflict(4091, "不能通过修改角色创建新 OWNER，每个租户只能有一个老板")
            .with_request_id(request_id));
    }

    let updated_user = if state.repository.is_postgres() {
        state
            .repository
            .update_user_role(
                postgres_pool_or_none(&state),
                auth.tenant_id,
                user_id,
                new_role,
            )
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;

        state
            .repository
            .find_user_by_id(postgres_pool_or_none(&state), auth.tenant_id, user_id)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?
            .ok_or_else(|| AppError::not_found("员工不存在").with_request_id(request_id.clone()))?
    } else {
        let mut users = state.users.lock().map_err(|_| {
            AppError::internal("用户状态锁异常").with_request_id(request_id.clone())
        })?;

        let user = users
            .values_mut()
            .find(|user| user.tenant_id == auth.tenant_id && user.id == user_id)
            .ok_or_else(|| AppError::not_found("员工不存在").with_request_id(request_id.clone()))?;
        user.role = new_role;
        user.clone()
    };

    let body = ApiResponse::success(to_user_data(&updated_user), request_id.clone());

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}

// ── reset_user_password ───────────────────────────────────────────────────────

pub async fn reset_user_password(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    AppJson(req): AppJson<ResetUserPasswordRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN"], &request_id)?;

    let user_id = Uuid::parse_str(id.trim())
        .map_err(|_| AppError::bad_request("id 格式错误，必须为 UUID"))
        .map_err(|err| err.with_request_id(request_id.clone()))?;
    let new_password = parse_required_text(&req.new_password, "new_password", &request_id)?;
    let new_password_hash = hash_password(&new_password);

    // 检查操作者是否有权管理目标用户
    let target_user = get_user_by_id(&state, auth.tenant_id, user_id, &request_id).await?;
    if !can_operator_manage_role(&auth.role, &target_user.role) {
        return Err(AppError::forbidden(format!(
            "无权重置「{}」角色用户的密码",
            target_user.role.as_str()
        ))
        .with_request_id(request_id));
    }

    if state.repository.is_postgres() {
        state
            .repository
            .reset_password(
                postgres_pool_or_none(&state),
                auth.tenant_id,
                user_id,
                &new_password_hash,
            )
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;
    } else {
        let mut users = state.users.lock().map_err(|_| {
            AppError::internal("用户状态锁异常").with_request_id(request_id.clone())
        })?;

        let user = users
            .values_mut()
            .find(|user| user.tenant_id == auth.tenant_id && user.id == user_id)
            .ok_or_else(|| AppError::not_found("员工不存在").with_request_id(request_id.clone()))?;
        user.password_hash = new_password_hash;
    }

    let body = ApiResponse::success(
        json!({
            "id": user_id,
            "reset": true
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

// ── 内部辅助函数 ──────────────────────────────────────────────────────────────

async fn get_user_by_id(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    request_id: &str,
) -> Result<User, AppError> {
    if state.repository.is_postgres() {
        state
            .repository
            .find_user_by_id(postgres_pool_or_none(state), tenant_id, user_id)
            .await
            .map_err(|err| err.with_request_id(request_id.to_string()))?
            .ok_or_else(|| AppError::not_found("员工不存在").with_request_id(request_id.to_string()))
    } else {
        let users = state.users.lock().map_err(|_| {
            AppError::internal("用户状态锁异常").with_request_id(request_id.to_string())
        })?;
        users
            .values()
            .find(|u| u.tenant_id == tenant_id && u.id == user_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("员工不存在").with_request_id(request_id.to_string()))
    }
}

async fn count_owners_in_tenant(
    state: &AppState,
    tenant_id: Uuid,
    request_id: &str,
) -> Result<usize, AppError> {
    if state.repository.is_postgres() {
        let users = state
            .repository
            .list_users_by_tenant(postgres_pool_or_none(state), tenant_id)
            .await
            .map_err(|err| err.with_request_id(request_id.to_string()))?;
        Ok(users.iter().filter(|u| u.role == UserRole::Owner).count())
    } else {
        let users = state.users.lock().map_err(|_| {
            AppError::internal("用户状态锁异常").with_request_id(request_id.to_string())
        })?;
        Ok(users
            .values()
            .filter(|u| u.tenant_id == tenant_id && u.role == UserRole::Owner)
            .count())
    }
}

// ── delete_user ───────────────────────────────────────────────────────────────

pub async fn delete_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER", "ADMIN"], &request_id)?;

    let user_id = Uuid::parse_str(id.trim())
        .map_err(|_| AppError::bad_request("id 格式错误，必须为 UUID"))
        .map_err(|err| err.with_request_id(request_id.clone()))?;

    // 不能删除自己
    if auth.user_id == user_id {
        return Err(AppError::forbidden("不能删除自己")
            .with_request_id(request_id));
    }

    // 检查操作者是否有权管理目标用户
    let target_user = get_user_by_id(&state, auth.tenant_id, user_id, &request_id).await?;
    if !can_operator_manage_role(&auth.role, &target_user.role) {
        return Err(AppError::forbidden(format!(
            "无权删除「{}」角色的用户",
            target_user.role.as_str()
        ))
        .with_request_id(request_id));
    }

    state
        .repository
        .delete_user(postgres_pool_or_none(&state), auth.tenant_id, user_id)
        .await
        .map_err(|err| err.with_request_id(request_id.clone()))?;

    let body = ApiResponse::success(
        json!({ "id": user_id, "deleted": true }),
        request_id.clone(),
    );

    Ok((
        StatusCode::OK,
        build_response_headers(&request_id, false),
        Json(body),
    )
        .into_response())
}
