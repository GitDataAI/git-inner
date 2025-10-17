use tonic::{Request, Response, Status};
use crate::rpc::gitfs::{RepositoryInfoRequest, RepositoryInitRequest, RepositoryInitResponse, RpcRepository};
use crate::rpc::service::RpcServiceCore;
use crate::serve::AppCore;


#[async_trait::async_trait]
impl crate::rpc::gitfs::repository_service_server::RepositoryService for RpcServiceCore {
    async fn init(&self, request: Request<RepositoryInitRequest>) -> Result<Response<RepositoryInitResponse>, Status> {
        self.app.init_repository(request.into_inner()).await
            .map_err(|e| Status::internal(format!("{:?}",e)))
            .map(|r| Response::new(r))
    }

    async fn set_public(&self, request: Request<RpcRepository>) -> Result<Response<RpcRepository>, Status> {
        self
            .app
            .set_public(request.into_inner())
            .await
            .map_err(|e| Status::internal(format!("{:?}",e)))
            .map(|r| Response::new(r))
    }

    async fn set_private(&self, request: Request<RpcRepository>) -> Result<Response<RpcRepository>, Status> {
        self
            .app
            .set_private(request.into_inner())
            .await
            .map_err(|e| Status::internal(format!("{:?}",e)))
            .map(|r| Response::new(r))
    }

    async fn delete(&self, request: Request<RpcRepository>) -> Result<Response<RpcRepository>, Status> {
        todo!()
    }

    async fn info(&self, request: Request<RepositoryInfoRequest>) -> Result<Response<RpcRepository>, Status> {
        self
            .app
            .repo_info(request.into_inner())
            .await
            .map_err(|e| Status::internal(format!("{:?}",e)))
            .map(|r| Response::new(r))
    }
}