use crate::streamhub::errors::StreamHubError;

use {
    crate::container::amf0::errors::Amf0WriteError, crate::container::errors::FlvMuxerError,
    futures::channel::mpsc::SendError, thiserror::Error, tokio::sync::oneshot::error::RecvError,
};

#[derive(Debug, Error)]
#[error("{value}")]
pub struct ServerError {
    pub value: ServerErrorValue,
}

#[derive(Debug, Error)]
pub enum ServerErrorValue {
    #[error("server error")]
    Error,
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct HttpFLvError {
    pub value: HttpFLvErrorValue,
}

#[derive(Debug, Error)]
pub enum HttpFLvErrorValue {
    #[error("server error")]
    Error,
    #[error("flv muxer error: {}", _0)]
    MuxerError(FlvMuxerError),
    #[error("amf write error: {}", _0)]
    Amf0WriteError(Amf0WriteError),
    #[error("metadata error: {}", _0)]
    MpscSendError(SendError),
    #[error("event execute error: {}", _0)]
    ChannelError(StreamHubError),
    #[error("tokio: oneshot receiver err: {}", _0)]
    RecvError(RecvError),
    #[error("channel recv error")]
    ChannelRecvError,
    #[error("send frame data error")]
    SendFrameDataErr,
    #[error("unexpected frame data: {0}")]
    UnexpectedFrameData(String),
    #[error("missing frame receiver")]
    MissingFrameReceiver,
}

impl From<FlvMuxerError> for HttpFLvError {
    fn from(error: FlvMuxerError) -> Self {
        HttpFLvError {
            value: HttpFLvErrorValue::MuxerError(error),
        }
    }
}

impl From<SendError> for HttpFLvError {
    fn from(error: SendError) -> Self {
        HttpFLvError {
            value: HttpFLvErrorValue::MpscSendError(error),
        }
    }
}

impl From<Amf0WriteError> for HttpFLvError {
    fn from(error: Amf0WriteError) -> Self {
        HttpFLvError {
            value: HttpFLvErrorValue::Amf0WriteError(error),
        }
    }
}

impl From<StreamHubError> for HttpFLvError {
    fn from(error: StreamHubError) -> Self {
        HttpFLvError {
            value: HttpFLvErrorValue::ChannelError(error),
        }
    }
}

impl From<RecvError> for HttpFLvError {
    fn from(error: RecvError) -> Self {
        HttpFLvError {
            value: HttpFLvErrorValue::RecvError(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== ServerError Tests ==========

    #[test]
    fn test_server_error_value_display() {
        let error = ServerErrorValue::Error;
        assert_eq!(format!("{}", error), "server error");
    }

    #[test]
    fn test_server_error_debug() {
        let error = ServerError {
            value: ServerErrorValue::Error,
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ServerError"));
    }

    // ========== HttpFLvErrorValue Display Tests ==========

    #[test]
    fn test_httpflv_error_value_error_display() {
        let error = HttpFLvErrorValue::Error;
        assert_eq!(format!("{}", error), "server error");
    }

    #[test]
    fn test_httpflv_error_value_channel_recv_error_display() {
        let error = HttpFLvErrorValue::ChannelRecvError;
        assert_eq!(format!("{}", error), "channel recv error");
    }

    #[test]
    fn test_httpflv_error_value_send_frame_data_err_display() {
        let error = HttpFLvErrorValue::SendFrameDataErr;
        assert_eq!(format!("{}", error), "send frame data error");
    }

    // ========== HttpFLvError Display Tests ==========

    #[test]
    fn test_httpflv_error_display() {
        let error = HttpFLvError {
            value: HttpFLvErrorValue::Error,
        };
        assert_eq!(format!("{}", error), "server error");
    }

    #[test]
    fn test_httpflv_error_display_channel_recv() {
        let error = HttpFLvError {
            value: HttpFLvErrorValue::ChannelRecvError,
        };
        assert_eq!(format!("{}", error), "channel recv error");
    }

    // ========== From Trait Tests ==========

    #[test]
    fn test_httpflv_error_from_flv_muxer_error() {
        use crate::bytesio::bytes_errors::{BytesWriteError, BytesWriteErrorValue};
        use crate::container::errors::{FlvMuxerError, MuxerErrorValue};
        let write_error = BytesWriteError {
            value: BytesWriteErrorValue::OutofIndex,
        };
        let muxer_error = FlvMuxerError {
            value: MuxerErrorValue::BytesWriteError(write_error),
        };
        let http_error: HttpFLvError = muxer_error.into();
        match http_error.value {
            HttpFLvErrorValue::MuxerError(_) => {}
            _ => panic!("Expected MuxerError variant"),
        }
    }

    #[test]
    fn test_httpflv_error_from_amf0_write_error() {
        use crate::container::amf0::errors::{Amf0WriteError, Amf0WriteErrorValue};
        let amf_error = Amf0WriteError(Amf0WriteErrorValue::NormalStringTooLong);
        let http_error: HttpFLvError = amf_error.into();
        match &http_error.value {
            HttpFLvErrorValue::Amf0WriteError(_) => {}
            _ => panic!("Expected Amf0WriteError variant"),
        }
    }

    #[test]
    fn test_httpflv_error_from_stream_hub_error() {
        use crate::streamhub::errors::{StreamHubError, StreamHubErrorValue};
        let hub_error = StreamHubError {
            value: StreamHubErrorValue::NoAppName,
        };
        let http_error: HttpFLvError = hub_error.into();
        match http_error.value {
            HttpFLvErrorValue::ChannelError(_) => {}
            _ => panic!("Expected ChannelError variant"),
        }
    }

    // ========== Error Variant Coverage ==========

    #[test]
    fn test_all_httpflv_error_variants() {
        assert!(matches!(HttpFLvErrorValue::Error, HttpFLvErrorValue::Error));
        assert!(matches!(
            HttpFLvErrorValue::ChannelRecvError,
            HttpFLvErrorValue::ChannelRecvError
        ));
        assert!(matches!(
            HttpFLvErrorValue::SendFrameDataErr,
            HttpFLvErrorValue::SendFrameDataErr
        ));
        assert!(matches!(
            HttpFLvErrorValue::UnexpectedFrameData("x".to_string()),
            HttpFLvErrorValue::UnexpectedFrameData(_)
        ));
        assert!(matches!(
            HttpFLvErrorValue::MissingFrameReceiver,
            HttpFLvErrorValue::MissingFrameReceiver
        ));
        // Variants with wrapped errors are tested via From traits above
    }
}
