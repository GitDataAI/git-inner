use std::sync::{Arc, Mutex};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use actix_web::web::{Data, Path};
use serde::{Deserialize, Serialize};
use crate::app::AppCore;
use crate::stream::DataStream;
use crate::transaction::{GitProtoVersion, Transaction, TransactionService};

#[derive(Deserialize,Serialize,Clone,Debug)]
pub struct RefsQuery {
    service: TransactionService
}
pub async fn refs(
    req: HttpRequest,
    path: Path<(String, String)>,
    app: Data<AppCore>,
    query: web::Query<RefsQuery>
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
    let version = match req.headers().get("Git-Protocol") {
        Some(header) => {
            if header.to_str().unwrap().contains("version=2") {
                GitProtoVersion::V2
            } else {
                GitProtoVersion::V1
            }
        },
        None => GitProtoVersion::V1
    };
    let transaction = Transaction {
        service: query.service.clone(),
        repository: repo,
        version,
    };
    let refs = match transaction.http_advertise().await {
        Ok(refs) => refs,
        Err(_) => return HttpResponse::InternalServerError()
            .body("Error getting refs")
    };
    HttpResponse::Ok()
        .insert_header(("Pragma", "no-cache"))
        .insert_header(("Cache-Control", "no-cache, max-age=0, must-revalidate"))
        .insert_header(("Expires", "Fri, 01 Jan 1980 00:00:00 GMT"))
        .insert_header(("Content-Type",match query.service {
            TransactionService::UploadPack => "application/x-git-upload-pack-advertisement",
            TransactionService::ReceivePack => "application/x-git-receive-pack-advertisement",
            TransactionService::UploadPackLs => "",
            TransactionService::ReceivePackLs => ""
        }))
        .body(refs)
}