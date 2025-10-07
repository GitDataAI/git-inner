use crate::transaction::service::TransactionService;
use crate::transaction::Transaction;
use bytes::Bytes;

impl Transaction {
    pub async fn http_advertise_header(&self) {
        let head = Bytes::from(format!(
            "# service={}\n",
            match self.service {
                TransactionService::UploadPack => "git-upload-pack",
                TransactionService::ReceivePack => "git-receive-pack",
                TransactionService::UploadPackLs => "git-upload-pack",
                TransactionService::ReceivePackLs => "git-receive-pack",
            }
        ));
        self.call_back.send_pkt_line(head).await;
    }
}