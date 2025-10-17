use mongodb::bson::Uuid;
use serde::{Deserialize, Serialize};
use crate::objects::commit::Commit;
use crate::objects::tag::Tag;
use crate::objects::tree::Tree;
use crate::sha::HashValue;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OdbMongoCommit {
    pub repo_uid: Uuid,
    pub hash: HashValue,
    pub commit: Commit,
}


