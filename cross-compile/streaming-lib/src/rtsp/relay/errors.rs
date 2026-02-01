use {std::io::Error, thiserror::Error, tokio::sync::broadcast::error::RecvError};

#[derive(Debug, Error)]
#[error("{value}")]
pub struct RelayError {
    pub value: PushClientErrorValue,
}

impl From<Error> for RelayError {
    fn from(err: Error) -> Self {
        Self {
            value: PushClientErrorValue::IOError(err),
        }
    }
}

impl From<RecvError> for RelayError {
    fn from(err: RecvError) -> Self {
        Self {
            value: PushClientErrorValue::ReceiveError(err),
        }
    }
}

#[derive(Debug, Error)]
pub enum PushClientErrorValue {
    #[error("receive error")]
    ReceiveError(#[from] RecvError),

    #[error("send error")]
    SendError,
    #[error("io error")]
    IOError(#[from] Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error as IoError, ErrorKind};

    // ========== PushClientErrorValue Display Tests ==========

    #[test]
    fn test_push_client_error_value_send_error_display() {
        let err = PushClientErrorValue::SendError;
        assert_eq!(format!("{}", err), "send error");
    }

    #[test]
    fn test_push_client_error_value_io_error_display() {
        let io_err = IoError::new(ErrorKind::ConnectionRefused, "connection refused");
        let err = PushClientErrorValue::IOError(io_err);
        assert_eq!(format!("{}", err), "io error");
    }

    #[test]
    fn test_push_client_error_value_receive_error_display() {
        let recv_err = RecvError::Closed;
        let err = PushClientErrorValue::ReceiveError(recv_err);
        assert_eq!(format!("{}", err), "receive error");
    }

    // ========== RelayError From Trait Tests ==========

    #[test]
    fn test_relay_error_from_io_error() {
        let io_err = IoError::new(ErrorKind::TimedOut, "timed out");
        let err: RelayError = io_err.into();
        assert!(matches!(err.value, PushClientErrorValue::IOError(_)));
    }

    #[test]
    fn test_relay_error_from_recv_error_closed() {
        let recv_err = RecvError::Closed;
        let err: RelayError = recv_err.into();
        assert!(matches!(err.value, PushClientErrorValue::ReceiveError(_)));
    }

    #[test]
    fn test_relay_error_from_recv_error_lagged() {
        let recv_err = RecvError::Lagged(10);
        let err: RelayError = recv_err.into();
        assert!(matches!(err.value, PushClientErrorValue::ReceiveError(_)));
    }

    // ========== RelayError Display Tests ==========

    #[test]
    fn test_relay_error_display_send_error() {
        let err = RelayError {
            value: PushClientErrorValue::SendError,
        };
        assert_eq!(format!("{}", err), "send error");
    }

    #[test]
    fn test_relay_error_display_io_error() {
        let io_err = IoError::new(ErrorKind::NotConnected, "not connected");
        let err = RelayError {
            value: PushClientErrorValue::IOError(io_err),
        };
        assert_eq!(format!("{}", err), "io error");
    }

    // ========== Debug Trait Tests ==========

    #[test]
    fn test_relay_error_debug() {
        let err = RelayError {
            value: PushClientErrorValue::SendError,
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("RelayError"));
        assert!(debug_str.contains("SendError"));
    }

    #[test]
    fn test_push_client_error_value_debug() {
        let io_err = IoError::new(ErrorKind::Other, "other error");
        let err = PushClientErrorValue::IOError(io_err);
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("IOError"));
    }
}
