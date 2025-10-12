use tonic::{Request, Response, Status};
use crate::rpc::gitfs::{CommitGetRequest, CommitGetResponse, CommitHeadRequest, CommitHeadResponse, CommitLogRequest, CommitLogResponse};
use crate::serve::AppCore;

#[derive(Clone)]
pub struct RpcCommitService {
    pub app: AppCore,
}

#[tonic::async_trait]
impl crate::rpc::gitfs::commit_service_server::CommitService for RpcCommitService {
    async fn head(&self, request: Request<CommitHeadRequest>) -> Result<Response<CommitHeadResponse>, Status> {
        todo!()
    }

    async fn get(&self, request: Request<CommitGetRequest>) -> Result<Response<CommitGetResponse>, Status> {
        todo!()
    }

    async fn log(&self, request: Request<CommitLogRequest>) -> Result<Response<CommitLogResponse>, Status> {
        todo!()
    }
}