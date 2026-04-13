use axum::extract::{Path, Query, State};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    entities::{common::ProblemAnswer, pretest_problem, problem, study_quiz_problem},
    error::{AppError, BusinessError},
    response::ok,
    state::AppState,
};

// ── Views ──

#[derive(Debug, Serialize)]
pub struct ProblemListView {
    pub id: i32,
    pub content: String,
    pub choice_a: String,
    pub choice_b: String,
    pub choice_c: String,
    pub choice_d: String,
    pub answer: ProblemAnswer,
    pub explanation: String,
    pub bookmarked: bool,
    pub created_at: i64,
}

impl From<problem::Model> for ProblemListView {
    fn from(m: problem::Model) -> Self {
        Self {
            id: m.id,
            content: m.content,
            choice_a: m.choice_a,
            choice_b: m.choice_b,
            choice_c: m.choice_c,
            choice_d: m.choice_d,
            answer: m.answer,
            explanation: m.explanation,
            bookmarked: m.bookmarked,
            created_at: m.created_at.timestamp_millis(),
        }
    }
}

// ── Payloads ──

#[derive(Debug, Deserialize)]
pub struct ProblemQuery {
    #[serde(default)]
    pub bookmarked: Option<bool>,
    #[serde(default)]
    pub wrong: Option<bool>,
}

// ── Handlers ──

/// GET /api/v1/problems
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<ProblemQuery>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let mut select = problem::Entity::find().filter(problem::Column::UserId.eq(auth_user.user_id));

    if query.bookmarked == Some(true) {
        select = select.filter(problem::Column::Bookmarked.eq(true));
    }

    if query.wrong == Some(true) {
        // Load all candidate problems and filter in-memory for wrong answers
        // (cross-table column comparison not straightforward with SeaORM)
        let all_problems = select
            .order_by_desc(problem::Column::CreatedAt)
            .all(&state.db)
            .await?;

        // Collect problem IDs that have wrong answers
        let wrong_pretest_ids: std::collections::HashSet<i32> = {
            let pretest_records = pretest_problem::Entity::find()
                .filter(pretest_problem::Column::ChosenAnswer.is_not_null())
                .all(&state.db)
                .await?;

            let problem_map: std::collections::HashMap<i32, problem::Model> =
                all_problems.iter().map(|p| (p.id, p.clone())).collect();

            pretest_records
                .iter()
                .filter(|pp| {
                    if let (Some(chosen), Some(p)) =
                        (pp.chosen_answer, problem_map.get(&pp.problem_id))
                    {
                        chosen != p.answer
                    } else {
                        false
                    }
                })
                .map(|pp| pp.problem_id)
                .collect()
        };

        let wrong_quiz_ids: std::collections::HashSet<i32> = {
            let quiz_records = study_quiz_problem::Entity::find()
                .filter(study_quiz_problem::Column::ChosenAnswer.is_not_null())
                .all(&state.db)
                .await?;

            let problem_map: std::collections::HashMap<i32, problem::Model> =
                all_problems.iter().map(|p| (p.id, p.clone())).collect();

            quiz_records
                .iter()
                .filter(|qp| {
                    if let (Some(chosen), Some(p)) =
                        (qp.chosen_answer, problem_map.get(&qp.problem_id))
                    {
                        chosen != p.answer
                    } else {
                        false
                    }
                })
                .map(|qp| qp.problem_id)
                .collect()
        };

        let views: Vec<ProblemListView> = all_problems
            .into_iter()
            .filter(|p| wrong_pretest_ids.contains(&p.id) || wrong_quiz_ids.contains(&p.id))
            .map(ProblemListView::from)
            .collect();

        return Ok(ok(views));
    }

    let records = select
        .order_by_desc(problem::Column::CreatedAt)
        .all(&state.db)
        .await?;

    let views: Vec<ProblemListView> = records.into_iter().map(ProblemListView::from).collect();
    Ok(ok(views))
}

/// PATCH /api/v1/problems/{id}/bookmark
pub async fn toggle_bookmark(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let record = problem::Entity::find_by_id(id)
        .filter(problem::Column::UserId.eq(auth_user.user_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ProblemNotFound))?;

    let new_bookmarked = !record.bookmarked;
    let mut active: problem::ActiveModel = record.into();
    active.bookmarked = Set(new_bookmarked);
    active.update(&state.db).await?;

    Ok(ok(serde_json::json!({"bookmarked": new_bookmarked})))
}
