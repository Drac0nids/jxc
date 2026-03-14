use redis::Client as RedisClient;
use sqlx::{PgPool, postgres::PgPoolOptions, raw_sql};
use tracing::{info, warn};

use crate::{
    config::{AppConfig, StorageBackend},
    error::AppError,
};

#[derive(Debug, Clone)]
pub struct PersistenceHandles {
    pub backend: StorageBackend,
    pub postgres: Option<PgPool>,
    pub redis: Option<RedisClient>,
}

impl PersistenceHandles {
    pub fn memory() -> Self {
        Self {
            backend: StorageBackend::Memory,
            postgres: None,
            redis: None,
        }
    }
}

pub async fn initialize_persistence(config: &AppConfig) -> Result<PersistenceHandles, AppError> {
    let mut handles = PersistenceHandles {
        backend: config.storage_backend,
        postgres: None,
        redis: None,
    };

    if matches!(config.storage_backend, StorageBackend::Postgres) {
        let database_url = config.database_url.as_deref().ok_or_else(|| {
            AppError::internal("STORAGE_BACKEND=postgres 时必须配置 DATABASE_URL")
        })?;

        let pool = PgPoolOptions::new()
            .max_connections(config.postgres_max_connections)
            .connect(database_url)
            .await
            .map_err(|err| AppError::internal(format!("初始化 PostgreSQL 连接池失败: {err}")))?;

        apply_postgres_migrations(&pool).await?;

        handles.postgres = Some(pool);
    }

    if let Some(redis_url) = config.redis_url.as_deref() {
        let redis_client = RedisClient::open(redis_url)
            .map_err(|err| AppError::internal(format!("初始化 Redis 客户端失败: {err}")))?;
        handles.redis = Some(redis_client);
    } else if matches!(config.storage_backend, StorageBackend::Postgres) {
        warn!("STORAGE_BACKEND=postgres 但 REDIS_URL 未配置，将继续使用内存态幂等缓存");
    }

    info!(
        storage_backend = config.storage_backend.as_str(),
        backend = handles.backend.as_str(),
        postgres_enabled = handles.postgres.is_some(),
        redis_enabled = handles.redis.is_some(),
        "persistence bootstrap completed"
    );

    Ok(handles)
}

async fn apply_postgres_migrations(pool: &PgPool) -> Result<(), AppError> {
    let migrations = [
        ("0001_init", include_str!("../migrations/0001_init.sql")),
        (
            "0002_core_tables",
            include_str!("../migrations/0002_core_tables.sql"),
        ),
        (
            "0003_business_tables",
            include_str!("../migrations/0003_business_tables.sql"),
        ),
        (
            "0004_barcode_lookup_cache",
            include_str!("../migrations/0004_barcode_lookup_cache.sql"),
        ),
        (
            "0005_stock_logs_snapshot_sell_price",
            include_str!("../migrations/0005_stock_logs_snapshot_sell_price.sql"),
        ),
        (
            "0006_order_item_product_name_snapshot",
            include_str!("../migrations/0006_order_item_product_name_snapshot.sql"),
        ),
        (
            "0007_stock_logs_snapshot_inbound_unit_cost",
            include_str!("../migrations/0007_stock_logs_snapshot_inbound_unit_cost.sql"),
        ),
        (
            "0008_products_last_inbound_unit_cost_drop_wholesale_price",
            include_str!("../migrations/0008_products_last_inbound_unit_cost_drop_wholesale_price.sql"),
        ),
        (
            "0009_tenants_tenant_code_login",
            include_str!("../migrations/0009_tenants_tenant_code_login.sql"),
        ),
        (
            "0010_categories",
            include_str!("../migrations/0010_categories.sql"),
        ),
        (
            "0011_product_batches",
            include_str!("../migrations/0011_product_batches.sql"),
        ),
    ];

    for (name, sql) in migrations {
        // 先检查是否已经执行过，避免重复执行导致 DDL 冲突
        let already_applied: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = $1)",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap_or(false); // schema_migrations 表本身不存在时走 0001_init 创建

        if already_applied {
            tracing::debug!(migration = name, "skipping already applied migration");
            continue;
        }

        tracing::info!(migration = name, "applying migration");
        raw_sql(sql)
            .execute(pool)
            .await
            .map_err(|err| AppError::internal(format!("执行迁移 {name} 失败: {err}")))?;
    }

    info!("postgres migrations applied");
    Ok(())
}
