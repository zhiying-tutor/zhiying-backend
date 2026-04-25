mod checkins;
mod code_videos;
mod interactive_htmls;
mod internal;
mod knowledge_explanations;
mod knowledge_videos;
mod me;
mod placeholders;
mod problems;
mod public_config;
mod study_quizzes;
mod study_stages;
mod study_subjects;
mod study_tasks;
mod tokens;
mod user_views;
mod users;

use axum::{Router, extract::State, http::HeaderValue, routing::get};
use serde::Serialize;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{response::ok, state::AppState};

pub fn build_router(state: AppState) -> Router {
    let cors = if state.config.cors_allow_origin == "*" {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origin: HeaderValue = state
            .config
            .cors_allow_origin
            .parse()
            .expect("CORS_ALLOW_ORIGIN must be a valid header value");

        CorsLayer::new()
            .allow_origin(origin)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    Router::new()
        .route("/health", get(health))
        .nest("/api/v1", api_router())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

fn api_router() -> Router<AppState> {
    Router::new()
        .route("/config", get(public_config::get_config))
        .route("/users", axum::routing::post(users::create_user))
        .route("/tokens", axum::routing::post(tokens::create_token))
        .route("/me", get(me::get_me).patch(me::update_me))
        .route(
            "/checkins",
            axum::routing::post(checkins::check_in).get(checkins::list_checkins),
        )
        .route(
            "/knowledge-videos",
            axum::routing::post(knowledge_videos::create),
        )
        .route(
            "/knowledge-videos/{id}",
            get(knowledge_videos::get_by_id).patch(knowledge_videos::update),
        )
        .route("/code-videos", axum::routing::post(code_videos::create))
        .route(
            "/code-videos/{id}",
            get(code_videos::get_by_id).patch(code_videos::update),
        )
        .route(
            "/interactive-htmls",
            axum::routing::post(interactive_htmls::create),
        )
        .route(
            "/interactive-htmls/{id}",
            get(interactive_htmls::get_by_id).patch(interactive_htmls::update),
        )
        .route(
            "/knowledge-explanations",
            axum::routing::post(knowledge_explanations::create),
        )
        .route(
            "/knowledge-explanations/{id}",
            get(knowledge_explanations::get_by_id).patch(knowledge_explanations::update),
        )
        // Study subjects
        .route(
            "/study-subjects",
            axum::routing::post(study_subjects::create).get(study_subjects::list),
        )
        .route("/study-subjects/{id}", get(study_subjects::get_by_id))
        .route(
            "/study-subjects/{id}/pretest",
            get(study_subjects::get_pretest),
        )
        .route(
            "/study-subjects/{id}/pretest/{pretest_problem_id}",
            axum::routing::patch(study_subjects::update_pretest_problem),
        )
        .route(
            "/study-subjects/{id}/plan",
            axum::routing::post(study_subjects::create_plan),
        )
        // Study stages
        .route("/study-stages/{id}", get(study_stages::get_by_id))
        // Study tasks
        .route("/study-tasks/{id}", get(study_tasks::get_by_id))
        .route(
            "/study-tasks/{id}/complete",
            axum::routing::post(study_tasks::complete),
        )
        .route(
            "/study-tasks/{id}/knowledge-video",
            axum::routing::post(study_tasks::create_knowledge_video),
        )
        .route(
            "/study-tasks/{id}/interactive-html",
            axum::routing::post(study_tasks::create_interactive_html),
        )
        .route(
            "/study-tasks/{id}/explanation",
            axum::routing::post(study_tasks::create_explanation),
        )
        .route(
            "/study-tasks/{id}/quizzes",
            axum::routing::post(study_tasks::create_quiz).get(study_tasks::list_quizzes),
        )
        // Study quizzes
        .route("/study-quizzes/{id}", get(study_quizzes::get_by_id))
        .route(
            "/study-quizzes/{quiz_id}/problems/{study_quiz_problem_id}",
            axum::routing::patch(study_quizzes::update_problem),
        )
        .route(
            "/study-quizzes/{id}/submit",
            axum::routing::post(study_quizzes::submit),
        )
        // Problems
        .route("/problems", get(problems::list))
        .route(
            "/problems/{id}/bookmark",
            axum::routing::patch(problems::toggle_bookmark),
        )
        .nest("/internal", internal::router())
        .merge(placeholders::router())
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    database_url_scheme: String,
}

async fn health(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    let scheme = state
        .config
        .database_url
        .split(':')
        .next()
        .unwrap_or("unknown")
        .to_owned();

    ok(HealthResponse {
        status: "ok",
        database_url_scheme: scheme,
    })
}
