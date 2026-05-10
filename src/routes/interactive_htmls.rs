use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    entities::{interactive_html, user, user_interactive_html_link},
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
pub struct InteractiveHtmlView {
    pub id: i32,
    pub status: interactive_html::InteractiveHtmlStatus,
    pub prompt: String,
    pub object_key: Option<String>,
    pub public: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<interactive_html::Model> for InteractiveHtmlView {
    fn from(m: interactive_html::Model) -> Self {
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
    let cost = state.config.interactive_html_gold_cost;
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

    let record = interactive_html::ActiveModel {
        status: Set(interactive_html::InteractiveHtmlStatus::Queuing),
        prompt: Set(payload.prompt.clone()),
        object_key: Set(None),
        public: Set(payload.public),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await?;

    user_interactive_html_link::ActiveModel {
        interactive_html_id: Set(record.id),
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
        &state.config.interactive_html_exchange,
        &request,
    )
    .await
    {
        let tx = state.db.begin().await?;
        let mut active: interactive_html::ActiveModel = record.clone().into();
        active.status = Set(interactive_html::InteractiveHtmlStatus::Failed);
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

    Ok(created(InteractiveHtmlView::from(record)))
}

pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let pairs = user_interactive_html_link::Entity::find()
        .filter(user_interactive_html_link::Column::UserId.eq(auth_user.user_id))
        .order_by_desc(user_interactive_html_link::Column::CreatedAt)
        .find_also_related(interactive_html::Entity)
        .all(&state.db)
        .await?;

    let views: Vec<InteractiveHtmlView> = pairs
        .into_iter()
        .filter_map(|(_link, ih)| ih.map(InteractiveHtmlView::from))
        .collect();
    Ok(ok(views))
}

pub async fn get_by_id(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let _link = user_interactive_html_link::Entity::find_by_id(id)
        .filter(user_interactive_html_link::Column::UserId.eq(auth_user.user_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    let record = interactive_html::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    Ok(ok(InteractiveHtmlView::from(record)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let result = user_interactive_html_link::Entity::delete_many()
        .filter(user_interactive_html_link::Column::InteractiveHtmlId.eq(id))
        .filter(user_interactive_html_link::Column::UserId.eq(auth_user.user_id))
        .exec(&state.db)
        .await?;

    if result.rows_affected == 0 {
        return Err(AppError::business(BusinessError::ContentNotFound));
    }

    Ok(ok(serde_json::json!({"deleted": true})))
}
