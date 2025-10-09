use std::net::SocketAddr;
use crate::serve::AppCore;
use crate::ssh::handler::SshHandler;

pub struct SshServer {
    pub core: AppCore,
}


impl SshServer {
}


impl russh::server::Server for SshServer {
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


