use crate::objects::commit::Commit;
use crate::sha::HashValue;
use mongodb::bson::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OdbMongoCommit {
    pub repo_uid: Uuid,
    pub hash: HashValue,
    pub commit: Commit,
}
