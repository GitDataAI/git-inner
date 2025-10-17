use crate::error::GitInnerError;
use crate::rpc::gitfs::{RepositoryInitRequest, RepositoryInitResponse};
use crate::serve::AppCore;

impl AppCore {
    pub async fn init_repository(
        &self,
        req: RepositoryInitRequest,
    ) -> Result<RepositoryInitResponse, GitInnerError> {
        let owner = uuid::Uuid::parse_str(&req.owner).unwrap();
        let uid = uuid::Uuid::parse_str(&req.uid).unwrap();
        self.repo_store
            .create_repo(
                req.namespace,
                req.name,
                owner,
                match req.hash_version {
                    1 => 1,
                    2 => 2,
                    _ => 1,
                },
                uid,
                req.default_branch,
                !req.is_private,
            )
            .await
    }
}
