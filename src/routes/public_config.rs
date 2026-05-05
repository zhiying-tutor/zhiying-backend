use axum::extract::State;
use serde::Serialize;

use crate::{config::Config, response::ok, state::AppState};

#[derive(Debug, Serialize)]
pub struct PublicConfig {
    study_subject: StudySubjectConfig,
    storage: StorageConfig,
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
    }
}

/// GET /api/v1/config
pub async fn get_config(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    ok(build_public_config(&state.config))
}
