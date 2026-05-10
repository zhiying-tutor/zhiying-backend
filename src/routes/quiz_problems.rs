use axum::extract::{Path, State};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};

use crate::{
    auth::AuthUser,
    entities::{study_quiz, study_quiz_problem, study_stage, study_subject, study_task},
    error::{AppError, BusinessError},
    response::ok,
    state::AppState,
};

/// 校验某条 quiz_problem 归属当前用户。返回该记录。
async fn load_owned_quiz_problem<C: ConnectionTrait>(
    db: &C,
    quiz_problem_id: i32,
    user_id: i32,
) -> Result<study_quiz_problem::Model, AppError> {
    let qp = study_quiz_problem::Entity::find_by_id(quiz_problem_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StudyQuizProblemNotFound))?;

    let quiz = study_quiz::Entity::find_by_id(qp.study_quiz_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StudyQuizProblemNotFound))?;
    let task = study_task::Entity::find_by_id(quiz.study_task_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StudyQuizProblemNotFound))?;
    let stage = study_stage::Entity::find_by_id(task.study_stage_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StudyQuizProblemNotFound))?;
    let _subject = study_subject::Entity::find_by_id(stage.study_subject_id)
        .filter(study_subject::Column::UserId.eq(user_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StudyQuizProblemNotFound))?;

    Ok(qp)
}

/// PATCH /api/v1/quiz-problems/{id}/bookmark
pub async fn toggle_bookmark(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let qp = load_owned_quiz_problem(&state.db, id, auth_user.user_id).await?;

    let new_value = !qp.bookmarked;
    let mut active: study_quiz_problem::ActiveModel = qp.into();
    active.bookmarked = Set(new_value);
    active.update(&state.db).await?;

    Ok(ok(serde_json::json!({"bookmarked": new_value})))
}

/// PATCH /api/v1/quiz-problems/{id}/mistake-visibility
pub async fn toggle_mistake_visibility(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let qp = load_owned_quiz_problem(&state.db, id, auth_user.user_id).await?;

    let new_value = !qp.mistake_hidden;
    let mut active: study_quiz_problem::ActiveModel = qp.into();
    active.mistake_hidden = Set(new_value);
    active.update(&state.db).await?;

    Ok(ok(serde_json::json!({"mistake_hidden": new_value})))
}
