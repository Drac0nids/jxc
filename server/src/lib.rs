//! 极速云进销存服务端。
//!
//! 同时提供两种使用方式：
//!
//! 1. 独立进程：`src/main.rs` 读取环境变量后调用 [`serve`]，用于 SaaS 部署与桌面单机版的 sidecar。
//! 2. 进程内嵌：Android 单机版把本 crate 编译为 `cdylib`（`libjxc_server.so`），
//!    由 Flutter 侧通过 FFI 调用 [`jxc_start_server`] 在 App 进程内启动同一套
//!    Axum 路由 + SQLite 存储，从而无需任何服务端部署。

pub mod app;
pub mod config;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod models;
pub mod persistence;
pub mod repository;
pub mod repository_sqlite;
pub mod response;
pub mod routes;
pub mod state;

use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use tokio::net::TcpListener;
use tracing::info;

use crate::app::build_router;
use crate::config::{AppConfig, StorageBackend};
use crate::models::{Tenant, User, UserRole, hash_password};
use crate::persistence::initialize_persistence;
use crate::repository::build_repository_provider;
use crate::state::AppState;

/// 进程内嵌模式下已启动的服务端口（0 表示尚未启动）。
static EMBEDDED_PORT: AtomicI32 = AtomicI32::new(0);

/// 启动 HTTP 服务并一直运行到进程退出。
///
/// - `ready`：可选的启动结果回调。绑定成功后回传**实际监听端口**
///   （单机版使用端口 0 由系统分配，避免与其它应用冲突）。
pub async fn serve(
    config: AppConfig,
    ready: Option<std::sync::mpsc::Sender<Result<u16, String>>>,
) -> Result<(), String> {
    let persistence = initialize_persistence(&config)
        .await
        .map_err(|err| format!("failed to initialize persistence: {}", err.message))?;
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
        .map_err(|err| format!("failed to bind {addr}: {err}"))?;
    let port = listener
        .local_addr()
        .map_err(|err| format!("failed to read local addr: {err}"))?
        .port();

    if let Some(tx) = ready {
        let _ = tx.send(Ok(port));
    }

    info!("server listening on {}:{}", config.host, port);
    axum::serve(listener, app)
        .await
        .map_err(|err| format!("server runtime error: {err}"))
}

/// 单机模式首次启动时创建本地租户与初始管理员账号。
pub async fn ensure_sqlite_initialized(state: &AppState) {
    let tenant_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let tenant = Tenant {
        id: tenant_id,
        code: "local".to_string(),
        name: "本地企业".to_string(),
    };

    match state
        .repository
        .find_user_by_tenant_code_and_username(&state.persistence, "local", "admin")
        .await
    {
        Ok(None) => {
            info!("initializing sqlite standalone mode...");
            let _ = state
                .repository
                .create_tenant(&state.persistence, &tenant)
                .await;

            let admin = User {
                id: uuid::Uuid::new_v4(),
                tenant_id,
                username: "admin".to_string(),
                name: "管理员".to_string(),
                role: UserRole::Admin,
                password_hash: hash_password("admin123"),
            };
            if let Err(err) = state
                .repository
                .create_user(&state.persistence, &admin)
                .await
            {
                tracing::error!("failed to create default admin: {:?}", err);
            } else {
                info!("sqlite standalone mode initialized (admin/admin123)");
            }
        }
        Err(err) => {
            tracing::error!("failed to check existing admin: {:?}", err);
        }
        _ => {}
    }
}

/// 启动进程内嵌服务端（Android 单机版入口）。
///
/// `data_dir`：宿主 App 提供的可写目录（Android 下为应用私有目录），
/// SQLite 数据文件写入 `<data_dir>/jxc.db`。
///
/// 返回值：正整数为实际监听端口；负数为错误码（-1 启动失败 / -2 启动超时）。
/// 重复调用是安全的：已启动时直接返回既有端口。
///
/// # Safety
///
/// `data_dir` 必须是有效的、以 NUL 结尾的 UTF-8 C 字符串指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jxc_start_server(data_dir: *const std::ffi::c_char) -> i32 {
    let existing = EMBEDDED_PORT.load(Ordering::SeqCst);
    if existing > 0 {
        return existing;
    }

    if data_dir.is_null() {
        return -1;
    }
    let data_dir = match unsafe { std::ffi::CStr::from_ptr(data_dir) }.to_str() {
        Ok(value) => value.to_string(),
        Err(_) => return -1,
    };

    let (tx, rx) = std::sync::mpsc::channel::<Result<u16, String>>();

    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(err) => {
                let _ = tx.send(Err(format!("failed to build tokio runtime: {err}")));
                return;
            }
        };

        runtime.block_on(async move {
            let mut config = AppConfig::from_env();
            // 单机模式：本地 SQLite、仅监听回环地址、端口由系统分配、不依赖 Redis。
            config.storage_backend = StorageBackend::Sqlite;
            config.sqlite_path = format!("{data_dir}/jxc.db");
            config.host = "127.0.0.1".to_string();
            config.port = 0;
            config.redis_url = None;
            config.database_url = None;

            if let Err(err) = serve(config, Some(tx)).await {
                tracing::error!("embedded server stopped: {err}");
            }
        });
    });

    match rx.recv_timeout(Duration::from_secs(60)) {
        Ok(Ok(port)) => {
            EMBEDDED_PORT.store(port as i32, Ordering::SeqCst);
            port as i32
        }
        Ok(Err(err)) => {
            tracing::error!("embedded server failed to start: {err}");
            -1
        }
        Err(_) => {
            tracing::error!("embedded server start timed out");
            -2
        }
    }
}

/// 返回进程内嵌服务端当前监听端口（0 表示未启动）。
#[unsafe(no_mangle)]
pub extern "C" fn jxc_server_port() -> i32 {
    EMBEDDED_PORT.load(Ordering::SeqCst)
}
