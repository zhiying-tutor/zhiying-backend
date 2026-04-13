use axum::{Router, routing::get};

use crate::{
    error::{AppError, BusinessError},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/my-contents", get(not_implemented))
        .route("/public-contents", get(not_implemented))
}

async fn not_implemented() -> Result<(), AppError> {
    Err(AppError::business(BusinessError::FeatureNotImplemented))
}
