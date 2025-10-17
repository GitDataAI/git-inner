use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SshKeyModel {
    pub owner: Uuid,
    pub public_key: String,
    pub fingerprint: String,
    pub created_unix: u64,
    pub last_used_unix: Option<u64>,
}

impl SshKeyModel {}
