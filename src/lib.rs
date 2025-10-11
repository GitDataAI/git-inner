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
pub mod config;
pub mod auth;
pub mod control;
pub mod logs;
pub mod rpc;

/// Encode a string as a Git-style pkt-line and return it as a BytesMut buffer.
///
/// For empty input this produces the pkt-line flush marker (`"0000"`). For non-empty
/// input the returned buffer begins with a 4-digit lowercase hexadecimal length (total
/// length including the 4 length bytes) followed by the original input bytes.
///
/// # Examples
///
/// ```
/// use bytes::BytesMut;
/// // empty -> flush marker
/// assert_eq!(crate::write_pkt_line(String::new()), BytesMut::from(&b"0000"[..]));
///
/// // non-empty -> length header + payload
/// let buf = crate::write_pkt_line("hello\n".to_string());
/// // total length = 6 (payload) + 4 = 10 -> hex "000a"
/// assert!(buf.starts_with(&b"000a"[..]));
/// assert!(buf.ends_with(&b"hello\n"[..]));
/// ```
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
