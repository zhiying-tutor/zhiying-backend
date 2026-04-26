use axum::{Json, extract::State};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    auth::AuthUser,
    entities::{common::Gender, user},
    error::{AppError, BusinessError},
    response::ok,
    routes::user_views::UserView,
    state::AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateMeRequest {
    pub birth_year: Option<i32>,
    pub gender: Option<Gender>,
    #[validate(length(max = 1_024))]
    pub introduction: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUsernameRequest {
    #[validate(length(min = 3, max = 32))]
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct MeProfileView {
    id: i32,
    username: String,
    birth_year: Option<i32>,
    gender: Option<Gender>,
    introduction: String,
}

pub async fn get_me(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let user = user::Entity::find_by_id(auth_user.user_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;

    Ok(ok(UserView::from(user)))
}

pub async fn update_me(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<UpdateMeRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    payload.validate()?;

    let user = user::Entity::find_by_id(auth_user.user_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;

    let mut active_user: user::ActiveModel = user.into();

    if let Some(birth_year) = payload.birth_year {
        active_user.birth_year = Set(Some(birth_year));
    }

    if let Some(gender) = payload.gender {
        active_user.gender = Set(Some(gender));
    }

    if let Some(introduction) = payload.introduction {
        active_user.introduction = Set(introduction);
    }

    active_user.updated_at = Set(chrono::Utc::now());
    let updated = active_user.update(&state.db).await?;

    Ok(ok(MeProfileView {
        id: updated.id,
        username: updated.username,
        birth_year: updated.birth_year,
        gender: updated.gender,
        introduction: updated.introduction,
    }))
}

pub async fn update_username(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<UpdateUsernameRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    payload.validate()?;

    let existing_user = user::Entity::find_by_id(auth_user.user_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;

    if existing_user.username == payload.username {
        return Ok(ok(UserView::from(existing_user)));
    }

    let existing = user::Entity::find()
        .filter(user::Column::Username.eq(payload.username.as_str()))
        .one(&state.db)
        .await?;

    if existing.is_some() {
        return Err(AppError::business(BusinessError::UsernameAlreadyExists));
    }

    let mut active_user: user::ActiveModel = existing_user.into();
    active_user.username = Set(payload.username);
    active_user.updated_at = Set(chrono::Utc::now());

    let updated = active_user.update(&state.db).await?;

    Ok(ok(UserView::from(updated)))
}
