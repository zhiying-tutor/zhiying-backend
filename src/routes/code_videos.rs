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
    entities::{code_video, user},
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
pub struct CodeVideoView {
    pub id: i32,
    pub status: code_video::CodeVideoStatus,
    pub prompt: String,
    pub url: Option<String>,
    pub public: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<code_video::Model> for CodeVideoView {
    fn from(m: code_video::Model) -> Self {
        Self {
            id: m.id,
            status: m.status,
            prompt: m.prompt,
            url: m.url,
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
    let cost = state.config.code_video_diamond_cost;
    let tx = state.db.begin().await?;

    let existing_user = user::Entity::find_by_id(auth_user.user_id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;

    if existing_user.diamond < cost {
        return Err(AppError::business(BusinessError::InsufficientDiamonds));
    }

    let mut active_user: user::ActiveModel = existing_user.into();
    active_user.diamond = Set(active_user.diamond.unwrap() - cost);
    active_user.updated_at = Set(now);
    active_user.update(&tx).await?;

    let record = code_video::ActiveModel {
        user_id: Set(auth_user.user_id),
        status: Set(code_video::CodeVideoStatus::Queuing),
        prompt: Set(payload.prompt.clone()),
        url: Set(None),
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
        &state.config.code_video_service_url,
        &state.config.code_video_api_key,
        &request,
    )
    .await
    {
        let tx = state.db.begin().await?;
        let mut active: code_video::ActiveModel = record.clone().into();
        active.status = Set(code_video::CodeVideoStatus::Failed);
        active.updated_at = Set(Utc::now());
        active.update(&tx).await?;

        let refund_user = user::Entity::find_by_id(auth_user.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = refund_user.into();
        active_user.diamond = Set(active_user.diamond.unwrap() + cost);
        active_user.updated_at = Set(Utc::now());
        active_user.update(&tx).await?;

        tx.commit().await?;
        return Err(err);
    }

    Ok(created(CodeVideoView::from(record)))
}

pub async fn get_by_id(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let record = code_video::Entity::find_by_id(id)
        .filter(code_video::Column::UserId.eq(auth_user.user_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    Ok(ok(CodeVideoView::from(record)))
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

    let record = code_video::Entity::find_by_id(id)
        .filter(code_video::Column::UserId.eq(auth_user.user_id))
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    let mut active: code_video::ActiveModel = record.clone().into();
    let mut changed = false;

    if let Some(public) = payload.public {
        active.public = Set(public);
        changed = true;
    }

    if payload.retry {
        if record.status != code_video::CodeVideoStatus::Failed {
            return Err(AppError::business(BusinessError::InvalidContentStatus));
        }

        let cost = state.config.code_video_diamond_cost;
        let existing_user = user::Entity::find_by_id(auth_user.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;

        if existing_user.diamond < cost {
            return Err(AppError::business(BusinessError::InsufficientDiamonds));
        }

        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.diamond = Set(active_user.diamond.unwrap() - cost);
        active_user.updated_at = Set(now);
        active_user.update(&tx).await?;

        active.status = Set(code_video::CodeVideoStatus::Queuing);
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
                &state.config.code_video_service_url,
                &state.config.code_video_api_key,
                &request,
            )
            .await
            {
                let tx = state.db.begin().await?;
                let mut active: code_video::ActiveModel = updated.clone().into();
                active.status = Set(code_video::CodeVideoStatus::Failed);
                active.updated_at = Set(Utc::now());
                active.update(&tx).await?;

                let cost = state.config.code_video_diamond_cost;
                let refund_user = user::Entity::find_by_id(auth_user.user_id)
                    .one(&tx)
                    .await?
                    .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
                let mut active_user: user::ActiveModel = refund_user.into();
                active_user.diamond = Set(active_user.diamond.unwrap() + cost);
                active_user.updated_at = Set(Utc::now());
                active_user.update(&tx).await?;

                tx.commit().await?;
                return Err(err);
            }
        }

        Ok(ok(CodeVideoView::from(updated)))
    } else {
        tx.commit().await?;
        Ok(ok(CodeVideoView::from(record)))
    }
}
