use crate::transaction::Transaction;
use bytes::Bytes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitProtoVersion {
    V0 = 0,
    V1 = 1,
    V2 = 2,
    Unknown,
}

impl GitProtoVersion {
    pub fn from_str(s: &str) -> GitProtoVersion {
        match s {
            "0" => GitProtoVersion::V0,
            "1" => GitProtoVersion::V1,
            "2" => GitProtoVersion::V2,
            _ => GitProtoVersion::Unknown,
        }
    }
    pub fn to_str(&self) -> &'static str {
        match self {
            GitProtoVersion::V0 => "0",
            GitProtoVersion::V1 => "1",
            GitProtoVersion::V2 => "2",
            GitProtoVersion::Unknown => "unknown",
        }
    }
    pub fn from_u32(n: u32) -> GitProtoVersion {
        match n {
            0 => GitProtoVersion::V0,
            1 => GitProtoVersion::V1,
            2 => GitProtoVersion::V2,
            _ => GitProtoVersion::Unknown,
        }
    }
    pub fn to_u32(&self) -> u32 {
        match self {
            GitProtoVersion::V0 => 0,
            GitProtoVersion::V1 => 1,
            GitProtoVersion::V2 => 2,
            GitProtoVersion::Unknown => 0,
        }
    }
}

impl Transaction {
    pub async fn write_version(&self) {
        let version_str = match self.version {
            GitProtoVersion::V0 => "version 0\n",
            GitProtoVersion::V1 => "version 1\n",
            GitProtoVersion::V2 => "version 2\n",
            GitProtoVersion::Unknown => "version unknown\n",
        };
        let mut pkt = vec![];
        let len = version_str.len() + 4;
        pkt.extend_from_slice(format!("{:04x}", len).as_bytes());
        pkt.extend_from_slice(version_str.as_bytes());
        self.call_back.send(Bytes::from(pkt)).await;
    }
}
