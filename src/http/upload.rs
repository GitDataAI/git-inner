use std::io;
use actix_web::{web, HttpResponse, Responder};
use actix_web::web::Payload;
use async_stream::stream;
use tokio_stream::StreamExt;
use tracing::error;
use crate::callback::CallBack;
use crate::error::GitInnerError;
use crate::serve::AppCore;
use crate::transaction::{GitProtoVersion, ProtocolType, Transaction};
use crate::transaction::TransactionService::UploadPack;

pub async fn upload_pack(
    mut payload: Payload,
    path: web::Path<(String, String)>,
    app: web::Data<AppCore>,
    req: actix_web::HttpRequest,
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
    let version = match req.headers().get("Git-Protocol") {
        Some(header) => {
            if header.to_str().unwrap().contains("version=2") {
                GitProtoVersion::V2
            } else {
                GitProtoVersion::V1
            }
        }
        None => GitProtoVersion::V1,
    };
    let transaction = Transaction {
        service: UploadPack,
        repository: repo,
        version,
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
        let result = transaction.upload_pack(&mut Box::pin(stream)).await;
        match result {
            Ok(_) => {
            }
            Err(err) => {
                error!("Receive pack error: {:?}", err);
            }
        }
    });
    let stream = stream! {
        let mut receiver = call_back.receive.lock().await;
        while let Some(next) = receiver.recv().await {
            yield Ok::<_, io::Error>(next);
        }
    };
    HttpResponse::Ok()
        .keep_alive()
        .insert_header(("Pragma", "no-cache"))
        .insert_header(("Cache-Control", "no-cache, max-age=0, must-revalidate"))
        .insert_header(("Expires", "Fri, 01 Jan 1980 00:00:00 GMT"))
        .content_type("application/x-git-upload-pack-result")
        .streaming(stream)
}