use redis::Client as RedisClient;
use sqlx::{PgPool, SqlitePool, postgres::PgPoolOptions, sqlite::SqlitePoolOptions, raw_sql};
use tracing::{info, warn};

use crate::{
    config::{AppConfig, StorageBackend},
    error::AppError,
};

#[derive(Debug, Clone)]
pub struct PersistenceHandles {
    pub backend: StorageBackend,
    pub postgres: Option<PgPool>,
    pub sqlite: Option<SqlitePool>,
    pub redis: Option<RedisClient>,
}

impl PersistenceHandles {
    pub fn memory() -> Self {
        Self {
            backend: StorageBackend::Memory,
            postgres: None,
            sqlite: None,
            redis: None,
        }
    }
}

pub async fn initialize_persistence(config: &AppConfig) -> Result<PersistenceHandles, AppError> {
    let mut handles = PersistenceHandles {
        backend: config.storage_backend,
        postgres: None,
        sqlite: None,
        redis: None,
    };

    match config.storage_backend {
        StorageBackend::Postgres => {
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
        StorageBackend::Sqlite => {
            let sqlite_url = format!("sqlite://{}?mode=rwc", config.sqlite_path);
            let pool = SqlitePoolOptions::new()
                .max_connections(4)
                .connect(&sqlite_url)
                .await
                .map_err(|err| AppError::internal(format!("初始化 SQLite 数据库失败: {err}")))?;

            // 开启 WAL 模式，提升并发性能
            sqlx::query("PRAGMA journal_mode=WAL;")
                .execute(&pool)
                .await
                .map_err(|err| AppError::internal(format!("开启 SQLite WAL 模式失败: {err}")))?;

            sqlx::query("PRAGMA foreign_keys=ON;")
                .execute(&pool)
                .await
                .map_err(|err| AppError::internal(format!("开启 SQLite 外键约束失败: {err}")))?;

            apply_sqlite_migrations(&pool).await?;
            handles.sqlite = Some(pool);
        }
        StorageBackend::Memory => {}
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
        sqlite_enabled = handles.sqlite.is_some(),
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
        (
            "0012_products_track_batches",
            include_str!("../migrations/0012_products_track_batches.sql"),
        ),
        (
            "0013_batches_supplier",
            include_str!("../migrations/0013_batches_supplier.sql"),
        ),
        (
            "0014_suppliers",
            include_str!("../migrations/0014_suppliers.sql"),
        ),
        (
            "0015_drop_version_columns",
            include_str!("../migrations/0015_drop_version_columns.sql"),
        ),
        (
            "0016_serial_numbers",
            include_str!("../migrations/0016_serial_numbers.sql"),
        ),
        (
            "0017_restore_version_columns",
            include_str!("../migrations/0017_restore_version_columns.sql"),
        ),
    ];

    for (name, sql) in migrations {
        let already_applied: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = $1)",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

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

async fn apply_sqlite_migrations(pool: &SqlitePool) -> Result<(), AppError> {
    let migrations = [
        ("0001_init",                include_str!("../migrations/sqlite/0001_init.sql")),
        ("0002_core_tables",         include_str!("../migrations/sqlite/0002_core_tables.sql")),
        ("0003_business_tables",     include_str!("../migrations/sqlite/0003_business_tables.sql")),
        ("0004_barcode_lookup_cache",include_str!("../migrations/sqlite/0004_barcode_lookup_cache.sql")),
        ("0005_stock_logs_snapshot_sell_price",                     include_str!("../migrations/sqlite/0005_stock_logs_snapshot_sell_price.sql")),
        ("0006_order_item_product_name_snapshot",                   include_str!("../migrations/sqlite/0006_order_item_product_name_snapshot.sql")),
        ("0007_stock_logs_snapshot_inbound_unit_cost",              include_str!("../migrations/sqlite/0007_stock_logs_snapshot_inbound_unit_cost.sql")),
        ("0008_products_last_inbound_unit_cost_drop_wholesale_price",include_str!("../migrations/sqlite/0008_products_last_inbound_unit_cost_drop_wholesale_price.sql")),
        ("0009_tenants_tenant_code_login",  include_str!("../migrations/sqlite/0009_tenants_tenant_code_login.sql")),
        ("0010_categories",                 include_str!("../migrations/sqlite/0010_categories.sql")),
        ("0011_product_batches",            include_str!("../migrations/sqlite/0011_product_batches.sql")),
        ("0012_products_track_batches",     include_str!("../migrations/sqlite/0012_products_track_batches.sql")),
        ("0013_batches_supplier",           include_str!("../migrations/sqlite/0013_batches_supplier.sql")),
        ("0014_suppliers",                  include_str!("../migrations/sqlite/0014_suppliers.sql")),
        ("0015_drop_version_columns",       include_str!("../migrations/sqlite/0015_drop_version_columns.sql")),
        ("0016_serial_numbers",             include_str!("../migrations/sqlite/0016_serial_numbers.sql")),
        ("0017_restore_version_columns",    include_str!("../migrations/sqlite/0017_restore_version_columns.sql")),
    ];

    // 先执行第一个迁移（建 schema_migrations 表），不依赖该表
    let (first_name, first_sql) = migrations[0];
    sqlx::raw_sql(first_sql)
        .execute(pool)
        .await
        .map_err(|err| AppError::internal(format!("执行 SQLite 迁移 {first_name} 失败: {err}")))?;

    for (name, sql) in &migrations[1..] {
        let already_applied: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        if already_applied > 0 {
            tracing::debug!(migration = name, "skipping already applied sqlite migration");
            continue;
        }

        tracing::info!(migration = name, "applying sqlite migration");
        sqlx::raw_sql(sql)
            .execute(pool)
            .await
            .map_err(|err| AppError::internal(format!("执行 SQLite 迁移 {name} 失败: {err}")))?;
    }

    info!("sqlite migrations applied");
    Ok(())
}
