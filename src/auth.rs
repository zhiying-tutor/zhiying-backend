use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, BusinessError},
    state::AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32,
    pub username: String,
    pub exp: usize,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i32,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let authorization = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::business(BusinessError::MissingAuthorizationHeader))?;

        let token = authorization
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::business(BusinessError::InvalidAuthorizationHeader))?;

        let claims = decode_token(token, &state.config.jwt_secret)?;

        Ok(Self {
            user_id: claims.sub,
        })
    }
}

pub fn encode_token(
    user_id: i32,
    username: &str,
    secret: &str,
    ttl_days: i64,
) -> Result<String, AppError> {
    let exp = (Utc::now() + Duration::days(ttl_days)).timestamp() as usize;
    let claims = Claims {
        sub: user_id,
        username: username.to_owned(),
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| AppError::internal(format!("failed to issue jwt: {err}")))
}

fn decode_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::business(BusinessError::InvalidOrExpiredToken))
}
