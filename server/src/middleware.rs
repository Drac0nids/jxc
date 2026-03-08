use axum::{
    extract::Request,
    extract::State,
    http::{HeaderMap, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::AppError, response::resolve_request_id, state::AppState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub exp: i64,
    pub token_type: String,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
}

pub fn generate_token(
    secret: &str,
    tenant_id: Uuid,
    user_id: Uuid,
    role: &str,
    token_type: &str,
    expires_in_secs: i64,
) -> Result<String, AppError> {
    let claims = JwtClaims {
        sub: user_id.to_string(),
        tenant_id,
        user_id,
        role: role.to_string(),
        exp: (Utc::now() + Duration::seconds(expires_in_secs)).timestamp(),
        token_type: token_type.to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::internal(format!("generate jwt failed: {e}")))
}

pub fn verify_token(secret: &str, token: &str) -> Result<JwtClaims, AppError> {
    let mut validation = Validation::default();
    validation.leeway = 0;

    decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|d| d.claims)
    .map_err(|_| AppError::unauthorized("token 无效或已过期"))
}

pub fn extract_bearer_token(headers: &HeaderMap) -> Result<String, AppError> {
    let value = headers
        .get(AUTHORIZATION)
        .ok_or_else(|| AppError::unauthorized("缺少 Authorization"))?
        .to_str()
        .map_err(|_| AppError::unauthorized("Authorization 非法"))?;

    if let Some(token) = value.strip_prefix("Bearer ") {
        Ok(token.to_string())
    } else {
        Err(AppError::unauthorized("Authorization 格式错误"))
    }
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let request_id = resolve_request_id(req.headers());

    let token =
        extract_bearer_token(req.headers()).map_err(|e| e.with_request_id(request_id.clone()))?;
    let claims = verify_token(&state.config.jwt_secret, &token)
        .map_err(|e| e.with_request_id(request_id.clone()))?;

    if claims.token_type != "access" {
        return Err(AppError::unauthorized("token_type 非 access").with_request_id(request_id));
    }

    let auth_context = if state.repository.is_postgres() {
        let user = state
            .repository
            .find_user_by_id(
                state.persistence.postgres.as_ref(),
                claims.tenant_id,
                claims.user_id,
            )
            .await?
            .ok_or_else(|| {
                AppError::unauthorized("用户不存在或无效").with_request_id(request_id.clone())
            })?;

        AuthContext {
            tenant_id: user.tenant_id,
            user_id: user.id,
            role: user.role.as_str().to_string(),
        }
    } else {
        let users = state.users.lock().map_err(|_| {
            AppError::internal("用户状态锁异常").with_request_id(request_id.clone())
        })?;

        let user = users
            .values()
            .find(|u| u.id == claims.user_id && u.tenant_id == claims.tenant_id)
            .cloned()
            .ok_or_else(|| {
                AppError::unauthorized("用户不存在或无效").with_request_id(request_id.clone())
            })?;

        AuthContext {
            tenant_id: user.tenant_id,
            user_id: user.id,
            role: user.role.as_str().to_string(),
        }
    };

    req.extensions_mut().insert(auth_context);

    Ok(next.run(req).await)
}
