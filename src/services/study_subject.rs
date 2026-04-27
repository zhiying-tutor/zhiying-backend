use serde::Serialize;

use crate::error::AppError;
use crate::services::message_queue::{MessagePublisher, ROUTING_KEY_GENERATE};

#[derive(Debug, Serialize)]
pub struct PretestRequest {
    pub task_id: i32,
    pub prompt: String,
    pub total_stages: i32,
    pub language: String,
    pub target: String,
}

#[derive(Debug, Serialize)]
pub struct PlanRequest {
    pub task_id: i32,
    pub prompt: String,
    pub total_stages: i32,
    pub language: String,
    pub target: String,
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
    publisher: &dyn MessagePublisher,
    exchange: &str,
    body: &impl Serialize,
) -> Result<(), AppError> {
    let payload = serde_json::to_vec(body).map_err(|err| {
        tracing::error!(error = %err, "failed to serialize microservice request");
        AppError::internal("failed to serialize microservice request")
    })?;
    publisher
        .publish(exchange, ROUTING_KEY_GENERATE, &payload)
        .await
}

pub async fn dispatch_pretest(
    publisher: &dyn MessagePublisher,
    exchange: &str,
    request: &PretestRequest,
) -> Result<(), AppError> {
    dispatch(publisher, exchange, request).await
}

pub async fn dispatch_plan(
    publisher: &dyn MessagePublisher,
    exchange: &str,
    request: &PlanRequest,
) -> Result<(), AppError> {
    dispatch(publisher, exchange, request).await
}

pub async fn dispatch_quiz(
    publisher: &dyn MessagePublisher,
    exchange: &str,
    request: &QuizRequest,
) -> Result<(), AppError> {
    dispatch(publisher, exchange, request).await
}
