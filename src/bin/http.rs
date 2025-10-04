use actix_web::web::{get, post, scope};
use actix_web::{App, web};
use git_inner::app::AppCore;
use git_inner::app::mongo::MongoRepoManager;
use git_inner::http::receive::receive_pack;
use git_inner::http::refs::refs;
use object_store::local::LocalFileSystem;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let addr = SocketAddr::from(([0,0,0,0], 3000));

    let store = LocalFileSystem::new_with_prefix("./data")?;
    let optional = mongodb::options::ClientOptions::parse(
        "mongodb://root:root%402025@172.29.228.23:27017,172.29.228.23:27018,172.29.228.23:27019/git_inner?replicaSet=rs0&authSource=admin&retryWrites=true&w=majority"
    )
        .await?;
    let mongodb = mongodb::Client::with_options(optional)?;
    let manager = MongoRepoManager::new(mongodb, Arc::new(Box::new(store)));
    actix_web::HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .app_data(web::Data::new(AppCore {
                repo_store: Box::new(manager.clone()),
            }))
            .service(
                scope("/{namespace}/{repo_name}.git")
                    .route("/info/refs", get().to(refs))
                    .route("/git-receive-pack", post().to(receive_pack)),
            )
    })
    .bind(addr)?
    .run()
    .await?;
    Ok(())
}
