use crate::error::GitInnerError;
use crate::objects::ObjectTrait;
use crate::objects::signature::Signature;
use crate::objects::types::ObjectType;
use crate::sha::{HashValue, HashVersion};
use bincode::{Decode, Encode};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Decode, Encode)]
pub struct Commit {
    pub hash: HashValue,
    pub message: String,
    pub author: Signature,
    pub committer: Signature,
    pub parents: Vec<HashValue>,
    pub tree: Option<HashValue>,
    pub gpgsig: Option<Gpgsig>,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Decode, Encode)]
pub struct Gpgsig {
    pub signature: String,
}

// 例1：
// tree 7551d4da2e9c1ae9397c47709253b405fb6b6206\n
// parent ee98d64f596ae42fadf9eeae1d0efa22b14b0829\n
// author ZhenYi <434836402@qq.com> 1740189120 +0800\n
// committer ZhenYi <434836402@qq.com> 1740189120 +0800\n\n
// build(deps): Update dependencies and replace poem with actix-web\n

// 例2：
// tree 2e9a1fd02f006b122a8ffa06462452e0a5b0e62d\n
// parent 3fcd9409966c756d10c20125ce90ded69fa4f42f\n
// author allcontributors[bot] <46447321+allcontributors[bot]@users.noreply.github.com> 1744461715 +0000\n
// committer GitHub <noreply@github.com> 1744461715 +0000\n
// gpgsig -----BEGIN PGP SIGNATURE-----\n
// \n
// wsFcBAABCAAQBQJn+l+TCRC1aQ7uu5UhlAAAp9UQACW+Mant/8mFJ2tc+YVsj3/Y\n
// OXtNrJslR+QyIMB0zsG7jDZL1r1Rym8P4ApCRPVUtRUkXxhRSja3q8uoGUUZAHcs\n
// 3nkp4e/SOA7ri3las8dcImPKFErc3ojbDsspuBVTr2TbhjddsSt+OgOd+N/qdUMx\n
// ORNxYIvTdhwkXUTLZB8MLnvUWLdqHWkP5PkoTX+87lD7ftzUyuY1hXTXfkTydVOP\n
// W2ZzjFJdsNzLmHAiDzCTyYiBXlEzdYrM+i6HbZsricBAjJoLWxUrEZISCJt0Zne/\n
// 7uoInu3FN3fY6tRhgDZtw/WOezUmcfTgmZ5oNp/PEo4w3mOOTDlqnTHiEn69BSX6\n
// QkxPFydR5IXGyhki97naD4BWxI0e0qwbIJz5lrKAJThqMocofD+hBdb13vE0Jrrm\n
// h9m6eS4cOF8M5QRXE/jmvyT2wfIUaUDWuWwI5vJNLjtSDUgf5moScPBufFpKRi22\n
// OPCQeXQ9rIgFWcEq0+Yjheno7CZEKaayCvvlU5+QzrJ1X4//2bxpQP/pdDQW9BQY\n
// 5VfQiL0e7cHFxACLt2nRwoEIhjoxDIn5p8kLX/EgnkxQ/sV5wFwDcqY9C18JFutx\n
// 4gKt9Ev1qWCrnl3I3ZGTIYzP7e7ekqO5JKYmGwCOvOJLM0o1i5L4suZw6CAKjSMf\n
// s1mGXwic2SyX5DyL95Uk\n
// =FsXv\n
// -----END PGP SIGNATURE-----\n
// \n
// \n
// docs: update README.md [skip ci]

#[derive(Debug)]
pub enum ParseError {}

impl Commit {
    pub fn parse(input: Bytes, version: HashVersion) -> Result<Commit, GitInnerError> {
        let mut hash_prev = Vec::new();
        hash_prev.extend_from_slice(format!("commit {}\0", input.len()).as_bytes());
        hash_prev.extend_from_slice(&input);
        let hash = version.hash(Bytes::from(hash_prev));
        let input_str = str::from_utf8(&input).map_err(|_| GitInnerError::InvalidUtf8)?;
        let split_index = input_str.find("\n\n").ok_or(GitInnerError::InvalidUtf8)?;
        let header_str = &input_str[..split_index];
        let message = &input_str[split_index + 2..];
        let mut tree: Option<HashValue> = None;
        let mut parents: Vec<HashValue> = Vec::new();
        let mut author: Option<Signature> = None;
        let mut committer: Option<Signature> = None;
        let mut gpgsig: Option<String> = None;

        let mut lines = header_str.lines();
        let mut collecting_gpgsig = false;
        let mut gpgsig_buffer = String::new();

        while let Some(line) = lines.next() {
            if collecting_gpgsig {
                gpgsig_buffer.push_str(line);
                gpgsig_buffer.push('\n');
                if line.contains("END PGP SIGNATURE") {
                    collecting_gpgsig = false;
                    gpgsig = Some(gpgsig_buffer.trim_end().to_string());
                    gpgsig_buffer.clear();
                }
                continue;
            }

            if line.starts_with("tree ") {
                let hash_str = line["tree ".len()..].trim();
                tree = HashValue::from_str(hash_str);
            } else if line.starts_with("parent ") {
                let hash_str = line["parent ".len()..].trim();
                if let Some(parent_hash) = HashValue::from_str(hash_str) {
                    parents.push(parent_hash);
                }
            } else if line.starts_with("author ") {
                let data = line["author ".len()..].trim();
                author = Some(
                    Signature::from_data(format!("author {}", data).as_bytes().to_vec())
                        .map_err(|_| GitInnerError::MissingAuthor)?,
                );
            } else if line.starts_with("committer ") {
                let data = line["committer ".len()..].trim();
                committer = Some(
                    Signature::from_data(format!("committer {}", data).as_bytes().to_vec())
                        .map_err(|_| GitInnerError::MissingCommitter)?,
                );
            } else if line.starts_with("gpgsig ") {
                collecting_gpgsig = true;
                gpgsig_buffer.push_str(line);
                gpgsig_buffer.push('\n');
            }
        }

        Ok(Commit {
            hash,
            message: message.to_string(),
            author: author.ok_or(GitInnerError::MissingAuthor)?,
            committer: committer.ok_or(GitInnerError::MissingCommitter)?,
            parents,
            tree,
            gpgsig: gpgsig.map(|sig| Gpgsig { signature: sig }),
        })
    }
}

impl Display for Commit {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(tree) = &self.tree {
            writeln!(f, "tree {}", tree)?;
        }
        for parent in &self.parents {
            writeln!(f, "parent {}", parent)?;
        }
        writeln!(f, "author {}", self.author)?;
        writeln!(f, "committer {}", self.committer)?;
        if let Some(gpgsig) = &self.gpgsig {
            let sig_lines = gpgsig.signature.lines();
            writeln!(f, "gpgsig -----BEGIN PGP SIGNATURE-----")?;
            for line in sig_lines {
                writeln!(f, " {}", line)?;
            }
            writeln!(f, " -----END PGP SIGNATURE-----")?;
        }
        writeln!(f)?;
        write!(f, "{}", self.message)
    }
}

impl Debug for Commit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl ObjectTrait for Commit {
    fn get_type(&self) -> ObjectType {
        ObjectType::Commit
    }

    fn get_size(&self) -> usize {
        self.get_data().len()
    }

    fn get_data(&self) -> Bytes {
        Bytes::from(self.to_string())
    }
}
