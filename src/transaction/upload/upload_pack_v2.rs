use crate::capability::enums::GitCapability;
use crate::error::GitInnerError;
use crate::transaction::Transaction;
use crate::transaction::upload::UploadPackTransaction;
use crate::transaction::upload::command::UploadCommandType;
use bytes::{Buf, Bytes, BytesMut};
use futures_util::StreamExt;
use std::pin::Pin;
use tokio_stream::wrappers::ReceiverStream;

impl Transaction {
    pub async fn upload_pack_v2(
        &self,
        stream: &mut Pin<Box<ReceiverStream<Result<Bytes, GitInnerError>>>>,
    ) -> Result<(), GitInnerError> {
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

                if pkt_len == 0 || pkt_len == 1 {
                    commands.push(UploadCommandType::Flush);
                    buffer.advance(4);
                    continue;
                }

                if buffer.len() < pkt_len as usize {
                    break;
                }

                let line_bytes = buffer.split_to(pkt_len as usize);
                if line_bytes.len() < pkt_len as usize {
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

        for command in commands.clone() {
            if let UploadCommandType::Command(command) = command {
                match command.as_str() {
                    "ls-refs" => {
                        self.write_refs_head_info_v2(
                            commands.contains(&UploadCommandType::Symrefs),
                        )
                        .await?;
                        self.write_all_refs().await?;
                        self.call_back.send(Bytes::from("0000")).await;
                    }
                    "fetch" => {
                        let mut request = UploadPackTransaction::new(self.clone());
                        let mut found_common = false;
                        for cmd in commands.clone() {
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
                                        let pkt_line =
                                            format!("{:04x}{}", ack_msg.len() + 4, ack_msg);
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
                                    break;
                                }
                                _ => {}
                            }
                        }
                        if !commands.iter().any(|x| {
                            if let UploadCommandType::Have(_) = x {
                                true
                            } else {
                                false
                            }
                        }) {
                            found_common = true;
                        }
                        request.sideband = true;
                        if !found_common {
                            let nak_msg = "NAK\n";
                            let pkt_line = format!("{:04x}{}", nak_msg.len() + 4, nak_msg);
                            self.call_back.send(Bytes::from(pkt_line)).await;
                        } else {
                            request.upload_pack_encode().await?;
                        }
                    }
                    _ => return Err(GitInnerError::NotSupportCommand),
                }
            }
        }
        Ok(())
    }
}
