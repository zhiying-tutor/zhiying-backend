use axum::{
    Json,
    extract::{Query, State},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    entities::{user, user_checkin},
    error::{AppError, BusinessError},
    response::{created, ok},
    services::checkin::{
        makeup_cost, makeup_dates, missed_days_since_last_checkin, next_streak,
        reward_for_streak_day, reward_sum_for_streak_range,
    },
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CheckinListQuery {
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CheckinRequest {
    #[serde(default)]
    pub makeup: bool,
}

#[derive(Debug, Serialize)]
pub struct CheckinResponse {
    checkin_date: String,
    gold_reward: i32,
    makeup_applied: bool,
    makeup_days: i64,
    diamond_cost: i32,
    gold_cost: i32,
    total_checkin: i32,
    streak_checkin: i32,
}

pub async fn check_in(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CheckinRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let today = Utc::now().date_naive();
    let now = Utc::now();
    let tx = state.db.begin().await?;

    let existing_user = user::Entity::find_by_id(auth_user.user_id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;

    if existing_user.last_checkin == Some(today) {
        return Err(AppError::business(BusinessError::AlreadyCheckedInToday));
    }

    let missed_days = missed_days_since_last_checkin(existing_user.last_checkin, today);
    let makeup_applied = payload.makeup && missed_days > 0;
    let (gold_cost, diamond_cost) = if makeup_applied {
        makeup_cost(
            missed_days,
            state.config.checkin_makeup_gold_cost_per_day,
            state.config.checkin_makeup_diamond_cost,
        )
        .ok_or_else(|| AppError::internal("failed to calculate makeup cost"))?
    } else {
        (0, 0)
    };

    if makeup_applied && existing_user.gold < gold_cost {
        return Err(AppError::business(BusinessError::InsufficientGold));
    }

    if makeup_applied && existing_user.diamond < diamond_cost {
        return Err(AppError::business(BusinessError::InsufficientDiamonds));
    }

    if makeup_applied {
        if let Some(last_checkin) = existing_user.last_checkin {
            let reward_start_day = existing_user.streak_checkin + 1;
            for (index, missed_date) in makeup_dates(last_checkin, today).into_iter().enumerate() {
                let streak_day = reward_start_day
                    + i32::try_from(index)
                        .map_err(|_| AppError::internal("missed date index overflowed i32"))?;
                let reward =
                    reward_for_streak_day(streak_day, &state.config.checkin_reward_sequence)
                        .ok_or_else(|| AppError::internal("failed to calculate checkin reward"))?;

                user_checkin::ActiveModel {
                    user_id: Set(existing_user.id),
                    checkin_date: Set(missed_date),
                    gold_reward: Set(reward),
                    created_at: Set(now),
                    updated_at: Set(now),
                }
                .insert(&tx)
                .await?;
            }
        }
    }

    let streak = if makeup_applied {
        let missed_days = i32::try_from(missed_days)
            .map_err(|_| AppError::internal("missed_days overflowed i32"))?;
        existing_user.streak_checkin + missed_days + 1
    } else {
        next_streak(
            existing_user.last_checkin,
            today,
            existing_user.streak_checkin,
        )
    };
    let gold_reward = if makeup_applied {
        reward_sum_for_streak_range(
            existing_user.streak_checkin + 1,
            streak,
            &state.config.checkin_reward_sequence,
        )
        .ok_or_else(|| AppError::internal("failed to calculate makeup reward range"))?
    } else {
        reward_for_streak_day(streak, &state.config.checkin_reward_sequence)
            .ok_or_else(|| AppError::internal("failed to calculate checkin reward"))?
    };

    user_checkin::ActiveModel {
        user_id: Set(existing_user.id),
        checkin_date: Set(today),
        gold_reward: Set(
            reward_for_streak_day(streak, &state.config.checkin_reward_sequence)
                .ok_or_else(|| AppError::internal("failed to calculate today reward"))?,
        ),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&tx)
    .await?;

    let mut active_user: user::ActiveModel = existing_user.clone().into();
    let added_checkins = if makeup_applied {
        i32::try_from(missed_days).map_err(|_| AppError::internal("missed_days overflowed i32"))?
    } else {
        0
    };

    active_user.gold = Set(existing_user.gold + gold_reward - gold_cost);
    active_user.diamond = Set(existing_user.diamond - diamond_cost);
    active_user.total_checkin = Set(existing_user.total_checkin + added_checkins + 1);
    active_user.streak_checkin = Set(streak);
    active_user.last_checkin = Set(Some(today));
    active_user.updated_at = Set(now);
    active_user.update(&tx).await?;

    tx.commit().await?;

    Ok(created(CheckinResponse {
        checkin_date: today.to_string(),
        gold_reward,
        makeup_applied,
        makeup_days: if makeup_applied { missed_days } else { 0 },
        diamond_cost,
        gold_cost,
        total_checkin: existing_user.total_checkin + added_checkins + 1,
        streak_checkin: streak,
    }))
}

pub async fn list_checkins(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<CheckinListQuery>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let limit = query.limit.unwrap_or(30).min(100);

    let records = user_checkin::Entity::find()
        .filter(user_checkin::Column::UserId.eq(auth_user.user_id))
        .order_by_desc(user_checkin::Column::CheckinDate)
        .limit(limit)
        .all(&state.db)
        .await?;

    let data = records
        .into_iter()
        .map(|record| CheckinListItem {
            checkin_date: record.checkin_date.to_string(),
            gold_reward: record.gold_reward,
        })
        .collect::<Vec<_>>();

    Ok(ok(data))
}

#[derive(Debug, Serialize)]
pub struct CheckinListItem {
    checkin_date: String,
    gold_reward: i32,
}
