use sea_orm::DatabaseConnection;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: DatabaseConnection,
}

impl AppState {
    pub fn new(config: Config, db: DatabaseConnection) -> Self {
        Self { config, db }
    }
}
