use crate::error::GitInnerError;
use crate::objects::blob::Blob;
use crate::objects::commit::Commit;
use crate::objects::tag::Tag;
use crate::objects::tree::Tree;
use crate::odb::mongo::{OdbMongoCommit, OdbMongoTag, OdbMongoTree};
use crate::odb::{Odb, OdbTransaction};
use crate::sha::HashValue;
use async_trait::async_trait;
use mongodb::bson::{Uuid, doc};
use mongodb::{Client, ClientSession, Collection};
use object_store::path::Path;
use object_store::{ObjectStore, PutPayload};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

#[derive(Clone)]
pub struct OdbMongoTransaction {
    pub db_client: Client,
    pub session: Arc<Mutex<ClientSession>>,
    pub repo_uid: Uuid,
    pub commit: Collection<OdbMongoCommit>,
    pub tag: Collection<OdbMongoTag>,
    pub tree: Collection<OdbMongoTree>,
    pub store: Arc<Box<dyn ObjectStore>>,
    pub id: i64,
}

#[async_trait]
impl Odb for OdbMongoTransaction {
    async fn put_commit(&self, commit: &Commit) -> Result<HashValue, GitInnerError> {
        let obj = OdbMongoCommit {
            repo_uid: self.repo_uid,
            hash: commit.hash.clone(),
            commit: commit.clone(),
        };
        let mut session = self.session.lock().await;
        let result = self
            .commit
            .insert_one(obj)
            .session(&mut *session)
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)));
        match result {
            Ok(_) => Ok(commit.hash.clone()),
            Err(e) => Err(e),
        }
    }

    async fn get_commit(&self, hash: &HashValue) -> Result<Commit, GitInnerError> {
        let mut session = self.session.lock().await;
        let result = self
            .commit
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "hash": mongodb::bson::to_bson(&hash)?
            })
            .session(&mut *session)
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        match result {
            Some(obj) => Ok(obj.commit),
            None => Err(GitInnerError::ObjectNotFound(hash.clone())),
        }
    }

    async fn has_commit(&self, hash: &HashValue) -> Result<bool, GitInnerError> {
        let mut session = self.session.lock().await;
        let result = self
            .commit
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "hash": mongodb::bson::to_bson(&hash)?
            })
            .session(&mut *session)
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
        let mut session = self.session.lock().await;
        let result = self
            .tag
            .insert_one(obj)
            .session(&mut *session)
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)));
        match result {
            Ok(_) => Ok(tag.id.clone()),
            Err(e) => Err(e),
        }
    }

    async fn get_tag(&self, hash: &HashValue) -> Result<Tag, GitInnerError> {
        let mut session = self.session.lock().await;

        let result = self
            .tag
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "hash": mongodb::bson::to_bson(&hash)?
            })
            .session(&mut *session)
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        match result {
            Some(obj) => Ok(obj.tag),
            None => Err(GitInnerError::ObjectNotFound(hash.clone())),
        }
    }

    async fn has_tag(&self, hash: &HashValue) -> Result<bool, GitInnerError> {
        let mut session = self.session.lock().await;
        let result = self
            .tag
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "hash": mongodb::bson::to_bson(&hash)?
            })
            .session(&mut *session)
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
        let mut session = self.session.lock().await;
        let result = self
            .tree
            .insert_one(obj)
            .session(&mut *session)
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)));
        match result {
            Ok(_) => Ok(tree.id.clone()),
            Err(e) => Err(e),
        }
    }

    async fn get_tree(&self, hash: &HashValue) -> Result<Tree, GitInnerError> {
        let mut session = self.session.lock().await;
        let result = self
            .tree
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "hash": mongodb::bson::to_bson(&hash)?
            })
            .session(&mut *session)
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        match result {
            Some(obj) => Ok(obj.tree),
            None => Err(GitInnerError::ObjectNotFound(hash.clone())),
        }
    }

    async fn has_tree(&self, hash: &HashValue) -> Result<bool, GitInnerError> {
        let mut session = self.session.lock().await;
        let result = self
            .tree
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "hash": mongodb::bson::to_bson(&hash)?
            })
            .session(&mut *session)
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        match result {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn put_blob(&self, blob: Blob) -> Result<HashValue, GitInnerError> {
        let path = format!("{}/txn.{}/{}", self.repo_uid, self.id, blob.id.to_string());
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
        let result = match self
            .store
            .get(&Path::from(path))
            .await{
            Ok(result) => result,
            Err(_) => {
                let txn_path = format!("{}/txn.{}/{}", self.repo_uid, self.id, hash.to_string());
                let txn_result = self
                    .store
                    .get(&Path::from(txn_path))
                    .await;
                match txn_result {
                    Ok(result) => result,
                    Err(e) => {
                        return Err(GitInnerError::ObjectStoreError(format!("{}", e)));
                    }
                }
            }
        };
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
        let txn_path = format!("{}/txn.{}/{}", self.repo_uid, self.id,hash.to_string());
        let txn_result = self.store.head(&Path::from(txn_path)).await;
        Ok(result.is_ok() || txn_result.is_ok())
    }

    async fn begin_transaction(&self) -> Result<Box<dyn OdbTransaction>, GitInnerError> {
        unimplemented!()
    }
}

#[async_trait]
impl OdbTransaction for OdbMongoTransaction {
    async fn commit(&self) -> Result<(), GitInnerError> {
        let mut session = self.session.lock().await;
        let mut list = self.store.list(Option::from(&Path::from(format!(
            "{}/txn.{}",
            self.repo_uid, self.id
        ))));
        while let Some(Ok(next)) = list.next().await {
            self.store
                .copy_if_not_exists(
                    &next.location,
                    &Path::from(format!(
                        "{}/{}",
                        self.repo_uid,
                        next.location.filename().unwrap_or("")
                    )),
                )
                .await
                .map_err(|e| GitInnerError::ObjectStoreError(format!("{}", e)))?;
            self.store
                .delete(&next.location)
                .await
                .map_err(|e| GitInnerError::ObjectStoreError(format!("{}", e)))?;
        }
        session
            .commit_transaction()
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        Ok(())
    }

    async fn abort(&self) -> Result<(), GitInnerError> {
        let mut session = self.session.lock().await;
        session
            .abort_transaction()
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        let mut list = self.store.list(Option::from(&Path::from(format!(
            "{}/txn.{}",
            self.repo_uid, self.id
        ))));
        while let Some(Ok(next)) = list.next().await {
            self.store
                .delete(&next.location)
                .await
                .map_err(|e| GitInnerError::ObjectStoreError(format!("{}", e)))?;
        }
        Ok(())
    }

    async fn rollback(&self) -> Result<(), GitInnerError> {
        let mut session = self.session.lock().await;
        session
            .abort_transaction()
            .await
            .map_err(|e| GitInnerError::MongodbError(format!("{}", e)))?;
        let mut list = self.store.list(Option::from(&Path::from(format!(
            "{}/txn.{}",
            self.repo_uid, self.id
        ))));
        while let Some(Ok(next)) = list.next().await {
            self.store
                .delete(&next.location)
                .await
                .map_err(|e| GitInnerError::ObjectStoreError(format!("{}", e)))?;
        }
        Ok(())
    }
}
