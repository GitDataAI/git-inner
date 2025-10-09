use std::net::SocketAddr;
use std::sync::Arc;
use russh::server::Server;
use crate::error::GitInnerError;
use crate::serve::AppCore;
use crate::ssh::handler::SshHandler;

pub struct SshServer {
    pub core: AppCore,
}


impl SshServer {
    pub async fn run(&mut self) -> Result<(), GitInnerError> {
        let cfg = russh::server::Config::default();
        self
            .run_on_address(
                Arc::new(cfg),
                "0.0.0.0:22"
            )
            .await
            .map_err(|error| {
                GitInnerError::SshServerStartError(error.to_string())
            })?;
        Ok(())
    }
    pub async fn new() -> Result<Self, GitInnerError> {
        let app = AppCore::app()?;
        Ok(Self {
            core: app,
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


