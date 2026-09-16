mod app;
mod config;
mod error;
mod extractors;
mod middleware;
mod models;
mod persistence;
mod repository;
mod repository_sqlite;
mod response;
mod routes;
mod state;

use crate::app::build_router;
use crate::config::{AppConfig, StorageBackend};
use crate::persistence::initialize_persistence;
use crate::repository::build_repository_provider;
use crate::state::AppState;
use crate::models::{Tenant, User, UserRole, hash_password};
use uuid::Uuid;
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
        sqlite_enabled = state.persistence.sqlite.is_some(),
        redis_enabled = state.persistence.redis.is_some(),
        "storage initialized"
    );

    if config.storage_backend == StorageBackend::Sqlite {
        ensure_sqlite_initialized(&state).await;
    }

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

async fn ensure_sqlite_initialized(state: &AppState) {
    let tenant_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let tenant = Tenant {
        id: tenant_id,
        code: "local".to_string(),
        name: "本地企业".to_string(),
    };

    // 检查 admin 是否存在
    match state.repository.find_user_by_tenant_code_and_username(&state.persistence, "local", "admin").await {
        Ok(None) => {
            info!("initializing sqlite standalone mode...");
            let _ = state.repository.create_tenant(&state.persistence, &tenant).await;
            
            let admin = User {
                id: Uuid::new_v4(),
                tenant_id,
                username: "admin".to_string(),
                name: "管理员".to_string(),
                role: UserRole::Admin,
                password_hash: hash_password("admin123"),
            };
            if let Err(e) = state.repository.create_user(&state.persistence, &admin).await {
                tracing::error!("failed to create default admin: {:?}", e);
            } else {
                info!("sqlite standalone mode initialized (admin/admin123)");
            }
        }
        Err(e) => {
            tracing::error!("failed to check existing admin: {:?}", e);
        }
        _ => {}
    }
}
