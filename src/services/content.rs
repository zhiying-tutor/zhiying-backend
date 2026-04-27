use serde::Serialize;

use crate::error::AppError;
use crate::services::message_queue::{MessagePublisher, ROUTING_KEY_GENERATE};

#[derive(Debug, Serialize)]
pub struct GenerateRequest {
    pub task_id: i32,
    pub prompt: String,
}

pub async fn dispatch_to_service(
    publisher: &dyn MessagePublisher,
    exchange: &str,
    request: &GenerateRequest,
) -> Result<(), AppError> {
    let payload = serde_json::to_vec(request).map_err(|err| {
        tracing::error!(error = %err, "failed to serialize generate request");
        AppError::internal("failed to serialize generate request")
    })?;
    publisher
        .publish(exchange, ROUTING_KEY_GENERATE, &payload)
        .await
}
