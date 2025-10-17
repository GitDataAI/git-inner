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

#[derive(Deserialize, Serialize, Clone, Debug, Eq, PartialEq, Copy)]
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

#[derive(Clone)]
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
            HashValue::Sha1(sha1) => sha1.clone().state.to_vec(),
            HashValue::Sha256(sha256) => sha256.clone().state.to_vec(),
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

impl Serialize for HashValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            HashValue::Sha1(sha1) => serializer.serialize_str(&sha1.to_string()),
            HashValue::Sha256(sha256) => serializer.serialize_str(&sha256.to_string()),
        }
    }
}

impl<'de> Deserialize<'de> for HashValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Some(hash) = HashValue::from_str(&s) {
            return Ok(hash);
        }
        Err(serde::de::Error::custom("Invalid hash value"))
    }
}

impl PartialEq<Self> for HashValue {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl Eq for HashValue {}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{Bytes, BytesMut};
    use serde_json;

    #[test]
    fn test_hashversion_len() {
        assert_eq!(HashVersion::Sha1.len(), 20);
        assert_eq!(HashVersion::Sha256.len(), 32);
    }

    #[test]
    fn test_hashversion_default() {
        let sha1 = HashVersion::Sha1.default();
        let sha256 = HashVersion::Sha256.default();
        assert!(matches!(sha1, HashValue::Sha1(_)));
        assert!(matches!(sha256, HashValue::Sha256(_)));
    }

    #[test]
    fn test_hashversion_hash() {
        let data = Bytes::from_static(b"abc");
        let sha1 = HashVersion::Sha1.hash(data.clone());
        let sha256 = HashVersion::Sha256.hash(data);
        assert!(matches!(sha1, HashValue::Sha1(_)));
        assert!(matches!(sha256, HashValue::Sha256(_)));
    }

    #[test]
    fn test_hashvalue_from_bytes() {
        let sha1_bytes = BytesMut::from(&[0u8; 20][..]);
        let sha256_bytes = BytesMut::from(&[0u8; 32][..]);
        assert!(matches!(
            HashValue::from_bytes(&sha1_bytes),
            Some(HashValue::Sha1(_))
        ));
        assert!(matches!(
            HashValue::from_bytes(&sha256_bytes),
            Some(HashValue::Sha256(_))
        ));
        let invalid_bytes = BytesMut::from(&[0u8; 10][..]);
        assert!(HashValue::from_bytes(&invalid_bytes).is_none());
    }

    #[test]
    fn test_hashvalue_is_zero() {
        let sha1 = HashValue::Sha1(sha1::Sha1::new());
        let sha256 = HashValue::Sha256(sha256::Sha256::new());
        assert!(sha1.is_zero());
        assert!(sha256.is_zero());
    }

    #[test]
    fn test_hashvalue_raw_and_new() {
        let sha1 = HashValue::new(HashVersion::Sha1);
        let sha256 = HashValue::new(HashVersion::Sha256);
        assert_eq!(sha1.raw().len(), 20);
        assert_eq!(sha256.raw().len(), 32);
    }

    #[test]
    fn test_hashvalue_get_version() {
        let sha1 = HashValue::new(HashVersion::Sha1);
        let sha256 = HashValue::new(HashVersion::Sha256);
        assert_eq!(sha1.get_version(), HashVersion::Sha1);
        assert_eq!(sha256.get_version(), HashVersion::Sha256);
    }

    #[test]
    fn test_hashvalue_from_str() {
        let sha1_str = "0000000000000000000000000000000000000000";
        let sha256_str = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(matches!(
            HashValue::from_str(sha1_str),
            Some(HashValue::Sha1(_))
        ));
        assert!(matches!(
            HashValue::from_str(sha256_str),
            Some(HashValue::Sha256(_))
        ));
        assert!(HashValue::from_str("invalid").is_none());
    }

    #[test]
    fn test_hashvalue_display_debug_eq() {
        let sha1 = HashValue::new(HashVersion::Sha1);
        let sha256 = HashValue::new(HashVersion::Sha256);
        assert_eq!(format!("{:?}", sha1), format!("{}", sha1));
        assert_eq!(sha1, sha1.clone());
        assert_eq!(sha256, sha256.clone());
        assert_ne!(sha1, sha256);
    }

    #[test]
    fn test_hashvalue_serialize_deserialize() {
        let sha1 = HashValue::new(HashVersion::Sha1);
        let sha256 = HashValue::new(HashVersion::Sha256);
        let s1 = serde_json::to_string(&sha1).unwrap();
        let s2 = serde_json::to_string(&sha256).unwrap();
        let d1: HashValue = serde_json::from_str(&s1).unwrap();
        let d2: HashValue = serde_json::from_str(&s2).unwrap();
        assert_eq!(sha1, d1);
        assert_eq!(sha256, d2);
    }

    #[test]
    fn test_sha_trait_on_hashvalue() {
        let mut sha1 = HashValue::new(HashVersion::Sha1);
        let mut sha256 = HashValue::new(HashVersion::Sha256);
        sha1.update(b"abc");
        sha256.update(b"abc");
        let _ = sha1.finalize();
        let _ = sha256.finalize();
        sha1.reset();
        sha256.reset();
        assert!(sha1.is_zero());
        assert!(sha256.is_zero());
    }
}
