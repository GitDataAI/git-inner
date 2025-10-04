use serde::{Deserialize, Serialize};
use std::fmt::Display;
use bytes::Bytes;
use bytes::BytesMut;
use crate::sha::{HashValue, HashVersion};

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ObjectType {
    Unknown = 0,
    Commit = 1,
    Tree = 2,
    Blob = 3,
    Tag = 4,
    // 5 reserved
    OfsDelta = 6, // offset delta
    RefDelta = 7, // reference delta
}

impl ObjectType {
    pub fn hash_value(&self, hash_version: HashVersion, data: &[u8]) -> HashValue {
        let mut start = BytesMut::from(self.to_raw());
        start.extend_from_slice(data);
        hash_version.hash(Bytes::from(start))
    }
}

impl ObjectType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => ObjectType::Commit,
            2 => ObjectType::Tree,
            3 => ObjectType::Blob,
            4 => ObjectType::Tag,
            6 => ObjectType::OfsDelta,
            7 => ObjectType::RefDelta,
            _ => ObjectType::Unknown,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            ObjectType::Unknown => 0,
            ObjectType::Commit => 1,
            ObjectType::Tree => 2,
            ObjectType::Blob => 3,
            ObjectType::Tag => 4,
            ObjectType::OfsDelta => 6,
            ObjectType::RefDelta => 7,
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "commit" => ObjectType::Commit,
            "tree" => ObjectType::Tree,
            "blob" => ObjectType::Blob,
            "tag" => ObjectType::Tag,
            "ofs-delta" => ObjectType::OfsDelta,
            "ref-delta" => ObjectType::RefDelta,
            _ => ObjectType::Unknown,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            ObjectType::Unknown => "unknown",
            ObjectType::Commit => "commit",
            ObjectType::Tree => "tree",
            ObjectType::Blob => "blob",
            ObjectType::Tag => "tag",
            ObjectType::OfsDelta => "ofs-delta",
            ObjectType::RefDelta => "ref-delta",
        }
    }

    pub fn to_raw(&self) -> &'static [u8] {
        self.to_str().as_bytes()
    }
}

impl Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}
