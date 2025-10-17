use crate::objects::tag::Tag;
use crate::sha::HashValue;
use mongodb::bson::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OdbMongoTag {
    pub repo_uid: Uuid,
    pub hash: HashValue,
    pub tag: Tag,
}
