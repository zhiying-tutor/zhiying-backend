use serde::Serialize;

use crate::entities::{common::Gender, user};

#[derive(Debug, Serialize)]
pub struct UserView {
    id: i32,
    username: String,
    birth_year: Option<i32>,
    gender: Option<Gender>,
    introduction: String,
    exp: i32,
    gold: i32,
    diamond: i32,
    total_checkins: i32,
    streak_checkins: i32,
    last_checkin: Option<String>,
    last_login: Option<String>,
    created_at: i64,
    updated_at: i64,
}

impl From<user::Model> for UserView {
    fn from(model: user::Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
            birth_year: model.birth_year,
            gender: model.gender,
            introduction: model.introduction,
            exp: model.exp,
            gold: model.gold,
            diamond: model.diamond,
            total_checkins: model.total_checkins,
            streak_checkins: model.streak_checkins,
            last_checkin: model.last_checkin.map(|value| value.to_string()),
            last_login: model.last_login.map(|value| value.to_rfc3339()),
            created_at: model.created_at.timestamp_millis(),
            updated_at: model.updated_at.timestamp_millis(),
        }
    }
}
