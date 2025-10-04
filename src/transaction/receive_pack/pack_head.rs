use crate::error::GitInnerError;
use crate::objects::types::ObjectType;
use bytes::BytesMut;

#[derive(Debug)]
pub struct PackHead {
    pub object_type: ObjectType,
    pub object_size: usize,
    pub header_bytes: usize,
}
pub fn parse_object_header(buffer: &BytesMut) -> Result<PackHead, GitInnerError> {
    if buffer.is_empty() {
        return Err(GitInnerError::InvalidData);
    }

    let first_byte = buffer[0];
    let type_bits = (first_byte >> 4) & 0x07;
    let mut object_size = (first_byte & 0x0F) as usize;
    let mut header_bytes = 1;
    let mut shift = 4;
    if (first_byte & 0x80) != 0 {
        let mut pos = 1;
        loop {
            if pos >= buffer.len() {
                return Err(GitInnerError::InvalidData);
            }
            let byte = buffer[pos];
            object_size |= ((byte & 0x7F) as usize) << shift;
            header_bytes += 1;
            shift += 7;

            if (byte & 0x80) == 0 {
                break;
            }

            pos += 1;

            if header_bytes > 10 {
                return Err(GitInnerError::InvalidData);
            }
        }
    }

    let object_type = ObjectType::from_u8(type_bits);

    Ok(PackHead {
        object_type,
        object_size,
        header_bytes,
    })
}
