#![allow(non_local_definitions)] // Suppression for non_local_definitions required by thiserror proc-macros
use super::bytesio_errors::BytesIOError;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BytesReadErrorValue {
    #[error("not enough bytes to read")]
    NotEnoughBytes,
    #[error("empty stream")]
    EmptyStream,
    #[error("io error: {}", _0)]
    IO(#[from] io::Error),
    #[error("index out of range")]
    IndexOutofRange,
    #[error("bytesio read error: {}", _0)]
    BytesIOError(BytesIOError),
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct BytesReadError {
    pub value: BytesReadErrorValue,
}

impl From<BytesReadErrorValue> for BytesReadError {
    fn from(val: BytesReadErrorValue) -> Self {
        BytesReadError { value: val }
    }
}

impl From<io::Error> for BytesReadError {
    fn from(error: io::Error) -> Self {
        BytesReadError {
            value: BytesReadErrorValue::IO(error),
        }
    }
}

impl From<BytesIOError> for BytesReadError {
    fn from(error: BytesIOError) -> Self {
        BytesReadError {
            value: BytesReadErrorValue::BytesIOError(error),
        }
    }
}

#[derive(Debug, Error)]
pub enum BytesWriteErrorValue {
    #[error("io error")]
    IO(#[from] io::Error),
    #[error("bytes io error: {}", _0)]
    BytesIOError(BytesIOError),
    #[error("write time out")]
    Timeout,
    #[error("outof index")]
    OutofIndex,
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct BytesWriteError {
    pub value: BytesWriteErrorValue,
}

impl From<io::Error> for BytesWriteError {
    fn from(error: io::Error) -> Self {
        BytesWriteError {
            value: BytesWriteErrorValue::IO(error),
        }
    }
}

impl From<BytesIOError> for BytesWriteError {
    fn from(error: BytesIOError) -> Self {
        BytesWriteError {
            value: BytesWriteErrorValue::BytesIOError(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error as IoError, ErrorKind};

    // ========== BytesReadErrorValue Display Tests ==========

    #[test]
    fn test_bytes_read_error_value_not_enough_bytes_display() {
        let err = BytesReadErrorValue::NotEnoughBytes;
        assert_eq!(format!("{}", err), "not enough bytes to read");
    }

    #[test]
    fn test_bytes_read_error_value_empty_stream_display() {
        let err = BytesReadErrorValue::EmptyStream;
        assert_eq!(format!("{}", err), "empty stream");
    }

    #[test]
    fn test_bytes_read_error_value_index_out_of_range_display() {
        let err = BytesReadErrorValue::IndexOutofRange;
        assert_eq!(format!("{}", err), "index out of range");
    }

    #[test]
    fn test_bytes_read_error_value_io_display() {
        let io_err = IoError::new(ErrorKind::NotFound, "file not found");
        let err = BytesReadErrorValue::IO(io_err);
        assert!(format!("{}", err).contains("io error"));
    }

    // ========== BytesReadError From Trait Tests ==========

    #[test]
    fn test_bytes_read_error_from_value() {
        let value = BytesReadErrorValue::NotEnoughBytes;
        let err: BytesReadError = value.into();
        assert!(matches!(err.value, BytesReadErrorValue::NotEnoughBytes));
    }

    #[test]
    fn test_bytes_read_error_from_io_error() {
        let io_err = IoError::new(ErrorKind::PermissionDenied, "access denied");
        let err: BytesReadError = io_err.into();
        assert!(matches!(err.value, BytesReadErrorValue::IO(_)));
    }

    #[test]
    fn test_bytes_read_error_from_bytesio_error() {
        use super::super::bytesio_errors::{BytesIOError, BytesIOErrorValue};
        let bytesio_err = BytesIOError {
            value: BytesIOErrorValue::NotEnoughBytes,
        };
        let err: BytesReadError = bytesio_err.into();
        assert!(matches!(err.value, BytesReadErrorValue::BytesIOError(_)));
    }

    // ========== BytesReadError Display Tests ==========

    #[test]
    fn test_bytes_read_error_display_not_enough_bytes() {
        let err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        assert_eq!(format!("{}", err), "not enough bytes to read");
    }

    #[test]
    fn test_bytes_read_error_display_empty_stream() {
        let err = BytesReadError {
            value: BytesReadErrorValue::EmptyStream,
        };
        assert_eq!(format!("{}", err), "empty stream");
    }

    // ========== BytesWriteErrorValue Display Tests ==========

    #[test]
    fn test_bytes_write_error_value_io_display() {
        let io_err = IoError::new(ErrorKind::WriteZero, "write zero");
        let err = BytesWriteErrorValue::IO(io_err);
        assert_eq!(format!("{}", err), "io error");
    }

    #[test]
    fn test_bytes_write_error_value_timeout_display() {
        let err = BytesWriteErrorValue::Timeout;
        assert_eq!(format!("{}", err), "write time out");
    }

    #[test]
    fn test_bytes_write_error_value_outof_index_display() {
        let err = BytesWriteErrorValue::OutofIndex;
        assert_eq!(format!("{}", err), "outof index");
    }

    // ========== BytesWriteError From Trait Tests ==========

    #[test]
    fn test_bytes_write_error_from_io_error() {
        let io_err = IoError::new(ErrorKind::BrokenPipe, "broken pipe");
        let err: BytesWriteError = io_err.into();
        assert!(matches!(err.value, BytesWriteErrorValue::IO(_)));
    }

    #[test]
    fn test_bytes_write_error_from_bytesio_error() {
        use super::super::bytesio_errors::{BytesIOError, BytesIOErrorValue};
        let bytesio_err = BytesIOError {
            value: BytesIOErrorValue::EmptyStream,
        };
        let err: BytesWriteError = bytesio_err.into();
        assert!(matches!(err.value, BytesWriteErrorValue::BytesIOError(_)));
    }

    // ========== BytesWriteError Display Tests ==========

    #[test]
    fn test_bytes_write_error_display_timeout() {
        let err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        assert_eq!(format!("{}", err), "write time out");
    }

    #[test]
    fn test_bytes_write_error_display_outof_index() {
        let err = BytesWriteError {
            value: BytesWriteErrorValue::OutofIndex,
        };
        assert_eq!(format!("{}", err), "outof index");
    }

    // ========== Debug Trait Tests ==========

    #[test]
    fn test_bytes_read_error_debug() {
        let err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("BytesReadError"));
        assert!(debug_str.contains("NotEnoughBytes"));
    }

    #[test]
    fn test_bytes_write_error_debug() {
        let err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("BytesWriteError"));
        assert!(debug_str.contains("Timeout"));
    }
}
