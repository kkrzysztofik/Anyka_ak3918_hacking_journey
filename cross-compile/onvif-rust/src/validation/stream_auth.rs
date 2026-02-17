use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::config::ConfigRuntime;
use crate::users::{PasswordManager, UserStorage};
use streaming_lib::common::auth::{Auth, AuthAlgorithm, AuthType, CredentialValidator};

fn users_file_path(config_path: &str) -> PathBuf {
    Path::new(config_path)
        .parent()
        .unwrap_or(Path::new("/etc/onvif"))
        .join("users.toml")
}

fn generate_unpredictable_stream_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Build stream authentication for validation mode using ONVIF user credentials.
///
/// When `server.auth_enabled` is `false`, stream authentication is disabled.
/// When `server.auth_enabled` is `true`, this loads `<config_dir>/users.toml`
/// and builds a Basic-auth validator backed by `UserStorage` + `PasswordManager`.
pub fn build_stream_auth_for_validation_mode(
    config: &ConfigRuntime,
    config_path: &str,
) -> Result<Option<Auth>> {
    let auth_enabled = config.get_bool("server.auth_enabled").unwrap_or(true);
    if !auth_enabled {
        tracing::info!(
            "Validation mode stream authentication disabled (server.auth_enabled=false)"
        );
        return Ok(None);
    }

    let users_path = users_file_path(config_path);
    if !users_path.exists() {
        bail!(
            "Stream authentication is enabled but users file was not found: {}",
            users_path.display()
        );
    }

    let user_storage = UserStorage::new();
    user_storage
        .load_from_toml(&users_path)
        .with_context(|| format!("Failed to load users from {}", users_path.display()))?;
    if user_storage.is_empty() {
        bail!(
            "Stream authentication is enabled but users file is empty: {}",
            users_path.display()
        );
    }

    let user_storage = Arc::new(user_storage);
    let password_manager = Arc::new(PasswordManager::new());
    let validator_storage = Arc::clone(&user_storage);
    let validator_password_manager = Arc::clone(&password_manager);
    let validator: CredentialValidator = Arc::new(move |username, password| {
        validator_storage
            .get_user(username)
            .map(|user| validator_password_manager.verify_password(password, &user.password))
            .unwrap_or(false)
    });

    let realm = config
        .get_string("server.realm")
        .unwrap_or_else(|_| "ONVIF Camera".to_string());
    let auth = Auth::new(
        String::new(),
        generate_unpredictable_stream_token(),
        None,
        AuthAlgorithm::Simple,
        AuthType::Pull,
    )
    .with_credential_validator(validator)
    .with_basic_realm(realm);

    tracing::info!(
        users_path = %users_path.display(),
        "Validation mode stream authentication enabled"
    );
    Ok(Some(auth))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::users::UserLevel;

    fn make_config(auth_enabled: bool) -> ConfigRuntime {
        let config = ConfigRuntime::new(Default::default());
        config
            .set_bool("server.auth_enabled", auth_enabled)
            .expect("set auth_enabled");
        config
            .set_string("server.realm", "ONVIF Camera")
            .expect("set realm");
        config
    }

    #[test]
    fn test_build_stream_auth_validation_mode_auth_disabled_returns_none() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = temp.path().join("config.toml");
        let config = make_config(false);

        let auth = build_stream_auth_for_validation_mode(
            &config,
            config_path.to_str().expect("config path"),
        )
        .expect("build auth");
        assert!(auth.is_none());
    }

    #[test]
    fn test_build_stream_auth_validation_mode_missing_users_returns_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = temp.path().join("config.toml");
        let config = make_config(true);

        let result = build_stream_auth_for_validation_mode(
            &config,
            config_path.to_str().expect("config path"),
        );
        let message = match result {
            Ok(_) => panic!("expected missing users file error"),
            Err(error) => error.to_string(),
        };
        assert!(message.contains("users file was not found"));
    }

    #[test]
    fn test_build_stream_auth_validation_mode_empty_users_returns_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = temp.path().join("config.toml");
        let users_path = temp.path().join("users.toml");
        let config = make_config(true);

        std::fs::write(&users_path, "users = []\n").expect("write empty users file");

        let result = build_stream_auth_for_validation_mode(
            &config,
            config_path.to_str().expect("config path"),
        );
        let message = match result {
            Ok(_) => panic!("expected empty users file error"),
            Err(error) => error.to_string(),
        };
        assert!(message.contains("users file is empty"));
    }

    #[test]
    fn test_build_stream_auth_validation_mode_validates_basic_credentials() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = temp.path().join("config.toml");
        let users_path = temp.path().join("users.toml");
        let config = make_config(true);

        let storage = UserStorage::new();
        storage
            .create_user("admin", "secret", UserLevel::Administrator)
            .expect("create user");
        storage.save_to_toml(&users_path).expect("save users");

        let auth = build_stream_auth_for_validation_mode(
            &config,
            config_path.to_str().expect("config path"),
        )
        .expect("build auth")
        .expect("auth present");

        let ok = auth.authenticate_request("stream1", &None, Some("Basic YWRtaW46c2VjcmV0"), true);
        assert!(ok.is_ok());

        let fail =
            auth.authenticate_request("stream1", &None, Some("Basic YWRtaW46d3Jvbmc="), true);
        assert!(fail.is_err());

        let token_fallback = auth.authenticate_request(
            "stream1",
            &Some("token=__unused_token__".to_string()),
            None,
            true,
        );
        assert!(token_fallback.is_err());
    }
}
