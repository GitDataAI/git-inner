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

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
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

#[derive(Debug)]
pub enum ParseError {}

impl Commit {
    pub fn parse(input: Bytes, version: HashVersion) -> Result<Commit, GitInnerError> {
        // --- 先按原始 bytes 计算 hash（不要因为解析替换 CRLF 而影响哈希） ---
        let mut hash_prev = Vec::new();
        hash_prev.extend_from_slice(format!("commit {}\0", input.len()).as_bytes());
        hash_prev.extend_from_slice(&input);
        let hash = version.hash(Bytes::from(hash_prev));
        if hash.to_string() == "89830fdb21a8b52d53a8ed1e6d47fa452fbe35af" {
            println!("{:?}", input);
        }
        // --- 把 input 转为 &str，然后为解析做行结束正规化（仅解析用） ---
        let input_str = std::str::from_utf8(&input).map_err(|_| GitInnerError::InvalidUtf8)?;
        // Normalize CRLF -> LF to avoid Windows line ending issues during parsing.
        let normalized = if input_str.contains("\r\n") {
            input_str.replace("\r\n", "\n")
        } else {
            input_str.to_string()
        };

        // --- 定位 header/message 边界（找第一个连续的两个换行） ---
        let header_end_pos = normalized.find("\n\n").unwrap_or(normalized.len());
        let header = &normalized[..header_end_pos];
        let message = if header_end_pos == normalized.len() {
            ""
        } else {
            // 跳过两个换行符
            &normalized[header_end_pos + 2..]
        };

        // --- 解析 header ---
        let mut tree: Option<HashValue> = None;
        let mut parents: Vec<HashValue> = Vec::new();
        let mut author: Option<Signature> = None;
        let mut committer: Option<Signature> = None;
        let mut gpgsig: Option<String> = None;

        let mut collecting_gpgsig = false;
        let mut gpgsig_lines: Vec<&str> = Vec::new();

        for line in header.split('\n') {
            if collecting_gpgsig {
                // 保留原始行（包括可能的前导空格）
                gpgsig_lines.push(line);
                // 结束标志可能带前导空格 -> 用 trim_start() 比较
                if line.trim_start() == "-----END PGP SIGNATURE-----" {
                    collecting_gpgsig = false;
                    // 将收集到的行以 '\n' 拼回成一个字符串（不额外添加/去掉前导空格）
                    let sig = gpgsig_lines.join("\n");
                    gpgsig = Some(sig);
                    gpgsig_lines.clear();
                }
                continue;
            }

            if line.starts_with("gpgsig ") {
                // gpgsig header 行（后续行为 continuation，通常以单空格开头）
                collecting_gpgsig = true;
                gpgsig_lines.push(line);
                continue;
            }

            // 普通 header 字段解析
            if let Some(rest) = line.strip_prefix("tree ") {
                tree = HashValue::from_str(rest.trim());
            } else if let Some(rest) = line.strip_prefix("parent ") {
                if let Some(parent_hash) = HashValue::from_str(rest.trim()) {
                    parents.push(parent_hash);
                }
            } else if let Some(rest) = line.strip_prefix("author ") {
                author = Some(
                    Signature::from_data(format!("author {}", rest.trim()).as_bytes().to_vec())
                        .map_err(|_| GitInnerError::MissingAuthor)?,
                );
            } else if let Some(rest) = line.strip_prefix("committer ") {
                committer = Some(
                    Signature::from_data(format!("committer {}", rest.trim()).as_bytes().to_vec())
                        .map_err(|_| GitInnerError::MissingCommitter)?,
                );
            } else {
                // 忽略其它 header 行（capability 等）
            }
        }

        Ok(Commit {
            hash,
            message: message.to_string(),
            author: author.ok_or(GitInnerError::MissingAuthor)?,
            committer: committer.ok_or(GitInnerError::MissingCommitter)?,
            parents,
            tree,
            gpgsig: gpgsig.map(|s| Gpgsig { signature: s }),
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
            let mut parts = gpgsig.signature.split('\n');
            if let Some(first) = parts.next() {
                writeln!(f, "{}", first)?;
            }
            for line in parts {
                writeln!(f, "{}", line)?;
            }
            write!(f," \n")?;
        }
        writeln!(f)?;
        write!(f, "{}", self.message)
    }
}

impl Debug for Commit {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha::HashVersion;

    #[test]
    fn test_commit_parse_basic() {
        let commit_data = Bytes::from(
            "tree 7551d4da2e9c1ae9397c47709253b405fb6b6206\n\
             parent ee98d64f596ae42fadf9eeae1d0efa22b14b0829\n\
             author ZhenYi <434836402@qq.com> 1740189120 +0800\n\
             committer ZhenYi <434836402@qq.com> 1740189120 +0800\n\n\
             build(deps): Update dependencies and replace poem with actix-web\n"
        );

        let commit = Commit::parse(commit_data, HashVersion::Sha1).unwrap();

        assert_eq!(commit.tree.as_ref().unwrap().to_string(), "7551d4da2e9c1ae9397c47709253b405fb6b6206");
        assert_eq!(commit.parents.len(), 1);
        assert_eq!(commit.parents[0].to_string(), "ee98d64f596ae42fadf9eeae1d0efa22b14b0829");
        assert_eq!(commit.message, "build(deps): Update dependencies and replace poem with actix-web\n");
    }

    #[test]
    fn test_commit_parse_with_gpg_signature() {
        let commit_data = Bytes::from(br#"
tree 6dc1b8e401ddab32b91a5ea7979affb3fc92d2f8
parent f1d872891b5a6672183ebd6936dfce09c60d2061
author taoshengshi <taoshengshi01@gmail.com> 1751768083 +0800
committer GitHub <noreply@github.com> 1751768083 +0800
gpgsig -----BEGIN PGP SIGNATURE-----

 wsFcBAABCAAQBQJoadwTCRC1aQ7uu5UhlAAAmlAQAFIAa5RsJDtfVErjph7rGFtD
 xZH2XZll/9aETfLK1qwYouSvBoA1r/EXwrpn4IbZCYGKPgqN4opCNahgB+4j8KCY
 j7vr/h6GapZ+aZMjCIR30JABnXYD8ZQ5VIJA6sPSBSg45Cmtrt50mTsGnHPrub0B
 DAV1SRYiVNkn+vUgaHWs5qZCXLPhogrO5wpVCWQ0D2lnpCQlpBYZQlgYfqXZVUOd
 7tlae983QBSfYJauSqUSWXQOFr/02Qfqi9tEb7MsAWSX4llrCqYXENsq0tSw+AvD
 6AMLdyYweYWAJIvA0YUtFfHgTzyCq5HmaLv2X2vkHZJX+irOPecCbqVuu98ZOgqO
 wlftAd6tZwRa+wTX2amVKrNMsw5wZd3MyEttXuXMJ5TLRuH7WFJXTwlSROjt5Yf1
 AElNnQWe94x325a6qNi0ygKbf7+lKSrSz1e8j6Hl9B9TlES4Imn70jkFGmNubahB
 CEHwsRRlEBqqhOP0wxOZAJkia2ajyZb7v+CNOLSdDAcYkrWG25uDbQk5Fosuno6G
 k9YA3aw1Ao8bIitSsWQ9Ho7Q0sYAuf/eqr7Cw9CR2y4WEiZlrgHh87Nl1s35DBsM
 FwpWdHuWAw51MjUXHqfIE/WsjXs8+Uz7vL/II8tib+gS0MdtHn/ZRfC9x65EqO58
 dIa9HlBV1GFfsaKmaovm
 =b5jG
 -----END PGP SIGNATURE-----


Feat/doc (#189)

* add logo picture and chagne jzfs joint management picture

* change logo file name of dark mode

* change size of picture

* change alignment of picture

* change picture alignment

* update image
"#.to_vec());

        let commit = match Commit::parse(commit_data, HashVersion::Sha1) {
            Ok(c) => c,
            Err(e) => {
                println!("Failed to parse commit: {:?}", e);
                return;
            }
        };
        dbg!(commit.hash.to_string());
    }

    #[test]
    fn test_commit_parse_multiple_parents() {
        let commit_data = Bytes::from(
            "tree abcdef1234567890abcdef1234567890abcdef12\n\
             parent 1111111111111111111111111111111111111111\n\
             parent 2222222222222222222222222222222222222222\n\
             author Test <test@example.com> 1740189120 +0800\n\
             committer Test <test@example.com> 1740189120 +0800\n\n\
             Merge branch 'main'\n"
        );

        let commit = Commit::parse(commit_data, HashVersion::Sha1).unwrap();

        assert_eq!(commit.parents.len(), 2);
        assert_eq!(commit.parents[0].to_string(), "1111111111111111111111111111111111111111");
        assert_eq!(commit.parents[1].to_string(), "2222222222222222222222222222222222222222");
    }

    #[test]
    fn test_commit_display() {
        let commit_data = Bytes::from(
            "tree 7551d4da2e9c1ae9397c47709253b405fb6b6206\n\
             parent ee98d64f596ae42fadf9eeae1d0efa22b14b0829\n\
             author ZhenYi <434836402@qq.com> 1740189120 +0800\n\
             committer ZhenYi <434836402@qq.com> 1740189120 +0800\n\n\
             build(deps): Update dependencies and replace poem with actix-web\n"
        );

        let commit = Commit::parse(commit_data.clone(), HashVersion::Sha1).unwrap();
        let displayed = commit.to_string();

        assert!(displayed.starts_with("tree 7551d4da2e9c1ae9397c47709253b405fb6b6206"));
        assert!(displayed.contains("parent ee98d64f596ae42fadf9eeae1d0efa22b14b0829"));
        assert!(displayed.ends_with("build(deps): Update dependencies and replace poem with actix-web\n"));
    }

    #[test]
    fn test_commit_object_trait() {
        let commit_data = Bytes::from(
            "tree 7551d4da2e9c1ae9397c47709253b405fb6b6206\n\
             parent ee98d64f596ae42fadf9eeae1d0efa22b14b0829\n\
             author ZhenYi <434836402@qq.com> 1740189120 +0800\n\
             committer ZhenYi <434836402@qq.com> 1740189120 +0800\n\n\
             build(deps): Update dependencies and replace poem with actix-web\n"
        );

        let commit = Commit::parse(commit_data.clone(), HashVersion::Sha1).unwrap();

        assert_eq!(commit.get_type(), ObjectType::Commit);
        assert_eq!(commit.get_data(), Bytes::from(commit.to_string()));
    }

    #[test]
    fn test_commit_parse_error_cases() {
        // 测试缺少author的错误情况
        let invalid_commit_data = Bytes::from(
            "tree 7551d4da2e9c1ae9397c47709253b405fb6b6206\n\
             committer ZhenYi <434836402@qq.com> 1740189120 +0800\n\n\
             test message\n"
        );

        let result = Commit::parse(invalid_commit_data, HashVersion::Sha1);
        assert!(matches!(result, Err(GitInnerError::MissingAuthor)));

        // 测试缺少committer的错误情况
        let invalid_commit_data2 = Bytes::from(
            "tree 7551d4da2e9c1ae9397c47709253b405fb6b6206\n\
             author ZhenYi <434836402@qq.com> 1740189120 +0800\n\n\
             test message\n"
        );

        let result2 = Commit::parse(invalid_commit_data2, HashVersion::Sha1);
        assert!(matches!(result2, Err(GitInnerError::MissingCommitter)));
    }
}
