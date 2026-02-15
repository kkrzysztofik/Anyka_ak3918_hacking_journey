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

    // ========== InvalidPacketLoss Display Tests ==========

    #[test]
    fn test_rtcp_error_value_invalid_packet_loss_display() {
        let err = RtcpErrorValue::InvalidPacketLoss { value: 16777216 };
        let display = format!("{}", err);
        assert!(display.contains("cumulative packets lost exceeds 24-bit range"));
        assert!(display.contains("16777216"));
    }

    #[test]
    fn test_rtcp_error_value_invalid_packet_loss_zero() {
        let err = RtcpErrorValue::InvalidPacketLoss { value: 0 };
        let display = format!("{}", err);
        assert!(display.contains("0"));
    }

    #[test]
    fn test_rtcp_error_value_invalid_packet_loss_max_24bit() {
        let err = RtcpErrorValue::InvalidPacketLoss { value: 0x00FF_FFFF };
        let display = format!("{}", err);
        assert!(display.contains("16777215"));
    }

    // ========== InvalidAppLength Display Tests ==========

    #[test]
    fn test_rtcp_error_value_invalid_app_length_display() {
        let err = RtcpErrorValue::InvalidAppLength { length: 1 };
        let display = format!("{}", err);
        assert!(display.contains("invalid RTCP APP length"));
        assert!(display.contains("1"));
    }

    #[test]
    fn test_rtcp_error_value_invalid_app_length_zero() {
        let err = RtcpErrorValue::InvalidAppLength { length: 0 };
        let display = format!("{}", err);
        assert!(display.contains("0"));
    }

    // ========== From<RtcpErrorValue> for RtcpError Tests ==========

    #[test]
    fn test_rtcp_error_from_error_value_invalid_packet_loss() {
        let val = RtcpErrorValue::InvalidPacketLoss { value: 999 };
        let err: RtcpError = val.into();
        assert!(matches!(
            err.value,
            RtcpErrorValue::InvalidPacketLoss { value: 999 }
        ));
    }

    #[test]
    fn test_rtcp_error_from_error_value_invalid_app_length() {
        let val = RtcpErrorValue::InvalidAppLength { length: 42 };
        let err: RtcpError = val.into();
        assert!(matches!(
            err.value,
            RtcpErrorValue::InvalidAppLength { length: 42 }
        ));
    }

    #[test]
    fn test_rtcp_error_display_invalid_packet_loss() {
        let err = RtcpError {
            value: RtcpErrorValue::InvalidPacketLoss { value: 500 },
        };
        let display = format!("{}", err);
        assert!(display.contains("500"));
    }

    #[test]
    fn test_rtcp_error_display_invalid_app_length() {
        let err = RtcpError {
            value: RtcpErrorValue::InvalidAppLength { length: 3 },
        };
        let display = format!("{}", err);
        assert!(display.contains("3"));
    }

    #[test]
    fn test_rtcp_error_debug_invalid_packet_loss() {
        let err = RtcpError {
            value: RtcpErrorValue::InvalidPacketLoss { value: 123 },
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidPacketLoss"));
    }

    #[test]
    fn test_rtcp_error_debug_invalid_app_length() {
        let err = RtcpError {
            value: RtcpErrorValue::InvalidAppLength { length: 7 },
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidAppLength"));
    }
}
