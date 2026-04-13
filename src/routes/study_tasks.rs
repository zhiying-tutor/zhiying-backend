use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    entities::{
        interactive_html, knowledge_explanation, knowledge_video, study_quiz,
        study_quiz::StudyQuizStatus, study_stage, study_stage::StudyStageStatus, study_subject,
        study_subject::StudySubjectStatus, study_task, study_task::StudyTaskStatus, user,
    },
    error::{AppError, BusinessError},
    response::{created, ok},
    services::{
        content::{GenerateRequest, dispatch_to_service},
        study_subject::{QuizRequest, dispatch_quiz},
    },
    state::AppState,
};

// ── Views ──

#[derive(Debug, Serialize)]
pub struct StudyTaskView {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub sort_order: i32,
    pub status: StudyTaskStatus,
    pub knowledge_video_id: Option<i32>,
    pub interactive_html_id: Option<i32>,
    pub knowledge_explanation_id: Option<i32>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<study_task::Model> for StudyTaskView {
    fn from(m: study_task::Model) -> Self {
        Self {
            id: m.id,
            title: m.title,
            description: m.description,
            sort_order: m.sort_order,
            status: m.status,
            knowledge_video_id: m.knowledge_video_id,
            interactive_html_id: m.interactive_html_id,
            knowledge_explanation_id: m.knowledge_explanation_id,
            created_at: m.created_at.timestamp_millis(),
            updated_at: m.updated_at.timestamp_millis(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StudyQuizBriefView {
    pub id: i32,
    pub status: StudyQuizStatus,
    pub total_problems: i32,
    pub correct_problems: i32,
    pub created_at: i64,
}

// ── Payloads ──

#[derive(Debug, Deserialize)]
pub struct PromptRequest {
    pub prompt: String,
}

// ── Helpers ──

/// Load a study_task and verify ownership through the join chain.
async fn load_owned_task<C: sea_orm::ConnectionTrait>(
    db: &C,
    task_id: i32,
    user_id: i32,
) -> Result<(study_task::Model, study_stage::Model, study_subject::Model), AppError> {
    let task = study_task::Entity::find_by_id(task_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::TaskNotFound))?;

    let stage = study_stage::Entity::find_by_id(task.study_stage_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::TaskNotFound))?;

    let subject = study_subject::Entity::find_by_id(stage.study_subject_id)
        .filter(study_subject::Column::UserId.eq(user_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::TaskNotFound))?;

    Ok((task, stage, subject))
}

// ── Handlers ──

/// GET /api/v1/study-tasks/{id}
pub async fn get_by_id(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let (task, _, _) = load_owned_task(&state.db, id, auth_user.user_id).await?;
    Ok(ok(StudyTaskView::from(task)))
}

/// POST /api/v1/study-tasks/{id}/complete
pub async fn complete(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let tx = state.db.begin().await?;

    let (task, stage, subject) = load_owned_task(&tx, id, auth_user.user_id).await?;

    if task.status != StudyTaskStatus::Studying {
        return Err(AppError::business(BusinessError::InvalidStudyTaskStatus));
    }

    // 1. Mark task as Finished
    let mut active_task: study_task::ActiveModel = task.into();
    active_task.status = Set(StudyTaskStatus::Finished);
    active_task.updated_at = Set(Utc::now());
    active_task.update(&tx).await?;

    // 2. Update stage finished_tasks
    let new_finished_tasks = stage.finished_tasks + 1;
    let mut active_stage: study_stage::ActiveModel = stage.clone().into();
    active_stage.finished_tasks = Set(new_finished_tasks);

    // 3. Check if there's a next task in the same stage
    let next_task = study_task::Entity::find()
        .filter(study_task::Column::StudyStageId.eq(stage.id))
        .filter(study_task::Column::Status.eq(StudyTaskStatus::Locked))
        .order_by_asc(study_task::Column::SortOrder)
        .one(&tx)
        .await?;

    if let Some(next) = next_task {
        // Unlock next task
        let mut active_next: study_task::ActiveModel = next.into();
        active_next.status = Set(StudyTaskStatus::Studying);
        active_next.updated_at = Set(Utc::now());
        active_next.update(&tx).await?;
    }

    // 4. Check if stage is fully completed
    if new_finished_tasks >= stage.total_tasks {
        active_stage.status = Set(StudyStageStatus::Finished);
        active_stage.update(&tx).await?;

        // Update subject finished_stages
        let new_finished_stages = subject.finished_stages + 1;
        let mut active_subject: study_subject::ActiveModel = subject.clone().into();
        active_subject.finished_stages = Set(new_finished_stages);

        if new_finished_stages >= subject.total_stages {
            // All stages done
            active_subject.status = Set(StudySubjectStatus::Finished);
            active_subject.updated_at = Set(Utc::now());
            active_subject.update(&tx).await?;
        } else {
            active_subject.updated_at = Set(Utc::now());
            active_subject.update(&tx).await?;

            // Unlock next stage and its first task
            let next_stage = study_stage::Entity::find()
                .filter(study_stage::Column::StudySubjectId.eq(subject.id))
                .filter(study_stage::Column::Status.eq(StudyStageStatus::Locked))
                .order_by_asc(study_stage::Column::SortOrder)
                .one(&tx)
                .await?;

            if let Some(ns) = next_stage {
                let mut active_ns: study_stage::ActiveModel = ns.clone().into();
                active_ns.status = Set(StudyStageStatus::Studying);
                active_ns.update(&tx).await?;

                let first_task = study_task::Entity::find()
                    .filter(study_task::Column::StudyStageId.eq(ns.id))
                    .order_by_asc(study_task::Column::SortOrder)
                    .one(&tx)
                    .await?;

                if let Some(ft) = first_task {
                    let mut active_ft: study_task::ActiveModel = ft.into();
                    active_ft.status = Set(StudyTaskStatus::Studying);
                    active_ft.updated_at = Set(Utc::now());
                    active_ft.update(&tx).await?;
                }
            }
        }
    } else {
        active_stage.update(&tx).await?;
    }

    tx.commit().await?;
    Ok(ok(serde_json::json!({"success": true})))
}

/// POST /api/v1/study-tasks/{id}/knowledge-video
pub async fn create_knowledge_video(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<PromptRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let now = Utc::now();
    let cost = state.config.knowledge_video_diamond_cost;
    let tx = state.db.begin().await?;

    let (task, _, _) = load_owned_task(&tx, id, auth_user.user_id).await?;

    if task.status == StudyTaskStatus::Locked {
        return Err(AppError::business(BusinessError::InvalidStudyTaskStatus));
    }

    let existing_user = user::Entity::find_by_id(auth_user.user_id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;

    if existing_user.diamond < cost {
        return Err(AppError::business(BusinessError::InsufficientDiamonds));
    }

    let mut active_user: user::ActiveModel = existing_user.into();
    active_user.diamond = Set(active_user.diamond.unwrap() - cost);
    active_user.updated_at = Set(now);
    active_user.update(&tx).await?;

    let kv_record = knowledge_video::ActiveModel {
        user_id: Set(auth_user.user_id),
        status: Set(knowledge_video::KnowledgeVideoStatus::Queuing),
        prompt: Set(payload.prompt.clone()),
        url: Set(None),
        public: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await?;

    let mut active_task: study_task::ActiveModel = task.into();
    active_task.knowledge_video_id = Set(Some(kv_record.id));
    active_task.updated_at = Set(now);
    active_task.update(&tx).await?;

    tx.commit().await?;

    let request = GenerateRequest {
        task_id: kv_record.id,
        prompt: payload.prompt,
    };
    if let Err(err) = dispatch_to_service(
        &state.http_client,
        &state.config.knowledge_video_service_url,
        &state.config.knowledge_video_api_key,
        &request,
    )
    .await
    {
        let tx = state.db.begin().await?;
        let mut active: knowledge_video::ActiveModel = kv_record.clone().into();
        active.status = Set(knowledge_video::KnowledgeVideoStatus::Failed);
        active.updated_at = Set(Utc::now());
        active.update(&tx).await?;

        let refund_user = user::Entity::find_by_id(auth_user.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = refund_user.into();
        active_user.diamond = Set(active_user.diamond.unwrap() + cost);
        active_user.updated_at = Set(Utc::now());
        active_user.update(&tx).await?;

        tx.commit().await?;
        return Err(err);
    }

    Ok(created(serde_json::json!({
        "knowledge_video_id": kv_record.id,
    })))
}

/// POST /api/v1/study-tasks/{id}/interactive-html
pub async fn create_interactive_html(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<PromptRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let now = Utc::now();
    let cost = state.config.interactive_html_gold_cost;
    let tx = state.db.begin().await?;

    let (task, _, _) = load_owned_task(&tx, id, auth_user.user_id).await?;

    if task.status == StudyTaskStatus::Locked {
        return Err(AppError::business(BusinessError::InvalidStudyTaskStatus));
    }

    let existing_user = user::Entity::find_by_id(auth_user.user_id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;

    if existing_user.gold < cost {
        return Err(AppError::business(BusinessError::InsufficientGold));
    }

    let mut active_user: user::ActiveModel = existing_user.into();
    active_user.gold = Set(active_user.gold.unwrap() - cost);
    active_user.updated_at = Set(now);
    active_user.update(&tx).await?;

    let ih_record = interactive_html::ActiveModel {
        user_id: Set(auth_user.user_id),
        status: Set(interactive_html::InteractiveHtmlStatus::Queuing),
        prompt: Set(payload.prompt.clone()),
        url: Set(None),
        public: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await?;

    let mut active_task: study_task::ActiveModel = task.into();
    active_task.interactive_html_id = Set(Some(ih_record.id));
    active_task.updated_at = Set(now);
    active_task.update(&tx).await?;

    tx.commit().await?;

    let request = GenerateRequest {
        task_id: ih_record.id,
        prompt: payload.prompt,
    };
    if let Err(err) = dispatch_to_service(
        &state.http_client,
        &state.config.interactive_html_service_url,
        &state.config.interactive_html_api_key,
        &request,
    )
    .await
    {
        let tx = state.db.begin().await?;
        let mut active: interactive_html::ActiveModel = ih_record.clone().into();
        active.status = Set(interactive_html::InteractiveHtmlStatus::Failed);
        active.updated_at = Set(Utc::now());
        active.update(&tx).await?;

        let refund_user = user::Entity::find_by_id(auth_user.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = refund_user.into();
        active_user.gold = Set(active_user.gold.unwrap() + cost);
        active_user.updated_at = Set(Utc::now());
        active_user.update(&tx).await?;

        tx.commit().await?;
        return Err(err);
    }

    Ok(created(serde_json::json!({
        "interactive_html_id": ih_record.id,
    })))
}

/// POST /api/v1/study-tasks/{id}/explanation
pub async fn create_explanation(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<PromptRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let now = Utc::now();
    let tx = state.db.begin().await?;

    let (task, _, _) = load_owned_task(&tx, id, auth_user.user_id).await?;

    if task.status == StudyTaskStatus::Locked {
        return Err(AppError::business(BusinessError::InvalidStudyTaskStatus));
    }

    let ke_record = knowledge_explanation::ActiveModel {
        user_id: Set(auth_user.user_id),
        status: Set(knowledge_explanation::KnowledgeExplanationStatus::Queuing),
        prompt: Set(payload.prompt.clone()),
        content: Set(None),
        mindmap: Set(None),
        public: Set(false),
        cost: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await?;

    let mut active_task: study_task::ActiveModel = task.into();
    active_task.knowledge_explanation_id = Set(Some(ke_record.id));
    active_task.updated_at = Set(now);
    active_task.update(&tx).await?;

    tx.commit().await?;

    let request = GenerateRequest {
        task_id: ke_record.id,
        prompt: payload.prompt,
    };
    if let Err(err) = dispatch_to_service(
        &state.http_client,
        &state.config.knowledge_explanation_service_url,
        &state.config.knowledge_explanation_api_key,
        &request,
    )
    .await
    {
        let tx = state.db.begin().await?;
        let mut active: knowledge_explanation::ActiveModel = ke_record.clone().into();
        active.status = Set(knowledge_explanation::KnowledgeExplanationStatus::Failed);
        active.updated_at = Set(Utc::now());
        active.update(&tx).await?;
        tx.commit().await?;
        return Err(err);
    }

    Ok(created(serde_json::json!({
        "knowledge_explanation_id": ke_record.id,
    })))
}

/// POST /api/v1/study-tasks/{id}/quizzes
pub async fn create_quiz(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<PromptRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let now = Utc::now();
    let tx = state.db.begin().await?;

    let (task, _, _) = load_owned_task(&tx, id, auth_user.user_id).await?;

    if task.status == StudyTaskStatus::Locked {
        return Err(AppError::business(BusinessError::InvalidStudyTaskStatus));
    }

    // Count existing quizzes for this task
    let existing_count = study_quiz::Entity::find()
        .filter(study_quiz::Column::StudyTaskId.eq(task.id))
        .count(&tx)
        .await? as i32;

    let free_limit = state.config.study_quiz_free_limit_per_task;
    let cost = if existing_count < free_limit {
        0
    } else {
        state.config.study_quiz_extra_gold_cost
    };

    if cost > 0 {
        let existing_user = user::Entity::find_by_id(auth_user.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;

        if existing_user.gold < cost {
            return Err(AppError::business(BusinessError::InsufficientGold));
        }

        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.gold = Set(active_user.gold.unwrap() - cost);
        active_user.updated_at = Set(now);
        active_user.update(&tx).await?;
    }

    let quiz_record = study_quiz::ActiveModel {
        study_task_id: Set(task.id),
        status: Set(StudyQuizStatus::Queuing),
        cost: Set(cost),
        total_problems: Set(0),
        correct_problems: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await?;

    tx.commit().await?;

    let request = QuizRequest {
        task_id: quiz_record.id,
        prompt: payload.prompt,
    };
    if let Err(err) = dispatch_quiz(
        &state.http_client,
        &state.config.quiz_service_url,
        &state.config.quiz_api_key,
        &request,
    )
    .await
    {
        let tx = state.db.begin().await?;

        let mut active: study_quiz::ActiveModel = quiz_record.clone().into();
        active.status = Set(StudyQuizStatus::Failed);
        active.updated_at = Set(Utc::now());
        active.update(&tx).await?;

        if cost > 0 {
            let refund_user = user::Entity::find_by_id(auth_user.user_id)
                .one(&tx)
                .await?
                .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
            let mut active_user: user::ActiveModel = refund_user.into();
            active_user.gold = Set(active_user.gold.unwrap() + cost);
            active_user.updated_at = Set(Utc::now());
            active_user.update(&tx).await?;
        }

        tx.commit().await?;
        return Err(err);
    }

    Ok(created(serde_json::json!({
        "quiz_id": quiz_record.id,
        "cost": cost,
    })))
}

/// GET /api/v1/study-tasks/{id}/quizzes
pub async fn list_quizzes(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let (task, _, _) = load_owned_task(&state.db, id, auth_user.user_id).await?;

    let quizzes = study_quiz::Entity::find()
        .filter(study_quiz::Column::StudyTaskId.eq(task.id))
        .order_by_desc(study_quiz::Column::CreatedAt)
        .all(&state.db)
        .await?;

    let views: Vec<StudyQuizBriefView> = quizzes
        .into_iter()
        .map(|q| StudyQuizBriefView {
            id: q.id,
            status: q.status,
            total_problems: q.total_problems,
            correct_problems: q.correct_problems,
            created_at: q.created_at.timestamp_millis(),
        })
        .collect();

    Ok(ok(views))
}
