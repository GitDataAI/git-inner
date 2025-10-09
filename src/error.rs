use mongodb::bson;
use crate::sha::HashValue;

#[derive(Clone, Debug)]
pub enum GitInnerError {
    InvalidSha1String,
    InvalidSha256String,
    MissingBaseObject,
    DeltaBaseSizeMismatch,
    DeltaInvalidInstruction,
    DeltaResultSizeMismatch,
    UnexpectedEof,
    InvalidUtf8,
    InvalidData,
    ConversionError(String),
    InvalidSignatureType(String),
    InvalidSignature,
    InvalidTimestamp,
    MongodbError(String),
    DefaultBranchCannotBeDeleted,
    BJSONERROR(bson::ser::Error),
    ObjectNotFound(HashValue),
    MissingField(&'static str),
    InvalidTreeItem(String),
    InvalidDelta,
    MissingAuthor,
    MissingCommitter,
    ObjectStoreError(String),
    HashVersionError,
    UuidError,
    TreeParseError,
    TagParseError,
    CommitParseError,
    NotSupportVersion,
    DecompressionError,
    UnsupportedOfsDelta,
    InvalidHash,
    UnsupportedVersion,
    ZlibError,
    Payload(String),
    NotSupportCommand,
    Other(String),
    RusshError(String),
    SshServerStartError(String),
}

impl From<bson::ser::Error> for GitInnerError {
    fn from(e: bson::ser::Error) -> Self {
        GitInnerError::BJSONERROR(e)
    }
}

impl From<russh::Error> for GitInnerError {
    fn from(e: russh::Error) -> Self {
        GitInnerError::RusshError(format!("{}", e))
    }
}