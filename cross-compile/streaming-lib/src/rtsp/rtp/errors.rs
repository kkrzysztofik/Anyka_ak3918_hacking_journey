#![allow(non_local_definitions)]
use {
    failure::{Backtrace, Fail},
    std::fmt,
};

use crate::bytesio::bytes_errors::BytesReadError;
use crate::bytesio::bytes_errors::BytesWriteError;

#[derive(Debug)]
pub struct PackerError {
    pub value: PackerErrorValue,
}

impl Fail for PackerError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

impl fmt::Display for PackerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

#[derive(Debug, Fail)]
pub enum PackerErrorValue {
    #[fail(display = "bytes read error: {}", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error: {}", _0)]
    BytesWriteError(#[cause] BytesWriteError),
}

impl From<BytesReadError> for PackerError {
    fn from(error: BytesReadError) -> Self {
        PackerError {
            value: PackerErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for PackerError {
    fn from(error: BytesWriteError) -> Self {
        PackerError {
            value: PackerErrorValue::BytesWriteError(error),
        }
    }
}

#[derive(Debug)]
pub struct UnPackerError {
    pub value: UnPackerErrorValue,
}

#[derive(Debug, Fail)]
pub enum UnPackerErrorValue {
    #[fail(display = "bytes read error: {}", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error: {}", _0)]
    BytesWriteError(#[cause] BytesWriteError),
}

impl From<BytesReadError> for UnPackerError {
    fn from(error: BytesReadError) -> Self {
        UnPackerError {
            value: UnPackerErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for UnPackerError {
    fn from(error: BytesWriteError) -> Self {
        UnPackerError {
            value: UnPackerErrorValue::BytesWriteError(error),
        }
    }
}

impl fmt::Display for UnPackerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for UnPackerError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::bytes_errors::{BytesReadError, BytesReadErrorValue};
    use crate::bytesio::bytes_errors::{BytesWriteError, BytesWriteErrorValue};

    // ========== PackerErrorValue Display Tests ==========

    #[test]
    fn test_packer_error_value_bytes_read_display() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let err = PackerErrorValue::BytesReadError(read_err);
        assert!(format!("{}", err).contains("bytes read error"));
    }

    #[test]
    fn test_packer_error_value_bytes_write_display() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err = PackerErrorValue::BytesWriteError(write_err);
        assert!(format!("{}", err).contains("bytes write error"));
    }

    // ========== PackerError From Trait Tests ==========

    #[test]
    fn test_packer_error_from_bytes_read_error() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::EmptyStream,
        };
        let err: PackerError = read_err.into();
        assert!(matches!(err.value, PackerErrorValue::BytesReadError(_)));
    }

    #[test]
    fn test_packer_error_from_bytes_write_error() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::OutofIndex,
        };
        let err: PackerError = write_err.into();
        assert!(matches!(err.value, PackerErrorValue::BytesWriteError(_)));
    }

    // ========== PackerError Display Tests ==========

    #[test]
    fn test_packer_error_display() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let err = PackerError {
            value: PackerErrorValue::BytesReadError(read_err),
        };
        assert!(format!("{}", err).contains("bytes read error"));
    }

    // ========== UnPackerErrorValue Display Tests ==========

    #[test]
    fn test_unpacker_error_value_bytes_read_display() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::IndexOutofRange,
        };
        let err = UnPackerErrorValue::BytesReadError(read_err);
        assert!(format!("{}", err).contains("bytes read error"));
    }

    #[test]
    fn test_unpacker_error_value_bytes_write_display() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err = UnPackerErrorValue::BytesWriteError(write_err);
        assert!(format!("{}", err).contains("bytes write error"));
    }

    // ========== UnPackerError From Trait Tests ==========

    #[test]
    fn test_unpacker_error_from_bytes_read_error() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let err: UnPackerError = read_err.into();
        assert!(matches!(err.value, UnPackerErrorValue::BytesReadError(_)));
    }

    #[test]
    fn test_unpacker_error_from_bytes_write_error() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err: UnPackerError = write_err.into();
        assert!(matches!(err.value, UnPackerErrorValue::BytesWriteError(_)));
    }

    // ========== UnPackerError Display Tests ==========

    #[test]
    fn test_unpacker_error_display() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::EmptyStream,
        };
        let err = UnPackerError {
            value: UnPackerErrorValue::BytesReadError(read_err),
        };
        assert!(format!("{}", err).contains("bytes read error"));
    }

    // ========== Debug Trait Tests ==========

    #[test]
    fn test_packer_error_debug() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let err = PackerError {
            value: PackerErrorValue::BytesReadError(read_err),
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("PackerError"));
    }

    #[test]
    fn test_unpacker_error_debug() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err = UnPackerError {
            value: UnPackerErrorValue::BytesWriteError(write_err),
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("UnPackerError"));
    }
}
