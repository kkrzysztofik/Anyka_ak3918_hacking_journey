#![allow(non_local_definitions)]
use super::bytes_errors::BytesReadError;
use super::bytes_errors::BytesWriteError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BitErrorValue {
    #[error("bytes read error")]
    BytesReadError(#[from] BytesReadError),
    #[error("bytes write error")]
    BytesWriteError(#[from] BytesWriteError),
    #[error("the size is bigger than 64")]
    TooBig,
    #[error("invalid bit value: must be 0 or 1")]
    InvalidBitValue,
    #[error("cannot write the whole 8 bits")]
    CannotWrite8Bit,
    #[error("cannot read byte")]
    CannotReadByte,
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct BitError {
    pub value: BitErrorValue,
}

impl From<BitErrorValue> for BitError {
    fn from(val: BitErrorValue) -> Self {
        BitError { value: val }
    }
}

impl From<BytesReadError> for BitError {
    fn from(error: BytesReadError) -> Self {
        BitError {
            value: BitErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for BitError {
    fn from(error: BytesWriteError) -> Self {
        BitError {
            value: BitErrorValue::BytesWriteError(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::bytes_errors::{BytesReadError, BytesReadErrorValue};
    use crate::bytesio::bytes_errors::{BytesWriteError, BytesWriteErrorValue};

    // ========== BitErrorValue Display Tests ==========

    #[test]
    fn test_bit_error_value_too_big_display() {
        let err = BitErrorValue::TooBig;
        assert_eq!(format!("{}", err), "the size is bigger than 64");
    }

    #[test]
    fn test_bit_error_value_invalid_bit_value_display() {
        let err = BitErrorValue::InvalidBitValue;
        assert_eq!(format!("{}", err), "invalid bit value: must be 0 or 1");
    }

    #[test]
    fn test_bit_error_value_cannot_write_8_bit_display() {
        let err = BitErrorValue::CannotWrite8Bit;
        assert_eq!(format!("{}", err), "cannot write the whole 8 bits");
    }

    #[test]
    fn test_bit_error_value_cannot_read_byte_display() {
        let err = BitErrorValue::CannotReadByte;
        assert_eq!(format!("{}", err), "cannot read byte");
    }

    #[test]
    fn test_bit_error_value_bytes_read_error_display() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let err = BitErrorValue::BytesReadError(read_err);
        assert_eq!(format!("{}", err), "bytes read error");
    }

    #[test]
    fn test_bit_error_value_bytes_write_error_display() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::OutofIndex,
        };
        let err = BitErrorValue::BytesWriteError(write_err);
        assert_eq!(format!("{}", err), "bytes write error");
    }

    // ========== BitError Display Tests ==========

    #[test]
    fn test_bit_error_display_delegates_to_value() {
        let err = BitError {
            value: BitErrorValue::TooBig,
        };
        assert_eq!(format!("{}", err), "the size is bigger than 64");
    }

    // ========== From Trait Tests ==========

    #[test]
    fn test_bit_error_from_bit_error_value() {
        let err: BitError = BitErrorValue::TooBig.into();
        assert!(matches!(err.value, BitErrorValue::TooBig));
    }

    #[test]
    fn test_bit_error_from_bytes_read_error() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let bit_err: BitError = read_err.into();
        assert!(matches!(bit_err.value, BitErrorValue::BytesReadError(_)));
    }

    #[test]
    fn test_bit_error_from_bytes_write_error() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::OutofIndex,
        };
        let bit_err: BitError = write_err.into();
        assert!(matches!(bit_err.value, BitErrorValue::BytesWriteError(_)));
    }

    // ========== Debug Trait Tests ==========

    #[test]
    fn test_bit_error_debug() {
        let err = BitError {
            value: BitErrorValue::InvalidBitValue,
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("BitError"));
        assert!(debug_str.contains("InvalidBitValue"));
    }
}
