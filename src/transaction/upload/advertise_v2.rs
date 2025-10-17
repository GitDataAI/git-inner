use crate::error::GitInnerError;
use crate::transaction::Transaction;
use bytes::Bytes;

impl Transaction {
    pub async fn write_advertise_v2(&self) -> Result<(), GitInnerError> {
        let agent = "agent=git/1.51\n".to_string();
        let sha_version = match self.repository.hash_version {
            crate::sha::HashVersion::Sha1 => "sha1",
            crate::sha::HashVersion::Sha256 => "sha256",
        };
        let object_format = format!("object-format={}\n", sha_version);
        let fetch = "fetch=shallow filter wait-for-done\n";
        let server_option = "server-option\n";
        let ls_refs = "ls-refs=unborn\n";
        self.call_back.send_pkt_line(Bytes::from(agent)).await;
        self.call_back.send_pkt_line(Bytes::from(ls_refs)).await;
        self.call_back.send_pkt_line(Bytes::from(fetch)).await;
        self.call_back
            .send_pkt_line(Bytes::from(server_option))
            .await;
        self.call_back
            .send_pkt_line(Bytes::from(object_format))
            .await;
        self.call_back.send(Bytes::from("0000")).await;
        Ok(())
    }
}
