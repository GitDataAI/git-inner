use crate::error::GitInnerError;
use crate::sha::HashValue;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait RefsManager: Send + Sync {
    async fn head(&self) -> Result<RefItem, GitInnerError>;
    async fn refs(&self) -> Result<Vec<RefItem>, GitInnerError>;
    async fn tags(&self) -> Result<Vec<RefItem>, GitInnerError>;
    async fn branches(&self) -> Result<Vec<RefItem>, GitInnerError>;
    async fn del_refs(&self, ref_name: String) -> Result<(), GitInnerError>;
    async fn create_refs(
        &self,
        ref_name: String,
        ref_value: HashValue,
    ) -> Result<(), GitInnerError>;
    async fn update_refs(
        &self,
        ref_name: String,
        ref_value: HashValue,
    ) -> Result<(), GitInnerError>;
    async fn get_refs(&self, ref_name: String) -> Result<RefItem, GitInnerError>;
    async fn exists_refs(&self, ref_name: String) -> Result<bool, GitInnerError>;
    async fn get_value_refs(&self, ref_name: String) -> Result<HashValue, GitInnerError>;
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RefItem {
    pub name: String,
    pub value: HashValue,
    pub is_branch: bool,
    pub is_tag: bool,
    pub is_head: bool,
}

pub mod mongo;
