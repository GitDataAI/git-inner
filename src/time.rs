use crate::error::GitInnerError;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize, Serialize, Clone, Debug, Copy, Eq, PartialEq)]
pub struct Time {
    pub seconds: u32,
    pub nanos: u32,
}

impl Time {
    pub fn new(seconds: u32, nanos: u32) -> Self {
        Time { seconds, nanos }
    }
    pub fn to_system_time(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + std::time::Duration::new(self.seconds as u64, self.nanos)
    }
    pub fn from_system_time(system_time: SystemTime) -> Result<Self, GitInnerError> {
        match system_time.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let seconds = duration
                    .as_secs()
                    .try_into()
                    .map_err(|_| GitInnerError::TimeError("Time is too large".to_string()))?;
                let nanos = duration.subsec_nanos();
                Ok(Time { seconds, nanos })
            }
            Err(_) => Err(GitInnerError::TimeError(
                "Time is before the UNIX epoch".to_string(),
            )),
        }
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.seconds, self.nanos)
    }
}
