use crate::capability::enums::GitCapability;
use crate::sha::HashValue;
use crate::transaction::Transaction;

#[derive(Clone)]
pub struct UploadPackTransaction {
    pub want: Vec<HashValue>,
    pub have: Vec<HashValue>,
    pub shallow: Vec<HashValue>,
    pub sideband: bool,
    pub thin: bool,
    pub depth: Option<u32>,
    pub no_progress: bool,
    pub no_done: bool,
    pub include_tag: bool,
    pub capabilities: Vec<GitCapability>,
    pub txn: Transaction,
}

impl UploadPackTransaction {
    pub fn new(txn: Transaction) -> Self {
        Self {
            want: vec![],
            have: vec![],
            shallow: vec![],
            sideband: false,
            thin: false,
            depth: None,
            no_progress: false,
            no_done: false,
            include_tag: false,
            capabilities: vec![],
            txn,
        }
    }
}



pub mod command;
pub mod encode_pack;
pub mod recursion;
pub mod advertise_v2;
pub mod upload_pack;
pub mod upload_pack_v2;