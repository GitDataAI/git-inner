use crate::error::GitInnerError;
use crate::sha::Sha;
use bincode::{Decode, Encode};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1 as ExternalSha1};
use std::hash::Hash;

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Decode, Encode)]
pub struct Sha1 {
    state: [u8; 20],
    buffer: Vec<u8>,
}

impl Sha1 {
    pub(crate) fn from_bytes(p0: Bytes) -> Sha1 {
        let mut sha1 = sha1::Sha1::new();
        sha1.update(p0);
        Sha1 {
            state: <[u8; 20]>::from(sha1.finalize()),
            buffer: Vec::new(),
        }
    }
    pub fn from_vec(p0: Vec<u8>) -> Option<Sha1> {
        if p0.len() != 20 {
            return None;
        }
        let state: [u8; 20] = p0.try_into().ok()?;
        Some(Sha1 {
            state,
            buffer: Vec::new(),
        })
    }
}

impl Sha1 {
    pub fn new() -> Sha1 {
        Sha1 {
            state: [0; 20],
            buffer: Vec::new(),
        }
    }
    pub fn is_zero(&self) -> bool {
        self.state == [0; 20]
    }
    pub fn from_str(s: &str) -> Result<Sha1, GitInnerError> {
        if s.len() != 40 {
            return Err(GitInnerError::InvalidSha1String);
        }
        let mut state = [0; 20];
        for i in 0..20 {
            state[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(|_| GitInnerError::InvalidSha1String)?;
        }
        Ok(Sha1 {
            state,
            buffer: Vec::new(),
        })
    }
}

impl Default for Sha1 {
    fn default() -> Self {
        Sha1::new()
    }
}

impl std::fmt::Display for Sha1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.state))?;
        Ok(())
    }
}

impl Hash for Sha1 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for i in 0..20 {
            state.write_u8(self.state[i]);
        }
    }
}

impl Sha for Sha1 {
    fn update(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    fn finalize(&mut self) -> Vec<u8> {
        let mut hasher = ExternalSha1::new();
        hasher.update(&self.buffer);
        let result = hasher.finalize();
        self.state.copy_from_slice(&result[..20]);
        self.state.to_vec()
    }

    fn reset(&mut self) {
        self.state = [0; 20];
        self.buffer.clear();
    }
}
