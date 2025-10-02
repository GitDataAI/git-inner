use crate::error::GitInnerError;
use crate::objects::blob::Blob;
use crate::objects::commit::Commit;
use crate::objects::tag::Tag;
use crate::objects::tree::Tree;
use crate::objects::types::ObjectType;
use crate::sha::HashValue;
use async_trait::async_trait;

pub mod localstore;


pub enum Object  {
    Tree(Tree),
    Commit(Commit),
    Blob(Blob),
    Tag(Tag),
    None,
}

impl Object {
    pub fn object_type(&self) -> ObjectType {
        match self {
            Object::Tree(_) => ObjectType::Tree,
            Object::Commit(_) => ObjectType::Commit,
            Object::Blob(_) => ObjectType::Blob,
            Object::Tag(_) => ObjectType::Tag,
            _ => ObjectType::Unknown,
        }
    }
}

#[async_trait]
pub trait Odb: Send + Sync {
    async fn get_object(&self, object_id: HashValue) -> Option<Object>;
    async fn put_object(&self,object: Object) -> Result<HashValue, GitInnerError>;
    async fn exists(&self,  object_id: HashValue) -> Result<bool, GitInnerError>;
    async fn delete_object(&self,  object_id: HashValue) -> Result<bool, GitInnerError>;
    async fn list_objects(&self) -> Result<Vec<HashValue>, GitInnerError>;
    async fn clear_repo(&self) -> Result<(), GitInnerError>;
    async fn begin_transaction(&self) -> Result<Box<dyn OdbTransaction>, GitInnerError>;
}


#[async_trait]
pub trait OdbTransaction: Send + Sync {
    async fn commit(&self) -> Result<(), GitInnerError>;
    async fn abort(&self) -> Result<(), GitInnerError>;
    async fn rollback(&self) -> Result<(), GitInnerError>;
}