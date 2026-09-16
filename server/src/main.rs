use jxc_server::config::AppConfig;
use jxc_server::serve;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init();

    let config = AppConfig::from_env();
    if let Err(err) = serve(config, None).await {
        panic!("server failed: {err}");
    }
}
