pub mod advertise;
pub mod receive;
pub mod service;
pub mod upload;
pub mod version;

use crate::callback::CallBack;
use crate::repository::Repository;
pub(crate) use crate::transaction::service::TransactionService;
pub(crate) use crate::transaction::version::GitProtoVersion;

#[derive(Clone)]
pub struct Transaction {
    pub service: TransactionService,
    pub repository: Repository,
    pub version: GitProtoVersion,
    pub call_back: CallBack,
    pub protocol: ProtocolType,
}

#[derive(Clone)]
pub enum ProtocolType {
    Git,
    SSH,
    Http,
}
