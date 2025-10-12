use tonic::{Request, Response, Status};
use crate::rpc::gitfs::{RepositoryInfoRequest, RepositoryInitRequest, RepositoryInitResponse, RpcRepository};
use crate::serve::AppCore;

#[derive(Clone)]
pub struct RpcRepositoryService {
    pub app: AppCore,
}

#[async_trait::async_trait]
impl crate::rpc::gitfs::repository_service_server::RepositoryService for RpcRepositoryService {
    async fn init(&self, request: Request<RepositoryInitRequest>) -> Result<Response<RepositoryInitResponse>, Status> {
        todo!()
    }

    async fn set_public(&self, request: Request<RpcRepository>) -> Result<Response<RpcRepository>, Status> {
        todo!()
    }

    async fn set_private(&self, request: Request<RpcRepository>) -> Result<Response<RpcRepository>, Status> {
        todo!()
    }

    async fn delete(&self, request: Request<RpcRepository>) -> Result<Response<RpcRepository>, Status> {
        todo!()
    }

    async fn info(&self, request: Request<RepositoryInfoRequest>) -> Result<Response<RpcRepository>, Status> {
        todo!()
    }
}