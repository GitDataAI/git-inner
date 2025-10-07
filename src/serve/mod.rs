use std::sync::Arc;
use crate::error::GitInnerError;
use crate::repository::Repository;
use async_trait::async_trait;

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
}
pub mod mongo;
