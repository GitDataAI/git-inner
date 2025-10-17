use crate::config::ssh::SshConfig;
use crate::config::{AppConfig, CFG};
use crate::error::GitInnerError;
use crate::serve::AppCore;
use crate::ssh::handler::SshHandler;
use log::{info, warn};
use russh::keys::PublicKeyBase64;
use russh::keys::ssh_encoding::base64::Encoding;
use russh::keys::ssh_encoding::{DecodePem, EncodePem, LineEnding, base64};
use russh::server::Server;
use sha2::Digest;
use std::net::SocketAddr;
use std::sync::Arc;

pub struct SshServer {
    pub core: AppCore,
    pub config: SshConfig,
}

impl SshServer {
    /// Starts the SSH server using the configured host, port, and server key.
    ///
    /// If a server public key is configured the function uses it; otherwise it generates a new Ed25519 key,
    /// persists the new public key to the global configuration, and uses that key. The server is configured
    /// with large channel and event buffers and a short authentication rejection timeout before it begins
    /// listening on the configured address. The function returns an error if key decoding/encoding, configuration
    /// persistence, or server startup fails.
    ///
    /// # Returns
    ///
    /// `Ok(())` on successful startup, `Err(GitInnerError::SshServerStartError)` if the server fails to start.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tokio::runtime::Runtime;
    /// # use your_crate::ssh::service::SshServer;
    /// # fn main() {
    /// let rt = Runtime::new().unwrap();
    /// rt.block_on(async {
    ///     let mut server = SshServer::new().await.unwrap();
    ///     // Starts the server (may bind to configured address); in tests/CI this may fail if port is unavailable.
    ///     let _ = server.run().await;
    /// });
    /// # }
    /// ```
    pub async fn run(&mut self) -> Result<(), GitInnerError> {
        if !self.config.enabled {
            warn!("SSH server is disabled");
            return Ok(());
        }
        info!("Starting SSH server");
        let mut cfg = russh::server::Config::default();
        if let Some(public_key) = &self.config.server_public_key {
            let mut figure = sha2::Sha256::default();
            figure.update(public_key);
            let fingerprint = figure.finalize();
            info!(
                "SSH server public key fingerprint: sha256:{}",
                hex::encode(fingerprint)
            );
            let private_key = russh::keys::PrivateKey::decode_pem(
                &base64::Base64::decode_vec(public_key)
                    .map_err(|e| GitInnerError::SshServerStartError(e.to_string()))?,
            )
            .map_err(|e| GitInnerError::SshServerStartError(e.to_string()))?;
            cfg.keys = vec![private_key]
        } else {
            info!("SSH server public is empty, using new key");
            let private_key = russh::keys::PrivateKey::random(
                &mut russh::keys::key::safe_rng(),
                russh::keys::Algorithm::Ed25519,
            )
            .map_err(|e| GitInnerError::SshServerStartError(e.to_string()))?;
            let mut pem = vec![];
            let private_key_pem = private_key
                .encode_pem(LineEnding::LF, &mut pem)
                .map_err(|e| GitInnerError::SshServerStartError(e.to_string()))?;
            let private_key_pem = base64::Base64::encode_string(private_key_pem.as_bytes());
            let mut config = CFG.clone();
            config.ssh.server_public_key = Some(private_key_pem);
            config
                .save()
                .map_err(|e| GitInnerError::SshServerStartError(e.to_string()))?;
            self.config = config.ssh;
            let mut figure = sha2::Sha256::default();
            figure.update(private_key.public_key_base64().as_bytes());
            let fingerprint = figure.finalize();
            info!(
                "SSH server new public key fingerprint: sha256:{}",
                hex::encode(fingerprint)
            );
            cfg.keys = vec![private_key];
        }
        cfg.channel_buffer_size = usize::MAX;
        cfg.event_buffer_size = usize::MAX;
        cfg.auth_rejection_time = std::time::Duration::from_secs(3);
        self.run_on_address(
            Arc::new(cfg),
            format!("{}:{}", self.config.host, self.config.port),
        )
        .await
        .map_err(|error| GitInnerError::SshServerStartError(error.to_string()))?;
        Ok(())
    }
    /// Creates an SshServer initialized from the global application core and the current SSH configuration.
    ///
    /// # Returns
    /// `SshServer` initialized with the application core and the current SSH configuration, or a `GitInnerError` if acquiring the application core fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures::executor::block_on;
    /// let server = block_on(crate::ssh::service::SshServer::new()).unwrap();
    /// ```
    pub async fn new() -> Result<Self, GitInnerError> {
        let app = AppCore::app()?;
        let cfg = AppConfig::ssh();
        Ok(Self {
            core: app,
            config: cfg.clone(),
        })
    }
    /// Create and run an SSH server using the current application configuration.
    ///
    /// Starts the server (including key handling and listener setup) and runs it until it stops or an error occurs.
    ///
    /// # Returns
    ///
    /// `Ok(())` on successful startup and normal shutdown, `Err(GitInnerError)` if initialization or runtime fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     SshServer::ssh_spawn().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn ssh_spawn() -> Result<(), GitInnerError> {
        Self::new().await?.run().await
    }
}

impl Server for SshServer {
    type Handler = SshHandler;

    /// Creates a new SSH handler for an incoming connection.
    ///
    /// The returned handler is initialized with a clone of the server's core state and the
    /// optional peer socket address; `service` and `transaction` are unset.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Given a mutable `SshServer` named `server`:
    /// let handler = server.new_client(Some("127.0.0.1:22".parse().unwrap()));
    /// assert_eq!(handler.addr.unwrap().ip().to_string(), "127.0.0.1");
    /// ```
    fn new_client(&mut self, peer_addr: Option<SocketAddr>) -> Self::Handler {
        SshHandler {
            core: self.core.clone(),
            addr: peer_addr,
            service: None,
            transaction: None,
        }
    }
}
