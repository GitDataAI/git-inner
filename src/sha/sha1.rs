use crate::error::GitInnerError;
use crate::sha::Sha;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha1::digest::core_api::CoreWrapper;
use sha1::{Digest, Sha1Core};
use std::hash::Hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sha1 {
    pub state: [u8; 20],
    #[serde(skip)]
    buffer: CoreWrapper<Sha1Core>,
}

impl Sha1 {
    pub(crate) fn from_bytes(p0: Bytes) -> Sha1 {
        let mut sha1 = sha1::Sha1::new();
        sha1.update(p0);
        Sha1 {
            state: <[u8; 20]>::from(sha1.finalize()),
            buffer: sha1::Sha1::default(),
        }
    }
    pub fn from_vec(p0: Vec<u8>) -> Option<Sha1> {
        if p0.len() != 20 {
            return None;
        }
        let state: [u8; 20] = p0.try_into().ok()?;
        Some(Sha1 {
            state,
            buffer: sha1::Sha1::default(),
        })
    }
}

impl Sha1 {
    pub fn new() -> Sha1 {
        Sha1 {
            state: [0; 20],
            buffer: sha1::Sha1::default(),
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
            buffer: sha1::Sha1::default(),
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
        self.buffer.update(data);
    }

    fn finalize(&mut self) -> Vec<u8> {
        let result = self.buffer.clone().finalize();
        self.state.copy_from_slice(&result);
        self.state.to_vec()
    }

    fn reset(&mut self) {
        self.state = [0; 20];
        self.buffer.reset();
    }
}
