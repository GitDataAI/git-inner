use crate::config::rpc::RpcConfig;
use crate::rpc::gitfs::commit_service_server::CommitServiceServer;
use crate::rpc::gitfs::refs_service_server::RefsServiceServer;
use crate::rpc::gitfs::repository_service_server::RepositoryServiceServer;
use crate::rpc::gitfs::tree_service_server::TreeServiceServer;
use crate::rpc::service::RpcServiceCore;
use crate::serve::AppCore;

impl RpcServiceCore {
    pub fn new(app: AppCore) -> Self {
        Self { app }
    }
    pub async fn run(&self, config: RpcConfig) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("{}:{}", config.url, config.port).parse()?;
        log::info!("RPC server listening on {}", addr);
        tonic::transport::Server::builder()
            .add_service(CommitServiceServer::new(self.clone()))
            .add_service(RefsServiceServer::new(self.clone()))
            .add_service(RepositoryServiceServer::new(self.clone()))
            .add_service(TreeServiceServer::new(self.clone()))
            .serve(addr)
            .await?;
        Ok(())
    }
}
