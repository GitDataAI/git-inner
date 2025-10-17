use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RpcConfig {
    pub url: String,
    pub port: u16,
}
