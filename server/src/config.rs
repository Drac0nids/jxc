use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    Memory,
    Postgres,
}

impl StorageBackend {
    pub fn from_env(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "memory" => Self::Memory,
            "postgres" | "postgresql" | "pg" => Self::Postgres,
            _ => Self::Postgres,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Postgres => "postgres",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub access_token_exp_secs: i64,
    pub refresh_token_exp_secs: i64,
    pub allow_negative_stock: bool,
    pub storage_backend: StorageBackend,
    pub database_url: Option<String>,
    pub redis_url: Option<String>,
    pub postgres_max_connections: u32,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let host = env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("APP_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8080);

        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "change_me_in_prod".to_string());
        let access_token_exp_secs = env::var("JWT_ACCESS_EXPIRES")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(7200);
        let refresh_token_exp_secs = env::var("JWT_REFRESH_EXPIRES")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(604800);

        let allow_negative_stock = env::var("ALLOW_NEGATIVE_STOCK")
            .ok()
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);

        let storage_backend = env::var("STORAGE_BACKEND")
            .ok()
            .map(|v| StorageBackend::from_env(&v))
            .unwrap_or(StorageBackend::Postgres);

        let database_url = env::var("DATABASE_URL").ok().and_then(non_empty);
        let redis_url = env::var("REDIS_URL").ok().and_then(non_empty);
        let postgres_max_connections = env::var("PG_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(10);

        Self {
            host,
            port,
            jwt_secret,
            access_token_exp_secs,
            refresh_token_exp_secs,
            allow_negative_stock,
            storage_backend,
            database_url,
            redis_url,
            postgres_max_connections,
        }
    }
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
