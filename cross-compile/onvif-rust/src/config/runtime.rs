//! Runtime configuration manager with thread-safe access.
//!
//! This module provides thread-safe runtime access to configuration values
//! with change detection via a generation counter.

use std::collections::HashMap;
use std::sync::atomic::Ordering;

use portable_atomic::AtomicU64;
use std::sync::Arc;

use parking_lot::RwLock;
use thiserror::Error;

use super::schema::{ConfigSchema, ConfigValidationError, ConfigValueType};

/// Errors that can occur during configuration operations.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Configuration key not found.
    #[error("Configuration key not found: {0}")]
    KeyNotFound(String),

    /// Invalid configuration value type.
    #[error("Invalid type for key {key}: expected {expected}")]
    InvalidType { key: String, expected: String },

    /// Configuration validation failed.
    #[error("Validation error: {0}")]
    ValidationError(#[from] ConfigValidationError),

    /// Configuration parse error.
    #[error("Parse error for key {key}: {message}")]
    ParseError { key: String, message: String },
}

/// Result type for configuration operations.
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Application configuration data.
#[derive(Debug, Clone, Default)]
pub struct ApplicationConfig {
    /// Configuration values stored as strings.
    values: HashMap<String, String>,
}

impl ApplicationConfig {
    /// Create a new empty configuration.
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<&String> {
        self.values.get(key)
    }

    /// Set a value by key.
    pub fn set(&mut self, key: &str, value: String) {
        self.values.insert(key.to_string(), value);
    }

    /// Check if a key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Get all keys.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }

    /// Get all values as a reference.
    pub fn values(&self) -> &HashMap<String, String> {
        &self.values
    }

    /// Remove a key.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.values.remove(key)
    }
}

/// Thread-safe runtime configuration manager.
///
/// Provides atomic access to configuration values with change detection.
pub struct ConfigRuntime {
    /// Configuration data protected by RwLock.
    config: Arc<RwLock<ApplicationConfig>>,
    /// Configuration schema for validation and defaults.
    schema: ConfigSchema,
    /// Generation counter for change detection.
    generation: AtomicU64,
}

impl ConfigRuntime {
    /// Create a new runtime configuration manager.
    pub fn new(config: ApplicationConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            schema: ConfigSchema::new(),
            generation: AtomicU64::new(0),
        }
    }

    /// Create a new runtime configuration with a custom schema.
    pub fn with_schema(config: ApplicationConfig, schema: ConfigSchema) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            schema,
            generation: AtomicU64::new(0),
        }
    }

    /// Get the current configuration generation.
    ///
    /// This can be used for change detection - compare saved generation
    /// with current to detect if configuration has changed.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Increment the generation counter.
    fn bump_generation(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// Get an integer configuration value.
    pub fn get_int(&self, key: &str) -> ConfigResult<i64> {
        let config = self.config.read();

        // Try to get from config first
        if let Some(value) = config.get(key) {
            return value.parse().map_err(|_| ConfigError::ParseError {
                key: key.to_string(),
                message: "not a valid integer".to_string(),
            });
        }

        // Fall back to default
        self.schema
            .default_value(key)
            .ok_or_else(|| ConfigError::KeyNotFound(key.to_string()))?
            .parse()
            .map_err(|_| ConfigError::ParseError {
                key: key.to_string(),
                message: "default is not a valid integer".to_string(),
            })
    }

    /// Set an integer configuration value.
    pub fn set_int(&self, key: &str, value: i64) -> ConfigResult<()> {
        let value_str = value.to_string();

        // Validate if parameter exists in schema
        if let Some(param) = self.schema.get_parameter(key) {
            if param.value_type != ConfigValueType::Int {
                return Err(ConfigError::InvalidType {
                    key: key.to_string(),
                    expected: "integer".to_string(),
                });
            }
            self.schema.validate(key, &value_str)?;
        }

        let mut config = self.config.write();
        config.set(key, value_str);
        self.bump_generation();
        Ok(())
    }

    /// Get a string configuration value.
    pub fn get_string(&self, key: &str) -> ConfigResult<String> {
        let config = self.config.read();

        // Try to get from config first
        if let Some(value) = config.get(key) {
            return Ok(value.clone());
        }

        // Fall back to default
        self.schema
            .default_value(key)
            .map(|s| s.to_string())
            .ok_or_else(|| ConfigError::KeyNotFound(key.to_string()))
    }

    /// Set a string configuration value.
    pub fn set_string(&self, key: &str, value: &str) -> ConfigResult<()> {
        // Validate if parameter exists in schema
        if let Some(param) = self.schema.get_parameter(key) {
            if param.value_type != ConfigValueType::String {
                return Err(ConfigError::InvalidType {
                    key: key.to_string(),
                    expected: "string".to_string(),
                });
            }
            self.schema.validate(key, value)?;
        }

        let mut config = self.config.write();
        config.set(key, value.to_string());
        self.bump_generation();
        Ok(())
    }

    /// Get a boolean configuration value.
    pub fn get_bool(&self, key: &str) -> ConfigResult<bool> {
        let config = self.config.read();

        // Try to get from config first
        if let Some(value) = config.get(key) {
            return value.parse().map_err(|_| ConfigError::ParseError {
                key: key.to_string(),
                message: "not a valid boolean".to_string(),
            });
        }

        // Fall back to default
        self.schema
            .default_value(key)
            .ok_or_else(|| ConfigError::KeyNotFound(key.to_string()))?
            .parse()
            .map_err(|_| ConfigError::ParseError {
                key: key.to_string(),
                message: "default is not a valid boolean".to_string(),
            })
    }

    /// Set a boolean configuration value.
    pub fn set_bool(&self, key: &str, value: bool) -> ConfigResult<()> {
        let value_str = value.to_string();

        // Validate if parameter exists in schema
        if let Some(param) = self.schema.get_parameter(key) {
            if param.value_type != ConfigValueType::Bool {
                return Err(ConfigError::InvalidType {
                    key: key.to_string(),
                    expected: "boolean".to_string(),
                });
            }
            self.schema.validate(key, &value_str)?;
        }

        let mut config = self.config.write();
        config.set(key, value_str);
        self.bump_generation();
        Ok(())
    }

    /// Get a float configuration value.
    pub fn get_float(&self, key: &str) -> ConfigResult<f64> {
        let config = self.config.read();

        // Try to get from config first
        if let Some(value) = config.get(key) {
            return value.parse().map_err(|_| ConfigError::ParseError {
                key: key.to_string(),
                message: "not a valid float".to_string(),
            });
        }

        // Fall back to default
        self.schema
            .default_value(key)
            .ok_or_else(|| ConfigError::KeyNotFound(key.to_string()))?
            .parse()
            .map_err(|_| ConfigError::ParseError {
                key: key.to_string(),
                message: "default is not a valid float".to_string(),
            })
    }

    /// Set a float configuration value.
    pub fn set_float(&self, key: &str, value: f64) -> ConfigResult<()> {
        let value_str = value.to_string();

        // Validate if parameter exists in schema
        if let Some(param) = self.schema.get_parameter(key) {
            if param.value_type != ConfigValueType::Float {
                return Err(ConfigError::InvalidType {
                    key: key.to_string(),
                    expected: "float".to_string(),
                });
            }
            self.schema.validate(key, &value_str)?;
        }

        let mut config = self.config.write();
        config.set(key, value_str);
        self.bump_generation();
        Ok(())
    }

    /// Get a snapshot of all configuration values.
    pub fn snapshot(&self) -> ApplicationConfig {
        self.config.read().clone()
    }

    /// Replace the entire configuration.
    pub fn replace(&self, config: ApplicationConfig) {
        *self.config.write() = config;
        self.bump_generation();
    }

    /// Get access to the schema.
    pub fn schema(&self) -> &ConfigSchema {
        &self.schema
    }
}

impl Clone for ConfigRuntime {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            schema: self.schema.clone(),
            generation: AtomicU64::new(self.generation.load(Ordering::SeqCst)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_config() {
        let mut config = ApplicationConfig::new();
        config.set("test.key", "value".to_string());

        assert!(config.contains("test.key"));
        assert_eq!(config.get("test.key"), Some(&"value".to_string()));

        config.remove("test.key");
        assert!(!config.contains("test.key"));
    }

    #[test]
    fn test_config_runtime_int() {
        let config = ApplicationConfig::new();
        let runtime = ConfigRuntime::new(config);

        // Get default value
        let port = runtime.get_int("server.port").unwrap();
        assert_eq!(port, 80);

        // Set value
        runtime.set_int("server.port", 8080).unwrap();
        let port = runtime.get_int("server.port").unwrap();
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_config_runtime_string() {
        let config = ApplicationConfig::new();
        let runtime = ConfigRuntime::new(config);

        // Get default value
        let manufacturer = runtime.get_string("device.manufacturer").unwrap();
        assert_eq!(manufacturer, "Anyka");

        // Set value
        runtime.set_string("device.manufacturer", "Custom").unwrap();
        let manufacturer = runtime.get_string("device.manufacturer").unwrap();
        assert_eq!(manufacturer, "Custom");
    }

    #[test]
    fn test_config_runtime_bool() {
        let config = ApplicationConfig::new();
        let runtime = ConfigRuntime::new(config);

        // Get default value
        let auth = runtime.get_bool("server.auth_enabled").unwrap();
        assert!(auth);

        // Set value
        runtime.set_bool("server.auth_enabled", false).unwrap();
        let auth = runtime.get_bool("server.auth_enabled").unwrap();
        assert!(!auth);
    }

    #[test]
    fn test_config_runtime_float() {
        let config = ApplicationConfig::new();
        let runtime = ConfigRuntime::new(config);

        // Get default value
        let speed = runtime.get_float("ptz.pan_speed").unwrap();
        assert!((speed - 0.5).abs() < f64::EPSILON);

        // Set value
        runtime.set_float("ptz.pan_speed", 0.75).unwrap();
        let speed = runtime.get_float("ptz.pan_speed").unwrap();
        assert!((speed - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_config_runtime_generation() {
        let config = ApplicationConfig::new();
        let runtime = ConfigRuntime::new(config);

        let gen1 = runtime.generation();
        runtime.set_int("server.port", 8080).unwrap();
        let gen2 = runtime.generation();

        assert!(gen2 > gen1);
    }

    #[test]
    fn test_config_runtime_validation() {
        let config = ApplicationConfig::new();
        let runtime = ConfigRuntime::new(config);

        // Valid value
        assert!(runtime.set_int("server.port", 8080).is_ok());

        // Out of range
        assert!(runtime.set_int("server.port", 0).is_err());
        assert!(runtime.set_int("server.port", 70000).is_err());
    }

    #[test]
    fn test_config_runtime_key_not_found() {
        let config = ApplicationConfig::new();
        let runtime = ConfigRuntime::new(config);

        let result = runtime.get_int("nonexistent.key");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_runtime_snapshot() {
        let mut config = ApplicationConfig::new();
        config.set("test.key", "value".to_string());
        let runtime = ConfigRuntime::new(config);

        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.get("test.key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_config_runtime_replace() {
        let config1 = ApplicationConfig::new();
        let runtime = ConfigRuntime::new(config1);

        let mut config2 = ApplicationConfig::new();
        config2.set("new.key", "new_value".to_string());

        let gen1 = runtime.generation();
        runtime.replace(config2);
        let gen2 = runtime.generation();

        assert!(gen2 > gen1);
        assert_eq!(
            runtime.snapshot().get("new.key"),
            Some(&"new_value".to_string())
        );
    }

    #[test]
    fn test_config_runtime_get_int_with_default_fallback() {
        // Test that get_int falls back to schema default when key not in config
        let config = ApplicationConfig::new(); // Empty config
        let runtime = ConfigRuntime::new(config);

        // server.port has a default of 80 in schema
        let port = runtime.get_int("server.port").unwrap();
        assert_eq!(port, 80);
    }

    #[test]
    fn test_config_runtime_get_float_with_default_fallback() {
        // Test that get_float falls back to schema default when key not in config
        let config = ApplicationConfig::new(); // Empty config
        let runtime = ConfigRuntime::new(config);

        // ptz.pan_speed has a default in schema
        let speed = runtime.get_float("ptz.pan_speed").unwrap();
        assert!((speed - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_config_runtime_get_bool_with_default_fallback() {
        // Test that get_bool falls back to schema default when key not in config
        let config = ApplicationConfig::new(); // Empty config
        let runtime = ConfigRuntime::new(config);

        // server.auth_enabled has a default in schema
        let auth = runtime.get_bool("server.auth_enabled").unwrap();
        assert!(auth); // Default is true
    }

    #[test]
    fn test_config_runtime_get_string_with_default_fallback() {
        // Test that get_string falls back to schema default when key not in config
        let config = ApplicationConfig::new(); // Empty config
        let runtime = ConfigRuntime::new(config);

        // device.manufacturer has a default in schema
        let manufacturer = runtime.get_string("device.manufacturer").unwrap();
        assert_eq!(manufacturer, "Anyka");
    }

    #[test]
    fn test_config_runtime_parse_error_in_config() {
        // Test parse error when config has invalid value
        let mut config = ApplicationConfig::new();
        config.set("server.port", "not_a_number".to_string());
        let runtime = ConfigRuntime::new(config);

        let result = runtime.get_int("server.port");
        assert!(result.is_err());
        match result {
            Err(ConfigError::ParseError { key, .. }) => assert_eq!(key, "server.port"),
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_config_runtime_parse_error_float() {
        // Test parse error when config has invalid float value
        let mut config = ApplicationConfig::new();
        config.set("ptz.pan_speed", "not_a_float".to_string());
        let runtime = ConfigRuntime::new(config);

        let result = runtime.get_float("ptz.pan_speed");
        assert!(result.is_err());
        match result {
            Err(ConfigError::ParseError { key, .. }) => assert_eq!(key, "ptz.pan_speed"),
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_config_runtime_set_type_mismatch() {
        // Test type validation when setting a value with wrong type
        let config = ApplicationConfig::new();
        let runtime = ConfigRuntime::new(config);

        // Try to set a float value on an int parameter
        let result = runtime.set_float("server.port", 80.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_application_config_contains() {
        let mut config = ApplicationConfig::new();
        config.set("test.key", "value".to_string());

        assert!(config.contains("test.key"));
        assert!(!config.contains("nonexistent.key"));
    }

    #[test]
    fn test_application_config_keys() {
        let mut config = ApplicationConfig::new();
        config.set("key1", "value1".to_string());
        config.set("key2", "value2".to_string());

        let keys: Vec<_> = config.keys().collect();
        assert!(keys.contains(&&"key1".to_string()));
        assert!(keys.contains(&&"key2".to_string()));
    }

    #[test]
    fn test_application_config_remove() {
        let mut config = ApplicationConfig::new();
        config.set("test.key", "value".to_string());

        assert!(config.contains("test.key"));
        let removed = config.remove("test.key");
        assert_eq!(removed, Some("value".to_string()));
        assert!(!config.contains("test.key"));
    }

    #[test]
    fn test_config_runtime_with_schema() {
        let config = ApplicationConfig::new();
        let schema = ConfigSchema::new();
        let runtime = ConfigRuntime::with_schema(config, schema);

        // Should still work with explicit schema
        let port = runtime.get_int("server.port").unwrap();
        assert_eq!(port, 80);
    }
}
