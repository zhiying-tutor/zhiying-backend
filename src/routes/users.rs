use axum::{Json, extract::State};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use validator::Validate;

use crate::{
    entities::user,
    error::{AppError, BusinessError},
    response::created,
    routes::user_views::UserView,
    services::password::hash_password,
    state::AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 32))]
    pub username: String,
    #[validate(length(min = 8, max = 72))]
    pub password: String,
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    payload.validate()?;

    let existing = user::Entity::find()
        .filter(user::Column::Username.eq(payload.username.as_str()))
        .one(&state.db)
        .await?;

    if existing.is_some() {
        return Err(AppError::business(BusinessError::UsernameAlreadyExists));
    }

    let now = Utc::now();
    let password = hash_password(&payload.password)?;

    let created_user = user::ActiveModel {
        username: Set(payload.username),
        password: Set(password),
        last_login: Set(None),
        birth_year: Set(None),
        gender: Set(None),
        introduction: Set(String::new()),
        exp: Set(0),
        gold: Set(0),
        diamond: Set(state.config.register_bonus_diamonds),
        total_checkins: Set(0),
        streak_checkins: Set(0),
        last_checkin: Set(None),
        invited_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    Ok(created(UserView::from(created_user)))
}
