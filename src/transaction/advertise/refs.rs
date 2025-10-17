use crate::capability::enums::GitCapability;
use crate::error::GitInnerError;
use crate::sha::HashVersion;
use crate::transaction::Transaction;
use crate::transaction::service::TransactionService;
use crate::write_pkt_line;
use bstr::ByteSlice;
use bytes::BytesMut;

impl Transaction {
    pub async fn write_refs_head_info(&self) -> Result<(), GitInnerError> {
        let mut capabilities = GitCapability::basic();
        match self.service {
            TransactionService::UploadPack | TransactionService::UploadPackLs => {
                capabilities.extend_from_slice(&GitCapability::upload())
            }
            TransactionService::ReceivePack | TransactionService::ReceivePackLs => {
                capabilities.extend_from_slice(&GitCapability::receive())
            }
        }
        let sha_version = GitCapability::ObjectFormat(match self.repository.hash_version {
            HashVersion::Sha1 => "sha1".to_string(),
            HashVersion::Sha256 => "sha256".to_string(),
        });
        capabilities.push(sha_version);
        let head = self.repository.refs.head().await?;
        let mut result = BytesMut::new();
        result.extend_from_slice(
            format!(
                "{} HEAD\0{}\n",
                head.value.to_string(),
                capabilities
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .as_bytes(),
        );
        self.call_back.send_pkt_line(result.freeze()).await;
        Ok(())
    }
    pub async fn write_refs_head_info_v2(&self, symref: bool) -> Result<(), GitInnerError> {
        let head = self.repository.refs.head().await?;
        let mut result = BytesMut::new();
        let symref_str = if symref {
            format!("symref=HEAD:{}", head.name.to_string())
        } else {
            String::new()
        };
        result.extend_from_slice(
            format!("{} HEAD\0{}\n", head.value.to_string(), symref_str).as_bytes(),
        );
        self.call_back.send_pkt_line(result.freeze()).await;
        Ok(())
    }
    pub async fn write_all_refs(&self) -> Result<(), GitInnerError> {
        let refs = self.repository.refs.refs().await?;
        for ref_item in refs {
            let mut result = BytesMut::new();
            result.extend_from_slice(
                write_pkt_line(format!(
                    "{} {}",
                    ref_item.value.to_string(),
                    ref_item.name.to_string()
                ))
                .as_bytes(),
            );
            self.call_back.send(result.freeze()).await;
        }
        Ok(())
    }
}
