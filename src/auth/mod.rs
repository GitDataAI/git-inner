use crate::error::GitInnerError;

#[async_trait::async_trait]
pub trait Auth:Send + Sync + 'static  {
    async fn authenticate(&self, username: &str, password: &str, namespace: &str, repo: &str) -> Result<AccessLevel, GitInnerError>;
    async fn auth_public_key(&self, public_key: &str, namespace: &str, repo: &str) -> Result<AccessLevel, GitInnerError>;
}

pub enum AccessLevel {
    Read,
    Write,
    Admin,
}