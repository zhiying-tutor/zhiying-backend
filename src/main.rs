mod auth;
mod config;
mod entities;
mod error;
mod migration;
mod response;
mod routes;
mod services;
mod state;

use std::net::SocketAddr;

use sea_orm::Database;
use sea_orm_migration::MigratorTrait;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{config::Config, migration::Migrator, routes::build_router, state::AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    init_tracing();

    let config = Config::from_env()?;
    let database = Database::connect(&config.database_url).await?;
    Migrator::up(&database, None).await?;
    let state = AppState::new(config.clone(), database);

    let app = build_router(state);
    let addr = SocketAddr::new(config.host, config.port);
    let listener = TcpListener::bind(addr).await?;

    info!("server listening on {}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "zhiying_backend=debug,tower_http=info,axum=info".into());

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install terminate handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
