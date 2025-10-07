use crate::capability::enums::GitCapability;
use crate::error::GitInnerError;
use crate::sha::{HashValue, HashVersion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadCommandType {
    Want(HashValue),
    Have(HashValue),
    Done,
    Shallow(HashValue),
    Deepen(i32),
    Capabilities(Vec<GitCapability>),
    Flush,

    // v2 only
    Command(String),
    // v2 only
    Agent(String),
    // v2 only
    Symrefs,
    // v2 only
    Unborn,
    // v2 only
    RefPrefix(String),
    ObjectFormat(String),
    Peel,
    ThinPack,
    OfsDelta
}

impl UploadCommandType {
    pub fn from_one_line(
        line: &str,
        hash_version: HashVersion,
    ) -> Result<Vec<UploadCommandType>, GitInnerError> {
        let line_str = line.trim();
        if line_str.is_empty() {
            return Ok(vec![]);
        }
        if line_str.starts_with("want ") {
            let parts: Vec<&str> = line_str[5..].split_whitespace().collect();
            if parts.is_empty() {
                return Err(GitInnerError::ConversionError(
                    "Missing hash after 'want'".into(),
                ));
            }

            let hash_str = parts[0];
            if hash_str.len() < hash_version.len() {
                return Err(GitInnerError::ConversionError(
                    "Invalid hash length".into(),
                ));
            }

            let hash = HashValue::from_str(hash_str)
                .ok_or(GitInnerError::ConversionError("Invalid hash value".into()))?;

            let capabilities = if parts.len() > 1 {
                parts[1..]
                    .iter()
                    .filter_map(|s| Option::from(GitCapability::from_str(s)))
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };

            let mut res = vec![];
            if !capabilities.is_empty() {
                res.push(UploadCommandType::Capabilities(capabilities));
            }
            res.push(UploadCommandType::Want(hash));
            return Ok(res);
        }
        if line_str.starts_with("have ") {
            let hash_str = &line_str[5..];
            let hash = HashValue::from_str(hash_str)
                .ok_or(GitInnerError::ConversionError("Invalid have hash".into()))?;
            return Ok(vec![UploadCommandType::Have(hash)]);
        }

        if line_str == "done" {
            return Ok(vec![UploadCommandType::Done]);
        }

        if line_str.starts_with("shallow ") {
            let hash_str = &line_str[8..];
            let hash = HashValue::from_str(hash_str)
                .ok_or(GitInnerError::ConversionError("Invalid shallow hash".into()))?;
            return Ok(vec![UploadCommandType::Shallow(hash)]);
        }
        if line_str.starts_with("deepen ") {
            let depth = line_str[7..]
                .parse::<i32>()
                .map_err(|_| GitInnerError::ConversionError("Invalid deepen value".into()))?;
            return Ok(vec![UploadCommandType::Deepen(depth)]);
        }
        if line_str.starts_with("command=") {
            let cmd = line_str[8..].to_string();
            return Ok(vec![UploadCommandType::Command(cmd)]);
        }
        if line_str.starts_with("agent=") {
            let agent = line_str[6..].to_string();
            return Ok(vec![UploadCommandType::Agent(agent)]);
        }
        if line_str == "symrefs" {
            return Ok(vec![UploadCommandType::Symrefs]);
        }
        if line_str == "unborn" {
            return Ok(vec![UploadCommandType::Unborn]);
        }
        if line_str.starts_with("ref-prefix ") {
            let prefix = line_str[11..].to_string();
            return Ok(vec![UploadCommandType::RefPrefix(prefix)]);
        }
        if line_str.starts_with("object-format=") {
            let format = line_str[14..].to_string();
            return Ok(vec![UploadCommandType::ObjectFormat(format)]);
        }
        if line_str == "peel" {
            return Ok(vec![UploadCommandType::Peel]);
        }
        if line_str == "thin-pack" {
            return Ok(vec![UploadCommandType::ThinPack]);
        }
        if line_str == "ofs-delta" {
            return Ok(vec![UploadCommandType::OfsDelta]);
        }
        if line_str == "0000" {
            return Ok(vec![UploadCommandType::Flush]);
        }

        Err(GitInnerError::ConversionError(format!(
            "Unknown upload-pack command: {}",
            line_str
        )))
    }
}
