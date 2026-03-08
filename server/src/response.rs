use axum::http::{HeaderMap, HeaderValue};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

pub const X_REQUEST_ID_HEADER: &str = "x-request-id";
pub const X_IDEMPOTENT_REPLAY_HEADER: &str = "x-idempotent-replay";

pub fn resolve_request_id(headers: &HeaderMap) -> String {
    headers
        .get(X_REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(generate_request_id)
}

pub fn build_response_headers(request_id: &str, is_idempotent_replay: bool) -> HeaderMap {
    let mut headers = HeaderMap::new();

    if let Ok(v) = HeaderValue::from_str(request_id) {
        headers.insert(X_REQUEST_ID_HEADER, v);
    }

    if is_idempotent_replay {
        headers.insert(X_IDEMPOTENT_REPLAY_HEADER, HeaderValue::from_static("true"));
    }

    headers
}

pub fn generate_request_id() -> String {
    format!("req_{}", Uuid::new_v4().simple())
}

#[derive(Debug, Serialize, Clone)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub code: i32,
    pub message: String,
    pub data: T,
    pub request_id: String,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn success(data: T, request_id: impl Into<String>) -> Self {
        Self {
            code: 200,
            message: "success".to_string(),
            data,
            request_id: request_id.into(),
        }
    }
}

impl ApiResponse<Value> {
    pub fn business_error(
        code: i32,
        message: impl Into<String>,
        data: Value,
        request_id: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            data,
            request_id: request_id.into(),
        }
    }
}
