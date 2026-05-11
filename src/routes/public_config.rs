use axum::extract::State;
use serde::Serialize;

use crate::{config::Config, response::ok, state::AppState};

#[derive(Debug, Serialize)]
pub struct PublicConfig {
    study_subject: StudySubjectConfig,
    storage: StorageConfig,
    resource: ResourceConfig,
    checkin: CheckinConfig,
}

#[derive(Debug, Serialize)]
pub struct CheckinConfig {
    reward_sequence: Vec<i32>,
    makeup_gold_cost_per_day: i32,
    makeup_diamond_cost: i32,
}

#[derive(Debug, Serialize)]
pub struct StudySubjectConfig {
    pricing: Vec<StudySubjectPricingItem>,
}

#[derive(Debug, Serialize)]
pub struct StudySubjectPricingItem {
    total_stages: i32,
    diamond_cost: i32,
}

#[derive(Debug, Serialize)]
pub struct StorageConfig {
    public_base: String,
    bucket: String,
}

#[derive(Debug, Serialize)]
pub struct ResourceConfig {
    knowledge_video_diamond_cost: i32,
    code_video_diamond_cost: i32,
    interactive_html_gold_cost: i32,
    study_quiz_free_limit_per_task: i32,
    study_quiz_extra_gold_cost: i32,
}

fn build_public_config(config: &Config) -> PublicConfig {
    let pricing = config
        .study_subject_diamond_costs
        .iter()
        .map(|(&total_stages, &diamond_cost)| StudySubjectPricingItem {
            total_stages,
            diamond_cost,
        })
        .collect();

    PublicConfig {
        study_subject: StudySubjectConfig { pricing },
        storage: StorageConfig {
            public_base: config.storage_public_base.clone(),
            bucket: config.storage_bucket.clone(),
        },
        resource: ResourceConfig {
            knowledge_video_diamond_cost: config.knowledge_video_diamond_cost,
            code_video_diamond_cost: config.code_video_diamond_cost,
            interactive_html_gold_cost: config.interactive_html_gold_cost,
            study_quiz_free_limit_per_task: config.study_quiz_free_limit_per_task,
            study_quiz_extra_gold_cost: config.study_quiz_extra_gold_cost,
        },
        checkin: CheckinConfig {
            reward_sequence: config.checkin_reward_sequence.clone(),
            makeup_gold_cost_per_day: config.checkin_makeup_gold_cost_per_day,
            makeup_diamond_cost: config.checkin_makeup_diamond_cost,
        },
    }
}

/// GET /api/v1/config
pub async fn get_config(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    ok(build_public_config(&state.config))
}
