use bincode::{Decode, Encode};
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::hash::Hash;

pub mod sha1;
pub mod sha256;

pub trait Sha {
    fn update(&mut self, data: &[u8]);
    fn finalize(&mut self) -> Vec<u8>;
    fn reset(&mut self);
}

#[derive(Deserialize, Serialize, Clone, Debug, Eq, PartialEq)]
pub enum HashVersion {
    Sha1,
    Sha256,
}

impl HashVersion {
    pub(crate) fn len(&self) -> usize {
        match self {
            HashVersion::Sha1 => 20,
            HashVersion::Sha256 => 32,
        }
    }
}

impl HashVersion {
    pub fn default(&self) -> HashValue {
        match self {
            HashVersion::Sha1 => HashValue::Sha1(sha1::Sha1::new()),
            HashVersion::Sha256 => HashValue::Sha256(sha256::Sha256::new()),
        }
    }
    pub fn hash(&self, data: Bytes) -> HashValue {
        match self {
            HashVersion::Sha1 => HashValue::Sha1(sha1::Sha1::from_bytes(data)),
            HashVersion::Sha256 => HashValue::Sha256(sha256::Sha256::from_bytes(data)),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Decode, Encode)]
pub enum HashValue {
    Sha1(sha1::Sha1),
    Sha256(sha256::Sha256),
}

impl HashValue {
    pub fn from_bytes(p0: &BytesMut) -> Option<HashValue> {
        let vec = p0.to_vec();
        match vec.len() {
            20 => Some(HashValue::Sha1(sha1::Sha1::from_vec(vec)?)),
            32 => Some(HashValue::Sha256(sha256::Sha256::from_vec(vec)?)),
            _ => None,
        }
    }
}

impl HashValue {
    pub fn is_zero(&self) -> bool {
        match self {
            HashValue::Sha1(sha1) => sha1.is_zero(),
            HashValue::Sha256(sha256) => sha256.is_zero(),
        }
    }
    pub fn raw(&self) -> Vec<u8> {
        match self {
            HashValue::Sha1(sha1) => sha1.clone().finalize(),
            HashValue::Sha256(sha256) => sha256.clone().finalize(),
        }
    }
    pub fn new(version: HashVersion) -> HashValue {
        match version {
            HashVersion::Sha1 => HashValue::Sha1(sha1::Sha1::new()),
            HashVersion::Sha256 => HashValue::Sha256(sha256::Sha256::new()),
        }
    }
    pub fn get_version(&self) -> HashVersion {
        match self {
            HashValue::Sha1(_) => HashVersion::Sha1,
            HashValue::Sha256(_) => HashVersion::Sha256,
        }
    }
    pub fn from_str(s: &str) -> Option<HashValue> {
        if s.len() == 40 {
            if let Ok(sha1) = sha1::Sha1::from_str(s) {
                return Some(HashValue::Sha1(sha1));
            }
        }
        if s.len() == 64 {
            if let Ok(sha256) = sha256::Sha256::from_str(s) {
                return Some(HashValue::Sha256(sha256));
            }
        }
        None
    }
}

impl Sha for HashValue {
    fn update(&mut self, data: &[u8]) {
        match self {
            HashValue::Sha1(sha1) => sha1.update(data),
            HashValue::Sha256(sha256) => sha256.update(data),
        }
    }

    fn finalize(&mut self) -> Vec<u8> {
        match self {
            HashValue::Sha1(sha1) => sha1.finalize(),
            HashValue::Sha256(sha256) => sha256.finalize(),
        }
    }

    fn reset(&mut self) {
        match self {
            HashValue::Sha1(sha1) => sha1.reset(),
            HashValue::Sha256(sha256) => sha256.reset(),
        }
    }
}

impl Hash for HashValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            HashValue::Sha1(sha1) => sha1.hash(state),
            HashValue::Sha256(sha256) => sha256.hash(state),
        }
    }
}

impl Display for HashValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashValue::Sha1(sha1) => write!(f, "{}", sha1.to_string()),
            HashValue::Sha256(sha256) => write!(f, "{}", sha256.to_string()),
        }
    }
}

impl Debug for HashValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}
