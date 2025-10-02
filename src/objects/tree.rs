use std::fmt::Display;
use bincode::{Decode, Encode};
use encoding_rs::GBK;
use serde::{Deserialize, Serialize};
use crate::error::GitInnerError;
use crate::sha::HashValue;

#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize, Hash, Encode, Decode)]
pub enum TreeItemMode {
    Blob,
    BlobExecutable,
    Tree,
    Commit,
    Link,
}

impl Display for TreeItemMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _print = match *self {
            TreeItemMode::Blob => "blob",
            TreeItemMode::BlobExecutable => "blob executable",
            TreeItemMode::Tree => "tree",
            TreeItemMode::Commit => "commit",
            TreeItemMode::Link => "link",
        };

        write!(f, "{}", String::from(_print))
    }
}

impl TreeItemMode {
    pub fn tree_item_type_from_bytes(mode: &[u8]) -> Result<TreeItemMode, GitInnerError> {
        Ok(match mode {
            b"40000" => TreeItemMode::Tree,
            b"100644" => TreeItemMode::Blob,
            b"100755" => TreeItemMode::BlobExecutable,
            b"120000" => TreeItemMode::Link,
            b"160000" => TreeItemMode::Commit,
            b"100664" => TreeItemMode::Blob,
            b"100640" => TreeItemMode::Blob,
            _ => {
                return Err(GitInnerError::InvalidTreeItem(
                    String::from_utf8(mode.to_vec()).unwrap(),
                ));
            }
        })
    }

    pub fn to_bytes(self) -> &'static [u8] {
        match self {
            TreeItemMode::Blob => b"100644",
            TreeItemMode::BlobExecutable => b"100755",
            TreeItemMode::Link => b"120000",
            TreeItemMode::Tree => b"40000",
            TreeItemMode::Commit => b"160000",
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Hash, Encode, Decode)]
pub struct TreeItem {
    pub mode: TreeItemMode,
    pub id: HashValue,
    pub name: String,
}


impl Display for TreeItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.mode,
            self.name,
            self.id.to_string()
        )
    }
}

impl TreeItem {
    pub fn new(mode: TreeItemMode, id: HashValue, name: String) -> TreeItem {
        TreeItem { mode, id, name }
    }
    pub fn to_data(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.mode.to_bytes());
        bytes.push(b' ');
        bytes.extend_from_slice(self.name.as_bytes());
        bytes.push(b'\0');
        bytes.extend_from_slice(&self.id.raw());
        bytes
    }
    pub fn is_tree(&self) -> bool {
        self.mode == TreeItemMode::Tree
    }

    pub fn is_blob(&self) -> bool {
        self.mode == TreeItemMode::Blob
    }
    pub fn is_commit(&self) -> bool {
        self.mode == TreeItemMode::Commit
    }
    pub fn is_link(&self) -> bool {
        self.mode == TreeItemMode::Link
    }
    pub fn is_blob_executable(&self) -> bool {
        self.mode == TreeItemMode::BlobExecutable
    }
}

#[derive(Eq, Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Tree {
    pub id: HashValue,
    pub tree_items: Vec<TreeItem>,
}

impl PartialEq for Tree {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Display for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Tree: {}", self.id.to_string())?;
        for item in &self.tree_items {
            writeln!(f, "{item}")?;
        }
        Ok(())
    }
}