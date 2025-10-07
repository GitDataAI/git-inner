use crate::objects::ObjectTrait;
use crate::objects::types::ObjectType;
use crate::sha::{HashValue, HashVersion, Sha};
use bytes::Bytes;
use std::fmt::Display;

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

impl Blob {
    pub fn parse(input: Bytes, version: HashVersion) -> Blob {
        let mut hash = version.default();
        hash.update(format!("blob {}\0", input.len()).as_bytes());
        hash.update(&input);
         hash.finalize();
        Blob { id: hash, data: input }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha::HashVersion;

    #[test]
    fn test_parse() {
        let blob = Blob::parse(Bytes::from("hello world"), HashVersion::Sha1);
        dbg!(blob);
    }
}
