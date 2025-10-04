use std::io;
use crate::app::AppCore;
use crate::error::GitInnerError;
use crate::transaction::TransactionService::ReceivePack;
use crate::transaction::receive_pack::report_status::{ReportStatus, ReportStatusKind};
use crate::transaction::{GitProtoVersion, Transaction};
use actix_web::web::Payload;
use actix_web::{HttpResponse, Responder, web};
use async_stream::stream;
use bytes::{Bytes};
use tokio_stream::StreamExt;
use tracing::error;

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
    let report_status = ReportStatus::new(1024);
    let report_status_tx = report_status.sender().clone();
    let mut transaction = Transaction {
        service: ReceivePack,
        repository: repo,
        version: GitProtoVersion::V1,
        report_status: report_status.clone(),
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
        let result = transaction.receive_pack(Box::pin(stream)).await;
        match result {
            Ok(_) => {
                report_status_tx.send(ReportStatusKind::Done).await.ok();
            }
            Err(err) => {
                error!("Receive pack error: {:?}", err);
                report_status_tx.send(ReportStatusKind::UnpackError(format!("{:?}", err))).await.ok();
            }
        }
    });

    let stream = stream! {
        let mut receiver = report_status.receiver.lock().await;
        while let Some(next) = receiver.recv().await {
            yield Ok::<_, io::Error>(Bytes::from(next.as_bytes()));
            if matches!(next, ReportStatusKind::Done | ReportStatusKind::SideDone) {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                break;
            }
            yield Ok::<_, io::Error>(Bytes::new());
        }
    };
    HttpResponse::Ok()
        .keep_alive()
        .content_type("application/x-git-receive-pack-result")
        .streaming(stream)
}