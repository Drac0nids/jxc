use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;

use crate::response::{ApiResponse, build_response_headers, generate_request_id};

#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub code: i32,
    pub message: String,
    pub data: serde_json::Value,
    pub request_id: Option<String>,
}

impl AppError {
    fn new(status: StatusCode, code: i32, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            data: json!({}),
            request_id: None,
        }
    }

    pub fn business(status: StatusCode, code: i32, message: impl Into<String>) -> Self {
        Self::new(status, code, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, 4010, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, 4030, message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, 4000, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, 4040, message)
    }

    pub fn conflict(code: i32, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, 5000, message)
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let request_id = self.request_id.unwrap_or_else(generate_request_id);

        let body =
            ApiResponse::business_error(self.code, self.message, self.data, request_id.clone());
        let headers = build_response_headers(&request_id, false);

        (self.status, headers, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::internal(format!("数据库错误: {err}"))
    }
}
