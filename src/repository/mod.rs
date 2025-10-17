use std::sync::Arc;
use crate::odb::Odb;
use crate::refs::RefsManager;
use crate::sha::HashVersion;
use uuid::Uuid;

#[derive(Clone)]
pub struct Repository {
    pub id: Uuid,
    pub default_branch: String,
    pub owner: Uuid,
    pub odb: Arc<Box<dyn Odb>>,
    pub refs: Arc<Box<dyn RefsManager>>,
    pub hash_version: HashVersion,
    pub is_public: bool,
}

pub mod refs;

pub mod init;
pub mod set;
pub mod info;