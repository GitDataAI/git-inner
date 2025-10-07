use crate::error::GitInnerError;
use crate::sha::HashValue;
use crate::sha::HashVersion;

#[derive(Debug, Clone)]
pub struct ReceiveCommand {
    pub old: HashValue,
    pub new: HashValue,
    pub ref_name: String,
}

impl ReceiveCommand {
    pub fn is_delete(&self) -> bool {
        self.new.is_zero()
    }
    pub fn is_update(&self) -> bool {
        !self.is_delete()
    }
    pub fn is_create(&self) -> bool {
        self.old.is_zero()
    }
    pub fn from_pkt_line(line: &[u8]) -> Result<Option<Self>, GitInnerError> {
        if line.len() < 4 {
            return Ok(None);
        }

        let len_str = std::str::from_utf8(&line[0..4])
            .map_err(|_| GitInnerError::ConversionError("Invalid pkt-line length".to_string()))?;
        let _len = u32::from_str_radix(len_str, 16).map_err(|_| {
            GitInnerError::ConversionError("Invalid pkt-line length format".to_string())
        })?;
        if _len == 0 {
            return Ok(None);
        }
        if line.len() < _len as usize {
            return Ok(None);
        }

        let line_str = std::str::from_utf8(&line[4.._len as usize])
            .map_err(|_| GitInnerError::ConversionError("Invalid UTF-8 in pkt-line".to_string()))?;
        let parts: Vec<&str> = line_str.trim().split(' ').collect();

        if parts.len() < 3 {
            return Ok(None);
        }

        let old_sha = parts[0];
        let new_sha = parts[1];
        let ref_name = parts[2];

        let old_hash = if old_sha.chars().all(|x| x == '0') {
            HashVersion::Sha1.default()
        } else {
            HashValue::from_str(old_sha).ok_or_else(|| {
                eprintln!("Failed to parse old SHA: {}", old_sha);
                GitInnerError::InvalidSha1String
            })?
        };

        let new_hash = if new_sha.chars().all(|x| x == '0') {
            HashVersion::Sha1.default()
        } else {
            HashValue::from_str(new_sha).ok_or_else(|| {
                eprintln!("Failed to parse new SHA: {}", new_sha);
                GitInnerError::InvalidSha1String
            })?
        };

        Ok(Some(ReceiveCommand {
            old: old_hash,
            new: new_hash,
            ref_name: ref_name.to_string().replace("\0", ""),
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::transaction::receive::command::ReceiveCommand;
    #[test]
    fn test_from_pkt_line_create_command() {
        let pkt_line = b"006b0000000000000000000000000000000000000000 cdfdb42577e2506715f8cfeacdbabc092bf63e8d refs/heads/experiment";
        let full_pkt_line = pkt_line.to_vec();

        let result = ReceiveCommand::from_pkt_line(&full_pkt_line);
        assert!(result.is_ok());
        let command = result.unwrap();
        assert!(command.is_some());
        let command = command.unwrap();

        assert!(command.is_create());
        assert!(!command.is_delete());
        assert!(command.is_update());
        assert_eq!(command.ref_name, "refs/heads/experiment");
        assert_eq!(
            format!("{}", command.old),
            "0000000000000000000000000000000000000000"
        );
        assert_eq!(
            format!("{}", command.new),
            "cdfdb42577e2506715f8cfeacdbabc092bf63e8d"
        );
    }

    #[test]
    fn test_from_pkt_line_update_command() {
        let pkt_line = b"0067ca82a6dff817ec66f44342007202690a93763949 15027957951b64cf874c3557a0f3547bd83b3ff6 refs/heads/master";
        let full_pkt_line = pkt_line.to_vec();

        let result = ReceiveCommand::from_pkt_line(&full_pkt_line);
        assert!(result.is_ok());
        let command = result.unwrap();
        assert!(command.is_some());
        let command = command.unwrap();

        assert!(!command.is_create());
        assert!(!command.is_delete());
        assert!(command.is_update());
        assert_eq!(command.ref_name, "refs/heads/master");
        assert_eq!(
            format!("{}", command.old),
            "ca82a6dff817ec66f44342007202690a93763949"
        );
        assert_eq!(
            format!("{}", command.new),
            "15027957951b64cf874c3557a0f3547bd83b3ff6"
        );
    }

    #[test]
    fn test_from_pkt_line_delete_command() {
        let pkt_line = b"006b15027957951b64cf874c3557a0f3547bd83b3ff6 0000000000000000000000000000000000000000 refs/heads/experiment";
        let full_pkt_line = pkt_line.to_vec();

        let result = ReceiveCommand::from_pkt_line(&full_pkt_line);
        assert!(result.is_ok());
        let command = result.unwrap();
        assert!(command.is_some());
        let command = command.unwrap();

        assert!(!command.is_create());
        assert!(command.is_delete());
        assert!(!command.is_update());
        assert_eq!(command.ref_name, "refs/heads/experiment");
        assert_eq!(
            format!("{}", command.old),
            "15027957951b64cf874c3557a0f3547bd83b3ff6"
        );
        assert_eq!(
            format!("{}", command.new),
            "0000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn test_from_pkt_line_flush_packet() {
        let flush_pkt = b"0000";

        let result = ReceiveCommand::from_pkt_line(flush_pkt);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_from_pkt_line() {
        let invalid_pkt = b"00a50000000000000000000000000000000000000000 56d999ae43df4c597dc240b39a77f64a5d8efbb4 refs/heads/main";

        let result = ReceiveCommand::from_pkt_line(invalid_pkt);
        dbg!(&result);
    }

    #[test]
    fn test_from_pkt_line_invalid_hex_length() {
        let invalid_pkt = b"xyzw0000000000000000000000000000000000000000 cdfdb42577e2506715f8cfeacdbabc092bf63e8d refs/heads/experiment";

        let result = ReceiveCommand::from_pkt_line(invalid_pkt);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_pkt_line_invalid_data_format() {
        let invalid_pkt = b"0032only_one_part";

        let result = ReceiveCommand::from_pkt_line(invalid_pkt);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
