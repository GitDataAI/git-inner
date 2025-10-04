use crate::error::GitInnerError;
use crate::objects::ObjectTrait;
use crate::objects::types::ObjectType;
use crate::sha::{HashValue, HashVersion};
use bincode::{Decode, Encode};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

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
        write!(f, "{} {} {}", self.mode, self.name, self.id.to_string())
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for item in &self.tree_items {
            writeln!(
                f,
                "{} {} {}\t{}",
                item.mode,
                match item.mode {
                    TreeItemMode::Blob => "blob",
                    TreeItemMode::BlobExecutable => "blob",
                    TreeItemMode::Tree => "tree",
                    TreeItemMode::Commit => "commit",
                    TreeItemMode::Link => "link",
                },
                item.id,
                item.name
            )?;
        }
        Ok(())
    }
}

impl ObjectTrait for Tree {
    fn get_type(&self) -> ObjectType {
        ObjectType::Tree
    }

    fn get_size(&self) -> usize {
        self.tree_items
            .iter()
            .map(|item| item.to_data().len())
            .sum()
    }

    fn get_data(&self) -> Bytes {
        let mut data = Vec::new();
        for item in &self.tree_items {
            data.extend_from_slice(&item.to_data());
        }
        Bytes::from(data)
    }
}

impl Tree {
    pub fn parse(input: Bytes, hash_version: HashVersion) -> Result<Tree, GitInnerError> {
        let mut tree_items = Vec::new();
        let mut pos = 0;
        let input_len = input.len();
        while pos < input_len {
            let space_pos = input[pos..]
                .iter()
                .position(|&b| b == b' ')
                .ok_or_else(|| GitInnerError::InvalidTreeItem("Missing space after mode".into()))?;
            let mode_bytes = &input[pos..pos + space_pos];
            let mode = TreeItemMode::tree_item_type_from_bytes(mode_bytes)?;

            pos += space_pos + 1;
            let null_pos = input[pos..]
                .iter()
                .position(|&b| b == b'\0')
                .ok_or_else(|| {
                    GitInnerError::InvalidTreeItem("Missing null after filename".into())
                })?;
            let name_bytes = &input[pos..pos + null_pos];
            let name = String::from_utf8(name_bytes.to_vec())
                .map_err(|_| GitInnerError::InvalidTreeItem("Filename not UTF-8".into()))?;

            pos += null_pos + 1;
            if pos + 20 > input_len {
                return Err(GitInnerError::InvalidTreeItem(
                    "Tree item hash truncated".into(),
                ));
            }
            let id = hash_version.hash(Bytes::from(input[pos..pos + 20].to_vec()));
            pos += 20;
            tree_items.push(TreeItem::new(mode, id, name));
        }
        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(format!("tree {}\0", input.len()).as_bytes());
        hash_input.extend_from_slice(&input);
        let id = hash_version.hash(Bytes::from(hash_input));

        Ok(Tree { id, tree_items })
    }
}
