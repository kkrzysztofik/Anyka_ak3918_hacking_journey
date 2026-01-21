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
