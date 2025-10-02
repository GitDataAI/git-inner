use actix_web::{web, App};
use std::net::SocketAddr;
use actix_web::web::{get, post, scope};
use git_inner::app::AppCore;
use git_inner::app::sqlx::SqliteConn;
use git_inner::http::receive::receive_pack;
use git_inner::http::refs::refs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>>  {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let addr = SocketAddr::from(([0,0,0,0], 3000));
    let sqlite_conn = SqliteConn::new(
        rusqlite::Connection::open("./identifier.sqlite")?
    );
    sqlite_conn.init_table().expect("init table error");
    actix_web::HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .app_data(web::Data::new(AppCore {
                repo_store: Box::new(sqlite_conn.clone()),
            }))
            .service(
                scope("/{namespace}/{repo_name}.git")
                    .route("/info/refs", get().to(refs))
                    .route("/git-receive-pack", post().to(receive_pack))
            )
    })
        .bind(addr)?
        .run()
        .await?;
    Ok(())
}