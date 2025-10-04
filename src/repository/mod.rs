use crate::odb::Odb;
use crate::refs::RefsManager;
use crate::sha::HashVersion;
use uuid::Uuid;

pub struct Repository {
    pub id: Uuid,
    pub odb: Box<dyn Odb>,
    pub refs: Box<dyn RefsManager>,
    pub hash_version: HashVersion,
}

pub mod refs;
