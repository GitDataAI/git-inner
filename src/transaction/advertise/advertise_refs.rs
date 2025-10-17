use crate::transaction::{GitProtoVersion, ProtocolType, Transaction, TransactionService};
use bytes::Bytes;

impl Transaction {
    pub async fn advertise_refs(&self) -> Result<(), crate::error::GitInnerError> {
        match self.protocol {
            ProtocolType::Git => {}
            ProtocolType::SSH => {}
            ProtocolType::Http => {
                self.http_advertise_header().await;
            }
        }
        match (&self.service, &self.version) {
            (
                TransactionService::UploadPack | TransactionService::UploadPackLs,
                GitProtoVersion::V2,
            ) => {
                self.call_back.send(Bytes::from("0000")).await;
                self.write_version().await;
                self.write_advertise_v2().await?;
            }
            (TransactionService::UploadPack | TransactionService::UploadPackLs, _)
            | (TransactionService::ReceivePack | TransactionService::ReceivePackLs, _) => {
                self.write_version().await;
                self.call_back.send(Bytes::from("0000")).await;
                self.write_refs_head_info().await?;
                self.write_all_refs().await?;
                self.call_back.send(Bytes::from("0000")).await;
            }
        }
        self.call_back.send(Bytes::new()).await;
        Ok(())
    }
}
