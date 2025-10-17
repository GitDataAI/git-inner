use crate::rpc::gitfs::RpcRepository;
use crate::serve::AppCore;

impl AppCore {
    /// Marks a repository as public and returns an updated repository object.
    ///
    /// Calls the underlying store to set the repository visibility to public, and returns a clone
    /// of the provided `RpcRepository` with `is_private` set to `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::rpc::RpcRepository;
    /// # // Ensure an async runtime is available when running this example.
    /// # async fn run_example(app: &crate::AppCore) -> Result<(), crate::error::GitInnerError> {
    /// let repo = RpcRepository { namespace: "alice".into(), name: "project".into(), is_private: true };
    /// let updated = app.set_public(repo.clone()).await?;
    /// assert_eq!(updated.is_private, false);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_public(&self, repo: RpcRepository) -> Result<RpcRepository, crate::error::GitInnerError> {
        self
            .repo_store
            .set_visibility(repo.namespace.clone(), repo.name.clone(), true)
            .await?;
        let mut repo = repo.clone();
        repo.is_private = false;
        Ok(repo)
    }
    /// Marks the given repository as private in the storage and returns a modified clone.
    ///
    /// On success returns the provided `RpcRepository` cloned with `is_private` set to `true`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crate::repository::set::AppCore;
    /// # use crate::rpc::RpcRepository;
    /// # async fn example(app: &AppCore, repo: RpcRepository) -> Result<(), crate::error::GitInnerError> {
    /// let updated = app.set_private(repo).await?;
    /// assert!(updated.is_private);
    /// # Ok(())
    /// # }
    /// ```
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