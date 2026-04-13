use reqwest::Client;
use serde::Serialize;

use crate::error::{AppError, BusinessError};

#[derive(Debug, Serialize)]
pub struct PretestRequest {
    pub task_id: i32,
    pub prompt: String,
}

#[derive(Debug, Serialize)]
pub struct PlanRequest {
    pub task_id: i32,
    pub prompt: String,
    pub pretest_results: Vec<PretestResult>,
}

#[derive(Debug, Serialize)]
pub struct PretestResult {
    pub problem_id: i32,
    pub content: String,
    pub choice_a: String,
    pub choice_b: String,
    pub choice_c: String,
    pub choice_d: String,
    pub answer: String,
    pub chosen_answer: Option<String>,
    pub confidence: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QuizRequest {
    pub task_id: i32,
    pub prompt: String,
}

async fn dispatch(
    client: &Client,
    url: &str,
    api_key: &str,
    body: &impl Serialize,
) -> Result<(), AppError> {
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .json(body)
        .send()
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to reach microservice");
            AppError::business(BusinessError::ServiceUnavailable)
        })?;

    if !response.status().is_success() {
        tracing::error!(status = %response.status(), "microservice returned error");
        return Err(AppError::business(BusinessError::ServiceUnavailable));
    }

    Ok(())
}

pub async fn dispatch_pretest(
    client: &Client,
    service_url: &str,
    api_key: &str,
    request: &PretestRequest,
) -> Result<(), AppError> {
    dispatch(client, service_url, api_key, request).await
}

pub async fn dispatch_plan(
    client: &Client,
    service_url: &str,
    api_key: &str,
    request: &PlanRequest,
) -> Result<(), AppError> {
    dispatch(client, service_url, api_key, request).await
}

pub async fn dispatch_quiz(
    client: &Client,
    service_url: &str,
    api_key: &str,
    request: &QuizRequest,
) -> Result<(), AppError> {
    dispatch(client, service_url, api_key, request).await
}
