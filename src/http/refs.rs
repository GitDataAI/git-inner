use crate::callback::CallBack;
use crate::serve::AppCore;
use crate::transaction::{GitProtoVersion, ProtocolType, Transaction, TransactionService};
use actix_web::web::{Data, Path};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use bytes::BytesMut;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RefsQuery {
    service: TransactionService,
}
pub async fn refs(
    req: HttpRequest,
    path: Path<(String, String)>,
    app: Data<AppCore>,
    query: web::Query<RefsQuery>,
) -> impl Responder {
    let (namespace, repo) = path.into_inner();
    let start = std::time::Instant::now();
    let repo = match app.repo_store.repo(namespace, repo).await {
        Ok(repo) => repo,
        Err(err) => {
            dbg!(err);
            dbg!(format!("{:?}", start.elapsed()));
            return HttpResponse::NotFound().body("Repo not found");
        }
    };
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
    let call_back = CallBack::new(20);
    let transaction = Transaction {
        service: query.service.clone(),
        repository: repo,
        version,
        call_back: call_back.clone(),
        protocol: ProtocolType::Http,
    };
    match transaction.advertise_refs().await {
        Ok(_) => {}
        Err(_) => {
        }
    }
    let mut result = BytesMut::new();
    let mut recv = call_back.receive.lock().await;
    while let Some(msg) = recv.recv().await {
        result.extend_from_slice(&msg);
        if msg.is_empty() {
            break;
        }
    }
    HttpResponse::Ok()
        .insert_header(("Pragma", "no-cache"))
        .insert_header(("Cache-Control", "no-cache, max-age=0, must-revalidate"))
        .insert_header(("Expires", "Fri, 01 Jan 1980 00:00:00 GMT"))
        .insert_header((
            "Content-Type",
            match query.service {
                TransactionService::UploadPack | TransactionService::UploadPackLs=> "application/x-git-upload-pack-advertisement",
                TransactionService::ReceivePack |  TransactionService::ReceivePackLs=> "application/x-git-receive-pack-advertisement",
            },
        ))
        .body(result.freeze())
}
