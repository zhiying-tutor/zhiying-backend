pub mod auth;
pub mod config;
pub mod entities;
pub mod error;
pub mod migration;
pub mod response;
pub mod routes;
pub mod services;
pub mod state;

use std::sync::Arc;

use axum::Router;
use deadpool_lapin::lapin::ConnectionProperties;
use deadpool_lapin::{Manager, Pool, Runtime};
use sea_orm::{Database, DbErr};
use sea_orm_migration::MigratorTrait;

use crate::services::message_queue::{
    LapinPublisher, MessagePublisher, ROUTING_KEY_GENERATE, TopologyEntry, declare_topology,
};
use crate::{config::Config, migration::Migrator, state::AppState};

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error(transparent)]
    Db(#[from] DbErr),
    #[error("rabbitmq error: {0}")]
    RabbitMq(String),
}

pub async fn build_app(config: Config) -> Result<Router, StartupError> {
    let database = Database::connect(&config.database_url).await?;
    Migrator::up(&database, None).await?;

    let pool = build_rabbitmq_pool(&config.rabbitmq_url)
        .map_err(|err| StartupError::RabbitMq(err.to_string()))?;
    let entries = topology_entries(&config);
    declare_topology(&pool, &entries)
        .await
        .map_err(|err| StartupError::RabbitMq(err.to_string()))?;
    let publisher: Arc<dyn MessagePublisher> = Arc::new(LapinPublisher::new(pool));

    Ok(routes::build_router(AppState::new(
        config, database, publisher,
    )))
}

pub async fn build_app_with_publisher(
    config: Config,
    publisher: Arc<dyn MessagePublisher>,
) -> Result<Router, DbErr> {
    let database = Database::connect(&config.database_url).await?;
    Migrator::up(&database, None).await?;
    Ok(routes::build_router(AppState::new(
        config, database, publisher,
    )))
}

fn build_rabbitmq_pool(url: &str) -> Result<Pool, deadpool_lapin::BuildError> {
    let manager = Manager::new(url.to_owned(), ConnectionProperties::default());
    Pool::builder(manager).runtime(Runtime::Tokio1).build()
}

fn topology_entries(config: &Config) -> Vec<TopologyEntry<'_>> {
    [
        config.knowledge_video_exchange.as_str(),
        config.code_video_exchange.as_str(),
        config.interactive_html_exchange.as_str(),
        config.knowledge_explanation_exchange.as_str(),
        config.pretest_exchange.as_str(),
        config.plan_exchange.as_str(),
        config.quiz_exchange.as_str(),
    ]
    .into_iter()
    .map(|exchange| TopologyEntry {
        exchange,
        queue_suffix: "generate",
        routing_key: ROUTING_KEY_GENERATE,
    })
    .collect()
}
