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
        common::ProblemAnswer, pretest_problem, problem, study_subject,
        study_subject::StudySubjectStatus, user,
    },
    error::{AppError, BusinessError},
    response::{created, ok},
    services::study_subject::{
        PlanRequest, PretestRequest, PretestResult, dispatch_plan, dispatch_pretest,
    },
    state::AppState,
};

// ── Views ──

#[derive(Debug, Serialize)]
pub struct StudySubjectView {
    pub id: i32,
    pub subject: String,
    pub status: StudySubjectStatus,
    pub total_stages: i32,
    pub finished_stages: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<study_subject::Model> for StudySubjectView {
    fn from(m: study_subject::Model) -> Self {
        Self {
            id: m.id,
            subject: m.subject,
            status: m.status,
            total_stages: m.total_stages,
            finished_stages: m.finished_stages,
            created_at: m.created_at.timestamp_millis(),
            updated_at: m.updated_at.timestamp_millis(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PretestProblemView {
    pub id: i32,
    pub problem: ProblemView,
    pub sort_order: i32,
    pub confidence: Option<pretest_problem::PretestConfidence>,
    pub chosen_answer: Option<ProblemAnswer>,
}

#[derive(Debug, Serialize)]
pub struct ProblemView {
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
pub struct CreateRequest {
    pub subject: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePretestProblemRequest {
    pub chosen_answer: ProblemAnswer,
    pub confidence: pretest_problem::PretestConfidence,
}

// ── Handlers ──

/// POST /api/v1/study-subjects
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let now = Utc::now();
    let cost = state.config.study_subject_diamond_cost;
    let tx = state.db.begin().await?;

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

    let record = study_subject::ActiveModel {
        user_id: Set(auth_user.user_id),
        subject: Set(payload.subject.clone()),
        status: Set(StudySubjectStatus::PretestQueuing),
        total_stages: Set(0),
        finished_stages: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await?;

    tx.commit().await?;

    let request = PretestRequest {
        task_id: record.id,
        prompt: payload.subject,
    };
    if let Err(err) = dispatch_pretest(
        &state.http_client,
        &state.config.pretest_service_url,
        &state.config.pretest_api_key,
        &request,
    )
    .await
    {
        let tx = state.db.begin().await?;

        let mut active: study_subject::ActiveModel = record.clone().into();
        active.status = Set(StudySubjectStatus::Failed);
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

    Ok(created(StudySubjectView::from(record)))
}

/// GET /api/v1/study-subjects
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let records = study_subject::Entity::find()
        .filter(study_subject::Column::UserId.eq(auth_user.user_id))
        .order_by_desc(study_subject::Column::CreatedAt)
        .all(&state.db)
        .await?;

    let views: Vec<StudySubjectView> = records.into_iter().map(StudySubjectView::from).collect();
    Ok(ok(views))
}

/// GET /api/v1/study-subjects/{id}
pub async fn get_by_id(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let record = study_subject::Entity::find_by_id(id)
        .filter(study_subject::Column::UserId.eq(auth_user.user_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StudySubjectNotFound))?;

    Ok(ok(StudySubjectView::from(record)))
}

/// GET /api/v1/study-subjects/{id}/pretest
pub async fn get_pretest(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let subject = study_subject::Entity::find_by_id(id)
        .filter(study_subject::Column::UserId.eq(auth_user.user_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StudySubjectNotFound))?;

    let pretest_problems = pretest_problem::Entity::find()
        .filter(pretest_problem::Column::StudySubjectId.eq(subject.id))
        .order_by_asc(pretest_problem::Column::SortOrder)
        .all(&state.db)
        .await?;

    let problem_ids: Vec<i32> = pretest_problems.iter().map(|pp| pp.problem_id).collect();
    let problems = problem::Entity::find()
        .filter(problem::Column::Id.is_in(problem_ids))
        .all(&state.db)
        .await?;

    let problem_map: std::collections::HashMap<i32, problem::Model> =
        problems.into_iter().map(|p| (p.id, p)).collect();

    let views: Vec<PretestProblemView> = pretest_problems
        .into_iter()
        .filter_map(|pp| {
            let p = problem_map.get(&pp.problem_id)?;
            Some(PretestProblemView {
                id: pp.id,
                problem: ProblemView {
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
                sort_order: pp.sort_order,
                confidence: pp.confidence,
                chosen_answer: pp.chosen_answer,
            })
        })
        .collect();

    Ok(ok(views))
}

/// PATCH /api/v1/study-subjects/{id}/pretest/{pretest_problem_id}
pub async fn update_pretest_problem(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path((id, pretest_problem_id)): Path<(i32, i32)>,
    Json(payload): Json<UpdatePretestProblemRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let subject = study_subject::Entity::find_by_id(id)
        .filter(study_subject::Column::UserId.eq(auth_user.user_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StudySubjectNotFound))?;

    if subject.status != StudySubjectStatus::PretestReady {
        return Err(AppError::business(BusinessError::InvalidStudySubjectStatus));
    }

    let pp = pretest_problem::Entity::find_by_id(pretest_problem_id)
        .filter(pretest_problem::Column::StudySubjectId.eq(subject.id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ProblemNotFound))?;

    let mut active: pretest_problem::ActiveModel = pp.into();
    active.chosen_answer = Set(Some(payload.chosen_answer));
    active.confidence = Set(Some(payload.confidence));
    active.update(&state.db).await?;

    Ok(ok(serde_json::json!({"success": true})))
}

/// POST /api/v1/study-subjects/{id}/plan
pub async fn create_plan(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let tx = state.db.begin().await?;

    let subject = study_subject::Entity::find_by_id(id)
        .filter(study_subject::Column::UserId.eq(auth_user.user_id))
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StudySubjectNotFound))?;

    if subject.status != StudySubjectStatus::PretestReady {
        return Err(AppError::business(BusinessError::InvalidStudySubjectStatus));
    }

    let mut active: study_subject::ActiveModel = subject.clone().into();
    active.status = Set(StudySubjectStatus::PlanQueuing);
    active.updated_at = Set(Utc::now());
    active.update(&tx).await?;

    // Gather pretest results for the plan microservice
    let pretest_problems = pretest_problem::Entity::find()
        .filter(pretest_problem::Column::StudySubjectId.eq(subject.id))
        .order_by_asc(pretest_problem::Column::SortOrder)
        .all(&tx)
        .await?;

    let problem_ids: Vec<i32> = pretest_problems.iter().map(|pp| pp.problem_id).collect();
    let problems = problem::Entity::find()
        .filter(problem::Column::Id.is_in(problem_ids))
        .all(&tx)
        .await?;
    let problem_map: std::collections::HashMap<i32, problem::Model> =
        problems.into_iter().map(|p| (p.id, p)).collect();

    let pretest_results: Vec<PretestResult> = pretest_problems
        .iter()
        .filter_map(|pp| {
            let p = problem_map.get(&pp.problem_id)?;
            Some(PretestResult {
                problem_id: p.id,
                content: p.content.clone(),
                choice_a: p.choice_a.clone(),
                choice_b: p.choice_b.clone(),
                choice_c: p.choice_c.clone(),
                choice_d: p.choice_d.clone(),
                answer: format!("{:?}", p.answer),
                chosen_answer: pp.chosen_answer.map(|a| format!("{:?}", a)),
                confidence: pp.confidence.map(|c| format!("{:?}", c)),
            })
        })
        .collect();

    tx.commit().await?;

    let request = PlanRequest {
        task_id: subject.id,
        prompt: subject.subject.clone(),
        pretest_results,
    };

    if let Err(err) = dispatch_plan(
        &state.http_client,
        &state.config.plan_service_url,
        &state.config.plan_api_key,
        &request,
    )
    .await
    {
        let tx = state.db.begin().await?;

        let mut active: study_subject::ActiveModel = subject.clone().into();
        active.status = Set(StudySubjectStatus::Failed);
        active.updated_at = Set(Utc::now());
        active.update(&tx).await?;

        let cost = state.config.study_subject_diamond_cost;
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

    Ok(ok(
        serde_json::json!({"id": subject.id, "status": "PLAN_QUEUING"}),
    ))
}
