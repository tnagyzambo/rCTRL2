//! Errors that can be encountered creating valid influx line protocol.

use std::time::SystemTimeError;

/// Errors that can be encountered creating valid influx line protocol.
// TODO: impl Display
#[derive(Debug)]
pub enum LineProtocolError {
    /// General Error
    // TODO: Get rid of this
    Error,

    /// Error geting current time for line protocol timestamp
    FailedToGetSystemTime,
}

impl From<SystemTimeError> for LineProtocolError {
    fn from(_error: SystemTimeError) -> Self {
        // TODO: Make this more informative
        LineProtocolError::FailedToGetSystemTime
    }
}
