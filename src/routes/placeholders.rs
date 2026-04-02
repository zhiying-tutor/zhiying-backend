use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::{
    error::{AppError, BusinessError},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/study-plans", post(not_implemented).get(not_implemented))
        .route(
            "/study-plans/{id}",
            get(not_implemented).delete(not_implemented),
        )
        .route("/study-stages/{id}", get(not_implemented))
        .route("/study-tasks/{id}", get(not_implemented))
        .route("/study-tasks/{id}/complete", post(not_implemented))
        .route(
            "/study-tasks/{task_id}/problems/{problem_id}/submit",
            post(not_implemented),
        )
        .route("/problems", get(not_implemented))
        .route("/problems/{id}/bookmark", patch(not_implemented))
        .route("/study-plans/{id}/pretest", get(not_implemented))
        .route(
            "/study-plans/{plan_id}/pretest/{pretest_id}/submit",
            post(not_implemented),
        )
        .route("/my-contents", get(not_implemented))
        .route("/public-contents", get(not_implemented))
}

async fn not_implemented() -> Result<(), AppError> {
    Err(AppError::business(BusinessError::FeatureNotImplemented))
}
