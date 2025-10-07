use crate::error::GitInnerError;
use crate::objects::blob::Blob;
use crate::objects::commit::Commit;
use crate::objects::tag::Tag;
use crate::objects::tree::Tree;
use crate::odb::mongo::transaction::OdbMongoTransaction;
use crate::odb::mongo::{OdbMongoCommit, OdbMongoTag, OdbMongoTree};
use crate::odb::{Odb, OdbTransaction};
use crate::sha::HashValue;
use async_trait::async_trait;
use mongodb::bson::{Uuid, doc};
use mongodb::{Client, Collection};
use object_store::path::Path;
use object_store::{ObjectStore, PutPayload};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct OdbMongoObject {
    pub repo_uid: Uuid,
    pub store: Arc<Box<dyn ObjectStore>>,
    pub db_client: Client,
    pub commit: Collection<OdbMongoCommit>,
    pub tag: Collection<OdbMongoTag>,
    pub tree: Collection<OdbMongoTree>,
}

#[async_trait]
impl Odb for OdbMongoObject {
    async fn put_commit(&self, commit: &Commit) -> Result<HashValue, GitInnerError> {
        let obj = OdbMongoCommit {
            repo_uid: self.repo_uid,
            hash: commit.hash.clone(),
            commit: commit.clone(),
        };
        let result = self
            .commit
            .insert_one(obj)
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)));
        match result {
            Ok(_) => Ok(commit.hash.clone()),
            Err(e) => Err(e),
        }
    }

    async fn get_commit(&self, hash: &HashValue) -> Result<Commit, GitInnerError> {
        let result = self
            .commit
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "hash": mongodb::bson::to_bson(&hash)?
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        match result {
            Some(obj) => Ok(obj.commit),
            None => Err(GitInnerError::ObjectNotFound(hash.clone())),
        }
    }

    async fn has_commit(&self, hash: &HashValue) -> Result<bool, GitInnerError> {
        let result = self
            .commit
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "hash": mongodb::bson::to_bson(&hash)?
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        match result {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn put_tag(&self, tag: &Tag) -> Result<HashValue, GitInnerError> {
        let obj = OdbMongoTag {
            repo_uid: self.repo_uid,
            hash: tag.id.clone(),
            tag: tag.clone(),
        };
        let result = self
            .tag
            .insert_one(obj)
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)));
        match result {
            Ok(_) => Ok(tag.id.clone()),
            Err(e) => Err(e),
        }
    }

    async fn get_tag(&self, hash: &HashValue) -> Result<Tag, GitInnerError> {
        let result = self
            .tag
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "hash": mongodb::bson::to_bson(&hash)?
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        match result {
            Some(obj) => Ok(obj.tag),
            None => Err(GitInnerError::ObjectNotFound(hash.clone())),
        }
    }

    async fn has_tag(&self, hash: &HashValue) -> Result<bool, GitInnerError> {
        let result = self
            .tag
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "hash": mongodb::bson::to_bson(&hash)?
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        match result {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn put_tree(&self, tree: &Tree) -> Result<HashValue, GitInnerError> {
        let obj = OdbMongoTree {
            repo_uid: self.repo_uid,
            hash: tree.id.clone(),
            tree: tree.clone(),
        };
        let result = self
            .tree
            .insert_one(obj)
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)));
        match result {
            Ok(_) => Ok(tree.id.clone()),
            Err(e) => Err(e),
        }
    }

    async fn get_tree(&self, hash: &HashValue) -> Result<Tree, GitInnerError> {
        let result = self
            .tree
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "hash": mongodb::bson::to_bson(&hash)?
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        match result {
            Some(obj) => Ok(obj.tree),
            None => Err(GitInnerError::ObjectNotFound(hash.clone())),
        }
    }

    async fn has_tree(&self, hash: &HashValue) -> Result<bool, GitInnerError> {
        let result = self
            .tree
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "hash": mongodb::bson::to_bson(&hash)?
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        match result {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn put_blob(&self, blob: Blob) -> Result<HashValue, GitInnerError> {
        let path = format!("{}/{}", self.repo_uid, blob.id.to_string());
        let result = self
            .store
            .put(&Path::from(path), PutPayload::from(blob.data))
            .await
            .map_err(|e| GitInnerError::ObjectStoreError(format!("{}", e)));
        match result {
            Ok(_) => Ok(blob.id.clone()),
            Err(e) => Err(e),
        }
    }

    async fn get_blob(&self, hash: &HashValue) -> Result<Blob, GitInnerError> {
        let path = format!("{}/{}", self.repo_uid, hash.to_string());
        let result = self
            .store
            .get(&Path::from(path))
            .await
            .map_err(|e| GitInnerError::ObjectStoreError(format!("{}", e)))?;
        Ok(Blob {
            id: hash.clone(),
            data: result
                .bytes()
                .await
                .map_err(|e| GitInnerError::ObjectStoreError(format!("{}", e)))?,
        })
    }

    async fn has_blob(&self, hash: &HashValue) -> Result<bool, GitInnerError> {
        let path = format!("{}/{}", self.repo_uid, hash.to_string());
        let result = self.store.head(&Path::from(path)).await;
        Ok(result.is_ok())
    }

    async fn begin_transaction(&self) -> Result<Box<dyn OdbTransaction>, GitInnerError> {
        let mut session = self
            .db_client
            .start_session()
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        session
            .start_transaction()
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        let transaction = OdbMongoTransaction {
            db_client: self.db_client.clone(),
            session: Arc::new(Mutex::new(session)),
            repo_uid: self.repo_uid.clone(),
            commit: self.commit.clone(),
            tag: self.tag.clone(),
            tree: self.tree.clone(),
            store: self.store.clone(),
            id: chrono::Utc::now().timestamp(),
        };
        Ok(Box::new(transaction))
    }
}
