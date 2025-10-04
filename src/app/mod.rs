use crate::error::GitInnerError;
use crate::repository::Repository;
use async_trait::async_trait;

pub struct AppCore {
    pub repo_store: Box<dyn RepoStore>,
}

#[async_trait]
pub trait RepoStore {
    async fn repo(&self, namespace: String, name: String) -> Result<Repository, GitInnerError>;
}

pub mod mongo;
