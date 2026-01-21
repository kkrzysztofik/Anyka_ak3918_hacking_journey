#![allow(non_local_definitions)]
use crate::bytesio::bits_errors::BitError;
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
