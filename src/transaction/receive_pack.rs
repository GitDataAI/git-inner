use crate::capability::GitCapability;
use crate::error::GitInnerError;
use crate::odb::OdbTransaction;
use crate::transaction::receive_pack::command::ReceiveCommand;
use crate::transaction::{GitProtoVersion, Transaction};
use bstr::ByteSlice;
use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;
use tracing::warn;

pub mod command;

#[derive(Debug, Clone)]
pub struct ReceivePackRequest {
    pub ref_upload: Vec<ReceiveCommand>,
    pub capabilities: Vec<GitCapability>,
}

#[derive(Debug, Clone)]
pub struct ReceivePackRequestInfo {
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
                return Ok(());
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
        let request = self.parse_receive_request(head).await?;
        self.parse_receive_head(request, stream, txn).await?;
        Ok(())
    }
    pub async fn parse_receive_request(
        &self,
        head: BytesMut,
    ) -> Result<ReceivePackRequest, GitInnerError> {
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
        Ok(ReceivePackRequest {
            ref_upload: refs,
            capabilities,
        })
    }
    pub async fn parse_receive_head(
        &mut self,
        receive_pack_request: ReceivePackRequest,
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
        let request_info = ReceivePackRequestInfo {
            version: match version {
                0 => GitProtoVersion::V0,
                1 => GitProtoVersion::V1,
                2 => GitProtoVersion::V2,
                _ => return Err(GitInnerError::NotSupportVersion),
            },
            pack_size,
        };
        self.parse_receive_object(receive_pack_request, request_info, stream, Arc::new(txn))
            .await?;
        Ok(())
    }
}

pub mod deltas;
pub mod pack_head;
pub mod parse_object;
pub mod parse_receive_object;
pub mod report_status;
pub mod zlib_decode;