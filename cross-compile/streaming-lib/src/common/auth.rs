use indexmap::IndexMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::BuildHasherDefault;
use std::sync::Arc;

type AuthIndexMap = IndexMap<String, String, BuildHasherDefault<DefaultHasher>>;
use base64::Engine;
use md5;
use serde_derive::Deserialize;

use crate::common::errors::{AuthError, AuthErrorValue};
use crate::scanf;

const DEFAULT_BASIC_REALM: &str = "streaming-lib";

#[derive(Debug, Deserialize, Clone, Default)]
pub enum AuthAlgorithm {
    #[default]
    #[serde(rename = "simple")]
    Simple,
    #[serde(rename = "md5")]
    Md5,
}

pub enum SecretCarrier {
    Query(String),
    Bearer(String),
}

pub type CredentialValidator = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;

pub fn get_secret(carrier: &SecretCarrier) -> Result<String, AuthError> {
    match carrier {
        SecretCarrier::Query(query) => {
            let mut query_pairs = AuthIndexMap::default();
            let pars_array: Vec<&str> = query.split('&').collect();
            for ele in pars_array {
                let (k, v) = scanf!(ele, '=', String, String);
                if k.is_none() || v.is_none() {
                    continue;
                }
                query_pairs.insert(k.unwrap(), v.unwrap());
            }

            query_pairs.get("token").map_or(
                Err(AuthError {
                    value: AuthErrorValue::NoTokenFound,
                }),
                |t| Ok(t.to_string()),
            )
        }
        SecretCarrier::Bearer(header) => {
            let invalid_format = Err(AuthError {
                value: AuthErrorValue::InvalidTokenFormat,
            });
            let (prefix, token) = scanf!(header, " ", String, String);

            //if prefix.is_none() || token.is_none() {
            //    invalid_format
            //} else if prefix.unwrap() != "Bearer" {
            //    invalid_format
            //} else {
            //    Ok(token.unwrap())
            //}
            //fix cargo clippy --fix --allow-dirty --allow-no-vcs warnings
            match token {
                Some(token_val) => match prefix {
                    Some(prefix_val) => {
                        if prefix_val != "Bearer" {
                            invalid_format
                        } else {
                            Ok(token_val)
                        }
                    }
                    None => invalid_format,
                },
                None => invalid_format,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthType {
    Pull,
    Push,
    Both,
    None,
}
#[derive(Clone)]
pub struct Auth {
    algorithm: AuthAlgorithm,
    key: String,
    password: String,
    push_password: Option<String>,
    credential_validator: Option<CredentialValidator>,
    basic_realm: String,
    pub auth_type: AuthType,
}

impl Auth {
    pub fn new(
        key: String,
        password: String,
        push_password: Option<String>,
        algorithm: AuthAlgorithm,
        auth_type: AuthType,
    ) -> Self {
        Self {
            algorithm,
            key,
            password,
            push_password,
            credential_validator: None,
            basic_realm: DEFAULT_BASIC_REALM.to_string(),
            auth_type,
        }
    }

    pub fn with_credential_validator(mut self, validator: CredentialValidator) -> Self {
        self.credential_validator = Some(validator);
        self
    }

    pub fn with_basic_realm(mut self, realm: impl Into<String>) -> Self {
        let realm = realm.into();
        if !realm.is_empty() {
            self.basic_realm = realm;
        }
        self
    }

    pub fn basic_challenge(&self) -> String {
        format!("Basic realm=\"{}\"", self.basic_realm)
    }

    pub fn authenticate_request(
        &self,
        stream_name: &str,
        query_string: &Option<String>,
        authorization_header: Option<&str>,
        is_pull: bool,
    ) -> Result<(), AuthError> {
        if !self.requires_auth(is_pull) {
            return Ok(());
        }

        let mut auth_err = AuthErrorValue::NoTokenFound;

        if let Some(query) = query_string {
            let query_secret = Some(SecretCarrier::Query(query.clone()));
            match self.authenticate(stream_name, &query_secret, is_pull) {
                Ok(()) => return Ok(()),
                Err(err) => auth_err = err.value,
            }
        }

        if let Some(header) = authorization_header {
            let is_basic = header
                .trim_start()
                .get(..6)
                .map(|prefix| prefix.eq_ignore_ascii_case("Basic "))
                .unwrap_or(false);
            if is_basic {
                let (username, password) = parse_basic_credentials(header)?;
                if let Some(validator) = &self.credential_validator {
                    if validator(&username, &password) {
                        return Ok(());
                    }
                    auth_err = AuthErrorValue::InvalidCredentials;
                } else {
                    auth_err = AuthErrorValue::InvalidCredentials;
                }
            } else {
                let bearer_secret = Some(SecretCarrier::Bearer(header.to_string()));
                match self.authenticate(stream_name, &bearer_secret, is_pull) {
                    Ok(()) => return Ok(()),
                    Err(err) => auth_err = err.value,
                }
            }
        }

        Err(AuthError { value: auth_err })
    }

    pub fn authenticate(
        &self,
        stream_name: &str,
        secret: &Option<SecretCarrier>,
        is_pull: bool,
    ) -> Result<(), AuthError> {
        if self.requires_auth(is_pull) {
            let mut auth_err_reason: String = String::from("there is no token str found.");
            let mut err: AuthErrorValue = AuthErrorValue::NoTokenFound;

            /*Here we should do auth and it must be successful. */
            if let Some(secret_value) = secret {
                let token = get_secret(secret_value)?;
                if self.check(stream_name, token.as_str(), is_pull) {
                    return Ok(());
                }
                auth_err_reason = format!("token is not correct: {}", token);
                err = AuthErrorValue::TokenIsNotCorrect;
            }

            log::error!(
                "Auth error stream_name: {} auth type: {:?} pull: {} reason: {}",
                stream_name,
                self.auth_type,
                is_pull,
                auth_err_reason,
            );
            return Err(AuthError { value: err });
        }
        Ok(())
    }

    fn requires_auth(&self, is_pull: bool) -> bool {
        (self.auth_type == AuthType::Both)
            || (is_pull && (self.auth_type == AuthType::Pull))
            || (!is_pull && (self.auth_type == AuthType::Push))
    }

    fn check(&self, stream_name: &str, auth_str: &str, is_pull: bool) -> bool {
        let password = if is_pull {
            &self.password
        } else {
            self.push_password.as_ref().unwrap_or(&self.password)
        };

        match self.algorithm {
            AuthAlgorithm::Simple => password == auth_str,
            AuthAlgorithm::Md5 => {
                let raw_data = format!("{}{}", self.key, stream_name);
                let digest_str = format!("{:x}", md5::compute(raw_data));
                auth_str == digest_str
            }
        }
    }
}

fn parse_basic_credentials(header: &str) -> Result<(String, String), AuthError> {
    let trimmed = header.trim();
    let (scheme, encoded) = trimmed.split_once(' ').ok_or(AuthError {
        value: AuthErrorValue::InvalidTokenFormat,
    })?;

    if !scheme.eq_ignore_ascii_case("Basic") || encoded.is_empty() {
        return Err(AuthError {
            value: AuthErrorValue::InvalidTokenFormat,
        });
    }

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| AuthError {
            value: AuthErrorValue::InvalidCredentials,
        })?;
    let credential_text = std::str::from_utf8(&decoded).map_err(|_| AuthError {
        value: AuthErrorValue::InvalidCredentials,
    })?;
    let (username, password) = credential_text.split_once(':').ok_or(AuthError {
        value: AuthErrorValue::InvalidCredentials,
    })?;

    Ok((username.to_string(), password.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================
    // AuthAlgorithm Tests
    // ============================================

    #[test]
    fn test_auth_algorithm_default() {
        let algo = AuthAlgorithm::default();
        assert!(matches!(algo, AuthAlgorithm::Simple));
    }

    // ============================================
    // get_secret Tests - Query String
    // ============================================

    #[test]
    fn test_get_secret_query_success() {
        let carrier = SecretCarrier::Query("token=test123&other=value".to_string());
        let result = get_secret(&carrier);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test123");
    }

    #[test]
    fn test_get_secret_query_multiple_params() {
        let carrier = SecretCarrier::Query("key1=value1&token=my_token&key2=value2".to_string());
        let result = get_secret(&carrier);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "my_token");
    }

    #[test]
    fn test_get_secret_query_no_token() {
        let carrier = SecretCarrier::Query("key1=value1&key2=value2".to_string());
        let result = get_secret(&carrier);
        assert!(result.is_err());
        match result.unwrap_err().value {
            AuthErrorValue::NoTokenFound => {}
            _ => panic!("Expected NoTokenFound error"),
        }
    }

    #[test]
    fn test_get_secret_query_empty() {
        let carrier = SecretCarrier::Query("".to_string());
        let result = get_secret(&carrier);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_secret_query_token_only() {
        let carrier = SecretCarrier::Query("token=abc123".to_string());
        let result = get_secret(&carrier);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "abc123");
    }

    // ============================================
    // get_secret Tests - Bearer Token
    // ============================================

    #[test]
    fn test_get_secret_bearer_success() {
        let carrier = SecretCarrier::Bearer("Bearer test123".to_string());
        let result = get_secret(&carrier);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test123");
    }

    #[test]
    fn test_get_secret_bearer_invalid_prefix() {
        let carrier = SecretCarrier::Bearer("Basic test123".to_string());
        let result = get_secret(&carrier);
        assert!(result.is_err());
        match result.unwrap_err().value {
            AuthErrorValue::InvalidTokenFormat => {}
            _ => panic!("Expected InvalidTokenFormat error"),
        }
    }

    #[test]
    fn test_get_secret_bearer_no_space() {
        let carrier = SecretCarrier::Bearer("Bearertest123".to_string());
        let result = get_secret(&carrier);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_secret_bearer_lowercase() {
        let carrier = SecretCarrier::Bearer("bearer test123".to_string());
        let result = get_secret(&carrier);
        assert!(result.is_err()); // Must be "Bearer" (case-sensitive)
    }

    // ============================================
    // Auth Construction Tests
    // ============================================

    #[test]
    fn test_auth_new() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Both,
        );
        assert_eq!(auth.key, "key");
        assert_eq!(auth.password, "password");
        assert_eq!(auth.push_password, None);
        assert_eq!(auth.basic_realm, DEFAULT_BASIC_REALM);
        assert!(auth.credential_validator.is_none());
        assert_eq!(auth.auth_type, AuthType::Both);
    }

    #[test]
    fn test_auth_new_with_push_password() {
        let auth = Auth::new(
            "key".to_string(),
            "pull_password".to_string(),
            Some("push_password".to_string()),
            AuthAlgorithm::Simple,
            AuthType::Both,
        );
        assert_eq!(auth.password, "pull_password");
        assert_eq!(auth.push_password, Some("push_password".to_string()));
    }

    #[test]
    fn test_parse_basic_credentials_success() {
        let result = parse_basic_credentials("Basic YWRtaW46c2VjcmV0");
        assert!(result.is_ok());
        let (username, password) = result.expect("credentials");
        assert_eq!(username, "admin");
        assert_eq!(password, "secret");
    }

    #[test]
    fn test_parse_basic_credentials_invalid_format() {
        let result = parse_basic_credentials("Digest xyz");
        assert!(result.is_err());
        assert!(matches!(
            result.expect_err("error").value,
            AuthErrorValue::InvalidTokenFormat
        ));
    }

    #[test]
    fn test_parse_basic_credentials_invalid_payload() {
        let result = parse_basic_credentials("Basic invalid$$");
        assert!(result.is_err());
        assert!(matches!(
            result.expect_err("error").value,
            AuthErrorValue::InvalidCredentials
        ));
    }

    #[test]
    fn test_basic_challenge_default_and_custom_realm() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Pull,
        );
        assert_eq!(auth.basic_challenge(), "Basic realm=\"streaming-lib\"");

        let custom = auth.with_basic_realm("ONVIF Camera");
        assert_eq!(custom.basic_challenge(), "Basic realm=\"ONVIF Camera\"");
    }

    #[test]
    fn test_authenticate_request_query_token_success() {
        let auth = Auth::new(
            "key".to_string(),
            "password123".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Pull,
        );
        let result = auth.authenticate_request(
            "stream1",
            &Some("token=password123".to_string()),
            None,
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_authenticate_request_basic_credentials_success() {
        let auth = Auth::new(
            "key".to_string(),
            "password123".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Pull,
        )
        .with_credential_validator(Arc::new(|username, password| {
            username == "admin" && password == "secret"
        }));
        let result =
            auth.authenticate_request("stream1", &None, Some("Basic YWRtaW46c2VjcmV0"), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_authenticate_request_basic_credentials_failure() {
        let auth = Auth::new(
            "key".to_string(),
            "password123".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Pull,
        )
        .with_credential_validator(Arc::new(|username, password| {
            username == "admin" && password == "secret"
        }));
        let result =
            auth.authenticate_request("stream1", &None, Some("Basic YWRtaW46d3Jvbmc="), true);
        assert!(result.is_err());
        assert!(matches!(
            result.expect_err("error").value,
            AuthErrorValue::InvalidCredentials
        ));
    }

    // ============================================
    // Simple Authentication Tests
    // ============================================

    #[test]
    fn test_auth_simple_success() {
        let auth = Auth::new(
            "key".to_string(),
            "password123".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Both,
        );
        let stream_name = "test_stream".to_string();
        let secret = Some(SecretCarrier::Query("token=password123".to_string()));
        let result = auth.authenticate(&stream_name, &secret, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_auth_simple_failure() {
        let auth = Auth::new(
            "key".to_string(),
            "password123".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Both,
        );
        let stream_name = "test_stream".to_string();
        let secret = Some(SecretCarrier::Query("token=wrong_password".to_string()));
        let result = auth.authenticate(&stream_name, &secret, true);
        assert!(result.is_err());
        match result.unwrap_err().value {
            AuthErrorValue::TokenIsNotCorrect => {}
            _ => panic!("Expected TokenIsNotCorrect error"),
        }
    }

    #[test]
    fn test_auth_simple_bearer_token() {
        let auth = Auth::new(
            "key".to_string(),
            "password123".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Both,
        );
        let stream_name = "test_stream".to_string();
        let secret = Some(SecretCarrier::Bearer("Bearer password123".to_string()));
        let result = auth.authenticate(&stream_name, &secret, true);
        assert!(result.is_ok());
    }

    // ============================================
    // MD5 Authentication Tests
    // ============================================

    #[test]
    fn test_auth_md5_success() {
        let auth = Auth::new(
            "secret_key".to_string(),
            "".to_string(), // Not used for MD5
            None,
            AuthAlgorithm::Md5,
            AuthType::Both,
        );
        let stream_name = "test_stream".to_string();

        // Calculate expected MD5: md5("secret_key" + "test_stream")
        let raw_data = format!("{}{}", "secret_key", "test_stream");
        let digest = md5::compute(raw_data);
        let digest_str = format!("{:x}", digest);

        let secret = Some(SecretCarrier::Query(format!("token={}", digest_str)));
        let result = auth.authenticate(&stream_name, &secret, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_auth_md5_failure() {
        let auth = Auth::new(
            "secret_key".to_string(),
            "".to_string(),
            None,
            AuthAlgorithm::Md5,
            AuthType::Both,
        );
        let stream_name = "test_stream".to_string();
        let secret = Some(SecretCarrier::Query("token=wrong_digest".to_string()));
        let result = auth.authenticate(&stream_name, &secret, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_md5_different_stream_names() {
        let auth = Auth::new(
            "key".to_string(),
            "".to_string(),
            None,
            AuthAlgorithm::Md5,
            AuthType::Both,
        );

        // Stream 1
        let stream1 = "stream1".to_string();
        let raw_data1 = format!("{}{}", "key", "stream1");
        let digest1 = format!("{:x}", md5::compute(raw_data1));
        let secret1 = Some(SecretCarrier::Query(format!("token={}", digest1)));
        assert!(auth.authenticate(&stream1, &secret1, true).is_ok());

        // Stream 2 (different digest)
        let stream2 = "stream2".to_string();
        let raw_data2 = format!("{}{}", "key", "stream2");
        let digest2 = format!("{:x}", md5::compute(raw_data2));
        let secret2 = Some(SecretCarrier::Query(format!("token={}", digest2)));
        assert!(auth.authenticate(&stream2, &secret2, true).is_ok());

        // Stream 1 token should not work for stream 2
        assert!(auth.authenticate(&stream2, &secret1, true).is_err());
    }

    // ============================================
    // Auth Type Tests
    // ============================================

    #[test]
    fn test_auth_type_pull_only_success() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Pull,
        );
        let stream_name = "test_stream".to_string();
        let secret = Some(SecretCarrier::Query("token=password".to_string()));

        // Pull request should require auth
        let result = auth.authenticate(&stream_name, &secret, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_auth_type_pull_only_failure_no_secret() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Pull,
        );
        let stream_name = "test_stream".to_string();
        let secret = None;

        // Pull request without secret should fail
        let result = auth.authenticate(&stream_name, &secret, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_type_pull_only_push_allowed() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Pull,
        );
        let stream_name = "test_stream".to_string();
        let secret = None;

        // Push request should be allowed (no auth required)
        let result = auth.authenticate(&stream_name, &secret, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_auth_type_push_only_success() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Push,
        );
        let stream_name = "test_stream".to_string();
        let secret = Some(SecretCarrier::Query("token=password".to_string()));

        // Push request should require auth
        let result = auth.authenticate(&stream_name, &secret, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_auth_type_push_only_pull_allowed() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Push,
        );
        let stream_name = "test_stream".to_string();
        let secret = None;

        // Pull request should be allowed (no auth required)
        let result = auth.authenticate(&stream_name, &secret, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_auth_type_both_requires_auth() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Both,
        );
        let stream_name = "test_stream".to_string();
        let secret = None;

        // Both pull and push should require auth
        assert!(auth.authenticate(&stream_name, &secret, true).is_err());
        assert!(auth.authenticate(&stream_name, &secret, false).is_err());
    }

    #[test]
    fn test_auth_type_none_allows_all() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::None,
        );
        let stream_name = "test_stream".to_string();
        let secret = None;

        // No auth required for any request
        assert!(auth.authenticate(&stream_name, &secret, true).is_ok());
        assert!(auth.authenticate(&stream_name, &secret, false).is_ok());
    }

    // ============================================
    // Push Password Tests
    // ============================================

    #[test]
    fn test_auth_push_password_different() {
        let auth = Auth::new(
            "key".to_string(),
            "pull_password".to_string(),
            Some("push_password".to_string()),
            AuthAlgorithm::Simple,
            AuthType::Both,
        );
        let stream_name = "test_stream".to_string();

        // Pull should use pull_password
        let pull_secret = Some(SecretCarrier::Query("token=pull_password".to_string()));
        assert!(auth.authenticate(&stream_name, &pull_secret, true).is_ok());
        let pull_secret_wrong = Some(SecretCarrier::Query("token=push_password".to_string()));
        assert!(
            auth.authenticate(&stream_name, &pull_secret_wrong, true)
                .is_err()
        );

        // Push should use push_password
        let push_secret = Some(SecretCarrier::Query("token=push_password".to_string()));
        assert!(auth.authenticate(&stream_name, &push_secret, false).is_ok());
        let push_secret_wrong = Some(SecretCarrier::Query("token=pull_password".to_string()));
        assert!(
            auth.authenticate(&stream_name, &push_secret_wrong, false)
                .is_err()
        );
    }

    #[test]
    fn test_auth_push_password_fallback() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None, // No push password specified
            AuthAlgorithm::Simple,
            AuthType::Both,
        );
        let stream_name = "test_stream".to_string();

        // Push should fall back to pull password
        let secret = Some(SecretCarrier::Query("token=password".to_string()));
        assert!(auth.authenticate(&stream_name, &secret, false).is_ok());
    }

    // ============================================
    // Error Handling Tests
    // ============================================

    #[test]
    fn test_auth_no_token_found() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Both,
        );
        let stream_name = "test_stream".to_string();
        let secret = Some(SecretCarrier::Query("key=value".to_string())); // No token param
        let result = auth.authenticate(&stream_name, &secret, true);
        assert!(result.is_err());
        match result.unwrap_err().value {
            AuthErrorValue::NoTokenFound => {}
            _ => panic!("Expected NoTokenFound error"),
        }
    }

    #[test]
    fn test_auth_invalid_bearer_format() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Both,
        );
        let stream_name = "test_stream".to_string();
        let secret = Some(SecretCarrier::Bearer("InvalidFormat".to_string()));
        let result = auth.authenticate(&stream_name, &secret, true);
        assert!(result.is_err());
        match result.unwrap_err().value {
            AuthErrorValue::InvalidTokenFormat => {}
            _ => panic!("Expected InvalidTokenFormat error"),
        }
    }

    // ============================================
    // Stream Name Validation Tests
    // ============================================

    #[test]
    fn test_auth_different_stream_names() {
        let auth = Auth::new(
            "key".to_string(),
            "password".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Both,
        );

        // Same password should work for different stream names with Simple auth
        let stream1 = "stream1".to_string();
        let stream2 = "stream2".to_string();
        let secret = Some(SecretCarrier::Query("token=password".to_string()));

        assert!(auth.authenticate(&stream1, &secret, true).is_ok());
        assert!(auth.authenticate(&stream2, &secret, true).is_ok());
    }

    #[test]
    fn test_auth_md5_stream_name_matters() {
        let auth = Auth::new(
            "key".to_string(),
            "".to_string(),
            None,
            AuthAlgorithm::Md5,
            AuthType::Both,
        );

        let stream1 = "stream1".to_string();
        let raw_data1 = format!("{}{}", "key", "stream1");
        let digest1 = format!("{:x}", md5::compute(raw_data1));
        let secret1 = Some(SecretCarrier::Query(format!("token={}", digest1)));

        // Should work for stream1
        assert!(auth.authenticate(&stream1, &secret1, true).is_ok());

        // Should not work for stream2
        let stream2 = "stream2".to_string();
        assert!(auth.authenticate(&stream2, &secret1, true).is_err());
    }

    // ============================================
    // Edge Cases
    // ============================================

    #[test]
    fn test_auth_empty_password() {
        let auth = Auth::new(
            "key".to_string(),
            "".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Both,
        );
        let stream_name = "test_stream".to_string();
        let secret = Some(SecretCarrier::Query("token=".to_string()));
        let result = auth.authenticate(&stream_name, &secret, true);
        assert!(result.is_ok()); // Empty password matches empty token
    }

    #[test]
    fn test_auth_empty_key_md5() {
        let auth = Auth::new(
            "".to_string(),
            "".to_string(),
            None,
            AuthAlgorithm::Md5,
            AuthType::Both,
        );
        let stream_name = "test_stream".to_string();
        let raw_data = format!("{}{}", "", "test_stream");
        let digest = format!("{:x}", md5::compute(raw_data));
        let secret = Some(SecretCarrier::Query(format!("token={}", digest)));
        let result = auth.authenticate(&stream_name, &secret, true);
        assert!(result.is_ok());
    }
}
