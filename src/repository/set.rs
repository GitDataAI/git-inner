use crate::rpc::gitfs::RpcRepository;
use crate::serve::AppCore;

impl AppCore {
    pub async fn set_public(&self, repo: RpcRepository) -> Result<RpcRepository, crate::error::GitInnerError> {
        self
            .repo_store
            .set_visibility(repo.namespace.clone(), repo.name.clone(), true)
            .await?;
        let mut repo = repo.clone();
        repo.is_private = false;
        Ok(repo)
    }
    pub async fn set_private(&self, repo: RpcRepository) -> Result<RpcRepository, crate::error::GitInnerError> {
        self
            .repo_store
            .set_visibility(repo.namespace.clone(), repo.name.clone(), false)
            .await?;
        let mut repo = repo.clone();
        repo.is_private = true;
        Ok(repo)
    }
}