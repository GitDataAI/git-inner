use crate::repository::Repository;
use crate::transaction::receive_pack::report_status::ReportStatus;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum TransactionService {
    #[serde(rename = "git-upload-pack")]
    UploadPack,
    #[serde(rename = "git-receive-pack")]
    ReceivePack,
    #[serde(rename = "git-upload-pack-ls")]
    UploadPackLs,
    #[serde(rename = "git-receive-pack-ls")]
    ReceivePackLs,
}

#[derive(Debug, Clone)]
pub enum GitProtoVersion {
    V0,
    V1,
    V2,
}

pub struct Transaction {
    pub service: TransactionService,
    pub repository: Repository,
    pub version: GitProtoVersion,
    pub report_status: ReportStatus,
}

pub mod receive_pack;
pub mod refs;
pub mod upload_pack;

impl TransactionService {
    pub fn from_string(s: &str) -> Option<TransactionService> {
        match s {
            "git-upload-pack" => Some(TransactionService::UploadPack),
            "git-receive-pack" => Some(TransactionService::ReceivePack),
            "git-upload-pack-ls" => Some(TransactionService::UploadPackLs),
            "git-receive-pack-ls" => Some(TransactionService::ReceivePackLs),
            _ => None,
        }
    }
    pub fn to_string(&self) -> &'static str {
        match self {
            TransactionService::UploadPack => "git-upload-pack",
            TransactionService::ReceivePack => "git-receive-pack",
            TransactionService::UploadPackLs => "git-upload-pack-ls",
            TransactionService::ReceivePackLs => "git-receive-pack-ls",
        }
    }
    pub fn is_ls(&self) -> bool {
        match self {
            TransactionService::UploadPackLs | TransactionService::ReceivePackLs => true,
            _ => false,
        }
    }
    pub fn is_pack(&self) -> bool {
        match self {
            TransactionService::UploadPack | TransactionService::ReceivePack => true,
            _ => false,
        }
    }
}
