#![allow(non_local_definitions)]
use failure::{Backtrace, Fail};
use std::fmt;

#[derive(Debug)]
pub struct AuthError {
    pub value: AuthErrorValue,
}

#[derive(Debug, Fail)]
pub enum AuthErrorValue {
    #[fail(display = "token is not correct.")]
    TokenIsNotCorrect,
    #[fail(display = "no token found.")]
    NoTokenFound,
    #[fail(display = "invalid token format.")]
    InvalidTokenFormat,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for AuthError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== AuthErrorValue Display Tests ==========

    #[test]
    fn test_auth_error_value_token_not_correct_display() {
        let err = AuthErrorValue::TokenIsNotCorrect;
        assert_eq!(format!("{}", err), "token is not correct.");
    }

    #[test]
    fn test_auth_error_value_no_token_found_display() {
        let err = AuthErrorValue::NoTokenFound;
        assert_eq!(format!("{}", err), "no token found.");
    }

    #[test]
    fn test_auth_error_value_invalid_token_format_display() {
        let err = AuthErrorValue::InvalidTokenFormat;
        assert_eq!(format!("{}", err), "invalid token format.");
    }

    // ========== AuthError Display Tests ==========

    #[test]
    fn test_auth_error_display_token_not_correct() {
        let err = AuthError {
            value: AuthErrorValue::TokenIsNotCorrect,
        };
        assert_eq!(format!("{}", err), "token is not correct.");
    }

    #[test]
    fn test_auth_error_display_no_token_found() {
        let err = AuthError {
            value: AuthErrorValue::NoTokenFound,
        };
        assert_eq!(format!("{}", err), "no token found.");
    }

    #[test]
    fn test_auth_error_display_invalid_token_format() {
        let err = AuthError {
            value: AuthErrorValue::InvalidTokenFormat,
        };
        assert_eq!(format!("{}", err), "invalid token format.");
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
