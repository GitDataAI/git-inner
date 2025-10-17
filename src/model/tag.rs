use mongodb::bson::Uuid;
use serde::{Deserialize, Serialize};
use crate::objects::tag::Tag;
use crate::objects::tree::Tree;
use crate::sha::HashValue;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OdbMongoTag {
    pub repo_uid: Uuid,
    pub hash: HashValue,
    pub tag: Tag,
}


