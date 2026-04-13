use axum::extract::{Path, State};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;

use crate::{
    auth::AuthUser,
    entities::{
        study_stage, study_stage::StudyStageStatus, study_subject, study_task,
        study_task::StudyTaskStatus,
    },
    error::{AppError, BusinessError},
    response::ok,
    state::AppState,
};

#[derive(Debug, Serialize)]
pub struct StudyStageDetailView {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub sort_order: i32,
    pub status: StudyStageStatus,
    pub total_tasks: i32,
    pub finished_tasks: i32,
    pub created_at: i64,
    pub tasks: Vec<StudyTaskBriefView>,
}

#[derive(Debug, Serialize)]
pub struct StudyTaskBriefView {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub sort_order: i32,
    pub status: StudyTaskStatus,
    pub created_at: i64,
}

/// GET /api/v1/study-stages/{id}
pub async fn get_by_id(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let stage = study_stage::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StageNotFound))?;

    // Verify ownership through study_subject
    let _subject = study_subject::Entity::find_by_id(stage.study_subject_id)
        .filter(study_subject::Column::UserId.eq(auth_user.user_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StageNotFound))?;

    let tasks = study_task::Entity::find()
        .filter(study_task::Column::StudyStageId.eq(stage.id))
        .order_by_asc(study_task::Column::SortOrder)
        .all(&state.db)
        .await?;

    let task_views: Vec<StudyTaskBriefView> = tasks
        .into_iter()
        .map(|t| StudyTaskBriefView {
            id: t.id,
            title: t.title,
            description: t.description,
            sort_order: t.sort_order,
            status: t.status,
            created_at: t.created_at.timestamp_millis(),
        })
        .collect();

    Ok(ok(StudyStageDetailView {
        id: stage.id,
        title: stage.title,
        description: stage.description,
        sort_order: stage.sort_order,
        status: stage.status,
        total_tasks: stage.total_tasks,
        finished_tasks: stage.finished_tasks,
        created_at: stage.created_at.timestamp_millis(),
        tasks: task_views,
    }))
}
