use std::fmt::Display;
use crate::objects::signature::Signature;
use crate::objects::types::ObjectType;
use crate::sha::HashValue;

#[derive(Eq, Debug, Clone)]
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

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "object {}\ntype {}\ntag {}\ntagger {}\n\n{}",
            self.object_hash, self.object_type, self.tag_name, self.tagger, self.message
        )
    }
}