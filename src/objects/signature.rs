use std::fmt::{Debug, Display};
use std::str::FromStr;
use bincode::{Decode, Encode};
use bstr::ByteSlice;
use chrono::Offset;
use serde::{Deserialize, Serialize};
use crate::error::GitInnerError;

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Decode, Encode)]
pub enum SignatureType {
    Author,
    Committer,
    Tagger,
}

impl SignatureType {
    pub fn from_data(data: Vec<u8>) -> Result<Self, GitInnerError> {
        let s = String::from_utf8(data.to_vec())
            .map_err(|e| GitInnerError::ConversionError(e.to_string()))?;
        SignatureType::from_str(s.as_str())
    }

    
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            SignatureType::Author => "author".to_string().into_bytes(),
            SignatureType::Committer => "committer".to_string().into_bytes(),
            SignatureType::Tagger => "tagger".to_string().into_bytes(),
        }
    }
}

impl Display for SignatureType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SignatureType::Author => write!(f, "author"),
            SignatureType::Committer => write!(f, "committer"),
            SignatureType::Tagger => write!(f, "tagger"),
        }
    }
}
impl FromStr for SignatureType {
    type Err = GitInnerError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "author" => Ok(SignatureType::Author),
            "committer" => Ok(SignatureType::Committer),
            "tagger" => Ok(SignatureType::Tagger),
            _ => Err(GitInnerError::InvalidSignatureType(s.to_string())),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Decode, Encode)]
pub struct Signature {
    pub signature_type: SignatureType,
    pub name: String,
    pub email: String,
    pub timestamp: usize,
    pub timezone: String,
}

impl Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let date = chrono::DateTime::<chrono::Utc>::from_timestamp(self.timestamp as i64, 0).unwrap();
        writeln!(f, "{} <{}> Data: {} {}", self.name, self.email, date, self.timezone)
    }
}

impl Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Default for Signature {
    fn default() -> Self {
        Signature {
            signature_type: SignatureType::Author,
            name: "".to_string(),
            email: "".to_string(),
            timestamp: 0,
            timezone: "".to_string(),
        }
    }
}

impl Signature {
    pub fn from_data(data: Vec<u8>) -> Result<Signature, GitInnerError> {
        let mut sign = data;

        let name_start = sign.find_byte(0x20)
            .ok_or(GitInnerError::InvalidSignature)?;

        let signature_type = SignatureType::from_data(sign[..name_start].to_vec())?;

        let (name, email) = {
            let email_start = sign.find_byte(0x3C)
                .ok_or(GitInnerError::InvalidSignature)?;
            let email_end = sign.find_byte(0x3E)
                .ok_or(GitInnerError::InvalidSignature)?;
            unsafe {
                (
                    sign[name_start + 1..email_start - 1]
                        .to_str_unchecked()
                        .to_string(),
                    sign[email_start + 1..email_end]
                        .to_str_unchecked()
                        .to_string(),
                )
            }
        };

        sign = sign[sign.find_byte(0x3E).unwrap() + 2..].to_vec();

        let timestamp_split = sign.find_byte(0x20)
            .ok_or(GitInnerError::InvalidSignature)?;

        let timestamp = unsafe {
            sign[0..timestamp_split]
                .to_str_unchecked()
                .parse::<usize>()
                .map_err(|_| GitInnerError::InvalidTimestamp)?
        };

        let timezone = unsafe { sign[timestamp_split + 1..].to_str_unchecked().to_string() };

        Ok(Signature {
            signature_type,
            name,
            email,
            timestamp,
            timezone,
        })
    }

    pub fn to_data(&self) -> Result<Vec<u8>, GitInnerError> {
        let mut sign = Vec::new();

        sign.extend_from_slice(&self.signature_type.to_bytes());
        sign.extend_from_slice(&[0x20]);

        sign.extend_from_slice(self.name.as_bytes());
        sign.extend_from_slice(&[0x20]);

        sign.extend_from_slice(format!("<{}>", self.email).as_bytes());
        sign.extend_from_slice(&[0x20]);

        sign.extend_from_slice(self.timestamp.to_string().as_bytes());
        sign.extend_from_slice(&[0x20]);

        sign.extend_from_slice(self.timezone.as_bytes());

        Ok(sign)
    }

    
    pub fn new(sign_type: SignatureType, author: String, email: String) -> Signature {
        let local_time = chrono::Local::now();

        let offset = local_time.offset().fix().local_minus_utc();

        let hours = offset / 60 / 60;

        let minutes = offset / 60 % 60;

        let offset_str = format!("{hours:+03}{minutes:02}");

        Signature {
            signature_type: sign_type,
            name: author,
            email,
            timestamp: chrono::Utc::now().timestamp() as usize,
            timezone: offset_str,
        }
    }
}