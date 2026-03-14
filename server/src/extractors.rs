use axum::{
    Json,
    async_trait,
    extract::{FromRequest, Request, rejection::JsonRejection},
};
use serde::de::DeserializeOwned;

use crate::{error::AppError, response::resolve_request_id};

pub struct AppJson<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for AppJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let request_id = resolve_request_id(req.headers());

        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) => Err(map_json_rejection(rejection, &request_id)),
        }
    }
}

fn map_json_rejection(rejection: JsonRejection, request_id: &str) -> AppError {
    AppError::business(
        rejection.status(),
        4000,
        format!("请求体解析失败：{}", rejection.body_text()),
    )
    .with_request_id(request_id.to_string())
}
