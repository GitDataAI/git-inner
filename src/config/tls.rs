use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TlsConfig {
    pub enable: bool,
    pub cert_file: String,
    pub key_file: String,
    pub ca_file: String,
}

impl Default for TlsConfig {
    fn default() -> Self {
        TlsConfig {
            enable: false,
            cert_file: "".to_string(),
            key_file: "".to_string(),
            ca_file: "".to_string(),
        }
    }
}
