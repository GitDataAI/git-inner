use crate::capability::enums::GitCapability;
use crate::error::GitInnerError;
use crate::transaction::upload::UploadPackTransaction;
use crate::transaction::upload::command::UploadCommandType;
use crate::transaction::{GitProtoVersion, Transaction};
use bytes::{Buf, Bytes, BytesMut};
use futures_util::StreamExt;
use std::pin::Pin;
use tokio_stream::wrappers::ReceiverStream;

impl Transaction {
    pub async fn upload_pack(
        &self,
        stream: &mut Pin<Box<ReceiverStream<Result<Bytes, GitInnerError>>>>,
    ) -> Result<(), GitInnerError> {
        if self.version == GitProtoVersion::V2 {
            self.upload_pack_v2(stream).await?;
            return Ok(());
        }
        let mut buffer = BytesMut::new();
        let mut commands = vec![];
        while let Some(next) = stream.next().await {
            let next = next?;
            buffer.extend_from_slice(&next);
            loop {
                if buffer.len() < 4 {
                    break;
                }
                let len_str = std::str::from_utf8(&buffer[..4]).map_err(|_| {
                    GitInnerError::ConversionError("Invalid pkt-line length".to_string())
                })?;
                let pkt_len = u32::from_str_radix(len_str, 16).map_err(|_| {
                    GitInnerError::ConversionError("Invalid pkt-line length format".to_string())
                })?;

                if pkt_len == 0 {
                    commands.push(UploadCommandType::Flush);
                    buffer.advance(4);
                    continue;
                }

                if buffer.len() < pkt_len as usize {
                    break;
                }

                let line_bytes = buffer.split_to(pkt_len as usize);
                if line_bytes.len() < 4 {
                    break;
                }
                let line_str = std::str::from_utf8(&line_bytes[4..])
                    .map_err(|_| GitInnerError::ConversionError("Invalid UTF-8 line".to_string()))?
                    .trim_end();
                let mut parsed = UploadCommandType::from_one_line(
                    line_str,
                    self.repository.hash_version.clone(),
                )?;
                commands.append(&mut parsed);
            }
        }

        let mut request = UploadPackTransaction::new(self.clone());
        let mut found_common = false;

        for cmd in commands {
            match cmd {
                UploadCommandType::Want(hash) => {
                    request.want.push(hash);
                }
                UploadCommandType::Have(hash) => {
                    let has_object = self.repository.odb.has_commit(&hash).await?
                        || self.repository.odb.has_tree(&hash).await?
                        || self.repository.odb.has_blob(&hash).await?
                        || self.repository.odb.has_tag(&hash).await?;

                    if has_object {
                        let ack_msg = format!("ACK {}\n", hash);
                        let pkt_line = format!("{:04x}{}", ack_msg.len() + 4, ack_msg);
                        self.call_back.send(Bytes::from(pkt_line)).await;
                        found_common = true;
                        request.have.push(hash);
                    }
                }
                UploadCommandType::Shallow(hash) => {
                    request.shallow.push(hash);
                }
                UploadCommandType::Deepen(depth) => {
                    request.depth = Some(depth as u32);
                }
                UploadCommandType::Capabilities(capabilities) => {
                    for capability in capabilities {
                        if capability == GitCapability::SideBand {
                            request.sideband = true;
                        } else if capability == GitCapability::ThinPack {
                            request.thin = true;
                        } else if capability == GitCapability::NoProgress {
                            request.no_progress = true;
                        } else if capability == GitCapability::NoDone {
                            request.no_done = true;
                        } else if capability == GitCapability::IncludeTag {
                            request.include_tag = true;
                        }
                        request.capabilities.push(capability);
                    }
                }
                UploadCommandType::Done => {
                    if !found_common {
                        let nak_msg = "NAK\n";
                        let pkt_line = format!("{:04x}{}", nak_msg.len() + 4, nak_msg);
                        self.call_back.send(Bytes::from(pkt_line)).await;
                    }
                    break;
                }
                _ => {}
            }
        }
        request.upload_pack_encode().await?;
        Ok(())
    }
}
