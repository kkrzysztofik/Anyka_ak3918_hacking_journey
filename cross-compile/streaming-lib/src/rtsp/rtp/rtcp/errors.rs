#![allow(non_local_definitions)]
use crate::bytesio::bytes_errors::BytesReadError;
use crate::bytesio::bytes_errors::BytesWriteError;
use failure::Fail;

#[derive(Debug)]
pub struct RtcpError {
    pub value: RtcpErrorValue,
}

#[derive(Debug, Fail)]
pub enum RtcpErrorValue {
    #[fail(display = "bytes read error: {}", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error: {}", _0)]
    BytesWriteError(BytesWriteError),
}

impl From<BytesReadError> for RtcpError {
    fn from(error: BytesReadError) -> Self {
        RtcpError {
            value: RtcpErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for RtcpError {
    fn from(error: BytesWriteError) -> Self {
        RtcpError {
            value: RtcpErrorValue::BytesWriteError(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::bytes_errors::{BytesReadError, BytesReadErrorValue};
    use crate::bytesio::bytes_errors::{BytesWriteError, BytesWriteErrorValue};

    // ========== RtcpErrorValue Display Tests ==========

    #[test]
    fn test_rtcp_error_value_bytes_read_display() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let err = RtcpErrorValue::BytesReadError(read_err);
        let display = format!("{}", err);
        assert!(display.contains("bytes read error"));
    }

    #[test]
    fn test_rtcp_error_value_bytes_write_display() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err = RtcpErrorValue::BytesWriteError(write_err);
        let display = format!("{}", err);
        assert!(display.contains("bytes write error"));
    }

    // ========== RtcpError From Trait Tests ==========

    #[test]
    fn test_rtcp_error_from_bytes_read_error() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::EmptyStream,
        };
        let err: RtcpError = read_err.into();
        assert!(matches!(err.value, RtcpErrorValue::BytesReadError(_)));
    }

    #[test]
    fn test_rtcp_error_from_bytes_write_error() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::OutofIndex,
        };
        let err: RtcpError = write_err.into();
        assert!(matches!(err.value, RtcpErrorValue::BytesWriteError(_)));
    }

    // ========== Debug Trait Tests ==========

    #[test]
    fn test_rtcp_error_debug() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let err = RtcpError {
            value: RtcpErrorValue::BytesReadError(read_err),
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("RtcpError"));
    }

    #[test]
    fn test_rtcp_error_value_debug() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err = RtcpErrorValue::BytesWriteError(write_err);
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("BytesWriteError"));
    }
}
