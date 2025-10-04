use crate::capability::GitCapability;
use crate::transaction::Transaction;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

/// Git report-status 类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportStatusKind {
    Ok(String),          // ok <ref>
    Ng(String, String),  // ng <ref> <message>
    UnpackOk,            // unpack ok
    UnpackError(String), // unpack error <message>
    Done,
    Info(String),

    // side-band 包装
    SideOK(String),
    SideNg(String, String),
    SideUnPack,
    SideUnpackError(String),
    SideInfo(String),
    SideDone,
}

/// Side-band
const BAND_PROGRESS: u8 = 1; // remote: 信息流
const BAND_REPORT: u8 = 2;   // report-status (unpack ok / ok ref)
const BAND_ERROR: u8 = 3;    // fatal

impl ReportStatusKind {
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            // --- Side-band 模式 ---
            ReportStatusKind::SideInfo(msg) => sb_pkt(BAND_REPORT, format!("{}", msg).as_bytes().to_vec()),
            ReportStatusKind::SideOK(r) => sb_pkt(BAND_PROGRESS, pkt(&format!("ok {}\n", r))),
            ReportStatusKind::SideNg(r, msg) => sb_pkt(BAND_ERROR, pkt(&format!("ng {} {}\n", r, msg))),
            ReportStatusKind::SideUnPack => sb_pkt(BAND_PROGRESS, pkt("unpack ok\n")),
            ReportStatusKind::SideUnpackError(msg) => sb_pkt(BAND_ERROR, pkt(&format!("unpack error {}\n", msg))),
            ReportStatusKind::SideDone => bend_pkt_flush(),

            // --- 普通 (非 side-band) 模式 ---
            ReportStatusKind::Info(msg) => pkt(&format!("remote: {}\n", msg)),
            ReportStatusKind::Ok(r) => pkt(&format!("ok {}\n", r)),
            ReportStatusKind::Ng(r, msg) => pkt(&format!("ng {} {}\n", r, msg)),
            ReportStatusKind::UnpackOk => pkt("unpack ok\n"),
            ReportStatusKind::UnpackError(msg) => pkt(&format!("unpack error {}\n", msg)),
            ReportStatusKind::Done => pkt(&pkt_flush()),
        }
    }
}

#[derive(Clone)]
pub struct ReportStatus {
    sender: Sender<ReportStatusKind>,
    pub receiver: Arc<Mutex<Receiver<ReportStatusKind>>>,
}

impl ReportStatus {
    pub fn new(buffer: usize) -> Self {
        let (sender, receiver) = mpsc::channel(buffer);
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn sender(&self) -> Sender<ReportStatusKind> {
        self.sender.clone()
    }

    pub async fn ok_ref(&self, r#ref: impl Into<String>) {
        let _ = self.sender.send(ReportStatusKind::Ok(r#ref.into())).await;
    }
    pub async fn ng_ref(&self, r#ref: impl Into<String>, msg: impl Into<String>) {
        let _ = self.sender.send(ReportStatusKind::Ng(r#ref.into(), msg.into())).await;
    }
    pub async fn unpack_ok(&self) {
        let _ = self.sender.send(ReportStatusKind::UnpackOk).await;
    }
    pub async fn unpack_error(&self, msg: impl Into<String>) {
        let _ = self.sender.send(ReportStatusKind::UnpackError(msg.into())).await;
    }
    pub async fn info(&self, msg: impl Into<String>) {
        let _ = self.sender.send(ReportStatusKind::Info(msg.into())).await;
    }
    pub async fn done(&self) {
        let _ = self.sender.send(ReportStatusKind::Done).await;
    }
}

impl Transaction {
    pub async fn report_status(&self, msg: ReportStatusKind, git_capability: Vec<GitCapability>) {
        if git_capability.contains(&GitCapability::ReportStatus) {
            let is_side = git_capability.contains(&GitCapability::SideBand)
                || git_capability.contains(&GitCapability::SideBand64k);

            let side_msg = if is_side { self.to_side(msg) } else { msg };
            let _ = self.report_status.sender.send(side_msg).await;
        }
    }

    fn to_side(&self, msg: ReportStatusKind) -> ReportStatusKind {
        match msg {
            ReportStatusKind::Ok(r) => ReportStatusKind::SideOK(r),
            ReportStatusKind::Ng(r, msg) => ReportStatusKind::SideNg(r, msg),
            ReportStatusKind::UnpackOk => ReportStatusKind::SideUnPack,
            ReportStatusKind::UnpackError(msg) => ReportStatusKind::SideUnpackError(msg),
            ReportStatusKind::Info(msg) => ReportStatusKind::SideInfo(msg),
            ReportStatusKind::Done => ReportStatusKind::SideDone,
            _ => ReportStatusKind::SideInfo("internal error".to_string()),
        }
    }
}

fn sb_pkt(band: u8, text: Vec<u8>) -> Vec<u8> {
    let payload = text;
    let len = payload.len() + 5; // 4字节长度 + 1字节band
    assert!(len <= 0xffff, "side-band line too long");
    let mut buf = format!("{:04x}", len).into_bytes();
    buf.push(band);
    buf.extend_from_slice(&*payload);
    buf
}

fn pkt(text: &str) -> Vec<u8> {
    let payload = text.as_bytes();
    let len = payload.len() + 4;
    assert!(len <= 0xffff, "pkt-line too long");
    let mut buf = format!("{:04x}", len).into_bytes();
    buf.extend_from_slice(payload);
    buf
}

fn pkt_flush() -> String {
    "0000".to_string()
}

/// 我不知道为什么要这样设计，也不清楚原理，但是逆向抓包 git-http-backend 发现他是这样返回的
///
/// build for git for windows 1.51.0.2
fn bend_pkt_flush() -> Vec<u8> {
    let basic = "00000000".as_bytes();
    let mut bend = vec![1];
    bend.extend_from_slice(basic);
    let len = bend.len();
    let head = format!("{:04x}", len).into_bytes();
    head.into_iter().chain(bend.into_iter()).collect()
}

#[test]
fn test_pkt() {
    use bytes::Bytes;
    println!("{:?}",Bytes::from(bend_pkt_flush()))
}