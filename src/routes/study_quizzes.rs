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
    entities::{
        common::ProblemAnswer, problem, study_quiz, study_quiz::StudyQuizStatus,
        study_quiz_problem, study_stage, study_subject, study_task,
    },
    error::{AppError, BusinessError},
    response::ok,
    state::AppState,
};

// ── Views ──

#[derive(Debug, Serialize)]
pub struct StudyQuizDetailView {
    pub id: i32,
    pub status: StudyQuizStatus,
    pub total_problems: i32,
    pub correct_problems: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub problems: Vec<StudyQuizProblemView>,
}

#[derive(Debug, Serialize)]
pub struct StudyQuizProblemView {
    pub id: i32,
    pub problem: QuizProblemDetail,
    pub sort_order: i32,
    pub chosen_answer: Option<ProblemAnswer>,
}

#[derive(Debug, Serialize)]
pub struct QuizProblemDetail {
    pub id: i32,
    pub content: String,
    pub choice_a: String,
    pub choice_b: String,
    pub choice_c: String,
    pub choice_d: String,
    pub answer: ProblemAnswer,
    pub explanation: String,
    pub bookmarked: bool,
}

// ── Payloads ──

#[derive(Debug, Deserialize)]
pub struct UpdateQuizProblemRequest {
    pub chosen_answer: ProblemAnswer,
}

// ── Helpers ──

/// Load a study_quiz and verify ownership through join chain.
async fn load_owned_quiz<C: sea_orm::ConnectionTrait>(
    db: &C,
    quiz_id: i32,
    user_id: i32,
) -> Result<study_quiz::Model, AppError> {
    let quiz = study_quiz::Entity::find_by_id(quiz_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::QuizNotFound))?;

    let task = study_task::Entity::find_by_id(quiz.study_task_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::QuizNotFound))?;

    let stage = study_stage::Entity::find_by_id(task.study_stage_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::QuizNotFound))?;

    let _subject = study_subject::Entity::find_by_id(stage.study_subject_id)
        .filter(study_subject::Column::UserId.eq(user_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::QuizNotFound))?;

    Ok(quiz)
}

// ── Handlers ──

/// GET /api/v1/study-quizzes/{id}
pub async fn get_by_id(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let quiz = load_owned_quiz(&state.db, id, auth_user.user_id).await?;

    let quiz_problems = study_quiz_problem::Entity::find()
        .filter(study_quiz_problem::Column::StudyQuizId.eq(quiz.id))
        .order_by_asc(study_quiz_problem::Column::SortOrder)
        .all(&state.db)
        .await?;

    let problem_ids: Vec<i32> = quiz_problems.iter().map(|qp| qp.problem_id).collect();
    let problems = problem::Entity::find()
        .filter(problem::Column::Id.is_in(problem_ids))
        .all(&state.db)
        .await?;
    let problem_map: std::collections::HashMap<i32, problem::Model> =
        problems.into_iter().map(|p| (p.id, p)).collect();

    let problem_views: Vec<StudyQuizProblemView> = quiz_problems
        .into_iter()
        .filter_map(|qp| {
            let p = problem_map.get(&qp.problem_id)?;
            Some(StudyQuizProblemView {
                id: qp.id,
                problem: QuizProblemDetail {
                    id: p.id,
                    content: p.content.clone(),
                    choice_a: p.choice_a.clone(),
                    choice_b: p.choice_b.clone(),
                    choice_c: p.choice_c.clone(),
                    choice_d: p.choice_d.clone(),
                    answer: p.answer,
                    explanation: p.explanation.clone(),
                    bookmarked: p.bookmarked,
                },
                sort_order: qp.sort_order,
                chosen_answer: qp.chosen_answer,
            })
        })
        .collect();

    Ok(ok(StudyQuizDetailView {
        id: quiz.id,
        status: quiz.status,
        total_problems: quiz.total_problems,
        correct_problems: quiz.correct_problems,
        created_at: quiz.created_at.timestamp_millis(),
        updated_at: quiz.updated_at.timestamp_millis(),
        problems: problem_views,
    }))
}

/// PATCH /api/v1/study-quizzes/{quiz_id}/problems/{study_quiz_problem_id}
pub async fn update_problem(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((quiz_id, sqp_id)): Path<(i32, i32)>,
    Json(payload): Json<UpdateQuizProblemRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let quiz = load_owned_quiz(&state.db, quiz_id, auth_user.user_id).await?;

    if quiz.status != StudyQuizStatus::Ready {
        return Err(AppError::business(BusinessError::InvalidStudyQuizStatus));
    }

    let sqp = study_quiz_problem::Entity::find_by_id(sqp_id)
        .filter(study_quiz_problem::Column::StudyQuizId.eq(quiz.id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StudyQuizProblemNotFound))?;

    let mut active: study_quiz_problem::ActiveModel = sqp.into();
    active.chosen_answer = Set(Some(payload.chosen_answer));
    active.update(&state.db).await?;

    Ok(ok(serde_json::json!({"success": true})))
}

/// POST /api/v1/study-quizzes/{id}/submit
pub async fn submit(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let tx = state.db.begin().await?;

    let quiz = load_owned_quiz(&tx, id, auth_user.user_id).await?;

    if quiz.status != StudyQuizStatus::Ready {
        return Err(AppError::business(BusinessError::InvalidStudyQuizStatus));
    }

    let quiz_problems = study_quiz_problem::Entity::find()
        .filter(study_quiz_problem::Column::StudyQuizId.eq(quiz.id))
        .all(&tx)
        .await?;

    // Check all answered
    if quiz_problems.iter().any(|qp| qp.chosen_answer.is_none()) {
        return Err(AppError::business(BusinessError::IncompleteQuizAnswers));
    }

    // Load problems to compute correct count
    let problem_ids: Vec<i32> = quiz_problems.iter().map(|qp| qp.problem_id).collect();
    let problems = problem::Entity::find()
        .filter(problem::Column::Id.is_in(problem_ids))
        .all(&tx)
        .await?;
    let problem_map: std::collections::HashMap<i32, problem::Model> =
        problems.into_iter().map(|p| (p.id, p)).collect();

    let correct_count = quiz_problems
        .iter()
        .filter(|qp| {
            if let (Some(chosen), Some(p)) = (qp.chosen_answer, problem_map.get(&qp.problem_id)) {
                chosen == p.answer
            } else {
                false
            }
        })
        .count() as i32;

    let mut active: study_quiz::ActiveModel = quiz.into();
    active.status = Set(StudyQuizStatus::Submitted);
    active.correct_problems = Set(correct_count);
    active.updated_at = Set(Utc::now());
    active.update(&tx).await?;

    tx.commit().await?;

    Ok(ok(serde_json::json!({
        "correct_problems": correct_count,
    })))
}
