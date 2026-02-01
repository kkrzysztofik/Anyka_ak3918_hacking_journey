use crate::bytesio::bytes_errors::BytesReadError;
use crate::bytesio::bytes_errors::BytesWriteError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{value}")]
pub struct RtcpError {
    pub value: RtcpErrorValue,
}

#[derive(Debug, Error)]
pub enum RtcpErrorValue {
    #[error(transparent)]
    BytesReadError(#[from] BytesReadError),
    #[error(transparent)]
    BytesWriteError(#[from] BytesWriteError),
    #[error("cumulative packets lost exceeds 24-bit range: {value}")]
    InvalidPacketLoss { value: u32 },
    #[error("invalid RTCP APP length: {length}")]
    InvalidAppLength { length: u16 },
}

impl From<RtcpErrorValue> for RtcpError {
    fn from(value: RtcpErrorValue) -> Self {
        RtcpError { value }
    }
}

impl From<BytesReadError> for RtcpError {
    fn from(err: BytesReadError) -> Self {
        RtcpError {
            value: RtcpErrorValue::BytesReadError(err),
        }
    }
}

impl From<BytesWriteError> for RtcpError {
    fn from(err: BytesWriteError) -> Self {
        RtcpError {
            value: RtcpErrorValue::BytesWriteError(err),
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
        // transparent errors forward to underlying error display
        assert!(display.contains("not enough bytes"));
    }

    #[test]
    fn test_rtcp_error_value_bytes_write_display() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err = RtcpErrorValue::BytesWriteError(write_err);
        let display = format!("{}", err);
        // transparent errors forward to underlying error display
        assert!(display.contains("write time out"));
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
