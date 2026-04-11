use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    entities::{knowledge_explanation, user},
    error::{AppError, BusinessError},
    response::{created, ok},
    services::content::{GenerateRequest, dispatch_to_service},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    pub prompt: String,
    #[serde(default)]
    pub public: bool,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeExplanationView {
    pub id: i32,
    pub status: knowledge_explanation::KnowledgeExplanationStatus,
    pub prompt: String,
    pub content: Option<String>,
    pub mindmap: Option<serde_json::Value>,
    pub public: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<knowledge_explanation::Model> for KnowledgeExplanationView {
    fn from(m: knowledge_explanation::Model) -> Self {
        Self {
            id: m.id,
            status: m.status,
            prompt: m.prompt,
            content: m.content,
            mindmap: m.mindmap.and_then(|s| serde_json::from_str(&s).ok()),
            public: m.public,
            created_at: m.created_at.timestamp_millis(),
            updated_at: m.updated_at.timestamp_millis(),
        }
    }
}

pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let now = Utc::now();
    let cost = state.config.knowledge_explanation_gold_cost;
    let tx = state.db.begin().await?;

    let existing_user = user::Entity::find_by_id(auth_user.user_id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;

    if existing_user.gold < cost {
        return Err(AppError::business(BusinessError::InsufficientGold));
    }

    let mut active_user: user::ActiveModel = existing_user.into();
    active_user.gold = Set(active_user.gold.unwrap() - cost);
    active_user.updated_at = Set(now);
    active_user.update(&tx).await?;

    let record = knowledge_explanation::ActiveModel {
        user_id: Set(auth_user.user_id),
        status: Set(knowledge_explanation::KnowledgeExplanationStatus::Queuing),
        prompt: Set(payload.prompt.clone()),
        content: Set(None),
        mindmap: Set(None),
        public: Set(payload.public),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await?;

    tx.commit().await?;

    let request = GenerateRequest {
        task_id: record.id,
        prompt: payload.prompt,
    };
    if let Err(err) = dispatch_to_service(
        &state.http_client,
        &state.config.knowledge_explanation_service_url,
        &state.config.knowledge_explanation_api_key,
        &request,
    )
    .await
    {
        let tx = state.db.begin().await?;
        let mut active: knowledge_explanation::ActiveModel = record.clone().into();
        active.status = Set(knowledge_explanation::KnowledgeExplanationStatus::Failed);
        active.updated_at = Set(Utc::now());
        active.update(&tx).await?;

        let refund_user = user::Entity::find_by_id(auth_user.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = refund_user.into();
        active_user.gold = Set(active_user.gold.unwrap() + cost);
        active_user.updated_at = Set(Utc::now());
        active_user.update(&tx).await?;

        tx.commit().await?;
        return Err(err);
    }

    Ok(created(KnowledgeExplanationView::from(record)))
}

pub async fn get_by_id(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let record = knowledge_explanation::Entity::find_by_id(id)
        .filter(knowledge_explanation::Column::UserId.eq(auth_user.user_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    Ok(ok(KnowledgeExplanationView::from(record)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    pub public: Option<bool>,
    #[serde(default)]
    pub retry: bool,
}

pub async fn update(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let now = Utc::now();
    let tx = state.db.begin().await?;

    let record = knowledge_explanation::Entity::find_by_id(id)
        .filter(knowledge_explanation::Column::UserId.eq(auth_user.user_id))
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    let mut active: knowledge_explanation::ActiveModel = record.clone().into();
    let mut changed = false;

    if let Some(public) = payload.public {
        active.public = Set(public);
        changed = true;
    }

    if payload.retry {
        if record.status != knowledge_explanation::KnowledgeExplanationStatus::Failed {
            return Err(AppError::business(BusinessError::InvalidContentStatus));
        }

        let cost = state.config.knowledge_explanation_gold_cost;
        let existing_user = user::Entity::find_by_id(auth_user.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;

        if existing_user.gold < cost {
            return Err(AppError::business(BusinessError::InsufficientGold));
        }

        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.gold = Set(active_user.gold.unwrap() - cost);
        active_user.updated_at = Set(now);
        active_user.update(&tx).await?;

        active.status = Set(knowledge_explanation::KnowledgeExplanationStatus::Queuing);
        active.content = Set(None);
        active.mindmap = Set(None);
        changed = true;
    }

    if changed {
        active.updated_at = Set(now);
        let updated = active.update(&tx).await?;
        tx.commit().await?;

        if payload.retry {
            let request = GenerateRequest {
                task_id: updated.id,
                prompt: updated.prompt.clone(),
            };
            if let Err(err) = dispatch_to_service(
                &state.http_client,
                &state.config.knowledge_explanation_service_url,
                &state.config.knowledge_explanation_api_key,
                &request,
            )
            .await
            {
                let tx = state.db.begin().await?;
                let mut active: knowledge_explanation::ActiveModel = updated.clone().into();
                active.status = Set(knowledge_explanation::KnowledgeExplanationStatus::Failed);
                active.updated_at = Set(Utc::now());
                active.update(&tx).await?;

                let cost = state.config.knowledge_explanation_gold_cost;
                let refund_user = user::Entity::find_by_id(auth_user.user_id)
                    .one(&tx)
                    .await?
                    .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
                let mut active_user: user::ActiveModel = refund_user.into();
                active_user.gold = Set(active_user.gold.unwrap() + cost);
                active_user.updated_at = Set(Utc::now());
                active_user.update(&tx).await?;

                tx.commit().await?;
                return Err(err);
            }
        }

        Ok(ok(KnowledgeExplanationView::from(updated)))
    } else {
        tx.commit().await?;
        Ok(ok(KnowledgeExplanationView::from(record)))
    }
}
