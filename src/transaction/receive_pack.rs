use crate::capability::GitCapability;
use crate::error::GitInnerError;
use crate::objects::commit::Commit;
use crate::objects::types::ObjectType;
use crate::odb::OdbTransaction;
use crate::transaction::receive_pack::command::ReceiveCommand;
use crate::transaction::{GitProtoVersion, Transaction};
use bstr::ByteSlice;
use bytes::{Buf, Bytes, BytesMut};
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{Stream, StreamExt};

pub mod command;

#[derive(Debug,Clone)]
pub struct ReceivePackRequest {
    pub ref_upload: Vec<ReceiveCommand>,
    pub capabilities: Vec<GitCapability>,
}

#[derive(Debug,Clone)]
pub struct ReceivePackRequestInfo {
    pub version: GitProtoVersion,
    pub pack_size: usize,
}

impl Transaction {
    pub async fn receive_pack(&mut self, mut stream: Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>>>>) -> Result<(), GitInnerError> {
        let mut head = BytesMut::new();
        let txn = self.repository.odb.begin_transaction().await?;
        while let Some(pack) = stream.next().await {
            let pack = pack?;
            if let Some(idx) = pack.find(b"PACK") {
                head.extend_from_slice(&pack[..idx]);
                let input = tokio_stream::iter(vec![Ok(Bytes::from(pack[idx..].to_vec()))])
                    .chain(stream);
                stream = Box::pin(input);
                break;
            }
            head.extend_from_slice(&pack);
        }
        let request = self.parse_receive_request(head).await?;
        dbg!(&request);
        self.parse_receive_head(request, stream, txn).await?;
        Ok(())
    }
    pub async fn parse_receive_request(&self, head: BytesMut) -> Result<ReceivePackRequest, GitInnerError> {
        let mut refs = vec![];
        let mut capabilities = vec![];
        dbg!(&head);
        for line in head.lines() {
            let str = line.to_str()
                .map_err(|_| GitInnerError::InvalidUtf8)?
                .to_string();
            if let Some(idx) = str.find("\0") {
                if let Ok(Some(pkt_line)) = dbg!(ReceiveCommand::from_pkt_line(&str[..idx].as_bytes())) {
                    refs.push(pkt_line);
                } 
                let caps = str[idx + 1..].split(' ')
                    .map(|s| GitCapability::from_str(s))
                    .collect::<Vec<_>>();
                capabilities = caps;
            } else {
                if let Ok(Some(pkt_line)) = ReceiveCommand::from_pkt_line(&str.as_bytes()) {
                    refs.push(pkt_line);
                }
            }
        }
        Ok(ReceivePackRequest {
            ref_upload: refs,
            capabilities,
        })
    }
    pub async fn parse_receive_head(&mut self, receive_pack_request: ReceivePackRequest, mut stream: Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>>>>, txn: Box<dyn OdbTransaction>) -> Result<(), GitInnerError> {
        let mut head = BytesMut::with_capacity(12);
        let mut remaining = 12;
        while remaining > 0 {
            if let Some(next) = stream.next().await {
                let next = next?;
                let take = std::cmp::min(next.len(), remaining);
                head.extend_from_slice(&next[..take]);
                remaining -= take;
                if take < next.len() {
                    let remaining_data = next.slice(take..);
                    let new_stream = tokio_stream::iter(vec![Ok(remaining_data)]).chain(stream);
                    stream = Box::pin(new_stream);
                    break;
                }
            } else {
                return Err(GitInnerError::UnexpectedEof);
            }
        }
        if head.len() != 12 {
            return Err(GitInnerError::InvalidData);
        }
        let version = (head[4] as usize) << 24 | (head[5] as usize) << 16 | (head[6] as usize) << 8 | (head[7] as usize);
        let pack_size = (head[8] as usize) << 24 | (head[9] as usize) << 16 | (head[10] as usize) << 8 | (head[11] as usize);
        let request_info = ReceivePackRequestInfo {
            version: match version {
                0 => GitProtoVersion::V0,
                1 => GitProtoVersion::V1,
                2 => GitProtoVersion::V2,
                _ => return Err(GitInnerError::NotSupportVersion),
            },
            pack_size,
        };
        dbg!(&request_info);
        self.parse_receive_object(receive_pack_request, request_info, stream, Arc::new(txn)).await?;
        Ok(())
    }
    pub async fn parse_receive_object(
        &mut self,
        receive_pack_request: ReceivePackRequest,
        request_info: ReceivePackRequestInfo,
        mut stream: Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>>>>,
        txn: Arc<Box<dyn OdbTransaction>>
    ) -> Result<(), GitInnerError> {
        let mut buffer = BytesMut::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.extend_from_slice(&chunk);
            while buffer.len() > 0 {
                // 1 字节 8 为
                //    11110001 01001000
                // type:  1
                // size:  2 + 4 + 8 + 16 + 128
                // 1 字节   type+size 头（低 3 位 type，高 4 位 size 低 7 位，继续读直到最高位为 0）
                let pack_head = match parse_object_header(&buffer) {
                    Ok(header) => header,
                    Err(_) => break,
                };

                let header_bytes = pack_head.header_bytes;
                if buffer.len() < header_bytes {
                    break;
                }
                if buffer.len() <= header_bytes {
                    break;
                }
                let compressed_data = Bytes::from(buffer[header_bytes..].to_vec());
                let decompressed_data = match Self::decompress_zlib_data(compressed_data) {
                    Ok(data) => data,
                    Err(_) => {
                        break;
                    }
                };
                let compressed_size = decompressed_data.1;
                let decompressed_bytes = decompressed_data.0;
                match pack_head.object_type {
                    ObjectType::Unknown => return Err(GitInnerError::InvalidObjectType),
                    _ => {
                        self.process_object_data(
                            pack_head.object_type,
                            &decompressed_bytes,
                            txn.clone(),
                        )?;
                    }
                }
                buffer.advance(header_bytes + compressed_size);
            }
        }
        Ok(())
    }

    fn decompress_zlib_data(data: bytes::Bytes) -> Result<(Vec<u8>, usize), GitInnerError> {
        use flate2::read::ZlibDecoder;
        use std::io::Read;
        let mut decoder = ZlibDecoder::new(&data[..]);
        let mut decompressed = Vec::new();
        match decoder.read_to_end(&mut decompressed) {
            Ok(_) => {
                let consumed = decoder.total_in() as usize;
                Ok((decompressed, consumed))
            },
            Err(_) => Err(GitInnerError::DecompressionError)
        }
    }

    fn process_object_data(&mut self, object_type: ObjectType, data: &[u8], txn: Arc<Box<dyn OdbTransaction>>) -> Result<(), GitInnerError> {
        match object_type {
            ObjectType::Commit => {
                self.handle_commit_object(data, txn)?;
            },
            ObjectType::Tree => {
                self.handle_tree_object(data, txn)?;
            },
            ObjectType::Blob => {
                self.handle_blob_object(data, txn)?;
            },
            ObjectType::Tag => {
                self.handle_tag_object(data, txn)?;
            },
            _ => {
                return Err(GitInnerError::NotSupportVersion);
            }
        }
        Ok(())
    }
    fn handle_commit_object(&mut self, data: &[u8], txn: Arc<Box<dyn OdbTransaction>>) -> Result<(), GitInnerError> {
        let bytes = bytes::Bytes::from(data.to_vec());
        let commit = Commit::parse(bytes,self.repository.hash_version.clone());
        if let Some(_commit) = commit {
        }
        Ok(())
    }

    fn handle_tree_object(&mut self, data: &[u8], txn: Arc<Box<dyn OdbTransaction>>) -> Result<(), GitInnerError> {
        dbg!();
        // 处理tree对象逻辑
        // 解析目录结构
        Ok(())
    }

    fn handle_blob_object(&mut self, data: &[u8], txn: Arc<Box<dyn OdbTransaction>>) -> Result<(), GitInnerError> {
        // 处理blob对象逻辑
        // 存储文件内容
        Ok(())
    }

    fn handle_tag_object(&mut self, data: &[u8], txn: Arc<Box<dyn OdbTransaction>>) -> Result<(), GitInnerError> {
        // 处理tag对象逻辑
        // 解析标签信息
        Ok(())
    }
}

pub struct PackHead {
    pub object_type: ObjectType,
    pub object_size: usize,
    pub header_bytes: usize,
}

fn parse_object_header(buffer: &BytesMut) -> Result<PackHead, GitInnerError> {
    if buffer.is_empty() {
        return Err(GitInnerError::InvalidData);
    }
    let first_byte = buffer[0];
    let object_type = (first_byte >> 4) & 0x07;
    let mut object_size = (first_byte & 0x0F) as usize;
    let mut header_bytes = 1;
    let mut shift = 4;
    if (first_byte & 0x80) != 0 {
        let mut pos = 1;
        while pos < buffer.len() {
            let byte = buffer[pos];
            object_size |= ((byte & 0x7F) as usize) << shift;
            header_bytes += 1;
            shift += 7;

            if (byte & 0x80) == 0 {
                break;
            }
            pos += 1;
        }
        if pos >= buffer.len() && (buffer[buffer.len()-1] & 0x80) != 0 {
            return Err(GitInnerError::InvalidData);
        }
    }
    Ok(PackHead {
        object_type: ObjectType::from_u8(object_type),
        object_size,
        header_bytes,
    })
}