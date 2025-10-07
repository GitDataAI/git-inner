use crate::error::GitInnerError;
use bytes::{Buf, Bytes, BytesMut};
use flate2::{Decompress, FlushDecompress, Status};
use futures_util::Stream;
use futures_util::StreamExt;
use std::pin::Pin;

pub async fn decompress_object_data(
    buffer: &mut BytesMut,
    stream: &mut Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>>>>,
    expected_size: usize,
) -> Result<Bytes, GitInnerError> {
    let mut decomp = Decompress::new(true);
    let mut object_data = Vec::with_capacity(expected_size);
    let mut tmp_out = [0u8; 8192];

    loop {
        if buffer.is_empty() {
            if let Some(chunk) = stream.next().await {
                buffer.extend_from_slice(&chunk?);
            } else {
                return Err(GitInnerError::UnexpectedEof);
            }
        }

        let before_in = decomp.total_in();
        let before_out = decomp.total_out();

        let status = decomp
            .decompress(&buffer, &mut tmp_out, FlushDecompress::None)
            .map_err(|_| GitInnerError::DecompressionError)?;

        let consumed_in = (decomp.total_in() - before_in) as usize;
        let produced_out = (decomp.total_out() - before_out) as usize;

        if consumed_in > 0 {
            buffer.advance(consumed_in);
        }
        if produced_out > 0 {
            object_data.extend_from_slice(&tmp_out[..produced_out]);
        }

        match status {
            Status::Ok => {
                continue;
            }
            Status::StreamEnd => {
                break;
            }
            Status::BufError => {
                if buffer.is_empty() {
                    if let Some(chunk) = stream.next().await {
                        buffer.extend_from_slice(&chunk?);
                        continue;
                    } else {
                        return Err(GitInnerError::UnexpectedEof);
                    }
                } else {
                    continue;
                }
            }
        }
    }

    if object_data.len() != expected_size {
        return Err(GitInnerError::DecompressionError);
    }

    Ok(Bytes::from(object_data))
}
pub async fn decode_ofs_delta_offset(
    buffer: &mut BytesMut,
    stream: &mut Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>>>>,
    current_offset: &mut usize,
    obj_start: u64,
) -> Result<u64, GitInnerError> {
    #[allow(unused)]
    let mut value: u64 = 0;

    // 先读一个字节
    while buffer.is_empty() {
        if let Some(chunk) = stream.next().await {
            buffer.extend_from_slice(&chunk?);
        } else {
            return Err(GitInnerError::UnexpectedEof);
        }
    }

    let mut byte = buffer[0];
    buffer.advance(1);
    *current_offset += 1;
    value = (byte & 0x7f) as u64;

    // 如果最高位不为 1，就直接返回
    if (byte & 0x80) == 0 {
        let base_offset = obj_start
            .checked_sub(value)
            .ok_or(GitInnerError::InvalidData)?;
        return Ok(base_offset);
    }

    // 否则继续解码后续字节
    loop {
        while buffer.is_empty() {
            if let Some(chunk) = stream.next().await {
                buffer.extend_from_slice(&chunk?);
            } else {
                return Err(GitInnerError::UnexpectedEof);
            }
        }

        byte = buffer[0];
        buffer.advance(1);
        *current_offset += 1;

        // 关键：ofs-delta 的编码方式是 ((value + 1) << 7) | low7bits
        value = ((value + 1) << 7) | (byte & 0x7f) as u64;

        if (byte & 0x80) == 0 {
            break;
        }
    }

    let base_offset = obj_start
        .checked_sub(value)
        .ok_or(GitInnerError::InvalidData)?;

    Ok(base_offset)
}
