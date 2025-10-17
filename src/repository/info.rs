use crate::error::GitInnerError;
use crate::rpc::gitfs::{RepositoryInfoRequest, RpcRepository};
use crate::serve::AppCore;

impl AppCore {
    /// Retrieves repository information identified by the request's namespace and name.
    ///
    /// # Returns
    ///
    /// `RpcRepository` on success, `GitInnerError` on error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crate::RepositoryInfoRequest;
    /// # // `core` should be an initialized `AppCore` instance available in your context.
    /// # #[tokio::test]
    /// # async fn example() {
    /// let req = RepositoryInfoRequest { namespace: "org".into(), name: "repo".into() };
    /// let _repo = core.repo_info(req).await;
    /// # }
    /// ```
    pub async fn repo_info(&self, req: RepositoryInfoRequest) -> Result<RpcRepository, GitInnerError> {
        self
            .repo_store
            .repo_info(req.namespace, req.name)
            .await
    }
}