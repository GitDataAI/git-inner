use crate::objects::commit::Commit;
use crate::objects::tag::Tag;
use crate::objects::tree::Tree;
use crate::sha::HashValue;
use mongodb::bson::Uuid;
use serde::{Deserialize, Serialize};

pub mod odb;
pub mod transaction;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OdbMongoCommit {
    pub repo_uid: Uuid,
    pub hash: HashValue,
    pub commit: Commit,
}


#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OdbMongoTag {
    pub repo_uid: Uuid,
    pub hash: HashValue,
    pub tag: Tag,
}


#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OdbMongoTree {
    pub repo_uid: Uuid,
    pub hash: HashValue,
    pub tree: Tree,
}
