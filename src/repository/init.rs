use crate::error::GitInnerError;
use crate::rpc::gitfs::{RepositoryInitRequest, RepositoryInitResponse};
use crate::serve::AppCore;

impl AppCore {
    /// Initializes a repository from the given request and returns the creation result.
    ///
    /// Parses `req.owner` and `req.uid` as UUIDs, maps `req.hash_version` to a supported value (1 or 2, defaulting to 1),
    /// and calls the repository store to create the repository using the request fields. This function will panic if
    /// `req.owner` or `req.uid` are not valid UUID strings.
    ///
    /// # Returns
    ///
    /// `Ok(RepositoryInitResponse)` on success, `Err(GitInnerError)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// # use some_crate::{AppCore, RepositoryInitRequest};
    /// # async fn example(app: &AppCore) {
    /// let req = RepositoryInitRequest {
    ///     namespace: "myorg".into(),
    ///     name: "myrepo".into(),
    ///     owner: "550e8400-e29b-41d4-a716-446655440000".into(),
    ///     uid: "550e8400-e29b-41d4-a716-446655440001".into(),
    ///     hash_version: 1,
    ///     default_branch: "main".into(),
    ///     is_private: false,
    /// };
    ///
    /// let result = app.init_repository(req).await;
    /// // handle `result`
    /// # }
    /// ```
    pub async fn init_repository(&self, req: RepositoryInitRequest) -> Result<RepositoryInitResponse, GitInnerError> {
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
                !req.is_private
            )
            .await
    }
}