use {
    crate::bytesio::bytes_errors::BytesReadError,
    crate::bytesio::{bytes_errors::BytesWriteError, bytesio_errors::BytesIOError},
    crate::common::errors::AuthError,
    crate::rtsp::rtp::errors::{PackerError, UnPackerError},
    crate::streamhub::errors::StreamHubError,
    std::io::Error,
    std::str::Utf8Error,
    thiserror::Error,
    tokio::sync::oneshot::error::RecvError,
};

#[derive(Debug, Error)]
pub enum SessionErrorValue {
    #[error("net io error: {}", _0)]
    BytesIOError(#[from] BytesIOError),
    #[error("bytes read error: {}", _0)]
    BytesReadError(#[from] BytesReadError),
    #[error("bytes write error: {}", _0)]
    BytesWriteError(#[from] BytesWriteError),
    #[error("Utf8Error: {}", _0)]
    Utf8Error(#[from] Utf8Error),
    #[error("UnPackerError: {}", _0)]
    UnPackerError(#[from] UnPackerError),
    #[error("stream hub event send error")]
    StreamHubEventSendErr,
    #[error("cannot receive frame data from stream hub")]
    CannotReceiveFrameData,
    #[error("pack error: {}", _0)]
    PackerError(#[from] PackerError),
    #[error("event execute error: {}", _0)]
    ChannelError(#[from] StreamHubError),
    #[error("tokio: oneshot receiver err: {}", _0)]
    RecvError(#[from] RecvError),
    #[error("auth err: {}", _0)]
    AuthError(#[from] AuthError),
    #[error("Channel receive error")]
    ChannelRecvError,
    #[error("io error: {}", _0)]
    IOError(#[from] Error),
    #[error("RTSP response status error")]
    RtspResponseStatusError,
    #[error("missing frame sender")]
    MissingFrameSender,
    #[error("missing frame receiver")]
    MissingFrameReceiver,
    #[error("missing client ports")]
    MissingClientPort,
    #[error("missing session id")]
    MissingSessionId,
    #[error("corrupted rtsp message: {0}")]
    RtspMessageCorrupted(String),
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct SessionError {
    pub value: SessionErrorValue,
}

impl From<SessionErrorValue> for SessionError {
    fn from(val: SessionErrorValue) -> Self {
        SessionError { value: val }
    }
}

impl From<BytesIOError> for SessionError {
    fn from(error: BytesIOError) -> Self {
        SessionError {
            value: SessionErrorValue::BytesIOError(error),
        }
    }
}

impl From<BytesReadError> for SessionError {
    fn from(error: BytesReadError) -> Self {
        SessionError {
            value: SessionErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for SessionError {
    fn from(error: BytesWriteError) -> Self {
        SessionError {
            value: SessionErrorValue::BytesWriteError(error),
        }
    }
}

impl From<Utf8Error> for SessionError {
    fn from(error: Utf8Error) -> Self {
        SessionError {
            value: SessionErrorValue::Utf8Error(error),
        }
    }
}

impl From<PackerError> for SessionError {
    fn from(error: PackerError) -> Self {
        SessionError {
            value: SessionErrorValue::PackerError(error),
        }
    }
}

impl From<UnPackerError> for SessionError {
    fn from(error: UnPackerError) -> Self {
        SessionError {
            value: SessionErrorValue::UnPackerError(error),
        }
    }
}

impl From<StreamHubError> for SessionError {
    fn from(error: StreamHubError) -> Self {
        SessionError {
            value: SessionErrorValue::ChannelError(error),
        }
    }
}

impl From<RecvError> for SessionError {
    fn from(error: RecvError) -> Self {
        SessionError {
            value: SessionErrorValue::RecvError(error),
        }
    }
}

impl From<AuthError> for SessionError {
    fn from(error: AuthError) -> Self {
        SessionError {
            value: SessionErrorValue::AuthError(error),
        }
    }
}

impl From<Error> for SessionError {
    fn from(error: Error) -> Self {
        SessionError {
            value: SessionErrorValue::IOError(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== SessionErrorValue Display Tests ==========

    #[test]
    fn test_session_error_value_stream_hub_event_send_err() {
        let error = SessionErrorValue::StreamHubEventSendErr;
        assert_eq!(format!("{}", error), "stream hub event send error");
    }

    #[test]
    fn test_session_error_value_cannot_receive_frame_data() {
        let error = SessionErrorValue::CannotReceiveFrameData;
        assert_eq!(
            format!("{}", error),
            "cannot receive frame data from stream hub"
        );
    }

    #[test]
    fn test_session_error_value_channel_recv_error() {
        let error = SessionErrorValue::ChannelRecvError;
        assert_eq!(format!("{}", error), "Channel receive error");
    }

    #[test]
    fn test_session_error_value_rtsp_response_status_error() {
        let error = SessionErrorValue::RtspResponseStatusError;
        assert_eq!(format!("{}", error), "RTSP response status error");
    }

    #[test]
    fn test_session_error_value_missing_frame_sender() {
        let error = SessionErrorValue::MissingFrameSender;
        assert_eq!(format!("{}", error), "missing frame sender");
    }

    // ========== SessionError Display Tests ==========

    #[test]
    fn test_session_error_display() {
        let error = SessionError {
            value: SessionErrorValue::StreamHubEventSendErr,
        };
        assert_eq!(format!("{}", error), "stream hub event send error");
    }

    #[test]
    fn test_session_error_debug() {
        let error = SessionError {
            value: SessionErrorValue::ChannelRecvError,
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("SessionError"));
        assert!(debug_str.contains("ChannelRecvError"));
    }

    // ========== From Trait Tests ==========

    #[test]
    fn test_session_error_from_bytes_read_error() {
        use crate::bytesio::bytes_errors::{BytesReadError, BytesReadErrorValue};
        let read_error = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let session_error: SessionError = read_error.into();
        match session_error.value {
            SessionErrorValue::BytesReadError(_) => {}
            _ => panic!("Expected BytesReadError variant"),
        }
    }

    #[test]
    fn test_session_error_from_bytes_write_error() {
        use crate::bytesio::bytes_errors::{BytesWriteError, BytesWriteErrorValue};
        let write_error = BytesWriteError {
            value: BytesWriteErrorValue::OutofIndex,
        };
        let session_error: SessionError = write_error.into();
        match session_error.value {
            SessionErrorValue::BytesWriteError(_) => {}
            _ => panic!("Expected BytesWriteError variant"),
        }
    }

    #[test]
    fn test_session_error_from_packer_error() {
        use crate::bytesio::bytes_errors::{BytesWriteError, BytesWriteErrorValue};
        use crate::rtsp::rtp::errors::{PackerError, PackerErrorValue};
        let write_error = BytesWriteError {
            value: BytesWriteErrorValue::OutofIndex,
        };
        let packer_error = PackerError {
            value: PackerErrorValue::BytesWriteError(write_error),
        };
        let session_error: SessionError = packer_error.into();
        match session_error.value {
            SessionErrorValue::PackerError(_) => {}
            _ => panic!("Expected PackerError variant"),
        }
    }

    #[test]
    fn test_session_error_from_unpacker_error() {
        use crate::bytesio::bytes_errors::{BytesReadError, BytesReadErrorValue};
        use crate::rtsp::rtp::errors::{UnPackerError, UnPackerErrorValue};
        let read_error = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let unpacker_error = UnPackerError {
            value: UnPackerErrorValue::BytesReadError(read_error),
        };
        let session_error: SessionError = unpacker_error.into();
        match session_error.value {
            SessionErrorValue::UnPackerError(_) => {}
            _ => panic!("Expected UnPackerError variant"),
        }
    }

    #[test]
    fn test_session_error_from_stream_hub_error() {
        use crate::streamhub::errors::{StreamHubError, StreamHubErrorValue};
        let hub_error = StreamHubError {
            value: StreamHubErrorValue::NoAppName,
        };
        let session_error: SessionError = hub_error.into();
        match session_error.value {
            SessionErrorValue::ChannelError(_) => {}
            _ => panic!("Expected ChannelError variant"),
        }
    }

    #[test]
    fn test_session_error_from_auth_error() {
        use crate::common::errors::{AuthError, AuthErrorValue};
        let auth_error = AuthError {
            value: AuthErrorValue::TokenIsNotCorrect,
        };
        let session_error: SessionError = auth_error.into();
        match session_error.value {
            SessionErrorValue::AuthError(_) => {}
            _ => panic!("Expected AuthError variant"),
        }
    }

    // ========== Fail Trait Tests ==========

    #[test]
    fn test_session_error_cause() {
        let error = SessionError {
            value: SessionErrorValue::StreamHubEventSendErr,
        };
        assert!(std::error::Error::source(&error).is_none());
    }

    #[test]
    fn test_session_error_backtrace() {
        let error = SessionError {
            value: SessionErrorValue::ChannelRecvError,
        };
        assert!(std::error::Error::backtrace(&error).is_none());
    }

    // ========== Error Variant Coverage ==========

    #[test]
    fn test_all_session_error_variants() {
        // Test that simple variants can be created
        let _ = SessionErrorValue::StreamHubEventSendErr;
        let _ = SessionErrorValue::CannotReceiveFrameData;
        let _ = SessionErrorValue::ChannelRecvError;
        let _ = SessionErrorValue::RtspResponseStatusError;
    }
}
