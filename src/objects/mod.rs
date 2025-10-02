use bytes::Bytes;

pub mod signature;
pub mod blob;
pub mod types;
pub mod commit;
pub mod tree;
pub mod tag;


pub trait ObjectTrait {
    fn get_type(&self) -> types::ObjectType;
    fn get_size(&self) -> usize;
    fn get_data(&self) -> Bytes;
}