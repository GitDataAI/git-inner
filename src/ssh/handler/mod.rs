use std::net::SocketAddr;
use crate::error::GitInnerError;
use crate::serve::AppCore;
use crate::transaction::{Transaction, TransactionService};

#[derive(Clone)]
pub struct SshHandler {
    pub core: AppCore,
    pub addr:  Option<SocketAddr>,
    pub service: Option<TransactionService>,
    pub transaction: Option<Transaction>,
}

impl russh::server::Handler for SshHandler {
    type Error = GitInnerError;
}