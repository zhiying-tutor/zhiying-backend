use axum::{Json, extract::State};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    auth::encode_token,
    entities::user,
    error::{AppError, BusinessError},
    response::ok,
    services::password::verify_password,
    state::AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTokenRequest {
    #[validate(length(min = 3, max = 32))]
    pub username: String,
    #[validate(length(min = 8, max = 72))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct TokenView {
    token: String,
}

pub async fn create_token(
    State(state): State<AppState>,
    Json(payload): Json<CreateTokenRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    payload.validate()?;

    let existing = user::Entity::find()
        .filter(user::Column::Username.eq(payload.username.as_str()))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::InvalidCredentials))?;

    if !verify_password(&payload.password, &existing.password)? {
        return Err(AppError::business(BusinessError::InvalidCredentials));
    }

    let mut user_to_update: user::ActiveModel = existing.clone().into();
    user_to_update.last_login = Set(Some(Utc::now()));
    user_to_update.updated_at = Set(Utc::now());
    user_to_update.update(&state.db).await?;

    let token = encode_token(
        existing.id,
        &existing.username,
        &state.config.jwt_secret,
        state.config.jwt_ttl_days,
    )?;

    Ok(ok(TokenView { token }))
}
