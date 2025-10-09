use bytes::{BufMut, BytesMut};

pub mod sha;

pub mod error;
pub mod objects;
pub mod odb;
pub mod refs;
pub mod hooks;
pub mod callback;
pub mod repository;
pub mod transaction;
pub mod capability;
pub mod serve;
pub mod http;
pub mod ssh;

pub fn write_pkt_line(data: String) -> BytesMut {
    let mut buf = BytesMut::new();
    if data.is_empty() {
        buf.put_slice(b"0000");
        return buf;
    }
    let total_len = data.len() + 4;
    buf.put_slice(format!("{:04x}", total_len).as_bytes());
    buf.put_slice(data.as_ref());
    buf.clone()
}

