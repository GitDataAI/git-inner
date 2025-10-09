use git_in::http::HttpServer;
use git_in::serve::mongo::MongoRepoManager;
use git_in::serve::AppCore;
use object_store::local::LocalFileSystem;
use std::sync::Arc;
/// Starts the application: initializes logging, local storage, and MongoDB; constructs the repository
/// manager and application core; then runs an HTTP server bound to 0.0.0.0:3000 until the server stops
/// or the process receives a Ctrl+C signal.
///
/// The function prints an error message if the HTTP server future resolves to an error and exits the
/// process immediately when a Ctrl+C signal is received.
///
/// # Returns
///
/// `Ok(())` after the server future completes or shutdown handling finishes; an `Err` is returned if
/// initialization fails (for example, creating the local file store or connecting to MongoDB).
///
/// # Examples
///
/// ```no_run
/// // Run the compiled binary; this function is the program's entry point and is not called directly.
/// ```
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
    let core = AppCore::new(Arc::new(Box::new(manager)), None);
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