use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{patch, post},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, TransactionTrait};
use serde::Deserialize;

use crate::{
    auth::{ServiceAuth, ServiceKind},
    entities::{
        code_video, common::ProblemAnswer, interactive_html, knowledge_explanation,
        knowledge_video, pretest_problem, problem, study_quiz, study_quiz::StudyQuizStatus,
        study_quiz_problem, study_stage, study_stage::StudyStageStatus, study_subject,
        study_subject::StudySubjectStatus, study_task, study_task::StudyTaskStatus, user,
    },
    error::{AppError, BusinessError},
    response::ok,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/knowledge-videos/{id}", patch(update_knowledge_video))
        .route("/code-videos/{id}", patch(update_code_video))
        .route("/interactive-htmls/{id}", patch(update_interactive_html))
        .route(
            "/knowledge-explanations/{id}",
            patch(update_knowledge_explanation),
        )
        .route("/study-subjects/{id}", post(callback_study_subject))
        .route("/study-quizzes/{id}", post(callback_study_quiz))
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
    pub url: Option<String>,
    pub content: Option<String>,
    pub mindmap: Option<String>,
}

// --- knowledge_video ---

async fn update_knowledge_video(
    State(state): State<AppState>,
    service_auth: ServiceAuth,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if service_auth.service != ServiceKind::KnowledgeVideo {
        return Err(AppError::business(BusinessError::InvalidApiKey));
    }

    let tx = state.db.begin().await?;
    let record = knowledge_video::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    let new_status = parse_knowledge_video_status(&payload.status)?;
    validate_knowledge_video_transition(record.status, new_status)?;

    let mut active: knowledge_video::ActiveModel = record.clone().into();
    active.status = Set(new_status);
    active.updated_at = Set(Utc::now());

    if new_status == knowledge_video::KnowledgeVideoStatus::Finished {
        active.url = Set(payload.url);
    }

    if new_status == knowledge_video::KnowledgeVideoStatus::Failed {
        let cost = state.config.knowledge_video_diamond_cost;
        let existing_user = user::Entity::find_by_id(record.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.diamond = Set(active_user.diamond.unwrap() + cost);
        active_user.updated_at = Set(Utc::now());
        active_user.update(&tx).await?;
    }

    active.update(&tx).await?;
    tx.commit().await?;

    Ok(ok(serde_json::json!({"id": id, "status": payload.status})))
}

fn parse_knowledge_video_status(
    s: &str,
) -> Result<knowledge_video::KnowledgeVideoStatus, AppError> {
    match s {
        "QUEUING" => Ok(knowledge_video::KnowledgeVideoStatus::Queuing),
        "GENERATING" => Ok(knowledge_video::KnowledgeVideoStatus::Generating),
        "FINISHED" => Ok(knowledge_video::KnowledgeVideoStatus::Finished),
        "FAILED" => Ok(knowledge_video::KnowledgeVideoStatus::Failed),
        _ => Err(AppError::business(BusinessError::InvalidContentStatus)),
    }
}

fn validate_knowledge_video_transition(
    from: knowledge_video::KnowledgeVideoStatus,
    to: knowledge_video::KnowledgeVideoStatus,
) -> Result<(), AppError> {
    use knowledge_video::KnowledgeVideoStatus::*;
    let valid = matches!(
        (from, to),
        (Queuing, Generating) | (Generating, Finished) | (Generating, Failed)
    );
    if !valid {
        return Err(AppError::business(BusinessError::InvalidContentStatus));
    }
    Ok(())
}

// --- code_video ---

async fn update_code_video(
    State(state): State<AppState>,
    service_auth: ServiceAuth,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if service_auth.service != ServiceKind::CodeVideo {
        return Err(AppError::business(BusinessError::InvalidApiKey));
    }

    let tx = state.db.begin().await?;
    let record = code_video::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    let new_status = parse_code_video_status(&payload.status)?;
    validate_code_video_transition(record.status, new_status)?;

    let mut active: code_video::ActiveModel = record.clone().into();
    active.status = Set(new_status);
    active.updated_at = Set(Utc::now());

    if new_status == code_video::CodeVideoStatus::Finished {
        active.url = Set(payload.url);
    }

    if new_status == code_video::CodeVideoStatus::Failed {
        let cost = state.config.code_video_diamond_cost;
        let existing_user = user::Entity::find_by_id(record.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.diamond = Set(active_user.diamond.unwrap() + cost);
        active_user.updated_at = Set(Utc::now());
        active_user.update(&tx).await?;
    }

    active.update(&tx).await?;
    tx.commit().await?;

    Ok(ok(serde_json::json!({"id": id, "status": payload.status})))
}

fn parse_code_video_status(s: &str) -> Result<code_video::CodeVideoStatus, AppError> {
    match s {
        "QUEUING" => Ok(code_video::CodeVideoStatus::Queuing),
        "GENERATING" => Ok(code_video::CodeVideoStatus::Generating),
        "FINISHED" => Ok(code_video::CodeVideoStatus::Finished),
        "FAILED" => Ok(code_video::CodeVideoStatus::Failed),
        _ => Err(AppError::business(BusinessError::InvalidContentStatus)),
    }
}

fn validate_code_video_transition(
    from: code_video::CodeVideoStatus,
    to: code_video::CodeVideoStatus,
) -> Result<(), AppError> {
    use code_video::CodeVideoStatus::*;
    let valid = matches!(
        (from, to),
        (Queuing, Generating) | (Generating, Finished) | (Generating, Failed)
    );
    if !valid {
        return Err(AppError::business(BusinessError::InvalidContentStatus));
    }
    Ok(())
}

// --- interactive_html ---

async fn update_interactive_html(
    State(state): State<AppState>,
    service_auth: ServiceAuth,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if service_auth.service != ServiceKind::InteractiveHtml {
        return Err(AppError::business(BusinessError::InvalidApiKey));
    }

    let tx = state.db.begin().await?;
    let record = interactive_html::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    let new_status = parse_interactive_html_status(&payload.status)?;
    validate_interactive_html_transition(record.status, new_status)?;

    let mut active: interactive_html::ActiveModel = record.clone().into();
    active.status = Set(new_status);
    active.updated_at = Set(Utc::now());

    if new_status == interactive_html::InteractiveHtmlStatus::Finished {
        active.url = Set(payload.url);
    }

    if new_status == interactive_html::InteractiveHtmlStatus::Failed {
        let cost = state.config.interactive_html_gold_cost;
        let existing_user = user::Entity::find_by_id(record.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.gold = Set(active_user.gold.unwrap() + cost);
        active_user.updated_at = Set(Utc::now());
        active_user.update(&tx).await?;
    }

    active.update(&tx).await?;
    tx.commit().await?;

    Ok(ok(serde_json::json!({"id": id, "status": payload.status})))
}

fn parse_interactive_html_status(
    s: &str,
) -> Result<interactive_html::InteractiveHtmlStatus, AppError> {
    match s {
        "QUEUING" => Ok(interactive_html::InteractiveHtmlStatus::Queuing),
        "GENERATING" => Ok(interactive_html::InteractiveHtmlStatus::Generating),
        "FINISHED" => Ok(interactive_html::InteractiveHtmlStatus::Finished),
        "FAILED" => Ok(interactive_html::InteractiveHtmlStatus::Failed),
        _ => Err(AppError::business(BusinessError::InvalidContentStatus)),
    }
}

fn validate_interactive_html_transition(
    from: interactive_html::InteractiveHtmlStatus,
    to: interactive_html::InteractiveHtmlStatus,
) -> Result<(), AppError> {
    use interactive_html::InteractiveHtmlStatus::*;
    let valid = matches!(
        (from, to),
        (Queuing, Generating) | (Generating, Finished) | (Generating, Failed)
    );
    if !valid {
        return Err(AppError::business(BusinessError::InvalidContentStatus));
    }
    Ok(())
}

// --- knowledge_explanation ---

async fn update_knowledge_explanation(
    State(state): State<AppState>,
    service_auth: ServiceAuth,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if service_auth.service != ServiceKind::KnowledgeExplanation {
        return Err(AppError::business(BusinessError::InvalidApiKey));
    }

    let tx = state.db.begin().await?;
    let record = knowledge_explanation::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::ContentNotFound))?;

    let new_status = parse_knowledge_explanation_status(&payload.status)?;
    validate_knowledge_explanation_transition(record.status, new_status)?;

    let mut active: knowledge_explanation::ActiveModel = record.clone().into();
    active.status = Set(new_status);
    active.updated_at = Set(Utc::now());

    if new_status == knowledge_explanation::KnowledgeExplanationStatus::Finished {
        active.content = Set(payload.content);
        active.mindmap = Set(payload.mindmap);
    }

    if new_status == knowledge_explanation::KnowledgeExplanationStatus::Failed {
        let existing_user = user::Entity::find_by_id(record.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.gold = Set(active_user.gold.unwrap() + record.cost);
        active_user.updated_at = Set(Utc::now());
        active_user.update(&tx).await?;
    }

    active.update(&tx).await?;
    tx.commit().await?;

    Ok(ok(serde_json::json!({"id": id, "status": payload.status})))
}

fn parse_knowledge_explanation_status(
    s: &str,
) -> Result<knowledge_explanation::KnowledgeExplanationStatus, AppError> {
    match s {
        "QUEUING" => Ok(knowledge_explanation::KnowledgeExplanationStatus::Queuing),
        "GENERATING" => Ok(knowledge_explanation::KnowledgeExplanationStatus::Generating),
        "FINISHED" => Ok(knowledge_explanation::KnowledgeExplanationStatus::Finished),
        "FAILED" => Ok(knowledge_explanation::KnowledgeExplanationStatus::Failed),
        _ => Err(AppError::business(BusinessError::InvalidContentStatus)),
    }
}

fn validate_knowledge_explanation_transition(
    from: knowledge_explanation::KnowledgeExplanationStatus,
    to: knowledge_explanation::KnowledgeExplanationStatus,
) -> Result<(), AppError> {
    use knowledge_explanation::KnowledgeExplanationStatus::*;
    let valid = matches!(
        (from, to),
        (Queuing, Generating) | (Generating, Finished) | (Generating, Failed)
    );
    if !valid {
        return Err(AppError::business(BusinessError::InvalidContentStatus));
    }
    Ok(())
}

// --- study_subject callback ---

#[derive(Debug, Deserialize)]
struct StudySubjectCallbackRequest {
    status: String,
    #[serde(default)]
    problems: Option<Vec<CallbackProblem>>,
    #[serde(default)]
    stages: Option<Vec<CallbackStage>>,
}

#[derive(Debug, Deserialize)]
struct CallbackProblem {
    content: String,
    choice_a: String,
    choice_b: String,
    choice_c: String,
    choice_d: String,
    answer: String,
    explanation: String,
}

#[derive(Debug, Deserialize)]
struct CallbackStage {
    title: String,
    description: String,
    tasks: Vec<CallbackTask>,
}

#[derive(Debug, Deserialize)]
struct CallbackTask {
    title: String,
    description: String,
}

async fn callback_study_subject(
    State(state): State<AppState>,
    service_auth: ServiceAuth,
    Path(id): Path<i32>,
    Json(payload): Json<StudySubjectCallbackRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let is_pretest = service_auth.service == ServiceKind::Pretest;
    let is_plan = service_auth.service == ServiceKind::Plan;

    if !is_pretest && !is_plan {
        return Err(AppError::business(BusinessError::InvalidApiKey));
    }

    let tx = state.db.begin().await?;
    let subject = study_subject::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::StudySubjectNotFound))?;

    let callback_status = payload.status.as_str();
    let current = subject.status;
    let now = Utc::now();

    // Determine the target status based on the 8x6 transition table
    let new_status = if is_pretest {
        match (current, callback_status) {
            (StudySubjectStatus::PretestQueuing, "GENERATING") => {
                StudySubjectStatus::PretestGenerating
            }
            (StudySubjectStatus::PretestQueuing, "FINISHED") => StudySubjectStatus::PretestReady,
            (StudySubjectStatus::PretestQueuing, "FAILED") => StudySubjectStatus::Failed,
            (StudySubjectStatus::PretestGenerating, "FINISHED") => StudySubjectStatus::PretestReady,
            (StudySubjectStatus::PretestGenerating, "FAILED") => StudySubjectStatus::Failed,
            _ => return Err(AppError::business(BusinessError::InvalidStudySubjectStatus)),
        }
    } else {
        // is_plan
        match (current, callback_status) {
            (StudySubjectStatus::PlanQueuing, "GENERATING") => StudySubjectStatus::PlanGenerating,
            (StudySubjectStatus::PlanQueuing, "FINISHED") => StudySubjectStatus::Studying,
            (StudySubjectStatus::PlanQueuing, "FAILED") => StudySubjectStatus::Failed,
            (StudySubjectStatus::PlanGenerating, "FINISHED") => StudySubjectStatus::Studying,
            (StudySubjectStatus::PlanGenerating, "FAILED") => StudySubjectStatus::Failed,
            _ => return Err(AppError::business(BusinessError::InvalidStudySubjectStatus)),
        }
    };

    let mut active: study_subject::ActiveModel = subject.clone().into();
    active.status = Set(new_status);
    active.updated_at = Set(now);

    // Handle FAILED → refund
    if new_status == StudySubjectStatus::Failed {
        let cost = state.config.study_subject_diamond_cost;
        let existing_user = user::Entity::find_by_id(subject.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.diamond = Set(active_user.diamond.unwrap() + cost);
        active_user.updated_at = Set(now);
        active_user.update(&tx).await?;
    }

    // Handle Pretest FINISHED → create problems
    if new_status == StudySubjectStatus::PretestReady {
        let problems = payload
            .problems
            .ok_or_else(|| AppError::internal("pretest FINISHED callback missing problems data"))?;

        for (i, p) in problems.into_iter().enumerate() {
            let answer = parse_problem_answer(&p.answer)?;
            let problem_record = problem::ActiveModel {
                user_id: Set(subject.user_id),
                content: Set(p.content),
                choice_a: Set(p.choice_a),
                choice_b: Set(p.choice_b),
                choice_c: Set(p.choice_c),
                choice_d: Set(p.choice_d),
                answer: Set(answer),
                explanation: Set(p.explanation),
                bookmarked: Set(false),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(&tx)
            .await?;

            pretest_problem::ActiveModel {
                study_subject_id: Set(subject.id),
                problem_id: Set(problem_record.id),
                sort_order: Set(i as i32),
                confidence: Set(None),
                chosen_answer: Set(None),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(&tx)
            .await?;
        }
    }

    // Handle Plan FINISHED → create stages and tasks + initialize unlock
    if new_status == StudySubjectStatus::Studying {
        let stages = payload
            .stages
            .ok_or_else(|| AppError::internal("plan FINISHED callback missing stages data"))?;

        let total_stages = stages.len() as i32;
        active.total_stages = Set(total_stages);

        for (si, s) in stages.into_iter().enumerate() {
            let is_first_stage = si == 0;
            let stage_status = if is_first_stage {
                StudyStageStatus::Studying
            } else {
                StudyStageStatus::Locked
            };

            let total_tasks = s.tasks.len() as i32;

            let stage_record = study_stage::ActiveModel {
                study_subject_id: Set(subject.id),
                title: Set(s.title),
                description: Set(s.description),
                sort_order: Set(si as i32),
                status: Set(stage_status),
                total_tasks: Set(total_tasks),
                finished_tasks: Set(0),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(&tx)
            .await?;

            for (ti, t) in s.tasks.into_iter().enumerate() {
                let is_first_task = is_first_stage && ti == 0;
                let task_status = if is_first_task {
                    StudyTaskStatus::Studying
                } else {
                    StudyTaskStatus::Locked
                };

                study_task::ActiveModel {
                    study_stage_id: Set(stage_record.id),
                    title: Set(t.title),
                    description: Set(t.description),
                    sort_order: Set(ti as i32),
                    status: Set(task_status),
                    knowledge_video_id: Set(None),
                    interactive_html_id: Set(None),
                    knowledge_explanation_id: Set(None),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(&tx)
                .await?;
            }
        }
    }

    active.update(&tx).await?;
    tx.commit().await?;

    Ok(ok(
        serde_json::json!({"id": id, "status": format!("{:?}", new_status)}),
    ))
}

fn parse_problem_answer(s: &str) -> Result<ProblemAnswer, AppError> {
    match s {
        "A" => Ok(ProblemAnswer::A),
        "B" => Ok(ProblemAnswer::B),
        "C" => Ok(ProblemAnswer::C),
        "D" => Ok(ProblemAnswer::D),
        _ => Err(AppError::internal(format!("invalid problem answer: {s}"))),
    }
}

// --- study_quiz callback ---

#[derive(Debug, Deserialize)]
struct StudyQuizCallbackRequest {
    status: String,
    #[serde(default)]
    problems: Option<Vec<CallbackProblem>>,
}

async fn callback_study_quiz(
    State(state): State<AppState>,
    service_auth: ServiceAuth,
    Path(id): Path<i32>,
    Json(payload): Json<StudyQuizCallbackRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    if service_auth.service != ServiceKind::Quiz {
        return Err(AppError::business(BusinessError::InvalidApiKey));
    }

    let tx = state.db.begin().await?;
    let quiz = study_quiz::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::business(BusinessError::QuizNotFound))?;

    let callback_status = payload.status.as_str();
    let now = Utc::now();

    let new_status = match (quiz.status, callback_status) {
        (StudyQuizStatus::Queuing, "GENERATING") => StudyQuizStatus::Generating,
        (StudyQuizStatus::Queuing, "FINISHED") => StudyQuizStatus::Ready,
        (StudyQuizStatus::Queuing, "FAILED") => StudyQuizStatus::Failed,
        (StudyQuizStatus::Generating, "FINISHED") => StudyQuizStatus::Ready,
        (StudyQuizStatus::Generating, "FAILED") => StudyQuizStatus::Failed,
        _ => return Err(AppError::business(BusinessError::InvalidStudyQuizStatus)),
    };

    let mut active: study_quiz::ActiveModel = quiz.clone().into();
    active.status = Set(new_status);
    active.updated_at = Set(now);

    // Handle FAILED → refund cost gold
    if new_status == StudyQuizStatus::Failed && quiz.cost > 0 {
        // Find the user through the join chain
        let task = study_task::Entity::find_by_id(quiz.study_task_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::TaskNotFound))?;
        let stage = study_stage::Entity::find_by_id(task.study_stage_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::StageNotFound))?;
        let subject = study_subject::Entity::find_by_id(stage.study_subject_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::StudySubjectNotFound))?;

        let existing_user = user::Entity::find_by_id(subject.user_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::UserNotFound))?;
        let mut active_user: user::ActiveModel = existing_user.into();
        active_user.gold = Set(active_user.gold.unwrap() + quiz.cost);
        active_user.updated_at = Set(now);
        active_user.update(&tx).await?;
    }

    // Handle FINISHED → create problems
    if new_status == StudyQuizStatus::Ready {
        let problems = payload
            .problems
            .ok_or_else(|| AppError::internal("quiz FINISHED callback missing problems data"))?;

        // Find user_id through the join chain
        let task = study_task::Entity::find_by_id(quiz.study_task_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::TaskNotFound))?;
        let stage = study_stage::Entity::find_by_id(task.study_stage_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::StageNotFound))?;
        let subject = study_subject::Entity::find_by_id(stage.study_subject_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::business(BusinessError::StudySubjectNotFound))?;

        let total = problems.len() as i32;
        active.total_problems = Set(total);

        for (i, p) in problems.into_iter().enumerate() {
            let answer = parse_problem_answer(&p.answer)?;
            let problem_record = problem::ActiveModel {
                user_id: Set(subject.user_id),
                content: Set(p.content),
                choice_a: Set(p.choice_a),
                choice_b: Set(p.choice_b),
                choice_c: Set(p.choice_c),
                choice_d: Set(p.choice_d),
                answer: Set(answer),
                explanation: Set(p.explanation),
                bookmarked: Set(false),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(&tx)
            .await?;

            study_quiz_problem::ActiveModel {
                study_quiz_id: Set(quiz.id),
                problem_id: Set(problem_record.id),
                sort_order: Set(i as i32),
                chosen_answer: Set(None),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(&tx)
            .await?;
        }
    }

    active.update(&tx).await?;
    tx.commit().await?;

    Ok(ok(
        serde_json::json!({"id": id, "status": format!("{:?}", new_status)}),
    ))
}
