use std::net::SocketAddr;
use std::sync::Arc;
use log::{info, warn};
use russh::keys::PublicKeyBase64;
use russh::keys::ssh_encoding::{base64, DecodePem, EncodePem, LineEnding};
use russh::keys::ssh_encoding::base64::Encoding;
use russh::server::Server;
use sha2::Digest;
use crate::config::{AppConfig, CFG};
use crate::config::ssh::SshConfig;
use crate::error::GitInnerError;
use crate::serve::AppCore;
use crate::ssh::handler::SshHandler;

pub struct SshServer {
    pub core: AppCore,
    pub config: SshConfig,
}


impl SshServer {
    pub async fn run(&mut self) -> Result<(), GitInnerError> {
        if !self.config.enabled {
            warn!("SSH server is disabled");
            return Ok(())
        }
        info!("Starting SSH server");
        let mut cfg = russh::server::Config::default();
        if let Some(public_key) = &self.config.server_public_key {
            let mut figure = sha2::Sha256::default();
            figure.update(public_key);
            let fingerprint = figure.finalize();
            info!("SSH server public key fingerprint: sha256:{}", hex::encode(fingerprint));
            let private_key = russh::keys::PrivateKey::decode_pem(
                &base64::Base64::decode_vec(public_key)
                    .map_err(|e| GitInnerError::SshServerStartError(e.to_string()))?
            )
                .map_err(|e| GitInnerError::SshServerStartError(e.to_string()))?;
            cfg.keys = vec![private_key]
        } else {
            info!("SSH server public is empty, using new key");
            let private_key = russh::keys::PrivateKey::random(
                &mut russh::keys::key::safe_rng(),
                russh::keys::Algorithm::Ed25519
            )
                .map_err(|e| GitInnerError::SshServerStartError(e.to_string()))?;
            let mut pem = vec![];
            let private_key_pem = private_key
                .encode_pem(LineEnding::LF, &mut pem)
                .map_err(|e| GitInnerError::SshServerStartError(e.to_string()))?;
            let private_key_pem = base64::Base64::encode_string(private_key_pem.as_bytes());
            let mut config = CFG.clone();
            config.ssh.server_public_key = Some(private_key_pem);
            config.save().map_err(|e| GitInnerError::SshServerStartError(e.to_string()))?;
            self.config = config.ssh;
            let mut figure = sha2::Sha256::default();
            figure.update(private_key.public_key_base64().as_bytes());
            let fingerprint = figure.finalize();
            info!("SSH server new public key fingerprint: sha256:{}", hex::encode(fingerprint));
            cfg.keys = vec![private_key];
        }
        cfg.channel_buffer_size = usize::MAX;
        cfg.event_buffer_size = usize::MAX;
        cfg.auth_rejection_time = std::time::Duration::from_secs(3);
        self
            .run_on_address(
                Arc::new(cfg),
                format!("{}:{}", self.config.host, self.config.port),
            )
            .await
            .map_err(|error| {
                GitInnerError::SshServerStartError(error.to_string())
            })?;
        Ok(())
    }
    pub async fn new() -> Result<Self, GitInnerError> {
        let app = AppCore::app()?;
        let cfg = AppConfig::ssh();
        Ok(Self {
            core: app,
            config: cfg.clone(),
        })
    }
    pub async fn ssh_spawn() -> Result<(), GitInnerError> {
        Self::new()
            .await?
            .run()
            .await
    }
}


impl Server for SshServer {
    type Handler = SshHandler;

    fn new_client(&mut self, peer_addr: Option<SocketAddr>) -> Self::Handler {
        SshHandler {
            core: self.core.clone(),
            addr: peer_addr,
            service: None,
            transaction: None,
        }
    }
}