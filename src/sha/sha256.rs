use std::fmt::Display;
use std::hash::{Hash, Hasher};
use bincode::{Decode, Encode};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Sha256 as ExternalSha256, Digest};
use crate::error::GitInnerError;
use crate::sha::Sha;
use crate::sha::sha1::Sha1;

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Decode, Encode)]
pub struct Sha256 {
    state: [u8; 32],
    buffer: Vec<u8>,
}

impl Sha256 {
    pub(crate) fn from_bytes(p0: Bytes) -> Sha256 {
        let mut sha2 = sha2::Sha256::new();
        sha2.update(&p0);
        Sha256 {
            state: sha2.finalize().into(),
            buffer: Vec::new(),
        }
    }
}

impl Sha256 {
    pub fn new() -> Self {
        Sha256 {
            state: [0; 32],
            buffer: Vec::new(),
        }
    }
    pub fn is_zero(&self) -> bool {
        self.state == [0; 32]
    }
    pub fn from_str(s: &str) -> Result<Self, GitInnerError> {
        if s.len() != 64 {
            return Err(GitInnerError::InvalidSha256String);
        }
        let mut state = [0; 32];
        for i in 0..32 {
            state[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(|_| GitInnerError::InvalidSha256String)?;
        }
        Ok(Sha256 {
            state,
            buffer: Vec::new(),
        })
    }
}
impl Default for Sha256 {
    fn default() -> Self {
        Sha256::new()
    }
}
impl Display for Sha256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.state))
    }
}
impl Hash for Sha256 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.state);
    }
}
impl Sha for Sha256 {
    fn update(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    fn finalize(&mut self) -> Vec<u8> {
        let mut hasher = ExternalSha256::new();
        hasher.update(&self.buffer);
        let result = hasher.finalize();
        self.state.copy_from_slice(&result[..32]);
        self.state.to_vec()
    }

    fn reset(&mut self) {
        self.state = [0; 32];
        self.buffer.clear();
    }
}