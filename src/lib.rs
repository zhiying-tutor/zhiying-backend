pub mod auth;
pub mod config;
pub mod entities;
pub mod error;
pub mod migration;
pub mod response;
pub mod routes;
pub mod services;
pub mod state;

use axum::Router;
use sea_orm::{Database, DbErr};
use sea_orm_migration::MigratorTrait;

use crate::{config::Config, migration::Migrator, state::AppState};

pub async fn build_app(config: Config) -> Result<Router, DbErr> {
    let database = Database::connect(&config.database_url).await?;
    Migrator::up(&database, None).await?;

    Ok(routes::build_router(AppState::new(config, database)))
}
