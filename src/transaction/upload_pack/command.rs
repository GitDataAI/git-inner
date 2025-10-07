use crate::capability::GitCapability;
use crate::error::GitInnerError;
use crate::sha::HashValue;

/// upload-pack 阶段的命令类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadCommandType {
    Want,
    Have,
    Done,
    Shallow,
    Deepen,
    Unknown(String),
}

/// 表示客户端 fetch 阶段发送的命令
#[derive(Debug, Clone)]
pub struct UploadCommand {
    pub cmd_type: UploadCommandType,
    pub hash: Option<HashValue>,
    pub args: Vec<String>,
    pub git_capability: Vec<GitCapability>,
}

impl UploadCommand {
    /// 从 pkt-line 数据解析出 UploadCommand
    pub fn from_pkt_line(line: &[u8]) -> Result<Option<Self>, GitInnerError> {
        if line.len() < 4 {
            return Ok(None);
        }
        let len_str = std::str::from_utf8(&line[0..4])
            .map_err(|_| GitInnerError::ConversionError("Invalid pkt-line length".to_string()))?;
        let _len = u32::from_str_radix(len_str, 16)
            .map_err(|_| GitInnerError::ConversionError("Invalid pkt-line length format".to_string()))?;
        if _len == 0 {
            return Ok(None);
        }

        if line.len() < _len as usize {
            return Ok(None);
        }

        let payload_end = std::cmp::min(_len as usize, line.len());
        let payload = &line[4..payload_end];

        let line_str = std::str::from_utf8(payload)
            .map_err(|_| GitInnerError::ConversionError("Invalid UTF-8 in pkt-line".to_string()))?;

        // 去除可能的换行或NUL结尾
        let trimmed = line_str.trim_end_matches('\n').trim_end_matches('\0').trim();

        if trimmed.is_empty() {
            return Ok(None);
        }

        // 拆分命令部分
        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let cmd_type = match cmd {
            "want" => UploadCommandType::Want,
            "have" => UploadCommandType::Have,
            "done" => UploadCommandType::Done,
            "shallow" => UploadCommandType::Shallow,
            "deepen" => UploadCommandType::Deepen,
            other => UploadCommandType::Unknown(other.to_string()),
        };
        let mut caps = vec![];

        let args: Vec<String> = parts.map(|s| {
            caps.push( GitCapability::from_str(s));
            s.to_string()
        }).collect();
        let hash = if matches!(cmd_type, UploadCommandType::Want | UploadCommandType::Have) {
            if let Some(arg) = args.get(0) {
                Some(HashValue::from_str(arg).ok_or(GitInnerError::InvalidHash)?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Some(Self {
            cmd_type,
            hash,
            args,
            git_capability: caps,
        }))
    }

    pub fn is_want(&self) -> bool {
        matches!(self.cmd_type, UploadCommandType::Want)
    }

    pub fn is_have(&self) -> bool {
        matches!(self.cmd_type, UploadCommandType::Have)
    }

    pub fn is_done(&self) -> bool {
        matches!(self.cmd_type, UploadCommandType::Done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_pkt_line_want() {
        // 长度计算: "want 1111111111111111111111111111111111111111" = 45字节 + 4前缀 = 49(0x31)
        let pkt = b"0031want 1111111111111111111111111111111111111111";
        let cmd = UploadCommand::from_pkt_line(pkt).unwrap().unwrap();
        assert!(cmd.is_want());
        assert_eq!(
            format!("{}", cmd.hash.unwrap()),
            "1111111111111111111111111111111111111111"
        );
    }

    #[test]
    fn test_from_pkt_line_have() {
        // 长度计算同上
        let pkt = b"0031have 2222222222222222222222222222222222222222";
        let cmd = UploadCommand::from_pkt_line(pkt).unwrap().unwrap();
        assert!(cmd.is_have());
        assert_eq!(
            format!("{}", cmd.hash.unwrap()),
            "2222222222222222222222222222222222222222"
        );
    }

    #[test]
    fn test_from_pkt_line_done() {
        // "done" 4字节 + 4前缀 = 8(0x8)
        let pkt = b"0008done";
        let cmd = UploadCommand::from_pkt_line(pkt).unwrap().unwrap();
        assert!(cmd.is_done());
        assert_eq!(cmd.hash, None);
    }

    #[test]
    fn test_from_pkt_line_flush_pkt() {
        let pkt = b"0000";
        let cmd = UploadCommand::from_pkt_line(pkt).unwrap();
        assert!(cmd.is_none());
    }

    #[test]
    fn test_from_pkt_line_invalid_length() {
        // 无效长度前缀
        let pkt = b"xxxxwant 1111111111111111111111111111111111111111";
        let cmd = UploadCommand::from_pkt_line(pkt);
        assert!(cmd.is_err());
    }

    #[test]
    fn test_from_pkt_line_shallow_and_deepen() {
        let pkt = b"0011shallow 3333333333333333333333333333333333333333";
        let cmd = UploadCommand::from_pkt_line(pkt).unwrap().unwrap();
        assert_eq!(cmd.cmd_type, UploadCommandType::Shallow);

        let pkt = b"000bdeepen 5";
        let cmd = UploadCommand::from_pkt_line(pkt).unwrap().unwrap();
        assert_eq!(cmd.cmd_type, UploadCommandType::Deepen);
    }
}
