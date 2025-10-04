use crate::error::GitInnerError;
use crate::objects::blob::Blob;
use crate::objects::commit::Commit;
use crate::objects::tag::Tag;
use crate::objects::tree::Tree;
use crate::sha::HashValue;
use async_trait::async_trait;

#[async_trait]
pub trait Odb: Send + Sync {
    async fn put_commit(&self, commit: &Commit) -> Result<HashValue, GitInnerError>;
    async fn get_commit(&self, hash: &HashValue) -> Result<Commit, GitInnerError>;
    async fn has_commit(&self, hash: &HashValue) -> Result<bool, GitInnerError>;
    async fn put_tag(&self, tag: &Tag) -> Result<HashValue, GitInnerError>;
    async fn get_tag(&self, hash: &HashValue) -> Result<Tag, GitInnerError>;
    async fn has_tag(&self, hash: &HashValue) -> Result<bool, GitInnerError>;
    async fn put_tree(&self, tree: &Tree) -> Result<HashValue, GitInnerError>;
    async fn get_tree(&self, hash: &HashValue) -> Result<Tree, GitInnerError>;
    async fn has_tree(&self, hash: &HashValue) -> Result<bool, GitInnerError>;
    async fn put_blob(&self, blob: Blob) -> Result<HashValue, GitInnerError>;
    async fn get_blob(&self, hash: &HashValue) -> Result<Blob, GitInnerError>;
    async fn has_blob(&self, hash: &HashValue) -> Result<bool, GitInnerError>;
    async fn begin_transaction(&self) -> Result<Box<dyn OdbTransaction>, GitInnerError>;
}

#[async_trait]
pub trait OdbTransaction: Send + Sync + Odb {
    async fn commit(&self) -> Result<(), GitInnerError>;
    async fn abort(&self) -> Result<(), GitInnerError>;
    async fn rollback(&self) -> Result<(), GitInnerError>;
}

pub mod mongo;
