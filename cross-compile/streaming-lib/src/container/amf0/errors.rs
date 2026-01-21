use {
    crate::bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    std::{io, string},
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum Amf0ReadErrorValue {
    #[error("Encountered unknown marker: {}", marker)]
    UnknownMarker { marker: u8 },
    #[error(transparent)]
    StringParseError(#[from] string::FromUtf8Error),
    #[error(transparent)]
    BytesReadError(#[from] BytesReadError),
    #[error("wrong type")]
    WrongType,
    #[error("invalid boolean value: {value}")]
    InvalidBoolean { value: u8 },
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct Amf0ReadError(#[from] pub Amf0ReadErrorValue);

impl From<BytesReadError> for Amf0ReadError {
    fn from(error: BytesReadError) -> Self {
        Amf0ReadError(Amf0ReadErrorValue::BytesReadError(error))
    }
}

impl From<string::FromUtf8Error> for Amf0ReadError {
    fn from(error: string::FromUtf8Error) -> Self {
        Amf0ReadError(Amf0ReadErrorValue::StringParseError(error))
    }
}

#[derive(Debug, Error)]
pub enum Amf0WriteErrorValue {
    #[error("normal string too long")]
    NormalStringTooLong,
    #[error("long string too long")]
    LongStringTooLong,
    #[error(transparent)]
    BufferWriteError(#[from] io::Error),
    #[error(transparent)]
    BytesWriteError(#[from] BytesWriteError),
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct Amf0WriteError(#[from] pub Amf0WriteErrorValue);

impl From<BytesWriteError> for Amf0WriteError {
    fn from(error: BytesWriteError) -> Self {
        Amf0WriteError(Amf0WriteErrorValue::BytesWriteError(error))
    }
}

impl From<io::Error> for Amf0WriteError {
    fn from(error: io::Error) -> Self {
        Amf0WriteError(Amf0WriteErrorValue::BufferWriteError(error))
    }
}
