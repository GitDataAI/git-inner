/// Git 协议能力枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GitCapability {
    /// 多 ACK 支持
    MultiAck,
    /// 多 ACK 详细模式
    MultiAckDetailed,
    /// 无完成支持
    NoDone,
    /// 瘦包支持
    ThinPack,
    /// 侧带支持（用于进度信息）
    SideBand,
    /// 侧带 64k 支持
    SideBand64k,
    /// OFS Delta 支持
    OfsDelta,
    /// 浅克隆支持
    Shallow,
    /// 延迟反馈支持
    DeferredFetch,
    /// 无进度支持
    NoProgress,
    /// 包含标签支持
    IncludeTag,
    /// 报告状态支持
    ReportStatus,
    /// 删除引用支持
    DeleteRefs,
    /// 静默模式
    Quiet,
    /// 原子推送支持
    Atomic,
    /// 推送选项支持
    PushOptions,
    /// Agent 信息
    Agent(String),
    /// 对象格式
    ObjectFormat(String),
    /// 符号引用
    Symref(String, String),
    /// 其他未知能力
    Other(String),
}

impl GitCapability {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match std::str::from_utf8(bytes) {
            Ok(s) => Some(GitCapability::from_str(s)),
            Err(_) => None,
        }
    }
    /// 从字符串解析能力
    pub fn from_str(s: &str) -> Self {
        match s {
            "multi_ack" => Self::MultiAck,
            "multi_ack_detailed" => Self::MultiAckDetailed,
            "no-done" => Self::NoDone,
            "thin-pack" => Self::ThinPack,
            "side-band" => Self::SideBand,
            "side-band-64k" => Self::SideBand64k,
            "ofs-delta" => Self::OfsDelta,
            "shallow" => Self::Shallow,
            "deferred-fetch" => Self::DeferredFetch,
            "no-progress" => Self::NoProgress,
            "include-tag" => Self::IncludeTag,
            "report-status" => Self::ReportStatus,
            "delete-refs" => Self::DeleteRefs,
            "quiet" => Self::Quiet,
            "atomic" => Self::Atomic,
            "push-options" => Self::PushOptions,
            _ => {
                if let Some(agent) = s.strip_prefix("agent=") {
                    Self::Agent(agent.to_string())
                } else if let Some(format) = s.strip_prefix("object-format=") {
                    Self::ObjectFormat(format.to_string())
                } else if let Some(symref) = s.strip_prefix("symref=") {
                    if let Some((from, to)) = symref.split_once(':') {
                        Self::Symref(from.to_string(), to.to_string())
                    } else {
                        Self::Other(s.to_string())
                    }
                } else {
                    Self::Other(s.to_string())
                }
            }
        }
    }

    /// 转换为字符串表示
    pub fn to_string(&self) -> String {
        match self {
            Self::MultiAck => "multi_ack".to_string(),
            Self::MultiAckDetailed => "multi_ack_detailed".to_string(),
            Self::NoDone => "no-done".to_string(),
            Self::ThinPack => "thin-pack".to_string(),
            Self::SideBand => "side-band".to_string(),
            Self::SideBand64k => "side-band-64k".to_string(),
            Self::OfsDelta => "ofs-delta".to_string(),
            Self::Shallow => "shallow".to_string(),
            Self::DeferredFetch => "deferred-fetch".to_string(),
            Self::NoProgress => "no-progress".to_string(),
            Self::IncludeTag => "include-tag".to_string(),
            Self::ReportStatus => "report-status".to_string(),
            Self::DeleteRefs => "delete-refs".to_string(),
            Self::Quiet => "quiet".to_string(),
            Self::Atomic => "atomic".to_string(),
            Self::PushOptions => "push-options".to_string(),
            Self::Agent(agent) => format!("agent={}", agent),
            Self::ObjectFormat(format) => format!("object-format={}", format),
            Self::Symref(from, to) => format!("symref={}:{}", from, to),
            Self::Other(s) => s.clone(),
        }
    }

    pub fn basic() -> Vec<GitCapability> {
        vec![
            GitCapability::SideBand,
            GitCapability::SideBand64k,
            GitCapability::Agent("git-inner".to_string()),
            GitCapability::ReportStatus,
        ]
    }

    pub fn upload() -> Vec<GitCapability> {
        let mut capabilities = Self::basic();
        capabilities.extend(vec![
            // GitCapability::OfsDelta,
            GitCapability::MultiAck,
            GitCapability::MultiAckDetailed,
            GitCapability::ThinPack,
            GitCapability::NoDone,
            GitCapability::IncludeTag,
            GitCapability::Shallow,
        ]);
        capabilities
    }

    pub fn receive() -> Vec<GitCapability> {
        let mut capabilities = Self::basic();
        capabilities.extend(vec![
            // GitCapability::OfsDelta,
            GitCapability::Atomic,
            GitCapability::PushOptions,
            GitCapability::DeleteRefs,
        ]);
        capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_capabilities() {
        assert_eq!(
            GitCapability::from_str("multi_ack"),
            GitCapability::MultiAck
        );
        assert_eq!(
            GitCapability::from_str("thin-pack"),
            GitCapability::ThinPack
        );
        assert_eq!(GitCapability::from_str("atomic"), GitCapability::Atomic);
    }

    #[test]
    fn test_parse_agent() {
        let cap = GitCapability::from_str("agent=git/2.40.0");
        assert_eq!(cap, GitCapability::Agent("git/2.40.0".to_string()));
    }

    #[test]
    fn test_parse_symref() {
        let cap = GitCapability::from_str("symref=HEAD:refs/heads/main");
        assert_eq!(
            cap,
            GitCapability::Symref("HEAD".to_string(), "refs/heads/main".to_string())
        );
    }

    #[test]
    fn test_to_string() {
        assert_eq!(GitCapability::MultiAck.to_string(), "multi_ack");
        assert_eq!(
            GitCapability::Agent("git/2.40.0".to_string()).to_string(),
            "agent=git/2.40.0"
        );
    }
}
