use std::sync::Arc;
use crate::error::GitInnerError;
use crate::repository::Repository;
use async_trait::async_trait;
use tokio::sync::OnceCell;

pub static APP: OnceCell<AppCore> = OnceCell::const_new();

#[derive(Clone)]
pub struct AppCore {
    pub repo_store: Arc<Box<dyn RepoStore>>,
}

#[async_trait]
pub trait RepoStore:Send + Sync + 'static  {
    async fn repo(&self, namespace: String, name: String) -> Result<Repository, GitInnerError>;
}


impl AppCore {
    /// Creates a new AppCore that holds the provided repository store.
    ///
    /// The `repo_store` is stored as an `Arc<Box<dyn RepoStore>>` and used by the AppCore for repository access.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use async_trait::async_trait;
    ///
    /// struct DummyStore;
    ///
    /// #[async_trait]
    /// impl crate::RepoStore for DummyStore {
    ///     async fn repo(&self, _namespace: String, _name: String) -> Result<crate::Repository, crate::GitInnerError> {
    ///         unimplemented!()
    ///     }
    /// }
    ///
    /// let store = Arc::new(Box::new(DummyStore));
    /// let app = crate::AppCore::new(store);
    /// ```
    pub fn new(repo_store: Arc<Box<dyn RepoStore>>) -> Self {
        Self { repo_store }
    }
    /// Initialize the global application singleton with this `AppCore`.
    ///
    /// On success the global `APP` is set to a clone of this instance; if the global
    /// singleton was already initialized this returns `GitInnerError::AppInitError`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use crate::serve::AppCore;
    ///
    /// // Construct an AppCore (example repo store omitted) and initialize the global.
    /// let repo_store = Arc::new(Box::new(/* impl RepoStore */));
    /// let app = AppCore::new(repo_store);
    /// app.init().expect("failed to initialize global app");
    /// ```
    pub fn init(&self) -> Result<(), GitInnerError> {
        APP.set(self.clone()).map_err(|_| GitInnerError::AppInitError)
    }
    /// Retrieve the globally initialized AppCore instance.
    ///
    /// Returns a cloned AppCore if the global has been initialized, or `GitInnerError::AppNotInit` if it has not.
    ///
    /// # Examples
    ///
    /// ```
    /// // If the global has not been initialized this returns an error.
    /// assert!(app().is_err());
    /// ```
    pub fn app() -> Result<AppCore, GitInnerError> {
        APP.get().cloned().ok_or(GitInnerError::AppNotInit)
    }
}
pub mod mongo;