use axum::{
    Json,
    extract::{Query, State},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, JoinType, QueryFilter,
    QuerySelect, RelationTrait,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    auth::AuthUser,
    entities::{
        common::{Gender, ProblemAnswer},
        study_quiz, study_quiz_problem, study_stage, study_subject, study_task, user,
    },
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
    pub active_study_subject_id: Option<i32>,
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
    active_study_subject_id: Option<i32>,
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

    if let Some(subject_id) = payload.active_study_subject_id {
        let owns = study_subject::Entity::find_by_id(subject_id)
            .filter(study_subject::Column::UserId.eq(auth_user.user_id))
            .one(&state.db)
            .await?
            .is_some();
        if !owns {
            return Err(AppError::business(BusinessError::StudySubjectNotFound));
        }
        active_user.active_study_subject_id = Set(Some(subject_id));
    }

    active_user.updated_at = Set(chrono::Utc::now());
    let updated = active_user.update(&state.db).await?;

    Ok(ok(MeProfileView {
        id: updated.id,
        username: updated.username,
        birth_year: updated.birth_year,
        gender: updated.gender,
        introduction: updated.introduction,
        active_study_subject_id: updated.active_study_subject_id,
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

// ── Mistakes / Bookmarks ──

#[derive(Debug, Serialize)]
pub struct QuizProblemReviewView {
    pub id: i32,
    pub sort_order: i32,
    pub content: String,
    pub choice_a: String,
    pub choice_b: String,
    pub choice_c: String,
    pub choice_d: String,
    pub answer: ProblemAnswer,
    pub explanation: String,
    pub chosen_answer: Option<ProblemAnswer>,
    pub bookmarked: bool,
    pub mistake_hidden: bool,
    pub created_at: i64,
    pub source: QuizProblemSource,
}

#[derive(Debug, Serialize)]
pub struct QuizProblemSource {
    pub quiz_id: i32,
    pub task_id: i32,
    pub task_title: String,
    pub stage_id: i32,
    pub stage_title: String,
    pub subject_id: i32,
    pub subject_name: String,
}

#[derive(Debug, Deserialize)]
pub struct MistakesQuery {
    #[serde(default)]
    pub include_hidden: Option<bool>,
}

/// GET /api/v1/me/mistakes
pub async fn list_mistakes(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<MistakesQuery>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let include_hidden = query.include_hidden.unwrap_or(false);

    let rows: Vec<(
        study_quiz_problem::Model,
        Option<study_quiz::Model>,
        Option<study_task::Model>,
        Option<study_stage::Model>,
        Option<study_subject::Model>,
    )> = study_quiz_problem::Entity::find()
        .find_also_related(study_quiz::Entity)
        .join(JoinType::InnerJoin, study_quiz::Relation::StudyTask.def())
        .join(JoinType::InnerJoin, study_task::Relation::StudyStage.def())
        .join(
            JoinType::InnerJoin,
            study_stage::Relation::StudySubject.def(),
        )
        .filter(study_subject::Column::UserId.eq(auth_user.user_id))
        .all(&state.db)
        .await?
        .into_iter()
        .map(|(qp, q)| (qp, q, None, None, None))
        .collect();

    // SeaORM 不能一次性 join 5 张表并把每张映射到 tuple，分两步取来源链。
    let views = build_review_views(&state, rows, |qp| {
        qp.chosen_answer.map(|c| c != qp.answer).unwrap_or(false)
            && (include_hidden || !qp.mistake_hidden)
    })
    .await?;

    Ok(ok(views))
}

/// GET /api/v1/me/bookmarks
pub async fn list_bookmarks(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let rows: Vec<(
        study_quiz_problem::Model,
        Option<study_quiz::Model>,
        Option<study_task::Model>,
        Option<study_stage::Model>,
        Option<study_subject::Model>,
    )> = study_quiz_problem::Entity::find()
        .find_also_related(study_quiz::Entity)
        .join(JoinType::InnerJoin, study_quiz::Relation::StudyTask.def())
        .join(JoinType::InnerJoin, study_task::Relation::StudyStage.def())
        .join(
            JoinType::InnerJoin,
            study_stage::Relation::StudySubject.def(),
        )
        .filter(study_subject::Column::UserId.eq(auth_user.user_id))
        .filter(study_quiz_problem::Column::Bookmarked.eq(true))
        .all(&state.db)
        .await?
        .into_iter()
        .map(|(qp, q)| (qp, q, None, None, None))
        .collect();

    let views = build_review_views(&state, rows, |_| true).await?;

    Ok(ok(views))
}

async fn build_review_views(
    state: &AppState,
    rows: Vec<(
        study_quiz_problem::Model,
        Option<study_quiz::Model>,
        Option<study_task::Model>,
        Option<study_stage::Model>,
        Option<study_subject::Model>,
    )>,
    keep: impl Fn(&study_quiz_problem::Model) -> bool,
) -> Result<Vec<QuizProblemReviewView>, AppError> {
    let filtered: Vec<(study_quiz_problem::Model, study_quiz::Model)> = rows
        .into_iter()
        .filter_map(|(qp, q, _, _, _)| q.map(|q| (qp, q)))
        .filter(|(qp, _)| keep(qp))
        .collect();

    if filtered.is_empty() {
        return Ok(Vec::new());
    }

    // 一次性取齐来源链
    let task_ids: Vec<i32> = filtered.iter().map(|(_, q)| q.study_task_id).collect();
    let tasks = study_task::Entity::find()
        .filter(study_task::Column::Id.is_in(task_ids.clone()))
        .all(&state.db)
        .await?;
    let task_map: std::collections::HashMap<i32, study_task::Model> =
        tasks.into_iter().map(|t| (t.id, t)).collect();

    let stage_ids: Vec<i32> = task_map.values().map(|t| t.study_stage_id).collect();
    let stages = study_stage::Entity::find()
        .filter(study_stage::Column::Id.is_in(stage_ids))
        .all(&state.db)
        .await?;
    let stage_map: std::collections::HashMap<i32, study_stage::Model> =
        stages.into_iter().map(|s| (s.id, s)).collect();

    let subject_ids: Vec<i32> = stage_map.values().map(|s| s.study_subject_id).collect();
    let subjects = study_subject::Entity::find()
        .filter(study_subject::Column::Id.is_in(subject_ids))
        .all(&state.db)
        .await?;
    let subject_map: std::collections::HashMap<i32, study_subject::Model> =
        subjects.into_iter().map(|s| (s.id, s)).collect();

    let mut views: Vec<QuizProblemReviewView> = filtered
        .into_iter()
        .filter_map(|(qp, q)| {
            let task = task_map.get(&q.study_task_id)?;
            let stage = stage_map.get(&task.study_stage_id)?;
            let subject = subject_map.get(&stage.study_subject_id)?;
            Some(QuizProblemReviewView {
                id: qp.id,
                sort_order: qp.sort_order,
                content: qp.content,
                choice_a: qp.choice_a,
                choice_b: qp.choice_b,
                choice_c: qp.choice_c,
                choice_d: qp.choice_d,
                answer: qp.answer,
                explanation: qp.explanation,
                chosen_answer: qp.chosen_answer,
                bookmarked: qp.bookmarked,
                mistake_hidden: qp.mistake_hidden,
                created_at: qp.created_at.timestamp_millis(),
                source: QuizProblemSource {
                    quiz_id: q.id,
                    task_id: task.id,
                    task_title: task.title.clone(),
                    stage_id: stage.id,
                    stage_title: stage.title.clone(),
                    subject_id: subject.id,
                    subject_name: subject.subject.clone(),
                },
            })
        })
        .collect();

    views.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(views)
}
