#![allow(non_local_definitions)]
use crate::bytesio::bytes_errors::BytesReadError;
use crate::bytesio::bytes_errors::BytesWriteError;
use failure::Backtrace;
use serde_json::error::Error;
use tokio::sync::oneshot::error::RecvError;

use {failure::Fail, std::fmt};
#[derive(Debug, Fail)]
pub enum StreamHubErrorValue {
    #[fail(display = "no app name")]
    NoAppName,
    #[fail(display = "no stream name")]
    NoStreamName,
    #[fail(display = "no app or stream name")]
    NoAppOrStreamName,
    #[fail(display = "exists")]
    Exists,
    #[fail(display = "send error")]
    SendError,
    #[fail(display = "send video error")]
    SendVideoError,
    #[fail(display = "send audio error")]
    SendAudioError,
    #[fail(display = "bytes read error")]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error")]
    BytesWriteError(BytesWriteError),
    #[fail(display = "not correct data sender type")]
    NotCorrectDataSenderType,
    #[fail(display = "Tokio oneshot recv error")]
    RecvError(RecvError),
    #[fail(display = "Serde json error")]
    SerdeError(Error),
    #[fail(display = "the client session error: {}", _0)]
    RtspClientSessionError(String),
}
#[derive(Debug)]
pub struct StreamHubError {
    pub value: StreamHubErrorValue,
}

impl fmt::Display for StreamHubError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for StreamHubError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

impl From<BytesReadError> for StreamHubError {
    fn from(error: BytesReadError) -> Self {
        StreamHubError {
            value: StreamHubErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for StreamHubError {
    fn from(error: BytesWriteError) -> Self {
        StreamHubError {
            value: StreamHubErrorValue::BytesWriteError(error),
        }
    }
}

impl From<RecvError> for StreamHubError {
    fn from(error: RecvError) -> Self {
        StreamHubError {
            value: StreamHubErrorValue::RecvError(error),
        }
    }
}

impl From<Error> for StreamHubError {
    fn from(error: Error) -> Self {
        StreamHubError {
            value: StreamHubErrorValue::SerdeError(error),
        }
    }
}

impl From<String> for StreamHubError {
    fn from(error: String) -> Self {
        StreamHubError {
            value: StreamHubErrorValue::RtspClientSessionError(error),
        }
    }
}

// impl From<CacheError> for ChannelError {
//     fn from(error: CacheError) -> Self {
//         ChannelError {
//             value: ChannelErrorValue::CacheError(error),
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::bytes_errors::{BytesReadError, BytesReadErrorValue};
    use crate::bytesio::bytes_errors::{BytesWriteError, BytesWriteErrorValue};

    // ========== StreamHubErrorValue Display Tests ==========

    #[test]
    fn test_streamhub_error_value_no_app_name_display() {
        let err = StreamHubErrorValue::NoAppName;
        assert_eq!(format!("{}", err), "no app name");
    }

    #[test]
    fn test_streamhub_error_value_no_stream_name_display() {
        let err = StreamHubErrorValue::NoStreamName;
        assert_eq!(format!("{}", err), "no stream name");
    }

    #[test]
    fn test_streamhub_error_value_no_app_or_stream_name_display() {
        let err = StreamHubErrorValue::NoAppOrStreamName;
        assert_eq!(format!("{}", err), "no app or stream name");
    }

    #[test]
    fn test_streamhub_error_value_exists_display() {
        let err = StreamHubErrorValue::Exists;
        assert_eq!(format!("{}", err), "exists");
    }

    #[test]
    fn test_streamhub_error_value_send_error_display() {
        let err = StreamHubErrorValue::SendError;
        assert_eq!(format!("{}", err), "send error");
    }

    #[test]
    fn test_streamhub_error_value_send_video_error_display() {
        let err = StreamHubErrorValue::SendVideoError;
        assert_eq!(format!("{}", err), "send video error");
    }

    #[test]
    fn test_streamhub_error_value_send_audio_error_display() {
        let err = StreamHubErrorValue::SendAudioError;
        assert_eq!(format!("{}", err), "send audio error");
    }

    #[test]
    fn test_streamhub_error_value_not_correct_data_sender_type_display() {
        let err = StreamHubErrorValue::NotCorrectDataSenderType;
        assert_eq!(format!("{}", err), "not correct data sender type");
    }

    #[test]
    fn test_streamhub_error_value_rtsp_client_session_error_display() {
        let err = StreamHubErrorValue::RtspClientSessionError("test error".to_string());
        assert!(format!("{}", err).contains("test error"));
    }

    // ========== StreamHubError From Trait Tests ==========

    #[test]
    fn test_streamhub_error_from_bytes_read_error() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let err: StreamHubError = read_err.into();
        assert!(matches!(err.value, StreamHubErrorValue::BytesReadError(_)));
    }

    #[test]
    fn test_streamhub_error_from_bytes_write_error() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err: StreamHubError = write_err.into();
        assert!(matches!(err.value, StreamHubErrorValue::BytesWriteError(_)));
    }

    #[test]
    fn test_streamhub_error_from_string() {
        let err: StreamHubError = "test session error".to_string().into();
        assert!(matches!(
            err.value,
            StreamHubErrorValue::RtspClientSessionError(_)
        ));
        assert!(format!("{}", err).contains("test session error"));
    }

    // ========== StreamHubError Display Tests ==========

    #[test]
    fn test_streamhub_error_display_no_app_name() {
        let err = StreamHubError {
            value: StreamHubErrorValue::NoAppName,
        };
        assert_eq!(format!("{}", err), "no app name");
    }

    #[test]
    fn test_streamhub_error_display_exists() {
        let err = StreamHubError {
            value: StreamHubErrorValue::Exists,
        };
        assert_eq!(format!("{}", err), "exists");
    }

    #[test]
    fn test_streamhub_error_display_send_error() {
        let err = StreamHubError {
            value: StreamHubErrorValue::SendError,
        };
        assert_eq!(format!("{}", err), "send error");
    }

    // ========== Debug Trait Tests ==========

    #[test]
    fn test_streamhub_error_debug() {
        let err = StreamHubError {
            value: StreamHubErrorValue::NoAppName,
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("StreamHubError"));
        assert!(debug_str.contains("NoAppName"));
    }

    #[test]
    fn test_streamhub_error_value_debug() {
        let err = StreamHubErrorValue::SendVideoError;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("SendVideoError"));
    }
}
