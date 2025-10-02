use std::fmt::{Debug, Display, Formatter};
use bincode::{Decode, Encode};
use bstr::ByteSlice;
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use crate::objects::signature::Signature;
use crate::sha::{HashValue, HashVersion};

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Decode, Encode)]
pub struct Commit {
    pub hash: HashValue,
    pub message: String,
    pub author: Signature,
    pub committer: Signature,
    pub parents: Vec<HashValue>,
    pub tree: Option<HashValue>,
    pub gpgsig: Option<Gpgsig>
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



impl Commit {
    pub fn parse(input: Bytes, version: HashVersion) -> Option<Commit> {
        let mut hash_prev = BytesMut::from(format!("commit {}\0", input.len()).as_bytes());
        hash_prev.extend_from_slice(&input);
        let hash = version.hash(Bytes::from(hash_prev));
        let split: Vec<&[u8]> = input.split_str("\n\n").into_iter().collect();
        if split.len() < 2 {
            return None;
        }
        let header = split.first().map(|x| x.to_str().unwrap_or_default())?.to_string();
        let mut lines = header.lines();
        let mut tree: Option<HashValue> = None;
        let mut parents: Vec<HashValue> = Vec::new();
        let mut author: Option<Signature> = None;
        let mut committer: Option<Signature> = None;
        let mut gpgsig: Option<Gpgsig> = None;
        let mut collecting_gpgsig = false;
        let mut gpgsig_content = String::new();
        
        while let Some(line) = lines.next() {
            if collecting_gpgsig {
                if line.starts_with(" ")
                    || line.starts_with("-")
                    || line.starts_with("=")
                    || line.starts_with("ws")
                    || line.is_empty()
                {
                    gpgsig_content.push_str(line);
                    gpgsig_content.push('\n');
                    continue;
                } else {
                    collecting_gpgsig = false;
                    gpgsig = Some(Gpgsig {
                        signature: gpgsig_content.trim_end().to_string(),
                    });
                    gpgsig_content = String::new();
                }
            }
            
            if line.starts_with("tree ") {
                if let Some(hash_str) = line.strip_prefix("tree ") {
                    if let Some(hash_value) = HashValue::from_str(hash_str) {
                        tree = Some(hash_value);
                    }
                }
            } else if line.starts_with("parent ") {
                if let Some(hash_str) = line.strip_prefix("parent ") {
                    for idx in hash_str.split(' ') {
                        if let Some(hash_value) = HashValue::from_str(idx) {
                            parents.push(hash_value);
                        }
                    }
                }
            } else if line.starts_with("author ") {
                if let Some(author_data) = line.strip_prefix("author ") {
                    if let Ok(sig) = Signature::from_data(format!("author {}", author_data).as_bytes().to_vec()) {
                        author = Some(sig);
                    }
                }
            } else if line.starts_with("committer ") {
                if let Some(committer_data) = line.strip_prefix("committer ") {
                    if let Ok(sig) = Signature::from_data(format!("committer {}", committer_data).as_bytes().to_vec()) {
                        committer = Some(sig);
                    }
                }
            } else if line.starts_with("gpgsig ") {
                collecting_gpgsig = true;
                if let Some(sig_data) = line.strip_prefix("gpgsig ") {
                    gpgsig_content.push_str("gpgsig ");
                    gpgsig_content.push_str(sig_data);
                    gpgsig_content.push('\n');
                }
            }
        }
        
        if collecting_gpgsig && !gpgsig_content.is_empty() {
            gpgsig = Some(Gpgsig {
                signature: gpgsig_content.trim_end().to_string(),
            });
        }
        
        let message = if split.len() >= 3 {
            split[2].to_str().unwrap_or_default().to_string()
        } else {
            split[1].to_str().unwrap_or_default().to_string()
        };
        Some(Commit {
            hash,
            message,
            author: author.unwrap_or_default(),
            committer: committer.unwrap_or_default(),
            parents,
            tree,
            gpgsig,
        })
    }
}


impl Display for Commit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "\ncommit {}\n", self.hash.to_string())?;
        if let Some(tree) = &self.tree {
            write!(f, "tree {}\n", tree.to_string())?;
        }
        write!(f, "parent\
         \n")?;
        for parent in &self.parents {
            write!(f, " - {}\n", parent.to_string())?;
        }
        write!(f, "author {}", self.author)?;
        write!(f, "committer {}", self.committer)?;
        if let Some(gpgsig) = &self.gpgsig {
            write!(f, "gpgsig {}\n", gpgsig.signature)?;
        }
        write!(f, "\n{}", self.message)
    }
}

impl Debug for Commit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}
