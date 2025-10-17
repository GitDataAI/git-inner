use crate::error::GitInnerError;
use crate::objects::commit::Commit;
use crate::objects::tag::Tag;
use crate::objects::types::ObjectType;
use crate::odb::OdbTransaction;
use crate::sha::HashValue;
use crate::transaction::Transaction;
use std::sync::Arc;

impl Transaction {
    pub async fn process_object_data(
        &mut self,
        object_type: ObjectType,
        data: &[u8],
        txn: Arc<Box<dyn OdbTransaction>>,
    ) -> Result<HashValue, GitInnerError> {
        match object_type {
            ObjectType::Commit => self.handle_commit_object(data, txn).await,
            ObjectType::Tree => self.handle_tree_object(data, txn).await,
            ObjectType::Blob => self.handle_blob_object(data, txn).await,
            ObjectType::Tag => self.handle_tag_object(data, txn).await,
            _ => Err(GitInnerError::NotSupportVersion),
        }
    }
    async fn handle_commit_object(
        &mut self,
        data: &[u8],
        txn: Arc<Box<dyn OdbTransaction>>,
    ) -> Result<HashValue, GitInnerError> {
        let bytes = bytes::Bytes::from(data.to_vec());
        let commit = Commit::parse(bytes, self.repository.hash_version.clone());
        if let Ok(commit) = commit {
            txn.put_commit(&commit).await?;
            return Ok(commit.hash);
        }
        return Err(GitInnerError::CommitParseError);
    }

    async fn handle_tree_object(
        &mut self,
        data: &[u8],
        txn: Arc<Box<dyn OdbTransaction>>,
    ) -> Result<HashValue, GitInnerError> {
        let bytes = bytes::Bytes::from(data.to_vec());
        let tree = crate::objects::tree::Tree::parse(bytes, self.repository.hash_version.clone());
        if let Ok(tree) = tree {
            txn.put_tree(&tree).await?;
            return Ok(tree.id);
        }
        return Err(GitInnerError::TreeParseError);
    }

    async fn handle_blob_object(
        &mut self,
        data: &[u8],
        txn: Arc<Box<dyn OdbTransaction>>,
    ) -> Result<HashValue, GitInnerError> {
        let bytes = bytes::Bytes::from(data.to_vec());
        let blob = crate::objects::blob::Blob::parse(bytes, self.repository.hash_version.clone());
        let hash = blob.id.clone();
        txn.put_blob(blob).await?;
        Ok(hash)
    }

    async fn handle_tag_object(
        &mut self,
        data: &[u8],
        txn: Arc<Box<dyn OdbTransaction>>,
    ) -> Result<HashValue, GitInnerError> {
        let bytes = bytes::Bytes::from(data.to_vec());
        let tag = Tag::parse(bytes, self.repository.hash_version.clone());
        if let Ok(tag) = tag {
            txn.put_tag(&tag).await?;
            return Ok(tag.id);
        }
        return Err(GitInnerError::TagParseError);
    }
}
