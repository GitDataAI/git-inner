use crate::error::GitInnerError;
use crate::rpc::gitfs::{RepositoryInfoRequest, RpcRepository};
use crate::serve::AppCore;

impl AppCore {
    pub async fn repo_info(
        &self,
        req: RepositoryInfoRequest,
    ) -> Result<RpcRepository, GitInnerError> {
        self.repo_store.repo_info(req.namespace, req.name).await
    }
}
