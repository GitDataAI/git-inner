use git_in::http::HttpServer;
use git_in::serve::mongo::MongoRepoManager;
use git_in::serve::AppCore;
use object_store::local::LocalFileSystem;
use std::sync::Arc;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let store = LocalFileSystem::new_with_prefix("./data")?;
    let optional = mongodb::options::ClientOptions::parse(
        "mongodb://root:root%402025@172.29.228.23:27017,172.29.228.23:27018,172.29.228.23:27019/git_inner?replicaSet=rs0&authSource=admin&retryWrites=true&w=majority"
    )
        .await?;
    let mongodb = mongodb::Client::with_options(optional)?;
    let manager = MongoRepoManager::new(mongodb, Arc::new(Box::new(store)));
    let core = AppCore::new(Arc::new(Box::new(manager)));
    let http = HttpServer::new("0.0.0.0".to_string(), 3000, core);
    tokio::select! {
        result = http => {
            if let Err(e) = result {
                eprintln!("HTTP server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("Received Ctrl+C, shutting down.");
            std::process::exit(0);
        },
    }
    Ok(())
}
