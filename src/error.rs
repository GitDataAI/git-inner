use actix_web::error::PayloadError;

#[derive(Clone, Debug, PartialEq)]
pub enum GitInnerError {
    TimeError(String),
    InvalidSignatureType(String),
    ConversionError(String),
    InvalidTimestamp,
    InvalidSignature,
    InvalidTreeItem(String),
    InvalidSha1String,
    InvalidSha256String,
    LockError,
    InvalidUtf8,
    UnexpectedEof,
    InvalidData,
    NotSupportVersion,
    DecompressionError,
    InvalidObjectType,
    SqliteError(String),
    HashVersionError,
    Payload(String),
}
