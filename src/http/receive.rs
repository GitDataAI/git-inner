use crate::auth::AccessLevel;
use crate::callback::CallBack;
use crate::error::GitInnerError;
use crate::serve::AppCore;
use crate::transaction::TransactionService::ReceivePack;
use crate::transaction::{GitProtoVersion, ProtocolType, Transaction};
use actix_web::http::header::Header;
use actix_web::web::Payload;
use actix_web::{web, HttpResponse, Responder};
use actix_web_httpauth::headers::authorization::{Authorization, Basic};
use async_stream::stream;
use std::io;
use tokio_stream::StreamExt;

/// Handle an HTTP Git "receive-pack" request for a repository and stream the service result.
///
/// This endpoint looks up the target repository, optionally enforces HTTP Basic authentication
/// (returns `401` with `WWW-Authenticate` if missing/invalid, `403` if credentials grant only read
/// access), forwards the request payload to the repository's receive-pack transaction, and returns
/// a streaming `HttpResponse` with the service output.
///
/// - `404 Not Found` when the repository cannot be located.
/// - `401 Unauthorized` when authentication is required but missing or invalid.
/// - `403 Forbidden` when authentication succeeds but grants only read access.
///
/// # Examples
///
/// ```
/// use actix_web::{web, App, HttpServer};
///
/// async fn main() -> std::io::Result<()> {
///     HttpServer::new(|| {
///         App::new().route(
///             "/{namespace}/{repo}/git-receive-pack",
///             web::post().to(crate::http::receive::receive_pack),
///         )
///     })
///     .bind(("127.0.0.1", 8080))?
///     .run()
///     .await
/// }
/// ```
pub async fn receive_pack(
    mut payload: Payload,
    path: web::Path<(String, String)>,
    app: web::Data<AppCore>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let (namespace, repo_name) = path.into_inner();
    let repo = match app.repo_store.repo(namespace.clone(), repo_name.clone()).await {
        Ok(repo) => repo,
        Err(err) => {
            dbg!(err);
            return HttpResponse::NotFound().body("Repo not found");
        }
    };
    if let Some(auth) = app.auth.clone() {
        match Authorization::<Basic>::parse(&req) {
            Ok(basic) => {
                let scheme = basic.into_scheme();
                let username = scheme.user_id().to_string();
                let password = scheme.password().unwrap_or("").to_string();
                match auth.authenticate(&username, &password, &namespace, &repo_name).await {
                    Ok(level) => {
                        match level {
                            AccessLevel::Read =>
                                return HttpResponse::Forbidden().body("Forbidden"),
                            _=> {}
                        }
                    }
                    Err(_) => {
                        return HttpResponse::Unauthorized()
                            .insert_header(("WWW-Authenticate", r#"Basic realm="Restricted""#))
                            .body("Unauthorized");
                    }
                }
            }
            Err(_) => {
                return HttpResponse::Unauthorized()
                    .insert_header(("WWW-Authenticate", r#"Basic realm="Restricted""#))
                    .body("Unauthorized");
            }
        }
    }
    let call_back = CallBack::new(1024);
    let mut transaction = Transaction {
        service: ReceivePack,
        repository: repo,
        version: GitProtoVersion::V1,
        call_back: call_back.clone(),
        protocol: ProtocolType::Http,
    };
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    tokio::task::spawn_local(async move {
        while let Some(next) = payload.next().await  {
            tx.send(next.map_err(|err| GitInnerError::Payload(err.to_string()))).await
                .ok();
        }
    });
    tokio::task::spawn_local(async move {
        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let _result = transaction.receive_pack(Box::pin(stream)).await;
        let _ = dbg!(_result);
    });

    let stream = stream! {
        let mut receiver = call_back.receive.lock().await;
        while let Some(next) = receiver.recv().await {
             if next.is_empty() {
                break;
            }
            yield Ok::<_, io::Error>(next);
        }
    };
    HttpResponse::Ok()
        .keep_alive()
        .content_type("application/x-git-receive-pack-result")
        .streaming(stream)
}