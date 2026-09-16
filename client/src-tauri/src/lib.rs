use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fs;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_shell::ShellExt;

/// Global state: the port the embedded server is listening on
pub struct ServerPort(pub Arc<Mutex<Option<u16>>>);

/// Find an available TCP port on localhost
fn find_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("could not bind to a free port")
        .local_addr()
        .expect("could not get local addr")
        .port()
}

/// Return the data directory where the SQLite database file should live
fn data_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("could not resolve app data dir")
}

/// Tauri command: frontend calls this to get the server's port
#[tauri::command]
fn get_server_port(state: State<'_, ServerPort>) -> Result<u16, String> {
    state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "server not yet started".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())
        .plugin(tauri_plugin_shell::init())
        .manage(ServerPort(Arc::new(Mutex::new(None))))
        .setup(|app| {
            let port = find_free_port();
            let app_data_dir = data_dir(app.handle());
            fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
            let db_path = app_data_dir.join("jxc.db");
            let db_path_str = db_path.to_string_lossy().to_string();

            // Store port in state so the frontend can retrieve it
            {
                let state = app.state::<ServerPort>();
                *state.0.lock().unwrap() = Some(port);
            }

            // Spawn the bundled server binary as a sidecar
            let sidecar = app
                .shell()
                .sidecar("server")
                .expect("server sidecar not found")
                .env("STORAGE_BACKEND", "sqlite")
                .env("SQLITE_PATH", &db_path_str)
                .env("APP_HOST", "127.0.0.1")
                .env("APP_PORT", port.to_string())
                .env("JWT_SECRET", "jxc-local-secret-do-not-expose");

            let (mut rx, child) = sidecar.spawn().expect("failed to spawn server sidecar");
            tauri::async_runtime::spawn(async move {
                let _child = child;
                while let Some(event) = rx.recv().await {
                    log::info!("sidecar event: {:?}", event);
                }
            });

            log::info!("Embedded server started on port {port}, db: {db_path_str}");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_server_port])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
