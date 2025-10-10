use log::{error, info};
use tokio::select;
use git_in::control::Control;
use git_in::http::HttpServer;
use git_in::logs::LogsStore;
use git_in::serve::mongo::init_app_by_mongodb;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    console_subscriber::init();
    init_app_by_mongodb().await;
    let log_store = LogsStore::new("./logs")?;
    let control = Control::new(log_store);
    let http_handle = control.spawn(async move {
        if let Err(e) = HttpServer::new("0.0.0.0".to_string(), 3000).run().await {
            error!("Control error: {}", e);
        } else {
            info!("HTTP server exited.");
        }
    });
    let collection = control.start_metrics_collection();
    select! {
        _ = http_handle => {
            info!("HTTP server task completed.");
        }
        _ = collection => {
            info!("Metrics logs server task completed.");
        }
        _ = tokio::signal::ctrl_c() => {
            control.stop().await;
            info!("Shutdown signal received.");
        }
    }
    Ok(())
}
