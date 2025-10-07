use crate::callback::CallBack;
use crate::error::GitInnerError;
use crate::serve::AppCore;
use crate::transaction::TransactionService::ReceivePack;
use crate::transaction::{GitProtoVersion, ProtocolType, Transaction};
use actix_web::web::Payload;
use actix_web::{web, HttpResponse, Responder};
use async_stream::stream;
use std::io;
use tokio_stream::StreamExt;

pub async fn receive_pack(
    mut payload: Payload,
    path: web::Path<(String, String)>,
    app: web::Data<AppCore>,
) -> impl Responder {
    let (namespace, repo) = path.into_inner();
    let repo = match app.repo_store.repo(namespace, repo).await {
        Ok(repo) => repo,
        Err(err) => {
            dbg!(err);
            return HttpResponse::NotFound().body("Repo not found");
        }
    };
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