use std::time::SystemTimeError;

#[derive(Debug)]
pub enum LineProtocolError {
    Error,
    FailedToGetSystemTime,
}

impl From<SystemTimeError> for LineProtocolError {
    fn from(_error: SystemTimeError) -> Self {
        LineProtocolError::FailedToGetSystemTime
    }
}
