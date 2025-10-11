use crate::repository::Repository;
use crate::rpc::gitfs::RpcRepository;
use crate::serve::AppCore;

pub mod gitfs;
pub mod service;

pub async fn rpc_repository_to_inner_repository(app_core: AppCore, rpc_repository: RpcRepository) -> Result<Repository, crate::error::GitInnerError> {
    let repo = app_core
        .repo_store
        .repo(rpc_repository.namespace, rpc_repository.name)
        .await?;
    Ok(repo)
}