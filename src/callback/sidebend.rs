

#[derive(Clone,Debug)]
#[derive(PartialEq)]
pub enum SideBend {
    SidebandFlush = 0,
    SidebandPrimary = 1,
    SidebandMessage = 2,
    SidebandRemoteError = 3,
}

impl SideBend {
    pub fn to_u32(&self) -> u32 {
        match self {
            SideBend::SidebandRemoteError => 3,
            SideBend::SidebandFlush => 0,
            SideBend::SidebandPrimary => 1,
            SideBend::SidebandMessage => 2,
        }
    }
    pub fn from_u32(u: u32) -> Option<SideBend> {
        match u {
            3 => Some(SideBend::SidebandRemoteError),
            0 => Some(SideBend::SidebandFlush),
            1 => Some(SideBend::SidebandPrimary),
            2 => Some(SideBend::SidebandMessage),
            _ => None,
        }
    }
}

/// 我不知道也不理解为什么要这样设计，也不清楚原理，但是逆向抓包 git-http-backend 发现他是这样返回的
///
/// build for git for windows 1.51.0.2
pub fn bend_pkt_flush() -> Vec<u8> {
    let basic = "00000000".as_bytes();
    let mut bend = vec![1];
    bend.extend_from_slice(basic);
    let len = bend.len();
    let head = format!("{:04x}", len).into_bytes();
    head.into_iter().chain(bend.into_iter()).collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebend_to_u32() {
        assert_eq!(SideBend::SidebandRemoteError.to_u32(), 3);
        assert_eq!(SideBend::SidebandFlush.to_u32(), 0);
        assert_eq!(SideBend::SidebandPrimary.to_u32(), 1);
        assert_eq!(SideBend::SidebandMessage.to_u32(), 2);
    }

    #[test]
    fn test_sidebend_from_u32_valid() {
        assert_eq!(SideBend::from_u32(3), Some(SideBend::SidebandRemoteError));
        assert_eq!(SideBend::from_u32(0), Some(SideBend::SidebandFlush));
        assert_eq!(SideBend::from_u32(1), Some(SideBend::SidebandPrimary));
        assert_eq!(SideBend::from_u32(2), Some(SideBend::SidebandMessage));
    }

    #[test]
    fn test_sidebend_from_u32_invalid() {
        assert_eq!(SideBend::from_u32(4), None);
        assert_eq!(SideBend::from_u32(5), None);
        assert_eq!(SideBend::from_u32(u32::MAX), None);
    }

    #[test]
    fn test_sidebend_clone() {
        let original = SideBend::SidebandPrimary;
        let cloned = original.clone();
        assert_eq!(original.to_u32(), cloned.to_u32());
    }

    #[test]
    fn test_sidebend_round_trip() {
        let variants = [
            SideBend::SidebandFlush,
            SideBend::SidebandPrimary,
            SideBend::SidebandMessage,
            SideBend::SidebandRemoteError,
        ];

        for variant in variants {
            let value = variant.to_u32();
            let reconstructed = SideBend::from_u32(value);
            assert_eq!(reconstructed, Some(variant));
        }
    }
}
