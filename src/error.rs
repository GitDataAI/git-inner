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
    AppInitError,
    AppNotInit,
}

impl From<bson::ser::Error> for GitInnerError {
    /// Convert a BSON serialization error into a `GitInnerError::BJSONERROR`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let bson_err: bson::ser::Error = /* obtained from BSON serialization */ unimplemented!();
    /// let git_err: GitInnerError = GitInnerError::from(bson_err);
    /// assert!(matches!(git_err, GitInnerError::BJSONERROR(_)));
    /// ```
    fn from(e: bson::ser::Error) -> Self {
        GitInnerError::BJSONERROR(e)
    }
}

impl From<russh::Error> for GitInnerError {
    /// Convert a `russh::Error` into a `GitInnerError::RusshError`.
    ///
    /// The resulting variant contains the original error's `Display` output as a `String`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let err: russh::Error = /* obtain a russh error */ unimplemented!();
    /// let git_err: crate::error::GitInnerError = err.into();
    /// match git_err {
    ///     crate::error::GitInnerError::RusshError(s) => assert!(!s.is_empty()),
    ///     _ => unreachable!(),
    /// }
    /// ```
    fn from(e: russh::Error) -> Self {
        GitInnerError::RusshError(format!("{}", e))
    }
}