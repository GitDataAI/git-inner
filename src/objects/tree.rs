use crate::error::GitInnerError;
use crate::objects::types::ObjectType;
use crate::objects::ObjectTrait;
use crate::sha::{HashValue, HashVersion};
use bincode::{Decode, Encode};
use bytes::{Bytes, BytesMut};
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
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = match *self {
            TreeItemMode::Blob => "blob",
            TreeItemMode::BlobExecutable => "blob executable",
            TreeItemMode::Tree => "tree",
            TreeItemMode::Commit => "commit",
            TreeItemMode::Link => "link",
        };
        write!(f, "{}", s)
    }
}

impl TreeItemMode {
    pub fn tree_item_type_from_bytes(mode: &[u8]) -> Result<TreeItemMode, GitInnerError> {
        Ok(match mode {
            b"040000" | b"40000" => TreeItemMode::Tree, // 兼容旧格式
            b"100644" | b"100664" | b"100640" => TreeItemMode::Blob,
            b"100755" => TreeItemMode::BlobExecutable,
            b"120000" => TreeItemMode::Link,
            b"160000" => TreeItemMode::Commit,
            _ => {
                return Err(GitInnerError::InvalidTreeItem(
                    String::from_utf8_lossy(mode).to_string(),
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

    pub fn to_str(self) -> &'static str {
        std::str::from_utf8(self.to_bytes()).unwrap()
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct TreeItem {
    pub mode: TreeItemMode,
    pub id: HashValue,
    pub name: String,
}

impl Display for TreeItem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} {} {}", self.mode, self.name, self.id)
    }
}

impl TreeItem {
    pub fn new(mode: TreeItemMode, id: HashValue, name: String) -> Self {
        Self { mode, id, name }
    }

    pub fn to_data(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.mode.to_bytes());
        bytes.push(b' ');
        bytes.extend_from_slice(self.name.as_bytes());
        bytes.push(0);
        let raw = self.id.raw();
        let raw_bytes = match raw.len() {
            20 | 32 => raw.clone(),
            40 | 64 => {
                hex::decode(raw).expect("invalid hex hash string")
            }
            len => panic!("unexpected hash length: {}", len),
        };

        bytes.extend_from_slice(&raw_bytes);
        bytes
    }
}


#[derive(Eq, Debug, Clone, Serialize, Deserialize)]
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
                item.mode.to_str(),
                match item.mode {
                    TreeItemMode::Blob | TreeItemMode::BlobExecutable => "blob",
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
        self.tree_items.iter().map(|i| i.to_data().len()).sum()
    }

    fn get_data(&self) -> Bytes {
        let mut data = Vec::new();
        for i in &self.tree_items {
            data.extend_from_slice(&i.to_data());
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
                .ok_or_else(|| GitInnerError::InvalidTreeItem("Missing null after filename".into()))?;
            let name_bytes = &input[pos..pos + null_pos];
            let name = String::from_utf8(name_bytes.to_vec())
                .map_err(|_| GitInnerError::InvalidTreeItem("Filename not UTF-8".into()))?;

            pos += null_pos + 1;
            if pos + 20 > input_len {
                return Err(GitInnerError::InvalidTreeItem("Tree item hash truncated".into()));
            }
            let id = HashValue::from_bytes(&BytesMut::from(&input[pos..pos + 20])).unwrap();
            pos += 20;

            tree_items.push(TreeItem::new(mode, id, name));
        }

        if pos != input_len {
            return Err(GitInnerError::InvalidTreeItem(format!(
                "Unexpected trailing bytes in tree: {}/{}",
                pos, input_len
            )));
        }

        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(format!("tree {}\0", input_len).as_bytes());
        hash_input.extend_from_slice(&input);
        let id = hash_version.hash(Bytes::from(hash_input));

        Ok(Tree { id, tree_items })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_tree_roundtrip() {
        let blob_hash = HashVersion::Sha1.hash(Bytes::from("hello world"));
        let sub_tree_hash = HashVersion::Sha1.hash(Bytes::from("subdir content"));

        let tree = Tree {
            id: HashVersion::Sha1.hash(Bytes::from("dummy")),
            tree_items: vec![
                TreeItem::new(TreeItemMode::Blob, blob_hash.clone(), "file.txt".into()),
                TreeItem::new(TreeItemMode::Tree, sub_tree_hash.clone(), "subdir".into()),
            ],
        };

        let data = tree.get_data();
        assert!(
            data.windows(b"040000 subdir".len()).any(|w| w == b"040000 subdir"),
            "Tree data missing '040000 subdir'"
        );

        // Roundtrip parse
        let parsed = Tree::parse(data.clone(), HashVersion::Sha1).unwrap();
        assert_eq!(parsed.tree_items.len(), 2);
        assert_eq!(parsed.tree_items[1].mode, TreeItemMode::Tree);
        assert_eq!(parsed.tree_items[1].name, "subdir");

        // Re-serialize to ensure deterministic structure
        let re_data = parsed.get_data();
        assert_eq!(data, re_data, "Tree serialization mismatch");

        // Display formatting sanity check
        let output = format!("{}", parsed);
        assert!(
            output.contains("040000 tree"),
            "Display output missing expected mode"
        );

        println!("Tree display:\n{}", output);
    }
}
