use reqwest::Client;
use serde::Serialize;

use crate::error::{AppError, BusinessError};

#[derive(Debug, Serialize)]
pub struct GenerateRequest {
    pub task_id: i32,
    pub prompt: String,
}

pub async fn dispatch_to_service(
    client: &Client,
    service_url: &str,
    api_key: &str,
    request: &GenerateRequest,
) -> Result<(), AppError> {
    let response = client
        .post(format!("{service_url}/generate"))
        .header("Authorization", format!("Bearer {api_key}"))
        .json(request)
        .send()
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to reach generation service");
            AppError::business(BusinessError::ServiceUnavailable)
        })?;

    if !response.status().is_success() {
        tracing::error!(
            status = %response.status(),
            "generation service returned error"
        );
        return Err(AppError::business(BusinessError::ServiceUnavailable));
    }

    Ok(())
}
