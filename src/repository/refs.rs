use crate::error::GitInnerError;
use crate::refs::RefItem;
use crate::repository::Repository;
use crate::sha::HashValue;



impl Repository {
    pub async fn refs_insert(&self, name: String, value: HashValue) -> Result<(), GitInnerError> {
        self.refs.create_refs(name, value).await
    }
    pub async fn refs_get(&self, name: String) -> Result<RefItem, GitInnerError> {
        self.refs.get_refs(name).await
    }
    pub async fn refs_update(&self, name: String, value: HashValue) -> Result<(), GitInnerError> {
        self.refs.update_refs(name, value).await
    }
    pub async fn refs_delete(&self, name: String) -> Result<(), GitInnerError> {
        self.refs.del_refs(name).await
    }
    pub async fn refs_list(&self) -> Result<Vec<RefItem>, GitInnerError> {
        self.refs.refs().await
    }
    pub async fn refs_exists(&self, name: String) -> Result<bool, GitInnerError> {
        self.refs.exists_refs(name).await
    }
    pub async fn refs_get_value(&self, name: String) -> Result<HashValue, GitInnerError> {
        self.refs.get_value_refs(name).await
    }
}