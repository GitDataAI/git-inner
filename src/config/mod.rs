use std::env::var;
use serde::{Deserialize, Serialize};
use crate::config::ssh::SshConfig;


lazy_static::lazy_static! {
    pub static ref CFG: AppConfig = AppConfig::load();
}


#[derive(Deserialize,Serialize,Clone,Debug,Default)]
pub struct AppConfig {
    pub(crate) ssh: SshConfig,
}

impl AppConfig {
    pub fn load() -> Self {
        let config_file_path = var("CONFIG_FILE").unwrap_or("config.toml".to_string());
        let config_content = match std::fs::read_to_string(&config_file_path) {
            Ok(content) => content,
            Err(_) => {
                Self::default().save().expect("Failed to save config file");
                return AppConfig::default();
            }
        };
        toml::from_str(&config_content).expect("Could not parse config file")
    }
    pub fn save(&self) -> std::io::Result<()> {
        let config_file_path = var("CONFIG_FILE").unwrap_or("config.toml".to_string());
        let toml_str = toml::to_string_pretty(self).expect("Could not serialize config");
        std::fs::write(config_file_path, toml_str)
    }
    pub fn cfg() -> &'static Self {
        &CFG
    }
    pub fn ssh() -> &'static SshConfig {
        &CFG.ssh
    }
}



pub mod ssh;
