use crate::error::GitInnerError;
use crate::objects::ObjectTrait;
use crate::objects::blob::Blob;
use crate::objects::commit::Commit;
use crate::objects::tag::Tag;
use crate::objects::tree::Tree;
use crate::sha::HashValue;
use crate::transaction::upload::UploadPackTransaction;
use crate::write_pkt_line;
use bytes::Bytes;
use flate2::write::ZlibEncoder;
use std::collections::HashSet;
use std::io::Write;

#[derive(Clone, Debug)]
pub enum Object {
    Commit(Commit),
    Tree(Tree),
    Blob(Blob),
    Tag(Tag),
}
impl UploadPackTransaction {
    pub async fn find_object(&self, hash: HashValue) -> Result<Option<Object>, GitInnerError> {
        if let Ok(commit) = self.txn.repository.odb.get_commit(&hash).await {
            return Ok(Some(Object::Commit(commit)));
        }
        if let Ok(tree) = self.txn.repository.odb.get_tree(&hash).await {
            return Ok(Some(Object::Tree(tree)));
        }
        if let Ok(tag) = self.txn.repository.odb.get_tag(&hash).await {
            return Ok(Some(Object::Tag(tag)));
        }
        if let Ok(blob) = self.txn.repository.odb.get_blob(&hash).await {
            return Ok(Some(Object::Blob(blob)));
        }

        Ok(None)
    }

    pub async fn recursion_pack_pool_found_iter(
        &self,
        objs: &mut Vec<Object>,
        visited: &mut HashSet<HashValue>,
        root: HashValue,
    ) -> Result<(), GitInnerError> {
        let mut stack = vec![(root, 0usize)];
        while let Some((hash, depth)) = stack.pop() {
            if !visited.insert(hash.clone()) || self.have.contains(&hash) {
                continue;
            }
            if let Some(max_depth) = self.depth {
                if depth >= max_depth as usize {
                    continue;
                }
            }
            let obj_opt = self.find_object(hash.clone()).await?;
            let Some(obj) = obj_opt else {
                continue;
            };
            match obj {
                Object::Commit(commit) => {
                    if let Some(tree) = commit.tree.clone() {
                        stack.push((tree, depth));
                    }
                    for parent in commit.parents.clone() {
                        stack.push((parent, depth + 1));
                    }
                    objs.push(Object::Commit(commit));
                }
                Object::Tree(tree) => {
                    for entry in tree.tree_items.clone() {
                        stack.push((entry.id.clone(), depth));
                    }
                    objs.push(Object::Tree(tree));
                }
                Object::Tag(tag) => {
                    if self.include_tag {
                        stack.push((tag.object_hash.clone(), depth));
                    }
                    objs.push(Object::Tag(tag));
                }
                Object::Blob(blob) => {
                    objs.push(Object::Blob(blob));
                }
            }
        }
        Ok(())
    }

    pub async fn send_shallow_info(
        &self,
        shallow_commits: &HashSet<HashValue>,
    ) -> Result<(), GitInnerError> {
        for hash in shallow_commits {
            self.txn
                .call_back
                .send(write_pkt_line(format!("shallow {}\n", hash)).freeze())
                .await;
        }
        Ok(())
    }
}

impl Object {
    pub fn zlib(&self) -> Result<Bytes, GitInnerError> {
        let body = match self {
            Object::Blob(blob) => blob.get_data(),
            Object::Tree(tree) => tree.get_data(),
            Object::Commit(commit) => commit.get_data(),
            Object::Tag(tag) => tag.get_data(),
        };

        let type_code = match self {
            Object::Commit(_) => 1u8,
            Object::Tree(_) => 2u8,
            Object::Blob(_) => 3u8,
            Object::Tag(_) => 4u8,
        };

        let mut header = vec![];
        let mut size = body.len();
        let mut first_byte = ((size & 0x0F) as u8) | (type_code << 4);
        size >>= 4;

        if size != 0 {
            first_byte |= 0x80;
        }
        header.push(first_byte);
        while size != 0 {
            let mut byte = (size & 0x7F) as u8;
            size >>= 7;
            if size != 0 {
                byte |= 0x80;
            }
            header.push(byte);
        }
        let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder
            .write_all(&body)
            .map_err(|_| GitInnerError::ZlibError)?;
        let compressed_body = encoder.finish().map_err(|_| GitInnerError::ZlibError)?;
        let mut result = header;
        result.extend_from_slice(&compressed_body);

        Ok(Bytes::from(result))
    }
}
