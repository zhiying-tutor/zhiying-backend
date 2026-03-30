use axum::{Json, http::StatusCode};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiSuccess<T> {
    pub success: bool,
    pub data: T,
}

pub fn ok<T>(data: T) -> (StatusCode, Json<ApiSuccess<T>>)
where
    T: Serialize,
{
    (
        StatusCode::OK,
        Json(ApiSuccess {
            success: true,
            data,
        }),
    )
}

pub fn created<T>(data: T) -> (StatusCode, Json<ApiSuccess<T>>)
where
    T: Serialize,
{
    (
        StatusCode::CREATED,
        Json(ApiSuccess {
            success: true,
            data,
        }),
    )
}
