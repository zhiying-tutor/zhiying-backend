use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::services::message_queue::MessagePublisher;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: DatabaseConnection,
    pub publisher: Arc<dyn MessagePublisher>,
}

impl AppState {
    pub fn new(
        config: Config,
        db: DatabaseConnection,
        publisher: Arc<dyn MessagePublisher>,
    ) -> Self {
        Self {
            config,
            db,
            publisher,
        }
    }
}
