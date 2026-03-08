mod app;
mod config;
mod error;
mod middleware;
mod models;
mod persistence;
mod repository;
mod response;
mod routes;
mod state;

use crate::app::build_router;
use crate::config::AppConfig;
use crate::persistence::initialize_persistence;
use crate::repository::build_repository_provider;
use crate::state::AppState;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = AppConfig::from_env();
    let persistence = initialize_persistence(&config)
        .await
        .unwrap_or_else(|err| panic!("failed to initialize persistence: {}", err.message));
    let repository = build_repository_provider(config.storage_backend);
    let state = AppState::new(config.clone()).with_runtime_infra(persistence, repository);

    info!(
        storage_backend = config.storage_backend.as_str(),
        repository = state.repository.name(),
        postgres_enabled = state.persistence.postgres.is_some(),
        redis_enabled = state.persistence.redis.is_some(),
        "storage initialized"
    );

    let app = build_router(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect("failed to bind tcp listener");

    info!("server listening on {}", addr);
    axum::serve(listener, app)
        .await
        .expect("failed to start axum server");
}
