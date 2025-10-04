use crate::error::GitInnerError;
use crate::objects::ObjectTrait;
use crate::objects::signature::Signature;
use crate::objects::types::ObjectType;
use crate::sha::{HashValue, HashVersion};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io::Write;

#[derive(Eq, Clone, Serialize, Deserialize, Debug)]
pub struct Tag {
    pub id: HashValue,
    pub object_hash: HashValue,
    pub object_type: ObjectType,
    pub tag_name: String,
    pub tagger: Signature,
    pub message: String,
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Tag {
    pub fn parse(input: Bytes, hash_version: HashVersion) -> Result<Tag, GitInnerError> {
        let input_str = str::from_utf8(&input).map_err(|_| GitInnerError::InvalidUtf8)?;
        let split_index = input_str
            .find("\n\n")
            .ok_or(GitInnerError::MissingField("message"))?;
        let header_str = &input_str[..split_index];
        let message = &input_str[split_index + 2..];

        let mut object_hash: Option<HashValue> = None;
        let mut object_type: Option<ObjectType> = None;
        let mut tag_name: Option<String> = None;
        let mut tagger: Option<Signature> = None;

        for line in header_str.lines() {
            if line.starts_with("object ") {
                let hash_str = line["object ".len()..].trim();
                object_hash = HashValue::from_str(hash_str);
            } else if line.starts_with("type ") {
                let type_str = line["type ".len()..].trim();
                object_type = Some(ObjectType::from_str(type_str));
            } else if line.starts_with("tag ") {
                tag_name = Some(line["tag ".len()..].trim().to_string());
            } else if line.starts_with("tagger ") {
                let tagger_data = line["tagger ".len()..].trim();
                tagger = Signature::from_data(tagger_data.as_bytes().to_vec()).ok();
            }
        }
        let object_hash = object_hash.ok_or(GitInnerError::MissingField("object"))?;
        let object_type = object_type.ok_or(GitInnerError::MissingField("type"))?;
        let tag_name = tag_name.ok_or(GitInnerError::MissingField("tag"))?;
        let tagger = tagger.ok_or(GitInnerError::MissingField("tagger"))?;
        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(format!("tag {}\0", input.len()).as_bytes());
        hash_input.extend_from_slice(&input);
        let id = hash_version.hash(Bytes::from(hash_input));
        Ok(Tag {
            id,
            object_hash,
            object_type,
            tag_name,
            tagger,
            message: message.to_string(),
        })
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "object {}", self.object_hash)?;
        writeln!(f, "type {}", self.object_type)?;
        writeln!(f, "tag {}", self.tag_name)?;
        writeln!(f, "tagger {}", self.tagger)?;
        writeln!(f)?;
        write!(f, "{}", self.message)
    }
}

impl ObjectTrait for Tag {
    fn get_type(&self) -> ObjectType {
        ObjectType::Tag
    }

    fn get_size(&self) -> usize {
        let mut size = 0;
        size += b"object ".len() + self.object_hash.raw().len() + b"\n".len();
        size += b"type ".len() + self.object_type.to_string().len() + b"\n".len();
        size += b"tag ".len() + self.tag_name.len() + b"\n".len();
        size += b"tagger ".len() + self.tagger.to_string().len() + b"\n".len();
        size += b"\n".len();
        size += self.message.as_bytes().len();
        size
    }

    fn get_data(&self) -> Bytes {
        let mut data = Vec::new();
        write!(data, "object {}\n", self.object_hash).unwrap();
        write!(data, "type {}\n", self.object_type).unwrap();
        write!(data, "tag {}\n", self.tag_name).unwrap();
        write!(data, "tagger {}\n", self.tagger).unwrap();
        write!(data, "\n").unwrap();
        data.extend_from_slice(self.message.as_bytes());
        Bytes::from(data)
    }
}
