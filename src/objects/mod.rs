use bytes::Bytes;

pub mod blob;
pub mod commit;
pub mod ofs_delta;
pub mod ref_delta;
pub mod signature;
pub mod tag;
pub mod tree;
pub mod types;
pub trait ObjectTrait {
    fn get_type(&self) -> types::ObjectType;
    fn get_size(&self) -> usize;
    fn get_data(&self) -> Bytes;
}
