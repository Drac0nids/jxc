use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::routes::common::postgres_pool_or_none;
use crate::{
    error::AppError,
    extractors::AppJson,
    middleware::generate_token,
    models::{Tenant, User, UserRole, hash_password},
    response::{ApiResponse, build_response_headers, resolve_request_id},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub tenant_name: Option<String>,
    pub username: String,
    pub name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    /// 租户码，登录时传入以实现租户隔离（首次登录后客户端记住，下次自动填写）
    pub tenant_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

pub async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    AppJson(req): AppJson<RegisterRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);

    let username = req.username.trim();
    let name = req.name.trim();
    let password = req.password.trim();
    let tenant_name = req
        .tenant_name
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned);

    if username.is_empty() || name.is_empty() || password.is_empty() {
        return Err(AppError::bad_request("用户名、姓名和密码不能为空").with_request_id(request_id));
    }

    let user = User {
        id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        username: username.to_string(),
        name: name.to_string(),
        role: UserRole::Owner,
        password_hash: hash_password(password),
    };

    // 生成6位大写字母+数字租户码，确保不重复时写入（极低概率冲突，生产可加重试）
    let tenant_code = generate_tenant_code();
    let tenant = Tenant {
        id: user.tenant_id,
        code: tenant_code.clone(),
        name: tenant_name.clone().unwrap_or_default(),
    };

    if state.repository.is_postgres() {
        state
            .repository
            .create_tenant(postgres_pool_or_none(&state), &tenant)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;
        state
            .repository
            .create_user(postgres_pool_or_none(&state), &user)
            .await
            .map_err(|err| err.with_request_id(request_id.clone()))?;
    } else {
        let mut users = state.users.lock().map_err(|_| {
            AppError::internal("用户状态锁异常").with_request_id(request_id.clone())
        })?;

        if users.contains_key(username) {
            return Err(AppError::conflict(4090, "用户名已存在")
                .with_data(json!({ "username": username }))
                .with_request_id(request_id));
        }

        users.insert(username.to_string(), user.clone());
    }

    let access_token = generate_token(
        &state.config.jwt_secret,
        user.tenant_id,
        user.id,
        user.role.as_str(),
        "access",
        state.config.access_token_exp_secs,
    )
    .map_err(|e| e.with_request_id(request_id.clone()))?;

    let refresh_token = generate_token(
        &state.config.jwt_secret,
        user.tenant_id,
        user.id,
        user.role.as_str(),
        "refresh",
        state.config.refresh_token_exp_secs,
    )
    .map_err(|e| e.with_request_id(request_id.clone()))?;

    let body = ApiResponse::success(
        json!({
            "registered": true,
            "tenant_code": tenant_code,
            "tenant_name": tenant_name,
            "access_token": access_token,
            "refresh_token": refresh_token,
            "expires_in": state.config.access_token_exp_secs,
            "tenant_id": user.tenant_id,
            "user_info": {
                "id": user.id,
                "name": user.name,
                "role": user.role.as_str()
            }
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

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    AppJson(req): AppJson<LoginRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);

    if req.username.trim().is_empty() || req.password.is_empty() {
        return Err(AppError::bad_request("用户名或密码不能为空").with_request_id(request_id));
    }

    let username = req.username.trim();
    let user = if state.repository.is_postgres() {
        // PostgreSQL 模式强制要求租户码，确保多租户严格隔离
        let code = req
            .tenant_code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                AppError::bad_request("请输入租户码").with_request_id(request_id.clone())
            })?;

        state
            .repository
            .find_user_by_tenant_code_and_username(
                postgres_pool_or_none(&state),
                code,
                username,
            )
            .await?
            .ok_or_else(|| {
                AppError::unauthorized("租户码或用户名/密码错误")
                    .with_request_id(request_id.clone())
            })?
    } else {
        // 内存模式（本地演示）无租户概念，直接按用户名查
        let users = state.users.lock().map_err(|_| {
            AppError::internal("用户状态锁异常").with_request_id(request_id.clone())
        })?;

        users.get(username).cloned().ok_or_else(|| {
            AppError::unauthorized("用户名或密码错误").with_request_id(request_id.clone())
        })?
    };

    if hash_password(&req.password) != user.password_hash {
        return Err(AppError::unauthorized("用户名或密码错误").with_request_id(request_id));
    }

    let access_token = generate_token(
        &state.config.jwt_secret,
        user.tenant_id,
        user.id,
        user.role.as_str(),
        "access",
        state.config.access_token_exp_secs,
    )
    .map_err(|e| e.with_request_id(request_id.clone()))?;

    let refresh_token = generate_token(
        &state.config.jwt_secret,
        user.tenant_id,
        user.id,
        user.role.as_str(),
        "refresh",
        state.config.refresh_token_exp_secs,
    )
    .map_err(|e| e.with_request_id(request_id.clone()))?;

    let body = ApiResponse::success(
        json!({
            "access_token": access_token,
            "refresh_token": refresh_token,
            "expires_in": state.config.access_token_exp_secs,
            "tenant_id": user.tenant_id,
            // 内存模式无 tenants 表，返回空；PostgreSQL 模式通过 tenant_code 查到后原样返回
            "tenant_code": req.tenant_code.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            "user_info": {
                "id": user.id,
                "name": user.name,
                "role": user.role.as_str()
            }
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

pub async fn refresh_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    AppJson(req): AppJson<RefreshTokenRequest>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);

    if req.refresh_token.trim().is_empty() {
        return Err(AppError::bad_request("refresh_token 不能为空").with_request_id(request_id));
    }

    let claims =
        crate::middleware::verify_token(&state.config.jwt_secret, req.refresh_token.trim())
            .map_err(|e| e.with_request_id(request_id.clone()))?;

    if claims.token_type != "refresh" {
        return Err(AppError::unauthorized("token_type 非 refresh").with_request_id(request_id));
    }

    let user = if state.repository.is_postgres() {
        state
            .repository
            .find_user_by_id(
                postgres_pool_or_none(&state),
                claims.tenant_id,
                claims.user_id,
            )
            .await?
            .ok_or_else(|| {
                AppError::unauthorized("用户不存在或无效").with_request_id(request_id.clone())
            })?
    } else {
        let users = state.users.lock().map_err(|_| {
            AppError::internal("用户状态锁异常").with_request_id(request_id.clone())
        })?;

        users
            .values()
            .find(|u| u.id == claims.user_id && u.tenant_id == claims.tenant_id)
            .cloned()
            .ok_or_else(|| {
                AppError::unauthorized("用户不存在或无效").with_request_id(request_id.clone())
            })?
    };

    let access_token = generate_token(
        &state.config.jwt_secret,
        user.tenant_id,
        user.id,
        user.role.as_str(),
        "access",
        state.config.access_token_exp_secs,
    )
    .map_err(|e| e.with_request_id(request_id.clone()))?;

    let body = ApiResponse::success(
        json!({
            "access_token": access_token,
            "expires_in": state.config.access_token_exp_secs,
            "tenant_id": user.tenant_id,
            "user_info": {
                "id": user.id,
                "name": user.name,
                "role": user.role.as_str()
            }
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

pub async fn logout(
    headers: HeaderMap,
    axum::extract::Extension(auth): axum::extract::Extension<crate::middleware::AuthContext>,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(&headers);
    let body = ApiResponse::success(
        json!({
            "logged_out": true,
            "user_id": auth.user_id
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

/// 生成6位大写字母+数字租户码，使用新 UUID 字节作为随机来源
/// 示例：ABC123、X7K9MQ
fn generate_tenant_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let bytes = Uuid::new_v4().into_bytes();
    bytes[..6]
        .iter()
        .map(|&b| CHARSET[(b as usize) % CHARSET.len()] as char)
        .collect()
}
