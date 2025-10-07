use crate::error::GitInnerError;
use crate::sha::Sha;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::digest::consts::U32;
use sha2::digest::core_api::{CoreWrapper, CtVariableCoreWrapper};
use sha2::{Digest, OidSha256, Sha256VarCore};
use std::fmt::Display;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sha256 {
    pub state: [u8; 32],
    #[serde(skip)]
    buffer: CoreWrapper<CtVariableCoreWrapper<Sha256VarCore, U32, OidSha256>>,
}

impl Sha256 {
    pub(crate) fn from_vec(p0: Vec<u8>) -> Option<Sha256> {
        if p0.len() != 32 {
            return None;
        }
        Some(Sha256 {
            state: p0.try_into().ok()?,
            buffer: sha2::Sha256::new(),
        })
    }
}

impl Sha256 {
    pub(crate) fn from_bytes(p0: Bytes) -> Sha256 {
        let mut sha2 = sha2::Sha256::new();
        sha2.update(&p0);
        Sha256 {
            state: sha2.finalize().into(),
            buffer: sha2::Sha256::new(),
        }
    }
}

impl Sha256 {
    pub fn new() -> Self {
        Sha256 {
            state: [0; 32],
            buffer: sha2::Sha256::new(),
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
            buffer: sha2::Sha256::new(),
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
        self.buffer.update(data);
    }

    fn finalize(&mut self) -> Vec<u8> {
        let result = self.buffer.clone().finalize();
        self.state.copy_from_slice(&result);
        self.state.to_vec()
    }

    fn reset(&mut self) {
        self.state = [0; 32];
        self.buffer.reset();
    }
}
