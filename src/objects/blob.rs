use std::fmt::Display;
use bytes::Bytes;
use crate::objects::ObjectTrait;
use crate::objects::types::ObjectType;
use crate::sha::HashValue;

#[derive(Eq, Debug, Clone)]
pub struct Blob {
    pub id: HashValue,
    pub data: Bytes,
}

impl PartialEq for Blob {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Display for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Type: Blob")?;
        writeln!(f, "Size: {}", self.data.len())
    }
}

impl ObjectTrait for Blob {
    fn get_type(&self) -> ObjectType {
        ObjectType::Blob
    }

    fn get_size(&self) -> usize {
        self.data.len()
    }

    fn get_data(&self) -> Bytes {
        self.data.clone()
    }
}