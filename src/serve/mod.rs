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
    pub fn new(repo_store: Arc<Box<dyn RepoStore>>) -> Self {
        Self { repo_store }
    }
    pub fn init(&self) -> Result<(), GitInnerError> {
        APP.set(self.clone()).map_err(|_| GitInnerError::AppInitError)
    }
    pub fn app() -> Result<AppCore, GitInnerError> {
        APP.get().cloned().ok_or(GitInnerError::AppNotInit)
    }
}
pub mod mongo;
