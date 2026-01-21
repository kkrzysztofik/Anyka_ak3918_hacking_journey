use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthErrorValue {
    #[error("token is not correct")]
    TokenIsNotCorrect,
    #[error("no token found")]
    NoTokenFound,
    #[error("invalid token format")]
    InvalidTokenFormat,
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct AuthError {
    pub value: AuthErrorValue,
}

impl From<AuthErrorValue> for AuthError {
    fn from(val: AuthErrorValue) -> Self {
        AuthError { value: val }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== AuthErrorValue Display Tests ==========

    #[test]
    fn test_auth_error_value_token_not_correct_display() {
        let err = AuthErrorValue::TokenIsNotCorrect;
        assert_eq!(format!("{}", err), "token is not correct");
    }

    #[test]
    fn test_auth_error_value_no_token_found_display() {
        let err = AuthErrorValue::NoTokenFound;
        assert_eq!(format!("{}", err), "no token found");
    }

    #[test]
    fn test_auth_error_value_invalid_token_format_display() {
        let err = AuthErrorValue::InvalidTokenFormat;
        assert_eq!(format!("{}", err), "invalid token format");
    }

    // ========== AuthError Display Tests ==========

    #[test]
    fn test_auth_error_display_token_not_correct() {
        let err = AuthError {
            value: AuthErrorValue::TokenIsNotCorrect,
        };
        assert_eq!(format!("{}", err), "token is not correct");
    }

    #[test]
    fn test_auth_error_display_no_token_found() {
        let err = AuthError {
            value: AuthErrorValue::NoTokenFound,
        };
        assert_eq!(format!("{}", err), "no token found");
    }

    #[test]
    fn test_auth_error_display_invalid_token_format() {
        let err = AuthError {
            value: AuthErrorValue::InvalidTokenFormat,
        };
        assert_eq!(format!("{}", err), "invalid token format");
    }

    // ========== Debug Trait Tests ==========

    #[test]
    fn test_auth_error_debug() {
        let err = AuthError {
            value: AuthErrorValue::TokenIsNotCorrect,
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("AuthError"));
        assert!(debug_str.contains("TokenIsNotCorrect"));
    }

    #[test]
    fn test_auth_error_value_debug() {
        let err = AuthErrorValue::NoTokenFound;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NoTokenFound"));
    }
}
