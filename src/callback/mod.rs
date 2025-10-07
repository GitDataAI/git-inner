use std::sync::Arc;
use bytes::{BufMut, Bytes, BytesMut};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use crate::callback::sidebend::SideBend;

#[derive(Clone)]
pub struct CallBack {
    pub callback: Sender<Bytes>,
    pub receive: Arc<Mutex<Receiver<Bytes>>>
}

impl CallBack {
    pub fn new(size: usize) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(size);
        Self {
            callback: tx,
            receive: Arc::new(Mutex::new(rx))
        }
    }
    pub async fn send(&self, kind: Bytes) {
        self.callback.send(kind).await.ok();
    }
    pub async fn send_pkt_line(&self, line: Bytes) {
        let len = line.len();
        let mut result = BytesMut::from(format!("{:04x}", len + 4).as_bytes());
        result.extend_from_slice(&line);
        self.send(result.freeze()).await;
    }
    pub async fn send_side_pkt_line(&self, line: Bytes, side: SideBend) {
        if side == SideBend::SidebandFlush {
            let result = BytesMut::from(
                format!("{:04x}", 1).as_bytes()
            );
            self.send(result.freeze()).await;
            return;
        }
        let len = line.len().saturating_add(1);
        let mut result = BytesMut::from(
            format!("{:04x}", len + 4).as_bytes()
        );
        result.put_u8(side.to_u32() as u8);
        result.extend_from_slice(&line);
        self.send(result.freeze()).await;
    }
}


pub mod sidebend;