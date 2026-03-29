use thiserror::Error;

use crate::io::bytes_errors::BytesReadError;
use crate::io::bytes_errors::BytesWriteError;
use crate::io::bytesio_errors::BytesIOError;

#[derive(Debug, Error)]
#[error("{value}")]
pub struct PackerError {
    pub value: PackerErrorValue,
}

#[derive(Debug, Error)]
pub enum PackerErrorValue {
    #[error("bytes read error: {0}")]
    BytesReadError(#[source] BytesReadError),
    #[error("bytes write error: {}", _0)]
    BytesWriteError(#[from] BytesWriteError),
    /// Underlying transport read/write failed while packing or sending RTSP/RTP data.
    #[error("bytes io error: {0}")]
    BytesIOError(#[from] BytesIOError),
    /// Invalid or oversized RTSP TCP interleaved (`$`) framing, or related reassembly errors.
    #[error("interleaved framing: {0}")]
    InterleavedFraming(String),
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

impl From<BytesIOError> for PackerError {
    fn from(error: BytesIOError) -> Self {
        PackerError {
            value: PackerErrorValue::BytesIOError(error),
        }
    }
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct UnPackerError {
    pub value: UnPackerErrorValue,
}

#[derive(Debug, Error)]
pub enum UnPackerErrorValue {
    #[error("bytes read error: {0}")]
    BytesReadError(#[source] BytesReadError),
    #[error("bytes write error: {}", _0)]
    BytesWriteError(#[from] BytesWriteError),
    #[error("invalid timestamp in RTP payload")]
    InvalidTimestamp,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::bytes_errors::{BytesReadError, BytesReadErrorValue};
    use crate::io::bytes_errors::{BytesWriteError, BytesWriteErrorValue};
    use crate::io::bytesio_errors::{BytesIOError, BytesIOErrorValue};

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

    #[test]
    fn test_packer_error_from_bytes_io_error_write_path() {
        let bytesio_err = BytesIOError {
            value: BytesIOErrorValue::NotEnoughBytes,
        };
        let err: PackerError = bytesio_err.into();
        assert!(matches!(err.value, PackerErrorValue::BytesIOError(_)));
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

    #[test]
    fn test_packer_error_display_bytes_io_error() {
        let bytesio_err = BytesIOError {
            value: BytesIOErrorValue::NotEnoughBytes,
        };
        let err: PackerError = bytesio_err.into();
        let s = format!("{}", err);
        assert!(s.contains("bytes io error"), "display was: {s}");
    }

    #[test]
    fn test_packer_error_value_interleaved_framing_display() {
        let err = PackerErrorValue::InterleavedFraming("bad channel".to_string());
        let s = format!("{}", err);
        assert!(s.contains("interleaved framing"), "display was: {s}");
        assert!(s.contains("bad channel"), "display was: {s}");
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
