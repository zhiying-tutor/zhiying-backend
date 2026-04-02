use axum::{
    Json, Router,
    extract::{Path, State},
    routing::patch,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, TransactionTrait};
use serde::Deserialize;

use crate::{
    auth::{ServiceAuth, ServiceKind},
    entities::{code_video, interactive_html, knowledge_explanation, knowledge_video, user},
    error::{AppError, BusinessError},
    response::ok,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/knowledge-videos/{id}", patch(update_knowledge_video))
        .route("/code-videos/{id}", patch(update_code_video))
        .route("/interactive-htmls/{id}", patch(update_interactive_html))
        .route(
            "/knowledge-explanations/{id}",
            patch(update_knowledge_explanation),
        )
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
    pub url: Option<String>,
    pub content: Option<String>,
    pub mindmap: Option<String>,
}

// --- knowledge_video ---

async fn update_knowledge_video(
    State(state): State<AppState>,
    service_auth: ServiceAuth,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if service_auth.service != ServiceKind::KnowledgeVideo {
        return Err(AppError::business(BusinessError::InvalidApiKey));
    }

    let tx = state.db.begin().await?;
    let record = knowledge_video::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    let new_status = parse_knowledge_video_status(&payload.status)?;
    validate_knowledge_video_transition(record.status, new_status)?;

    let mut active: knowledge_video::ActiveModel = record.clone().into();
    active.status = Set(new_status);
    active.updated_at = Set(Utc::now());

    if new_status == knowledge_video::KnowledgeVideoStatus::Finished {
        active.url = Set(payload.url);
    }

    if new_status == knowledge_video::KnowledgeVideoStatus::Failed {
        let cost = state.config.knowledge_video_diamond_cost;
        let existing_user = user::Entity::find_by_id(record.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.diamond = Set(active_user.diamond.unwrap() + cost);
        active_user.updated_at = Set(Utc::now());
        active_user.update(&tx).await?;
    }

    active.update(&tx).await?;
    tx.commit().await?;

    Ok(ok(serde_json::json!({"id": id, "status": payload.status})))
}

fn parse_knowledge_video_status(
    s: &str,
) -> Result<knowledge_video::KnowledgeVideoStatus, AppError> {
    match s {
        "QUEUING" => Ok(knowledge_video::KnowledgeVideoStatus::Queuing),
        "GENERATING" => Ok(knowledge_video::KnowledgeVideoStatus::Generating),
        "FINISHED" => Ok(knowledge_video::KnowledgeVideoStatus::Finished),
        "FAILED" => Ok(knowledge_video::KnowledgeVideoStatus::Failed),
        _ => Err(AppError::business(BusinessError::InvalidContentStatus)),
    }
}

fn validate_knowledge_video_transition(
    from: knowledge_video::KnowledgeVideoStatus,
    to: knowledge_video::KnowledgeVideoStatus,
) -> Result<(), AppError> {
    use knowledge_video::KnowledgeVideoStatus::*;
    let valid = matches!(
        (from, to),
        (Queuing, Generating) | (Generating, Finished) | (Generating, Failed)
    );
    if !valid {
        return Err(AppError::business(BusinessError::InvalidContentStatus));
    }
    Ok(())
}

// --- code_video ---

async fn update_code_video(
    State(state): State<AppState>,
    service_auth: ServiceAuth,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if service_auth.service != ServiceKind::CodeVideo {
        return Err(AppError::business(BusinessError::InvalidApiKey));
    }

    let tx = state.db.begin().await?;
    let record = code_video::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    let new_status = parse_code_video_status(&payload.status)?;
    validate_code_video_transition(record.status, new_status)?;

    let mut active: code_video::ActiveModel = record.clone().into();
    active.status = Set(new_status);
    active.updated_at = Set(Utc::now());

    if new_status == code_video::CodeVideoStatus::Finished {
        active.url = Set(payload.url);
    }

    if new_status == code_video::CodeVideoStatus::Failed {
        let cost = state.config.code_video_diamond_cost;
        let existing_user = user::Entity::find_by_id(record.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.diamond = Set(active_user.diamond.unwrap() + cost);
        active_user.updated_at = Set(Utc::now());
        active_user.update(&tx).await?;
    }

    active.update(&tx).await?;
    tx.commit().await?;

    Ok(ok(serde_json::json!({"id": id, "status": payload.status})))
}

fn parse_code_video_status(s: &str) -> Result<code_video::CodeVideoStatus, AppError> {
    match s {
        "QUEUING" => Ok(code_video::CodeVideoStatus::Queuing),
        "GENERATING" => Ok(code_video::CodeVideoStatus::Generating),
        "FINISHED" => Ok(code_video::CodeVideoStatus::Finished),
        "FAILED" => Ok(code_video::CodeVideoStatus::Failed),
        _ => Err(AppError::business(BusinessError::InvalidContentStatus)),
    }
}

fn validate_code_video_transition(
    from: code_video::CodeVideoStatus,
    to: code_video::CodeVideoStatus,
) -> Result<(), AppError> {
    use code_video::CodeVideoStatus::*;
    let valid = matches!(
        (from, to),
        (Queuing, Generating) | (Generating, Finished) | (Generating, Failed)
    );
    if !valid {
        return Err(AppError::business(BusinessError::InvalidContentStatus));
    }
    Ok(())
}

// --- interactive_html ---

async fn update_interactive_html(
    State(state): State<AppState>,
    service_auth: ServiceAuth,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if service_auth.service != ServiceKind::InteractiveHtml {
        return Err(AppError::business(BusinessError::InvalidApiKey));
    }

    let tx = state.db.begin().await?;
    let record = interactive_html::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    let new_status = parse_interactive_html_status(&payload.status)?;
    validate_interactive_html_transition(record.status, new_status)?;

    let mut active: interactive_html::ActiveModel = record.clone().into();
    active.status = Set(new_status);
    active.updated_at = Set(Utc::now());

    if new_status == interactive_html::InteractiveHtmlStatus::Finished {
        active.url = Set(payload.url);
    }

    if new_status == interactive_html::InteractiveHtmlStatus::Failed {
        let cost = state.config.interactive_html_gold_cost;
        let existing_user = user::Entity::find_by_id(record.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.gold = Set(active_user.gold.unwrap() + cost);
        active_user.updated_at = Set(Utc::now());
        active_user.update(&tx).await?;
    }

    active.update(&tx).await?;
    tx.commit().await?;

    Ok(ok(serde_json::json!({"id": id, "status": payload.status})))
}

fn parse_interactive_html_status(
    s: &str,
) -> Result<interactive_html::InteractiveHtmlStatus, AppError> {
    match s {
        "QUEUING" => Ok(interactive_html::InteractiveHtmlStatus::Queuing),
        "GENERATING" => Ok(interactive_html::InteractiveHtmlStatus::Generating),
        "FINISHED" => Ok(interactive_html::InteractiveHtmlStatus::Finished),
        "FAILED" => Ok(interactive_html::InteractiveHtmlStatus::Failed),
        _ => Err(AppError::business(BusinessError::InvalidContentStatus)),
    }
}

fn validate_interactive_html_transition(
    from: interactive_html::InteractiveHtmlStatus,
    to: interactive_html::InteractiveHtmlStatus,
) -> Result<(), AppError> {
    use interactive_html::InteractiveHtmlStatus::*;
    let valid = matches!(
        (from, to),
        (Queuing, Generating) | (Generating, Finished) | (Generating, Failed)
    );
    if !valid {
        return Err(AppError::business(BusinessError::InvalidContentStatus));
    }
    Ok(())
}

// --- knowledge_explanation ---

async fn update_knowledge_explanation(
    State(state): State<AppState>,
    service_auth: ServiceAuth,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if service_auth.service != ServiceKind::KnowledgeExplanation {
        return Err(AppError::business(BusinessError::InvalidApiKey));
    }

    let tx = state.db.begin().await?;
    let record = knowledge_explanation::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    let new_status = parse_knowledge_explanation_status(&payload.status)?;
    validate_knowledge_explanation_transition(record.status, new_status)?;

    let mut active: knowledge_explanation::ActiveModel = record.clone().into();
    active.status = Set(new_status);
    active.updated_at = Set(Utc::now());

    if new_status == knowledge_explanation::KnowledgeExplanationStatus::Finished {
        active.content = Set(payload.content);
        active.mindmap = Set(payload.mindmap);
    }

    if new_status == knowledge_explanation::KnowledgeExplanationStatus::Failed {
        let cost = state.config.knowledge_explanation_gold_cost;
        let existing_user = user::Entity::find_by_id(record.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.gold = Set(active_user.gold.unwrap() + cost);
        active_user.updated_at = Set(Utc::now());
        active_user.update(&tx).await?;
    }

    active.update(&tx).await?;
    tx.commit().await?;

    Ok(ok(serde_json::json!({"id": id, "status": payload.status})))
}

fn parse_knowledge_explanation_status(
    s: &str,
) -> Result<knowledge_explanation::KnowledgeExplanationStatus, AppError> {
    match s {
        "QUEUING" => Ok(knowledge_explanation::KnowledgeExplanationStatus::Queuing),
        "GENERATING" => Ok(knowledge_explanation::KnowledgeExplanationStatus::Generating),
        "FINISHED" => Ok(knowledge_explanation::KnowledgeExplanationStatus::Finished),
        "FAILED" => Ok(knowledge_explanation::KnowledgeExplanationStatus::Failed),
        _ => Err(AppError::business(BusinessError::InvalidContentStatus)),
    }
}

fn validate_knowledge_explanation_transition(
    from: knowledge_explanation::KnowledgeExplanationStatus,
    to: knowledge_explanation::KnowledgeExplanationStatus,
) -> Result<(), AppError> {
    use knowledge_explanation::KnowledgeExplanationStatus::*;
    let valid = matches!(
        (from, to),
        (Queuing, Generating) | (Generating, Finished) | (Generating, Failed)
    );
    if !valid {
        return Err(AppError::business(BusinessError::InvalidContentStatus));
    }
    Ok(())
}
