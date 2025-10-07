use crate::error::GitInnerError;
use crate::refs::{RefItem, RefsManager};
use crate::sha::{HashValue, HashVersion};
use async_trait::async_trait;
use futures_util::stream::TryStreamExt;
use mongodb::bson::{Uuid, doc};
use mongodb::{Client, Collection};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct MongoRefItem {
    pub repo_uid: Uuid,
    pub ref_item: RefItem,
}

pub struct MongoRefsManager {
    pub repo_uid: Uuid,
    pub default_branch: String,
    pub db_client: Client,
    pub refs: Collection<MongoRefItem>,
    pub hash_version: HashVersion,
}

#[async_trait]
impl RefsManager for MongoRefsManager {
    async fn head(&self) -> Result<RefItem, GitInnerError> {
        let result = self
            .refs
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "ref_item.is_head": true
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;

        match result {
            Some(mongo_ref_item) => Ok(mongo_ref_item.ref_item),
            None => Ok(RefItem {
                name: "HEAD".to_string(),
                value: self.hash_version.default(),
                is_branch: false,
                is_tag: false,
                is_head: true,
            }),
        }
    }

    async fn refs(&self) -> Result<Vec<RefItem>, GitInnerError> {
        let cursor = self
            .refs
            .find(doc! {
                "repo_uid": self.repo_uid
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;
        let ref_items: Vec<RefItem> = cursor
            .try_collect::<Vec<MongoRefItem>>()
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?
            .into_iter()
            .map(|mongo_ref_item| mongo_ref_item.ref_item)
            .collect();
        Ok(ref_items)
    }

    async fn tags(&self) -> Result<Vec<RefItem>, GitInnerError> {
        let cursor = self
            .refs
            .find(doc! {
                "repo_uid": self.repo_uid,
                "ref_item.is_tag": true
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;
        let ref_items: Vec<RefItem> = cursor
            .try_collect::<Vec<MongoRefItem>>()
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?
            .into_iter()
            .map(|mongo_ref_item| mongo_ref_item.ref_item)
            .collect();

        Ok(ref_items)
    }

    async fn branches(&self) -> Result<Vec<RefItem>, GitInnerError> {
        let cursor = self
            .refs
            .find(doc! {
                "repo_uid": self.repo_uid,
                "ref_item.is_branch": true
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;
        let ref_items: Vec<RefItem> = cursor
            .try_collect::<Vec<MongoRefItem>>()
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?
            .into_iter()
            .map(|mongo_ref_item| mongo_ref_item.ref_item)
            .collect();

        Ok(ref_items)
    }

    async fn del_refs(&self, ref_name: String) -> Result<(), GitInnerError> {
        if let Some(refs_name_prefix) = Some(ref_name.strip_prefix("refs/heads/").unwrap()) {
            if refs_name_prefix == self.default_branch {
                return Err(GitInnerError::DefaultBranchCannotBeDeleted);
            }
        }
        self.refs
            .delete_one(doc! {
                "repo_uid": self.repo_uid,
                "ref_item.name": ref_name
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;

        Ok(())
    }

    async fn create_refs(
        &self,
        ref_name: String,
        ref_value: HashValue,
    ) -> Result<(), GitInnerError> {
        let is_branch = ref_name.starts_with("refs/heads/");
        let is_tag = ref_name.starts_with("refs/tags/");
        let mut is_head = ref_name == "HEAD";
        if let Some(refs_name_prefix) = Some(ref_name.strip_prefix("refs/heads/").unwrap()) {
            if refs_name_prefix == self.default_branch {
                is_head = true;
            }
        }
        let ref_item = RefItem {
            name: ref_name.clone(),
            value: ref_value,
            is_branch,
            is_tag,
            is_head,
        };

        let mongo_ref_item = MongoRefItem {
            repo_uid: self.repo_uid,
            ref_item,
        };

        self.refs
            .insert_one(mongo_ref_item)
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;

        Ok(())
    }

    async fn update_refs(
        &self,
        ref_name: String,
        ref_value: HashValue,
    ) -> Result<(), GitInnerError> {
        let update = doc! {
            "$set": {
                "ref_item.value": mongodb::bson::to_bson(&ref_value)?
            }
        };
        self.refs
            .update_one(
                doc! {
                    "repo_uid": self.repo_uid,
                    "ref_item.name": ref_name
                },
                update,
            )
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;

        Ok(())
    }

    async fn get_refs(&self, ref_name: String) -> Result<RefItem, GitInnerError> {
        let result = self
            .refs
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "ref_item.name": ref_name
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;

        match result {
            Some(mongo_ref_item) => Ok(mongo_ref_item.ref_item),
            None => Err(GitInnerError::ObjectNotFound(self.hash_version.default())),
        }
    }

    async fn exists_refs(&self, ref_name: String) -> Result<bool, GitInnerError> {
        let result = self
            .refs
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "ref_item.name": ref_name
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;

        Ok(result.is_some())
    }

    async fn get_value_refs(&self, ref_name: String) -> Result<HashValue, GitInnerError> {
        let result = self
            .refs
            .find_one(doc! {
                "repo_uid": self.repo_uid,
                "ref_item.name": ref_name
            })
            .await
            .map_err(|e| GitInnerError::MongodbError(e.to_string()))?;

        match result {
            Some(mongo_ref_item) => Ok(mongo_ref_item.ref_item.value),
            None => Err(GitInnerError::ObjectNotFound(self.hash_version.default())),
        }
    }
}
