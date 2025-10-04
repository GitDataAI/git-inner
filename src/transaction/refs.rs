use crate::capability::GitCapability;
use crate::error::GitInnerError;
use crate::pkt_line::write_pkt_line;
use crate::transaction::{Transaction, TransactionService};
use bstr::ByteSlice;
use bytes::{Bytes, BytesMut};
use crate::sha::HashVersion;

impl Transaction {
    pub async fn advertise(&self) -> Result<Bytes, GitInnerError> {
        let mut capabilities = GitCapability::basic();
        match self.service {
            TransactionService::UploadPack => {
                capabilities.extend_from_slice(&GitCapability::upload())
            }
            TransactionService::ReceivePack => {
                capabilities.extend_from_slice(&GitCapability::receive())
            }
            TransactionService::UploadPackLs => {}
            TransactionService::ReceivePackLs => {}
        }
        let sha_version = GitCapability::ObjectFormat(match self.repository.hash_version {
            HashVersion::Sha1 => "sha1".to_string(),
            HashVersion::Sha256 => "sha256".to_string()
        });
        capabilities.push(sha_version);
        let head = self.repository.refs.head().await?;
        let refs = self.repository.refs.refs().await?;
        let mut result = BytesMut::new();
        if refs.is_empty() {
            result.extend_from_slice(
                write_pkt_line(format!(
                    "{} HEAD\0{}",
                    self.repository.hash_version.default().to_string(),
                    capabilities
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                ))
                .as_bytes(),
            );
            return Ok(Bytes::from(result));
        }
        result.extend_from_slice(
            write_pkt_line(format!(
                "{} HEAD\0{}",
                head.value.to_string(),
                capabilities
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ))
            .as_bytes(),
        );
        for ref_item in refs {
            result.extend_from_slice(
                write_pkt_line(format!(
                    "{} {}",
                    ref_item.value.to_string(),
                    ref_item.name.to_string()
                ))
                .as_bytes(),
            );
        }
        Ok(Bytes::from(result))
    }
    pub async fn http_advertise(&self) -> Result<Bytes, GitInnerError> {
        let advertise = self.advertise().await?;
        let byte = BytesMut::from(advertise);
        let mut head = BytesMut::from(write_pkt_line(format!(
            "# service={}",
            match self.service {
                TransactionService::UploadPack => "git-upload-pack",
                TransactionService::ReceivePack => "git-receive-pack",
                TransactionService::UploadPackLs => "git-upload-pack",
                TransactionService::ReceivePackLs => "git-receive-pack",
            }
        )));
        head.extend_from_slice(b"0000");
        head.extend_from_slice(&byte);
        head.extend_from_slice(b"0000");
        Ok(Bytes::from(head))
    }
}
