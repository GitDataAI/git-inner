use crate::serve::AppCore;
use actix_web::web::{scope, Data};
use actix_web::App;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct HttpServer {
    pub addr: String,
    pub port: u16,
    pub core: AppCore
}


impl HttpServer {
    pub fn new(addr: String, port: u16, core: AppCore) -> Self {
        Self {
            addr,
            port,
            core,
        }
    }
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.addr, self.port)
    }
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let core = self.core.clone();
        actix_web::HttpServer::new(move || {
            App::new()
                .app_data(Data::new(core.clone()))
                .wrap(actix_web::middleware::Logger::new(
                    "%a %r %s %b bytes in %D microseconds %{git-protocol}i"
                ))
                .service(
                    scope("/{namespace}/{repo_name}.git")
                        .route("/info/refs", actix_web::web::get().to(refs::refs))
                        .route("/git-receive-pack", actix_web::web::post().to(receive::receive_pack))
                        .route("/git-upload-pack", actix_web::web::post().to(upload::upload_pack))
                )
        })
            .bind(self.bind_addr())?
            .run()
            .await?;
        Ok(())
    }
}

impl Future for HttpServer {
    type Output = Result<(), Box<dyn std::error::Error>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let fut = this.run();
        tokio::pin!(fut);
        fut.poll(cx)
    }
}


pub mod refs;
pub mod receive;
pub mod upload;