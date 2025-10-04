use crate::error::GitInnerError;
use crate::objects::ObjectTrait;
use crate::objects::types::ObjectType;
use crate::sha::HashValue;
use bytes::{Bytes, BytesMut};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfsDelta {
    pub id: HashValue,
    pub base_offset: u64,
    pub delta_data: Bytes,
}

impl OfsDelta {
    pub fn apply_delta(base_obj: &Bytes, obj_bytes: &Bytes) -> Result<Bytes, GitInnerError> {
        let mut pos = 0usize;

        // 1) parse base_size (varint)
        let mut base_size: usize = 0;
        let mut shift = 0;
        loop {
            let byte = *obj_bytes.get(pos).ok_or(GitInnerError::UnexpectedEof)?;
            pos += 1;
            base_size |= ((byte & 0x7F) as usize) << shift;
            shift += 7;
            if (byte & 0x80) == 0 {
                break;
            }
        }

        // 2) parse result_size (varint)
        let mut result_size: usize = 0;
        shift = 0;
        loop {
            let byte = *obj_bytes.get(pos).ok_or(GitInnerError::UnexpectedEof)?;
            pos += 1;
            result_size |= ((byte & 0x7F) as usize) << shift;
            shift += 7;
            if (byte & 0x80) == 0 {
                break;
            }
        }

        // 3) sanity check: base_size must match actual base_obj length
        if base_size != base_obj.len() {
            dbg!("delta base_size mismatch", base_size, base_obj.len());
            return Err(GitInnerError::InvalidDelta);
        }

        let mut out = Vec::with_capacity(result_size);

        while pos < obj_bytes.len() {
            let opcode = *obj_bytes.get(pos).ok_or(GitInnerError::UnexpectedEof)?;
            pos += 1;

            if (opcode & 0x80) != 0 {
                // copy command
                let mut copy_offset: usize = 0;
                let mut copy_size: usize = 0;

                if opcode & 0x01 != 0 {
                    copy_offset |=
                        *obj_bytes.get(pos).ok_or(GitInnerError::UnexpectedEof)? as usize;
                    pos += 1;
                }
                if opcode & 0x02 != 0 {
                    copy_offset |=
                        (*obj_bytes.get(pos).ok_or(GitInnerError::UnexpectedEof)? as usize) << 8;
                    pos += 1;
                }
                if opcode & 0x04 != 0 {
                    copy_offset |=
                        (*obj_bytes.get(pos).ok_or(GitInnerError::UnexpectedEof)? as usize) << 16;
                    pos += 1;
                }
                if opcode & 0x08 != 0 {
                    copy_offset |=
                        (*obj_bytes.get(pos).ok_or(GitInnerError::UnexpectedEof)? as usize) << 24;
                    pos += 1;
                }

                if opcode & 0x10 != 0 {
                    copy_size |= *obj_bytes.get(pos).ok_or(GitInnerError::UnexpectedEof)? as usize;
                    pos += 1;
                }
                if opcode & 0x20 != 0 {
                    copy_size |=
                        (*obj_bytes.get(pos).ok_or(GitInnerError::UnexpectedEof)? as usize) << 8;
                    pos += 1;
                }
                if opcode & 0x40 != 0 {
                    copy_size |=
                        (*obj_bytes.get(pos).ok_or(GitInnerError::UnexpectedEof)? as usize) << 16;
                    pos += 1;
                }

                if copy_size == 0 {
                    copy_size = 0x10000;
                }

                // 安全加法，防止溢出
                let end = match copy_offset.checked_add(copy_size) {
                    Some(e) => e,
                    None => {
                        dbg!("copy offset overflow", copy_offset, copy_size);
                        return Err(GitInnerError::InvalidDelta);
                    }
                };

                if end > base_obj.len() {
                    dbg!(format!(
                        "delta copy out of range: pos={}, opcode={}, copy_offset={}, copy_size={}, end={}, base_len={}, base_size_from_header={}",
                        pos,
                        opcode,
                        copy_offset,
                        copy_size,
                        end,
                        base_obj.len(),
                        base_size
                    ));
                    return Err(GitInnerError::InvalidDelta);
                }

                out.extend_from_slice(&base_obj[copy_offset..end]);
            } else if opcode != 0 {
                // insert literal
                let insert_size = opcode as usize;
                if pos
                    .checked_add(insert_size)
                    .map(|v| v > obj_bytes.len())
                    .unwrap_or(true)
                {
                    dbg!(
                        "delta insert out of range",
                        pos,
                        insert_size,
                        obj_bytes.len()
                    );
                    return Err(GitInnerError::UnexpectedEof);
                }
                out.extend_from_slice(&obj_bytes[pos..pos + insert_size]);
                pos += insert_size;
            } else {
                // opcode == 0 invalid
                dbg!("invalid delta opcode 0 at pos", pos);
                return Err(GitInnerError::InvalidDelta);
            }
        }

        if out.len() != result_size {
            dbg!("result size mismatch", out.len(), result_size);
            return Err(GitInnerError::InvalidDelta);
        }
        Ok(Bytes::from(out))
    }
}

impl OfsDelta {
    pub fn new(
        base_offset: u64,
        delta_data: Bytes,
        hash_version: impl Fn(&Bytes) -> HashValue,
    ) -> Self {
        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(format!("ofs-delta {}\0", delta_data.len()).as_bytes());
        hash_input.extend_from_slice(&delta_data);
        let id = hash_version(&Bytes::from(hash_input));
        Self {
            id,
            base_offset,
            delta_data,
        }
    }
    pub fn parse(
        mut input: BytesMut,
        current_offset: usize,
        hash_version: impl Fn(&Bytes) -> HashValue,
    ) -> Result<Self, GitInnerError> {
        let mut i = 0;
        let mut ofs = 0usize;
        loop {
            let byte = *input.get(i).ok_or(GitInnerError::UnexpectedEof)?;
            i += 1;
            ofs = (ofs << 7) | ((byte & 0x7F) as usize);
            if (byte & 0x80) == 0 {
                break;
            }
        }
        let absolute_base_offset = current_offset - ofs;
        let delta_data = input.split_off(i);

        Ok(OfsDelta::new(
            absolute_base_offset as u64,
            Bytes::from(delta_data),
            hash_version,
        ))
    }
    pub fn size(&self) -> usize {
        self.delta_data.len()
    }
}

impl ObjectTrait for OfsDelta {
    fn get_type(&self) -> ObjectType {
        ObjectType::OfsDelta
    }

    fn get_size(&self) -> usize {
        self.delta_data.len()
    }

    fn get_data(&self) -> Bytes {
        self.delta_data.clone()
    }
}

impl std::fmt::Display for OfsDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Type: OfsDelta")?;
        writeln!(f, "Base Offset: {}", self.base_offset)?;
        writeln!(f, "Size: {}", self.delta_data.len())
    }
}
