use std::collections::BTreeMap;
use crate::error::GitInnerError;
use crate::objects::ObjectTrait;
use crate::objects::types::ObjectType;
use crate::odb::OdbTransaction;
use crate::sha::HashValue;
use bstr::ByteSlice;
use bytes::{Bytes, BytesMut};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefDelta {
    pub id: HashValue,
    pub base_sha: HashValue,
    pub delta_data: Bytes,
}

impl RefDelta {
    pub(crate) async fn apply_delta(
        base_hash: &HashValue,
        delta_data: &Bytes,
        txn: Arc<Box<dyn OdbTransaction>>,
        resolved_ofs: &BTreeMap<u64, (HashValue, Bytes, ObjectType)>,
    ) -> Result<(Bytes, ObjectType), GitInnerError> {
        let (base_obj_bytes, obj) = match resolved_ofs.iter().find(|(_, (hash, _, _))| hash == base_hash) {
            Some((_, (_, base_obj_bytes, obj))) => (base_obj_bytes.clone(), obj.clone()),
            None => {
                if txn.has_blob(base_hash).await? {
                    (txn.get_blob(base_hash).await?.get_data(), ObjectType::Blob)
                } else if txn.has_commit(base_hash).await? {
                    (txn.get_commit(base_hash).await?.get_data(), ObjectType::Commit)
                } else if txn.has_tree(base_hash).await? {
                    (txn.get_tree(base_hash).await?.get_data(), ObjectType::Tree)
                } else if txn.has_tag(base_hash).await? {
                    (txn.get_tag(base_hash).await?.get_data(), ObjectType::Tag)
                } else {
                    dbg!(&base_hash);
                    return Err(GitInnerError::MissingBaseObject);
                }
            }
        };

        let result = Self::apply_git_delta(&base_obj_bytes, delta_data)?;
        Ok((result, obj))
    }
    fn apply_git_delta(base: &Bytes, delta: &Bytes) -> Result<Bytes, GitInnerError> {
        let mut delta_reader = &delta[..];
        let base_size = Self::read_varint(&mut delta_reader)?;
        let result_size = Self::read_varint(&mut delta_reader)?;

        if base_size != base.len() {
            return Err(GitInnerError::DeltaBaseSizeMismatch);
        }
        let mut result = Vec::with_capacity(result_size);
        while !delta_reader.is_empty() {
            let opcode = delta_reader[0];
            delta_reader = &delta_reader[1..];
            if (opcode & 0x80) != 0 {
                let mut copy_offset = 0usize;
                let mut copy_size = 0usize;
                if (opcode & 0x01) != 0 {
                    copy_offset |= delta_reader[0] as usize;
                    delta_reader = &delta_reader[1..];
                }
                if (opcode & 0x02) != 0 {
                    copy_offset |= (delta_reader[0] as usize) << 8;
                    delta_reader = &delta_reader[1..];
                }
                if (opcode & 0x04) != 0 {
                    copy_offset |= (delta_reader[0] as usize) << 16;
                    delta_reader = &delta_reader[1..];
                }
                if (opcode & 0x08) != 0 {
                    copy_offset |= (delta_reader[0] as usize) << 24;
                    delta_reader = &delta_reader[1..];
                }
                if (opcode & 0x10) != 0 {
                    copy_size |= delta_reader[0] as usize;
                    delta_reader = &delta_reader[1..];
                }
                if (opcode & 0x20) != 0 {
                    copy_size |= (delta_reader[0] as usize) << 8;
                    delta_reader = &delta_reader[1..];
                }
                if (opcode & 0x40) != 0 {
                    copy_size |= (delta_reader[0] as usize) << 16;
                    delta_reader = &delta_reader[1..];
                }
                if copy_size == 0 {
                    copy_size = 0x10000;
                }
                result.extend_from_slice(&base[copy_offset..copy_offset + copy_size]);
            } else if opcode != 0 {
                let insert_size = opcode as usize;
                result.extend_from_slice(&delta_reader[..insert_size]);
                delta_reader = &delta_reader[insert_size..];
            } else {
                return Err(GitInnerError::DeltaInvalidInstruction);
            }
        }

        if result.len() != result_size {
            return Err(GitInnerError::DeltaResultSizeMismatch);
        }
        Ok(Bytes::from(result))
    }

    fn read_varint(input: &mut &[u8]) -> Result<usize, GitInnerError> {
        let mut result = 0usize;
        let mut shift = 0;
        loop {
            if input.is_empty() {
                return Err(GitInnerError::UnexpectedEof);
            }
            let byte = input[0];
            *input = &input[1..];
            result |= ((byte & 0x7F) as usize) << shift;
            shift += 7;
            if (byte & 0x80) == 0 {
                break;
            }
        }
        Ok(result)
    }
}
impl RefDelta {}

impl RefDelta {
    pub fn new(
        base_sha: HashValue,
        delta_data: Bytes,
        hash_version: impl Fn(&Bytes) -> HashValue,
    ) -> Self {
        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(format!("ref-delta {}\0", delta_data.len()).as_bytes());
        hash_input.extend_from_slice(&delta_data);
        let id = hash_version(&Bytes::from(hash_input));
        Self {
            id,
            base_sha,
            delta_data,
        }
    }

    pub fn parse(
        mut input: BytesMut,
        hash_len: usize,
        hash_version: impl Fn(&Bytes) -> HashValue,
    ) -> Result<Self, GitInnerError> {
        if input.len() < hash_len {
            return Err(GitInnerError::UnexpectedEof);
        }
        let base_sha_bytes = input.split_to(hash_len);
        let base_sha = HashValue::from_str(
            &base_sha_bytes
                .to_str()
                .map_err(|_| GitInnerError::InvalidUtf8)?,
        )
        .ok_or(GitInnerError::InvalidData)?;
        Ok(RefDelta::new(base_sha, Bytes::from(input), hash_version))
    }

    pub fn size(&self) -> usize {
        self.delta_data.len()
    }
}

impl ObjectTrait for RefDelta {
    fn get_type(&self) -> ObjectType {
        ObjectType::RefDelta
    }

    fn get_size(&self) -> usize {
        self.delta_data.len()
    }

    fn get_data(&self) -> Bytes {
        self.delta_data.clone()
    }
}

impl std::fmt::Display for RefDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Type: RefDelta")?;
        writeln!(f, "Base SHA: {}", self.base_sha)?;
        writeln!(f, "Size: {}", self.delta_data.len())
    }
}
