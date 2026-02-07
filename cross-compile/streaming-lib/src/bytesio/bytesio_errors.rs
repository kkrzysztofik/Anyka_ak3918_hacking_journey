#![allow(non_local_definitions)]
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BytesIOErrorValue {
    #[error("not enough bytes")]
    NotEnoughBytes,
    #[error("empty stream")]
    EmptyStream,
    #[error("io error: {0}")]
    IOError(io::Error),
    #[error("time out error: {}", _0)]
    TimeoutError(tokio::time::error::Elapsed),
    #[error("none return")]
    NoneReturn,
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct BytesIOError {
    pub value: BytesIOErrorValue,
}

impl From<BytesIOErrorValue> for BytesIOError {
    fn from(val: BytesIOErrorValue) -> Self {
        BytesIOError { value: val }
    }
}

impl From<io::Error> for BytesIOError {
    fn from(error: io::Error) -> Self {
        BytesIOError {
            value: BytesIOErrorValue::IOError(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error as IoError, ErrorKind};

    // ========== BytesIOErrorValue Display Tests ==========

    #[test]
    fn test_bytesio_error_value_not_enough_bytes_display() {
        let err = BytesIOErrorValue::NotEnoughBytes;
        assert_eq!(format!("{}", err), "not enough bytes");
    }

    #[test]
    fn test_bytesio_error_value_empty_stream_display() {
        let err = BytesIOErrorValue::EmptyStream;
        assert_eq!(format!("{}", err), "empty stream");
    }

    #[test]
    fn test_bytesio_error_value_io_error_display() {
        let io_err = IoError::new(ErrorKind::NotFound, "not found");
        let err = BytesIOErrorValue::IOError(io_err);
        assert_eq!(format!("{}", err), "io error: not found");
    }

    #[test]
    fn test_bytesio_error_value_none_return_display() {
        let err = BytesIOErrorValue::NoneReturn;
        assert_eq!(format!("{}", err), "none return");
    }

    // ========== BytesIOError From Trait Tests ==========

    #[test]
    fn test_bytesio_error_from_value() {
        let value = BytesIOErrorValue::NotEnoughBytes;
        let err: BytesIOError = value.into();
        assert!(matches!(err.value, BytesIOErrorValue::NotEnoughBytes));
    }

    #[test]
    fn test_bytesio_error_from_io_error() {
        let io_err = IoError::new(ErrorKind::PermissionDenied, "access denied");
        let err: BytesIOError = io_err.into();
        assert!(matches!(err.value, BytesIOErrorValue::IOError(_)));
    }

    // ========== BytesIOError Display Tests ==========

    #[test]
    fn test_bytesio_error_display_not_enough_bytes() {
        let err = BytesIOError {
            value: BytesIOErrorValue::NotEnoughBytes,
        };
        assert_eq!(format!("{}", err), "not enough bytes");
    }

    #[test]
    fn test_bytesio_error_display_empty_stream() {
        let err = BytesIOError {
            value: BytesIOErrorValue::EmptyStream,
        };
        assert_eq!(format!("{}", err), "empty stream");
    }

    #[test]
    fn test_bytesio_error_display_none_return() {
        let err = BytesIOError {
            value: BytesIOErrorValue::NoneReturn,
        };
        assert_eq!(format!("{}", err), "none return");
    }

    // ========== Debug Trait Tests ==========

    #[test]
    fn test_bytesio_error_debug() {
        let err = BytesIOError {
            value: BytesIOErrorValue::EmptyStream,
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("BytesIOError"));
        assert!(debug_str.contains("EmptyStream"));
    }

    #[test]
    fn test_bytesio_error_value_debug() {
        let err = BytesIOErrorValue::NotEnoughBytes;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotEnoughBytes"));
    }
}
