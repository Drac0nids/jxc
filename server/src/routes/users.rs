use axum::{
    Json,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::AppError,
    middleware::{AuthContext},
    models::{User, hash_password},
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};
use crate::routes::common::{
    postgres_pool_or_none, ensure_role, parse_required_text, parse_user_role_input, to_user_data,
    CreateUserRequest, ResetUserPasswordRequest, UpdateUserRoleRequest
};

pub async fn list_users(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER"], &request_id)?;

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

pub async fn create_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Json(req): Json<CreateUserRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER"], &request_id)?;

    let username = parse_required_text(&req.username, "username", &request_id)?;
    let name = parse_required_text(&req.name, "name", &request_id)?;
    let password = parse_required_text(&req.password, "password", &request_id)?;
    let role = parse_user_role_input(&req.role, &request_id)?;

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

pub async fn update_user_role(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRoleRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER"], &request_id)?;

    let user_id = Uuid::parse_str(id.trim())
        .map_err(|_| AppError::bad_request("id 格式错误，必须为 UUID"))
        .map_err(|err| err.with_request_id(request_id.clone()))?;
    let new_role = parse_user_role_input(&req.role, &request_id)?;

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

pub async fn reset_user_password(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ResetUserPasswordRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    ensure_role(&auth.role, &["OWNER"], &request_id)?;

    let user_id = Uuid::parse_str(id.trim())
        .map_err(|_| AppError::bad_request("id 格式错误，必须为 UUID"))
        .map_err(|err| err.with_request_id(request_id.clone()))?;
    let new_password = parse_required_text(&req.new_password, "new_password", &request_id)?;
    let new_password_hash = hash_password(&new_password);

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
