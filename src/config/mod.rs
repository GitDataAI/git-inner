use crate::config::ssh::SshConfig;
use serde::{Deserialize, Serialize};
use std::env::var;

lazy_static::lazy_static! {
    pub static ref CFG: AppConfig = AppConfig::load();
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct AppConfig {
    pub(crate) ssh: SshConfig,
}

impl AppConfig {
    /// Loads the application configuration from the configured file or the default path.
    ///
    /// If the configuration file is missing or cannot be read, the default configuration is saved to disk
    /// and returned. Panics if a configuration file exists but cannot be parsed as TOML.
    ///
    /// # Panics
    ///
    /// Panics when the configuration file is present but TOML deserialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = AppConfig::load();
    /// // use `cfg` as needed, e.g. access SSH config: `let _ssh = cfg.ssh();`
    /// ```
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
    /// Writes this configuration as pretty-formatted TOML to the file specified by the
    /// `CONFIG_FILE` environment variable or to `config.toml` if the variable is not set.
    ///
    /// Returns `Ok(())` if the file was written successfully, `Err` if an I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::env;
    /// use std::fs;
    /// // Ensure a deterministic path for the example
    /// env::set_var("CONFIG_FILE", "example_config.toml");
    /// let cfg = crate::config::AppConfig::default();
    /// cfg.save().expect("failed to save config");
    /// assert!(fs::metadata("example_config.toml").is_ok());
    /// let _ = fs::remove_file("example_config.toml");
    /// ```
    pub fn save(&self) -> std::io::Result<()> {
        let config_file_path = var("CONFIG_FILE").unwrap_or("config.toml".to_string());
        let toml_str = toml::to_string_pretty(self).expect("Could not serialize config");
        std::fs::write(config_file_path, toml_str)
    }
    /// Accesses the global application configuration.
    ///
    /// # Returns
    ///
    /// A `'static` reference to the singleton `AppConfig`.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = AppConfig::cfg();
    /// let b = AppConfig::cfg();
    /// assert!(std::ptr::eq(a, b));
    /// ```
    pub fn cfg() -> &'static Self {
        &CFG
    }
    /// Accesses the global SSH configuration.
    ///
    /// Provides a `'static` reference to the singleton `SshConfig` stored in the
    /// module-level configuration (`CFG`).
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::config::AppConfig;
    ///
    /// let _ssh = AppConfig::ssh();
    /// ```
    pub fn ssh() -> &'static SshConfig {
        &CFG.ssh
    }
}

pub mod ssh;
