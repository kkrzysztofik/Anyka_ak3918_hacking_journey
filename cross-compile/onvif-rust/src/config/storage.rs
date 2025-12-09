//! Configuration storage and persistence.
//!
//! This module handles loading and saving configuration from/to TOML files
//! with atomic write support.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use thiserror::Error;

use super::runtime::ApplicationConfig;
use super::schema::ConfigSchema;

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
    #[error("Invalid configuration structure: {0}")]
    InvalidStructure(String),
}

/// Result type for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

/// Configuration storage handler.
pub struct ConfigStorage {
    /// Path to the configuration file.
    path: String,
    /// Configuration schema.
    schema: ConfigSchema,
}

impl ConfigStorage {
    /// Create a new storage handler for the given path.
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            schema: ConfigSchema::new(),
        }
    }

    /// Load configuration from the file.
    pub fn load(path: &str) -> StorageResult<ApplicationConfig> {
        let storage = Self::new(path);
        storage.read()
    }

    /// Load configuration or create with defaults if file doesn't exist.
    pub fn load_or_default(path: &str) -> StorageResult<ApplicationConfig> {
        let storage = Self::new(path);

        if Path::new(path).exists() {
            storage.read()
        } else {
            Ok(storage.create_default())
        }
    }

    /// Read configuration from the file.
    pub fn read(&self) -> StorageResult<ApplicationConfig> {
        let content = fs::read_to_string(&self.path)?;
        self.parse_toml(&content)
    }

    /// Parse TOML content into ApplicationConfig.
    fn parse_toml(&self, content: &str) -> StorageResult<ApplicationConfig> {
        let toml_value: toml::Value = toml::from_str(content)?;
        let mut config = ApplicationConfig::new();

        // Parse TOML table recursively
        if let toml::Value::Table(table) = toml_value {
            self.parse_table(&mut config, &table, "");
        }

        // Apply defaults for missing values
        self.apply_defaults(&mut config);

        Ok(config)
    }

    /// Parse a TOML table recursively.
    #[allow(clippy::only_used_in_recursion)]
    fn parse_table(
        &self,
        config: &mut ApplicationConfig,
        table: &toml::map::Map<String, toml::Value>,
        prefix: &str,
    ) {
        for (key, value) in table {
            let full_key = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", prefix, key)
            };

            match value {
                toml::Value::Table(subtable) => {
                    self.parse_table(config, subtable, &full_key);
                }
                toml::Value::String(s) => {
                    config.set(&full_key, s.clone());
                }
                toml::Value::Integer(i) => {
                    config.set(&full_key, i.to_string());
                }
                toml::Value::Float(f) => {
                    config.set(&full_key, f.to_string());
                }
                toml::Value::Boolean(b) => {
                    config.set(&full_key, b.to_string());
                }
                toml::Value::Array(arr) => {
                    // Store arrays as comma-separated values
                    let values: Vec<String> = arr
                        .iter()
                        .map(|v| match v {
                            toml::Value::String(s) => s.clone(),
                            toml::Value::Integer(i) => i.to_string(),
                            toml::Value::Float(f) => f.to_string(),
                            toml::Value::Boolean(b) => b.to_string(),
                            _ => String::new(),
                        })
                        .collect();
                    config.set(&full_key, values.join(","));
                }
                toml::Value::Datetime(dt) => {
                    config.set(&full_key, dt.to_string());
                }
            }
        }
    }

    /// Apply default values for missing configuration keys.
    fn apply_defaults(&self, config: &mut ApplicationConfig) {
        for section in self.schema.sections() {
            for param in &section.parameters {
                if !config.contains(&param.key) && param.required {
                    config.set(&param.key, param.default.clone());
                }
            }
        }
    }

    /// Create a default configuration.
    pub fn create_default(&self) -> ApplicationConfig {
        let mut config = ApplicationConfig::new();
        self.apply_defaults(&mut config);
        config
    }

    /// Save configuration to the file atomically.
    ///
    /// Uses the temp-file-then-rename pattern to ensure atomic writes.
    pub fn save(&self, config: &ApplicationConfig) -> StorageResult<()> {
        let toml_content = self.to_toml(config)?;

        // Write to temp file first
        let temp_path = format!("{}.tmp", self.path);
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(toml_content.as_bytes())?;
        file.sync_all()?;

        // Rename temp file to actual file (atomic on most filesystems)
        fs::rename(&temp_path, &self.path)?;

        Ok(())
    }

    /// Convert configuration to TOML string.
    fn to_toml(&self, config: &ApplicationConfig) -> StorageResult<String> {
        // Build a structured TOML table
        let mut root = toml::map::Map::new();

        for (key, value) in config.values() {
            let parts: Vec<&str> = key.split('.').collect();
            if parts.len() == 2 {
                let section = parts[0];
                let param = parts[1];

                let table = root
                    .entry(section.to_string())
                    .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

                if let toml::Value::Table(t) = table {
                    // Try to convert to appropriate type
                    let toml_value = self.string_to_toml_value(key, value);
                    t.insert(param.to_string(), toml_value);
                }
            }
        }

        let toml_value = toml::Value::Table(root);
        Ok(toml::to_string_pretty(&toml_value)?)
    }

    /// Convert a string value to the appropriate TOML value based on schema.
    fn string_to_toml_value(&self, key: &str, value: &str) -> toml::Value {
        use super::schema::ConfigValueType;

        if let Some(param) = self.schema.get_parameter(key) {
            match param.value_type {
                ConfigValueType::Int => {
                    if let Ok(i) = value.parse::<i64>() {
                        return toml::Value::Integer(i);
                    }
                }
                ConfigValueType::Float => {
                    if let Ok(f) = value.parse::<f64>() {
                        return toml::Value::Float(f);
                    }
                }
                ConfigValueType::Bool => {
                    if let Ok(b) = value.parse::<bool>() {
                        return toml::Value::Boolean(b);
                    }
                }
                ConfigValueType::String => {
                    return toml::Value::String(value.to_string());
                }
            }
        }

        // Default to string
        toml::Value::String(value.to_string())
    }

    /// Get the file path.
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// Generate a sample configuration file content.
pub fn generate_sample_config() -> String {
    let schema = ConfigSchema::new();
    let mut output = String::new();

    output.push_str("# ONVIF Rust Configuration\n");
    output.push_str("# Generated sample configuration file\n\n");

    for section in schema.sections() {
        output.push_str(&format!("# {}\n", section.description));
        output.push_str(&format!("[{}]\n", section.name));

        for param in &section.parameters {
            // Remove section prefix from key for display
            let param_name = param
                .key
                .strip_prefix(&format!("{}.", section.name))
                .unwrap_or(&param.key);

            output.push_str(&format!("# {}\n", param.description));

            // Format value based on type
            use super::schema::ConfigValueType;
            let value = match param.value_type {
                ConfigValueType::String => format!("\"{}\"", param.default),
                ConfigValueType::Int | ConfigValueType::Float | ConfigValueType::Bool => {
                    param.default.clone()
                }
            };

            output.push_str(&format!("{} = {}\n", param_name, value));
            output.push('\n');
        }

        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_storage_parse_toml() {
        let toml_content = r#"[server]
port = 8080
auth_enabled = true
bind_address = "0.0.0.0"

[device]
manufacturer = "Test"
model = "TestModel"
"#;

        let storage = ConfigStorage::new("/tmp/test.toml");
        let config = storage.parse_toml(toml_content).unwrap();

        assert_eq!(config.get("server.port"), Some(&"8080".to_string()));
        assert_eq!(config.get("server.auth_enabled"), Some(&"true".to_string()));
        assert_eq!(config.get("device.manufacturer"), Some(&"Test".to_string()));
    }

    #[test]
    fn test_config_storage_defaults() {
        let storage = ConfigStorage::new("/tmp/test.toml");
        let config = storage.create_default();

        // Check that defaults are applied
        assert!(config.contains("server.port"));
        assert!(config.contains("device.manufacturer"));
    }

    #[test]
    fn test_config_storage_load_save() {
        // Create a temp file
        let mut temp_file = NamedTempFile::new().unwrap();
        let toml_content = r#"[server]
port = 9000

[device]
manufacturer = "LoadTest"
"#;
        write!(temp_file, "{}", toml_content).unwrap();

        // Load config
        let config = ConfigStorage::load(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.get("server.port"), Some(&"9000".to_string()));

        // Save config
        let storage = ConfigStorage::new(temp_file.path().to_str().unwrap());
        storage.save(&config).unwrap();

        // Reload and verify
        let reloaded = ConfigStorage::load(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(reloaded.get("server.port"), Some(&"9000".to_string()));
    }

    #[test]
    fn test_config_storage_load_or_default() {
        // Non-existent file should return defaults
        let config = ConfigStorage::load_or_default("/nonexistent/path/config.toml").unwrap();
        assert!(config.contains("server.port"));
    }

    #[test]
    fn test_generate_sample_config() {
        let sample = generate_sample_config();
        assert!(sample.contains("[server]"));
        assert!(sample.contains("[device]"));
        assert!(sample.contains("port"));
    }

    #[test]
    fn test_config_storage_to_toml() {
        let storage = ConfigStorage::new("/tmp/test.toml");
        let mut config = ApplicationConfig::new();
        config.set("server.port", "8080".to_string());
        config.set("server.auth_enabled", "true".to_string());
        config.set("device.manufacturer", "Test".to_string());

        let toml_str = storage.to_toml(&config).unwrap();
        assert!(toml_str.contains("[server]"));
        assert!(toml_str.contains("port = 8080"));
        assert!(toml_str.contains("auth_enabled = true"));
    }
}
