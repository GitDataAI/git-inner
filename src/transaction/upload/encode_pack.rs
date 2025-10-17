use crate::error::GitInnerError;
use crate::sha::Sha;
use crate::transaction::upload::UploadPackTransaction;
use crate::transaction::upload::recursion::Object;
use bstr::ByteSlice;
use bytes::{BufMut, Bytes, BytesMut};
use log::trace;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::task;

const MAX_PKT_LINE: usize = 0xfff0;
const MAX_PAYLOAD_PER_PKT: usize = MAX_PKT_LINE - 4 - 1;
const TARGET_PACK_BYTES: usize = usize::MAX;
const PACK_HEADER_LEN: usize = 12;

impl UploadPackTransaction {
    pub async fn upload_pack_encode(&self) -> Result<(), GitInnerError> {
        trace!("[upload_pack_encode] start");
        let wants = self.want.clone();
        let mut objs = Vec::new();
        let mut visited = HashSet::new();

        self.txn
            .call_back
            .send_pkt_line(Bytes::from_static(b"packfile\n"))
            .await;

        for want in &wants {
            self.recursion_pack_pool_found_iter(&mut objs, &mut visited, want.clone())
                .await?;
        }

        if self.sideband {
            let payload = format!("find pack {}\n", objs.len());
            let pkt = build_sideband_pkt(2, payload.as_bytes());
            self.txn.call_back.send(pkt).await;
        } else {
            self.txn
                .call_back
                .send_pkt_line(Bytes::from(format!("find pack {}\n", objs.len())))
                .await;
        }

        if objs.is_empty() {
            self.txn.call_back.send(Bytes::from_static(b"0000")).await;
            return Ok(());
        }

        let concurrency = 8usize;
        let objs_arc = Arc::new(objs);
        let mut compressed_list: Vec<(Object, Bytes)> = Vec::with_capacity(objs_arc.len());
        let mut index = 0usize;

        while index < objs_arc.len() {
            let mut handles = Vec::new();
            for i in index..(index + concurrency).min(objs_arc.len()) {
                let o = objs_arc[i].clone();
                let handle =
                    task::spawn_blocking(move || -> Result<(Object, Bytes), GitInnerError> {
                        let bytes = o.zlib()?;
                        Ok((o, bytes))
                    });
                handles.push(handle);
            }
            for h in handles {
                match h.await {
                    Ok(Ok((o, b))) => {
                        compressed_list.push((o, b));
                    }
                    Ok(Err(e)) => return Err(e),
                    Err(e) => {
                        return Err(GitInnerError::Other(format!("compress join error: {}", e)));
                    }
                }
            }
            index += concurrency;
        }

        let mut pos = 0usize;
        let total = compressed_list.len();
        let mut pack_idx = 1usize;
        let mut any_segment_sent = false;

        while pos < total {
            let mut temp_objs_bytes: Vec<Bytes> = Vec::new();
            let mut segment_objects = 0usize;
            let mut seg_est = PACK_HEADER_LEN;

            while pos < total {
                let cand_len = compressed_list[pos].1.len();
                if segment_objects > 0 && seg_est + cand_len > TARGET_PACK_BYTES {
                    break;
                }
                temp_objs_bytes.push(compressed_list[pos].1.clone());
                seg_est += cand_len;
                segment_objects += 1;
                pos += 1;
            }

            if segment_objects == 0 && pos < total {
                temp_objs_bytes.push(compressed_list[pos].1.clone());
                segment_objects = 1;
                seg_est += temp_objs_bytes.last().unwrap().len();
                pos += 1;
            }

            let mut seg_buf = BytesMut::with_capacity(seg_est + 64);
            let mut header = BytesMut::new();
            header.extend_from_slice(b"PACK");
            header.put_u32(2u32); // version 2
            header.put_u32(segment_objects as u32);
            seg_buf.extend_from_slice(&header);

            let mut hash = self.txn.repository.hash_version.default();

            // 包含 header
            hash.update(&header[..]);

            for b in &temp_objs_bytes {
                hash.update(&b[..]);
                seg_buf.extend_from_slice(&b[..]);
            }

            let final_hash = hash.finalize();
            seg_buf.extend_from_slice(final_hash.as_bytes());

            trace!(
                "pack segment {} built: {} objects, {} bytes total",
                pack_idx,
                segment_objects,
                seg_buf.len()
            );

            let raw = seg_buf.split().freeze();

            if self.sideband {
                let mut offset = 0usize;
                while offset < raw.len() {
                    let chunk_size = (raw.len() - offset).min(MAX_PAYLOAD_PER_PKT);
                    let chunk = raw.slice(offset..offset + chunk_size);
                    let pkt_len = 4 + 1 + chunk.len();
                    let mut pkt = BytesMut::with_capacity(pkt_len);
                    pkt.extend_from_slice(format!("{:04x}", pkt_len).as_bytes());
                    pkt.put_u8(1);
                    pkt.extend_from_slice(&chunk);
                    self.txn.call_back.send(pkt.freeze()).await;
                    offset += chunk_size;
                }
            } else {
                self.txn.call_back.send(Bytes::from(raw)).await;
            }

            if self.sideband {
                let percent = ((pos) * 100 / total).min(100);
                let progress_payload =
                    format!("pack segment {} progress: {}%\n", pack_idx, percent);
                let pkt = build_sideband_pkt(2, progress_payload.as_bytes());
                self.txn.call_back.send(pkt).await;
            } else {
                self.txn
                    .call_back
                    .send_pkt_line(Bytes::from(format!(
                        "pack segment {} progress: {}%\n",
                        pack_idx,
                        (pos * 100 / total)
                    )))
                    .await;
            }

            any_segment_sent = true;
            pack_idx += 1;
        }

        if any_segment_sent {
            self.txn.call_back.send(Bytes::from_static(b"0000")).await;
        }

        Ok(())
    }
}

fn build_sideband_pkt(band: u8, payload: &[u8]) -> Bytes {
    let total_len = 4 + 1 + payload.len();
    let mut pkt = BytesMut::with_capacity(total_len);
    pkt.extend_from_slice(format!("{:04x}", total_len).as_bytes());
    pkt.put_u8(band);
    pkt.extend_from_slice(payload);
    pkt.freeze()
}
