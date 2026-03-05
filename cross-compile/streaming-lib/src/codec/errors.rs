#![allow(non_local_definitions)]
use crate::io::bits_errors::BitError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum H264ErrorValue {
    #[error("bit error")]
    BitError(#[from] BitError),
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct H264Error {
    pub value: H264ErrorValue,
}

impl From<BitError> for H264Error {
    fn from(error: BitError) -> Self {
        H264Error {
            value: H264ErrorValue::BitError(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::bits_errors::{BitError, BitErrorValue};

    // ========== H264ErrorValue Display Tests ==========

    #[test]
    fn test_h264_error_value_bit_error_display() {
        let bit_err = BitError {
            value: BitErrorValue::TooBig,
        };
        let err = H264ErrorValue::BitError(bit_err);
        let display = format!("{}", err);
        assert_eq!(display, "bit error");
    }

    #[test]
    fn test_h264_error_value_debug() {
        let bit_err = BitError {
            value: BitErrorValue::InvalidBitValue,
        };
        let err = H264ErrorValue::BitError(bit_err);
        let debug = format!("{:?}", err);
        assert!(debug.contains("BitError"));
    }

    // ========== H264Error Display Tests ==========

    #[test]
    fn test_h264_error_display_delegates_to_value() {
        let bit_err = BitError {
            value: BitErrorValue::CannotReadByte,
        };
        let err = H264Error {
            value: H264ErrorValue::BitError(bit_err),
        };
        let display = format!("{}", err);
        assert_eq!(display, "bit error");
    }

    #[test]
    fn test_h264_error_debug() {
        let bit_err = BitError {
            value: BitErrorValue::TooBig,
        };
        let err = H264Error {
            value: H264ErrorValue::BitError(bit_err),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("H264Error"));
        assert!(debug.contains("BitError"));
    }

    // ========== From Trait Tests ==========

    #[test]
    fn test_h264_error_from_bit_error() {
        let bit_err = BitError {
            value: BitErrorValue::CannotWrite8Bit,
        };
        let h264_err: H264Error = bit_err.into();
        assert!(matches!(h264_err.value, H264ErrorValue::BitError(_)));
    }

    #[test]
    fn test_h264_error_from_bit_error_preserves_inner() {
        let bit_err = BitError {
            value: BitErrorValue::TooBig,
        };
        let h264_err: H264Error = bit_err.into();
        let H264ErrorValue::BitError(inner) = h264_err.value;
        assert!(matches!(inner.value, BitErrorValue::TooBig));
    }
}
