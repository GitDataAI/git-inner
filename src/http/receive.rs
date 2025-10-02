use crate::app::AppCore;
use crate::error::GitInnerError;
use crate::transaction::TransactionService::ReceivePack;
use crate::transaction::{GitProtoVersion, Transaction};
use actix_web::web::Payload;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use async_stream::stream;
use tokio_stream::StreamExt;

pub async fn receive_pack(
    req: HttpRequest,
    mut payload: Payload,
    path: web::Path<(String,String)>,
    app: web::Data<AppCore>,
) -> impl Responder {
    let (namespace, repo) = path.into_inner();
    let repo = match app.repo_store.repo(namespace, repo).await {
        Ok(repo) => repo,
        Err(err) => {
            dbg!(err);
            return HttpResponse::NotFound()
                .body("Repo not found")
        }
    };
    let mut transaction = Transaction {
        service: ReceivePack,
        repository: repo,
        version: GitProtoVersion::V1,
    };
    let stream = stream! {
        while let Some(next) = payload.next().await {
            yield next.map_err(|err| GitInnerError::Payload(err.to_string()))
        }
    };
    let result = transaction.receive_pack(Box::pin(stream)).await;
    dbg!(result);
    HttpResponse::Ok()
        .body("OK")
}