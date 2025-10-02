use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ObjectType {
    Unknown = 0,
    Commit = 1,
    Tree = 2,
    Blob = 3,
    Tag = 4,
    // Type 5 is reserved for future expansion
    OffsetDelta = 6,
    HashDelta = 7,
    OffsetZstdelta = 255, // Move this to an unused value
}

impl ObjectType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => ObjectType::Commit,
            2 => ObjectType::Tree,
            3 => ObjectType::Blob,
            4 => ObjectType::Tag,
            6 => ObjectType::OffsetDelta,
            7 => ObjectType::HashDelta,
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
            ObjectType::OffsetDelta => 6,
            ObjectType::HashDelta => 7,
            ObjectType::OffsetZstdelta => 255,
        }
    }
    pub fn from_str(value: &str) -> Self {
        match value {
            "commit" => ObjectType::Commit,
            "tree" => ObjectType::Tree,
            "blob" => ObjectType::Blob,
            "tag" => ObjectType::Tag,
            "offset-delta" => ObjectType::OffsetDelta,
            "hash-delta" => ObjectType::HashDelta,
            "offset-zstdelta" => ObjectType::OffsetZstdelta,
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
            ObjectType::OffsetDelta => "offset-delta",
            ObjectType::HashDelta => "hash-delta",
            ObjectType::OffsetZstdelta => "offset-zstdelta",
        }
    }
    pub fn to_raw(&self) -> &'static [u8] {
        match self {
            ObjectType::Unknown => b"unknown",
            ObjectType::Commit => b"commit",
            ObjectType::Tree => b"tree",
            ObjectType::Blob => b"blob",
            ObjectType::Tag => b"tag",
            ObjectType::OffsetDelta => b"offset-delta",
            ObjectType::HashDelta => b"hash-delta",
            ObjectType::OffsetZstdelta => b"offset-zstdelta",
        }
    }
}

impl Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}