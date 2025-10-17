use mongodb::bson::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MongoRepository {
    pub id: i32,
    pub name: String,
    pub namespace: String,
    pub uid: Uuid,
    pub owner: Uuid,
    pub hash_version: i32,
    pub default_branch: String,
    pub is_public: bool,
}
