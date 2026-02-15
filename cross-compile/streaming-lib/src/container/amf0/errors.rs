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

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Amf0ReadErrorValue Display Tests ==========

    #[test]
    fn test_amf0_read_error_unknown_marker_display() {
        let err = Amf0ReadErrorValue::UnknownMarker { marker: 0xFF };
        assert_eq!(format!("{}", err), "Encountered unknown marker: 255");
    }

    #[test]
    fn test_amf0_read_error_wrong_type_display() {
        let err = Amf0ReadErrorValue::WrongType;
        assert_eq!(format!("{}", err), "wrong type");
    }

    #[test]
    fn test_amf0_read_error_invalid_boolean_display() {
        let err = Amf0ReadErrorValue::InvalidBoolean { value: 5 };
        assert_eq!(format!("{}", err), "invalid boolean value: 5");
    }

    // ========== Amf0WriteErrorValue Display Tests ==========

    #[test]
    fn test_amf0_write_error_normal_string_too_long_display() {
        let err = Amf0WriteErrorValue::NormalStringTooLong;
        assert_eq!(format!("{}", err), "normal string too long");
    }

    #[test]
    fn test_amf0_write_error_long_string_too_long_display() {
        let err = Amf0WriteErrorValue::LongStringTooLong;
        assert_eq!(format!("{}", err), "long string too long");
    }

    // ========== From Trait Tests ==========

    #[test]
    fn test_amf0_read_error_from_bytes_read_error() {
        let read_err = BytesReadError {
            value: crate::bytesio::bytes_errors::BytesReadErrorValue::NotEnoughBytes,
        };
        let amf_err: Amf0ReadError = read_err.into();
        assert!(matches!(amf_err.0, Amf0ReadErrorValue::BytesReadError(_)));
    }

    #[test]
    fn test_amf0_write_error_from_bytes_write_error() {
        let write_err = BytesWriteError {
            value: crate::bytesio::bytes_errors::BytesWriteErrorValue::OutofIndex,
        };
        let amf_err: Amf0WriteError = write_err.into();
        assert!(matches!(amf_err.0, Amf0WriteErrorValue::BytesWriteError(_)));
    }

    #[test]
    fn test_amf0_write_error_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::Other, "test error");
        let amf_err: Amf0WriteError = io_err.into();
        assert!(matches!(
            amf_err.0,
            Amf0WriteErrorValue::BufferWriteError(_)
        ));
    }

    #[test]
    fn test_amf0_read_error_from_utf8_error() {
        let invalid_utf8 = vec![0xFF, 0xFE];
        let utf8_err = String::from_utf8(invalid_utf8).unwrap_err();
        let amf_err: Amf0ReadError = utf8_err.into();
        assert!(matches!(amf_err.0, Amf0ReadErrorValue::StringParseError(_)));
    }

    // ========== Wrapper Error Transparency Tests ==========

    #[test]
    fn test_amf0_read_error_transparent_display() {
        let err = Amf0ReadError(Amf0ReadErrorValue::WrongType);
        assert_eq!(format!("{}", err), "wrong type");
    }

    #[test]
    fn test_amf0_write_error_transparent_display() {
        let err = Amf0WriteError(Amf0WriteErrorValue::NormalStringTooLong);
        assert_eq!(format!("{}", err), "normal string too long");
    }
}
