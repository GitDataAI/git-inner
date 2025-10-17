use crate::error::GitInnerError;
use async_trait::async_trait;

#[async_trait]
pub trait ProtocolCapability {
    fn name(&self) -> &str;
    async fn advertise(&self) -> Result<(), GitInnerError> {
        Ok(())
    }
    async fn handle(&self) -> Result<(), GitInnerError> {
        Ok(())
    }
}

pub mod enums;
