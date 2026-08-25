//! Agent configuration loaded from `snmp.toml`.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Default path on the camera SD payload.
pub const DEFAULT_CONFIG_PATH: &str = "/mnt/anyka_hack/snmp.toml";

fn default_enabled() -> bool {
    true
}

fn default_port() -> u16 {
    161
}

fn default_community() -> String {
    "public".to_string()
}

/// SNMPv2c agent settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnmpConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_community")]
    pub community: String,
    #[serde(default)]
    pub sys_contact: String,
    #[serde(default)]
    pub sys_name: String,
    #[serde(default)]
    pub sys_location: String,
}

impl Default for SnmpConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            port: default_port(),
            community: default_community(),
            sys_contact: String::new(),
            sys_name: String::new(),
            sys_location: String::new(),
        }
    }
}

/// Errors loading or validating `snmp.toml`.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid port: must be non-zero")]
    InvalidPort,
    #[error("community must not be empty when SNMP is enabled")]
    EmptyCommunityWhenEnabled,
}

impl SnmpConfig {
    /// Load config from `path`. Missing file yields [`SnmpConfig::default`].
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(e) => return Err(ConfigError::Io(e)),
        };
        let config: Self = toml::from_str(&raw)?;
        if config.port == 0 {
            return Err(ConfigError::InvalidPort);
        }
        if config.enabled && config.community.is_empty() {
            return Err(ConfigError::EmptyCommunityWhenEnabled);
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let c = SnmpConfig::default();
        assert!(c.enabled);
        assert_eq!(c.port, 161);
        assert_eq!(c.community, "public");
        assert_eq!(c.sys_contact, "");
        assert_eq!(c.sys_name, "");
        assert_eq!(c.sys_location, "");
    }

    #[test]
    fn test_load_missing_file_returns_defaults() {
        let c = SnmpConfig::load("/no/such/snmp.toml").expect("defaults");
        assert_eq!(c, SnmpConfig::default());
    }

    #[test]
    fn test_load_parses_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        std::fs::write(
            &path,
            r#"
enabled = false
port = 1161
community = "monitor"
sys_contact = "ops@example"
sys_name = "cam-1"
sys_location = "lab"
"#,
        )
        .unwrap();
        let c = SnmpConfig::load(&path).unwrap();
        assert!(!c.enabled);
        assert_eq!(c.port, 1161);
        assert_eq!(c.community, "monitor");
        assert_eq!(c.sys_name, "cam-1");
    }

    #[test]
    fn test_load_rejects_port_zero() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        std::fs::write(&path, "port = 0\n").unwrap();
        assert!(SnmpConfig::load(&path).is_err());
    }

    #[test]
    fn test_load_rejects_enabled_without_community() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        std::fs::write(&path, "enabled = true\nport = 161\ncommunity = \"\"\n").unwrap();
        assert!(matches!(
            SnmpConfig::load(&path),
            Err(ConfigError::EmptyCommunityWhenEnabled)
        ));
    }
}
