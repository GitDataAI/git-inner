use std::pin::Pin;
use std::sync::Arc;
use bstr::ByteSlice;
use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use tokio_stream::Stream;
use tracing::log::warn;
use crate::capability::enums::GitCapability;
use crate::error::GitInnerError;
use crate::odb::OdbTransaction;
use crate::transaction::receive::command::ReceiveCommand;
use crate::transaction::Transaction;
use crate::transaction::version::GitProtoVersion;

pub mod command;
pub mod parse_objects;
pub mod parse_receive_object;
pub mod zlib_decode;

#[derive(Clone)]
pub struct ReceivePackTransaction {
    pub transaction: Transaction,
    pub ref_upload: Vec<ReceiveCommand>,
    pub capabilities: Vec<GitCapability>,
    pub version: GitProtoVersion,
    pub pack_size: usize,
}

impl Transaction {
    pub async fn receive_pack(
        &mut self,
        mut stream: Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>>>>,
    ) -> Result<(), GitInnerError> {
        let mut head = BytesMut::new();
        let txn = self.repository.odb.begin_transaction().await?;
        while let Some(pack) = stream.next().await {
            let pack = pack?;
            if pack == "0000" {
                tokio::task::yield_now().await;
                continue
            }
            if let Some(idx) = pack.find(b"PACK") {
                head.extend_from_slice(&pack[..idx]);
                let input =
                    tokio_stream::iter(vec![Ok(Bytes::from(pack[idx..].to_vec()))]).chain(stream);
                stream = Box::pin(input);
                break;
            } else {
                head.extend_from_slice(&pack);
            }
        }
        let (refs, caps) = self.parse_receive_request(head).await?;
        self.parse_receive_head(refs,caps, stream, txn).await?;
        Ok(())
    }
    pub async fn parse_receive_request(
        &self,
        head: BytesMut,
    ) -> Result<(Vec<ReceiveCommand>, Vec<GitCapability>), GitInnerError> {
        let mut refs = vec![];
        let mut capabilities = vec![];
        for line in head.lines() {
            let str = line
                .to_str()
                .map_err(|_| GitInnerError::InvalidUtf8)?
                .to_string();
            if let Some(idx) = str.find("\0") {
                if let Ok(Some(pkt_line)) = ReceiveCommand::from_pkt_line(&str.as_bytes()) {
                    refs.push(pkt_line);
                }
                let caps = str[idx + 1..]
                    .split(' ')
                    .map(|s| GitCapability::from_str(s))
                    .collect::<Vec<_>>();
                capabilities = caps;
            } else {
                if let Ok(Some(pkt_line)) = ReceiveCommand::from_pkt_line(&str.as_bytes()) {
                    refs.push(pkt_line);
                }
            }
        }
        Ok((refs, capabilities))
    }

    pub async fn parse_receive_head(
        &mut self,
        refs: Vec<ReceiveCommand>,
        caps: Vec<GitCapability>,
        mut stream: Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>>>>,
        txn: Box<dyn OdbTransaction>,
    ) -> Result<(), GitInnerError> {
        let mut head = BytesMut::with_capacity(12);
        let mut remaining = 12;
        let mut retry = 12;
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
                if retry == 0 {
                    return Err(GitInnerError::UnexpectedEof);
                } else {
                    warn!("retry receive pack");
                    retry -= 1;
                    continue;
                }
            }
        }
        if head.len() != 12 {
            return Err(GitInnerError::InvalidData);
        }
        let version = (head[4] as usize) << 24
            | (head[5] as usize) << 16
            | (head[6] as usize) << 8
            | (head[7] as usize);
        let pack_size = (head[8] as usize) << 24
            | (head[9] as usize) << 16
            | (head[10] as usize) << 8
            | (head[11] as usize);
        let mut receive_pack_request = ReceivePackTransaction {
            transaction: self.clone(),
            ref_upload: refs,
            capabilities: caps,
            version: GitProtoVersion::from_u32(version as u32),
            pack_size,
        };
        match receive_pack_request.version {
            GitProtoVersion::V0 | GitProtoVersion::V1 | GitProtoVersion::V2 => {
                receive_pack_request
                    .process_receive_pack(
                        stream,
                        Arc::from(txn),
                    )
                    .await?;
            }
            GitProtoVersion::Unknown => {
                dbg!();
            }
        }
        Ok(())
    }
}