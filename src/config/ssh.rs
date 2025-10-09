use serde::{Deserialize, Serialize};

#[derive(Deserialize,Serialize,Clone,Debug)]
pub struct SshConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub server_public_key: Option<String>,
}


impl Default for SshConfig {
    /// Creates the default SSH configuration.
    ///
    /// The default configuration has `enabled` set to `false`, `host` set to `"0.0.0.0"`,
    /// `port` set to `22`, an empty `user`, and `server_public_key` set to `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = SshConfig::default();
    /// assert!(!cfg.enabled);
    /// assert_eq!(cfg.host, "0.0.0.0");
    /// assert_eq!(cfg.port, 22);
    /// assert_eq!(cfg.user, "");
    /// assert!(cfg.server_public_key.is_none());
    /// ```
    fn default() -> Self {
        Self {
            enabled: false,
            host: "0.0.0.0".to_string(),
            port: 22,
            user: "".to_string(),
            server_public_key: None,
        }
    }
}