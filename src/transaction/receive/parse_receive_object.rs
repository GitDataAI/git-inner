use crate::error::GitInnerError;
use crate::objects::types::ObjectType;
use crate::odb::OdbTransaction;
use crate::transaction::receive::zlib_decode::decompress_object_data;
use crate::transaction::receive::ReceivePackTransaction;
use bytes::{Buf, Bytes, BytesMut};
use futures_util::Stream;
use futures_util::StreamExt;
use std::collections::{BTreeMap, HashMap};
use std::pin::Pin;
use std::sync::Arc;
use crate::callback::sidebend::{bend_pkt_flush, SideBend};
use crate::capability::enums::GitCapability;
use crate::objects::ref_delta::RefDelta;
use crate::sha::HashValue;
use crate::write_pkt_line;

impl ReceivePackTransaction {
    pub async fn process_receive_pack(
        &mut self,
        mut stream: Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>>>>,
        txn: Arc<Box<dyn OdbTransaction>>,
    ) -> Result<(), GitInnerError> {
        let mut buffer = BytesMut::new();
        let mut current_offset = 0usize;
        let mut pack_count = 0usize;
        let mut ref_delta = HashMap::new();
        let mut resolved_ofs: BTreeMap<u64, (HashValue, Bytes, ObjectType)> = BTreeMap::new();
        async fn ensure_buf(
            buffer: &mut BytesMut,
            stream: &mut Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>>>>,
            n: usize,
        ) -> Result<(), GitInnerError> {
            while buffer.len() < n {
                if let Some(chunk) = stream.next().await {
                    buffer.extend_from_slice(&chunk?);
                } else {
                    return Err(GitInnerError::UnexpectedEof);
                }
            }
            Ok(())
        }
        while pack_count < self.pack_size {
            let obj_start = current_offset;
            ensure_buf(&mut buffer, &mut stream, 1).await?;
            let first = buffer[0];
            let mut consumed = 1usize;
            let mut size = (first & 0x0F) as usize;
            let mut shift = 4;
            let mut byte = first;

            while (byte & 0x80) != 0 {
                ensure_buf(&mut buffer, &mut stream, consumed + 1).await?;
                byte = buffer[consumed];
                size |= ((byte & 0x7F) as usize) << shift;
                consumed += 1;
                shift += 7;
            }

            let type_id = (first >> 4) & 0x07;
            let object_type = match type_id {
                1 => ObjectType::Commit,
                2 => ObjectType::Tree,
                3 => ObjectType::Blob,
                4 => ObjectType::Tag,
                6 => ObjectType::OfsDelta,
                7 => ObjectType::RefDelta,
                _ => return Err(GitInnerError::InvalidData),
            };

            buffer.advance(consumed);
            current_offset += consumed;

            match object_type {
                ObjectType::Commit | ObjectType::Tree | ObjectType::Blob | ObjectType::Tag => {
                    let obj_bytes = decompress_object_data(&mut buffer, &mut stream, size).await?;
                    let hash = self.transaction.process_object_data(object_type, &obj_bytes, txn.clone()).await?;
                    resolved_ofs.insert(obj_start as u64, (hash, obj_bytes, object_type));
                }
                ObjectType::OfsDelta => {
                    return Err(GitInnerError::UnsupportedOfsDelta);
                }
                ObjectType::RefDelta => {
                    let hash_len = self.transaction.repository.hash_version.len();
                    ensure_buf(&mut buffer, &mut stream, hash_len).await?;
                    let base_hash_bytes = buffer.split_to(hash_len);
                    current_offset += hash_len;
                    let base_hash = HashValue::from_bytes(&base_hash_bytes)
                        .ok_or(GitInnerError::InvalidHash)?;
                    let delta_bytes = decompress_object_data(&mut buffer, &mut stream, size).await?;
                    ref_delta.insert(obj_start as u64, (base_hash, delta_bytes));
                }

                ObjectType::Unknown => {
                    self
                        .transaction
                        .call_back
                        .send(
                            Bytes::from(write_pkt_line("ERR Unsupported object type\n".to_string()))
                        )
                        .await;
                }
            }
            pack_count += 1;
        }
        let ref_total = ref_delta.len();
        let mut unresolved: HashMap<u64, (HashValue, Bytes)> = ref_delta;
        let mut resolved_count = 20;

        let sidebend =
                self.capabilities.contains(&GitCapability::SideBand) ||
                self.capabilities.contains(&GitCapability::SideBand64k);
        loop {
            resolved_count -= 1;
            if unresolved.is_empty() {
                break;
            }
            let mut resolved_in_round = Vec::new();
            let remaining_count = unresolved.len();
            for (obj_start, (base_hash, delta_bytes)) in unresolved.iter() {
                if let Ok((full_bytes, obj)) = RefDelta::apply_delta(base_hash, delta_bytes, txn.clone(), &resolved_ofs).await {
                    let hash = self.transaction.process_object_data(obj, &full_bytes, txn.clone()).await?;
                    resolved_ofs.insert(*obj_start, (hash, full_bytes, obj));
                    resolved_in_round.push(*obj_start);
                }
            }
            if resolved_in_round.is_empty() {
                return Err(GitInnerError::MissingBaseObject);
            }
            let resolved_in_round_count = resolved_in_round.len();
            for k in resolved_in_round {
                unresolved.remove(&k);
            }
            let progress = (ref_total - remaining_count) as f64 * 100.0 / ref_total as f64;
            if sidebend {
                self
                    .transaction
                    .call_back
                    .send_side_pkt_line(Bytes::from(format!(
                        "Progress: {:.2}% ({}/{})\n",
                        progress,
                        ref_total - remaining_count + resolved_in_round_count,
                        ref_total
                    )), SideBend::SidebandMessage)
                    .await;
            } else {
                self
                    .transaction
                    .call_back
                    .send(Bytes::from(write_pkt_line(format!(
                        "Progress: {:.2}% ({}/{})\n",
                        progress,
                        ref_total - remaining_count + resolved_in_round_count,
                        ref_total
                    ))))
                    .await;
            }
            if resolved_count == 0 {
                break;
            }
        }
        if !unresolved.is_empty() {
            return Err(GitInnerError::MissingBaseObject);
        }
        self
            .transaction
            .call_back
            .send_side_pkt_line(Bytes::from(write_pkt_line("unpack ok\n".to_string())), SideBend::SidebandPrimary)
            .await;

        txn.commit().await?;
        let mut ok = false;
        for idx in self.ref_upload.clone() {
            if idx.is_create() {
                if self.transaction.repository.refs.create_refs(idx.ref_name.clone(), idx.new).await.is_ok() {
                    ok = true;
                }
            } else if idx.is_update() {
                if self.transaction.repository.refs.update_refs(idx.ref_name.clone(), idx.new).await.is_ok() {
                    ok = true;
                }
            }
            if ok {
                if sidebend {
                    self
                        .transaction
                        .call_back
                        .send_side_pkt_line(Bytes::from(write_pkt_line(format!("ok {}\n", idx.ref_name))), SideBend::SidebandPrimary)
                        .await;
                } else {
                    self
                        .transaction
                        .call_back
                        .send(Bytes::from(write_pkt_line(format!("ok {}\n", idx.ref_name))))
                        .await;
                }
            }
        }
        self
            .transaction
            .call_back
            .send(bend_pkt_flush().into())
            .await;
        self
            .transaction
            .call_back
            .send(Bytes::new())
            .await;

        Ok(())
    }
}