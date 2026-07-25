//! Configuration storage and persistence.
//!
//! Handles loading and saving configuration from/to TOML files using
//! serde deserialization and atomic writes.

use std::fs;
use std::io;
use std::path::Path;

use thiserror::Error;

use super::types::AppConfig;

/// Errors that can occur during configuration storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// File I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// TOML parsing error.
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// TOML serialization error.
    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// Configuration file not found.
    #[error("Configuration file not found: {0}")]
    NotFound(String),

    /// Invalid configuration structure.
    #[error("Invalid configuration: {0}")]
    InvalidStructure(String),
}

/// Result type for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

/// Configuration storage handler.
pub struct ConfigStorage {
    /// Path to the configuration file.
    path: String,
}

impl ConfigStorage {
    /// Create a new storage handler for the given path.
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    /// Load configuration from a TOML file with validation.
    pub fn load(path: &str) -> StorageResult<AppConfig> {
        let content = fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        if let Err(errors) = config.validate() {
            return Err(StorageError::InvalidStructure(errors.join("; ")));
        }
        Ok(config)
    }

    /// Load configuration or create with defaults if file doesn't exist.
    pub fn load_or_default(path: &str) -> StorageResult<AppConfig> {
        if Path::new(path).exists() {
            Self::load(path)
        } else {
            Ok(AppConfig::default())
        }
    }

    /// Save configuration to the file atomically on a blocking thread.
    pub async fn save(&self, config: &AppConfig) -> StorageResult<()> {
        let content = toml::to_string_pretty(config)?;
        let path = self.path.clone();

        tokio::task::spawn_blocking(move || {
            super::file_ops::atomic_write(Path::new(&path), content.as_bytes(), None)
        })
        .await
        .map_err(|e| {
            StorageError::Io(io::Error::other(format!(
                "config save task panicked: {}",
                e
            )))
        })??;

        Ok(())
    }

    /// Get the file path.
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// Generate a sample configuration file content.
pub fn generate_sample_config() -> String {
    toml::to_string_pretty(&AppConfig::default()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_storage_load() {
        let toml_content = r#"[server]
port = 8080
auth_enabled = true
bind_address = "0.0.0.0"

[device]
manufacturer = "Test"
model = "TestModel"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", toml_content).unwrap();

        let config = ConfigStorage::load(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.server.port, 8080);
        assert!(config.server.auth_enabled);
        assert_eq!(config.device.manufacturer, "Test");
    }

    #[test]
    fn test_config_storage_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.server.port, 80);
        assert_eq!(config.device.manufacturer, "Anyka");
    }

    #[tokio::test]
    async fn test_config_storage_load_save() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let toml_content = r#"[server]
port = 9000

[device]
manufacturer = "LoadTest"
"#;
        write!(temp_file, "{}", toml_content).unwrap();

        // Load config
        let config = ConfigStorage::load(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.server.port, 9000);

        // Save config
        let storage = ConfigStorage::new(temp_file.path().to_str().unwrap());
        storage.save(&config).await.unwrap();

        // Reload and verify
        let reloaded = ConfigStorage::load(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(reloaded.server.port, 9000);
        assert_eq!(reloaded.device.manufacturer, "LoadTest");
    }

    #[test]
    fn test_config_storage_load_or_default() {
        let config = ConfigStorage::load_or_default("/nonexistent/path/config.toml").unwrap();
        assert_eq!(config.server.port, 80);
        assert_eq!(config.device.manufacturer, "Anyka");
    }

    #[test]
    fn test_generate_sample_config() {
        let sample = generate_sample_config();
        assert!(sample.contains("[server]"));
        assert!(sample.contains("[device]"));
        assert!(sample.contains("port = 80"));
    }

    #[tokio::test]
    async fn test_config_storage_save_creates_valid_toml() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = ConfigStorage::new(temp_file.path().to_str().unwrap());

        let mut config = AppConfig::default();
        config.server.port = 8080;
        config.server.auth_enabled = true;
        config.device.manufacturer = "Test".to_string();

        storage.save(&config).await.unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("[server]"));
        assert!(content.contains("port = 8080"));
        assert!(content.contains("auth_enabled = true"));
    }
}
