use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::Bytes;
use futures_util::Stream;
use futures_util::AsyncRead;
use crate::error::GitInnerError;
use futures_util::AsyncReadExt;

struct StreamToRead<S> {
    stream: S,
}

impl<S> StreamToRead<S> {
    fn new(stream: S) -> Self {
        Self { stream }
    }
}

impl<S> AsyncRead for StreamToRead<S>
where
    S: Stream<Item = Result<Bytes, GitInnerError>> + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match futures_util::ready!(Pin::new(&mut self.stream).poll_next(cx)) {
            Some(Ok(bytes)) => {
                let len = std::cmp::min(buf.len(), bytes.len());
                buf[..len].copy_from_slice(&bytes[..len]);
                Poll::Ready(Ok(len))
            }
            Some(Err(_)) => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "Stream error"))),
            None => Poll::Ready(Ok(0)),
        }
    }
}

pub async fn read_offset_encoding(stream: &mut Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>>>>) -> io::Result<(u64, usize)> {
    let mut value = 0;
    let mut offset = 0;
    loop {
        let (byte_value, more_bytes) = read_byte_and_check_continuation(stream).await?;
        offset += 1;
        value = (value << 7) | byte_value as u64;
        if !more_bytes {
            return Ok((value, offset));
        }
        value += 1;
    }
}

pub async fn read_byte_and_check_continuation(stream: &mut Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>>>>) -> io::Result<(u8, bool)> {
    let mut bytes = [0; 1];
    let mut stream_reader = StreamToRead::new(stream.as_mut());
    stream_reader.read_exact(&mut bytes).await?;
    let byte = bytes[0];
    let value = byte & 0b0111_1111;
    let msb = byte >= 128;
    Ok((value, msb))
}
