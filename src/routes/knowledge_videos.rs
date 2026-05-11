use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    entities::{knowledge_video, user, user_knowledge_video_link},
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
pub struct KnowledgeVideoView {
    pub id: i32,
    pub status: knowledge_video::KnowledgeVideoStatus,
    pub prompt: String,
    pub object_key: Option<String>,
    pub public: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<knowledge_video::Model> for KnowledgeVideoView {
    fn from(m: knowledge_video::Model) -> Self {
        Self {
            id: m.id,
            status: m.status,
            prompt: m.prompt,
            object_key: m.object_key,
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
    let cost = state.config.knowledge_video_diamond_cost;
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

    let record = knowledge_video::ActiveModel {
        status: Set(knowledge_video::KnowledgeVideoStatus::Queuing),
        prompt: Set(payload.prompt.clone()),
        object_key: Set(None),
        public: Set(payload.public),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await?;

    user_knowledge_video_link::ActiveModel {
        knowledge_video_id: Set(record.id),
        user_id: Set(auth_user.user_id),
        created_at: Set(now),
    }
    .insert(&tx)
    .await?;

    tx.commit().await?;

    let request = GenerateRequest {
        task_id: record.id,
        prompt: payload.prompt,
    };
    if let Err(err) = dispatch_to_service(
        state.publisher.as_ref(),
        &state.config.knowledge_video_exchange,
        &request,
    )
    .await
    {
        let tx = state.db.begin().await?;
        let mut active: knowledge_video::ActiveModel = record.clone().into();
        active.status = Set(knowledge_video::KnowledgeVideoStatus::Failed);
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

    Ok(created(KnowledgeVideoView::from(record)))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub q: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<ListQuery>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let mut select = user_knowledge_video_link::Entity::find()
        .filter(user_knowledge_video_link::Column::UserId.eq(auth_user.user_id))
        .order_by_desc(user_knowledge_video_link::Column::CreatedAt)
        .find_also_related(knowledge_video::Entity);

    if let Some(q) = query.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        select = select.filter(knowledge_video::Column::Prompt.contains(q));
    }

    let pairs = select.all(&state.db).await?;

    let views: Vec<KnowledgeVideoView> = pairs
        .into_iter()
        .filter_map(|(_link, kv)| kv.map(KnowledgeVideoView::from))
        .collect();
    Ok(ok(views))
}

pub async fn get_by_id(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let _link = user_knowledge_video_link::Entity::find_by_id(id)
        .filter(user_knowledge_video_link::Column::UserId.eq(auth_user.user_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    let record = knowledge_video::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    Ok(ok(KnowledgeVideoView::from(record)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let result = user_knowledge_video_link::Entity::delete_many()
        .filter(user_knowledge_video_link::Column::KnowledgeVideoId.eq(id))
        .filter(user_knowledge_video_link::Column::UserId.eq(auth_user.user_id))
        .exec(&state.db)
        .await?;

    if result.rows_affected == 0 {
        return Err(AppError::business(BusinessError::ContentNotFound));
    }

    Ok(ok(serde_json::json!({"deleted": true})))
}
