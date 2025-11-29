//! Configuration schema definition and validation.
//!
//! This module defines the configuration schema used to validate
//! configuration values and provide default values.

use std::collections::HashMap;

use thiserror::Error;

/// Configuration value types.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValueType {
    /// Integer value.
    Int,
    /// String value.
    String,
    /// Boolean value.
    Bool,
    /// Floating-point value.
    Float,
}

/// A configuration parameter definition.
#[derive(Debug, Clone)]
pub struct ConfigParameter {
    /// Parameter key (e.g., "server.port").
    pub key: String,
    /// Value type.
    pub value_type: ConfigValueType,
    /// Default value as string.
    pub default: String,
    /// Minimum value (for Int/Float).
    pub min: Option<f64>,
    /// Maximum value (for Int/Float).
    pub max: Option<f64>,
    /// Whether the parameter is required.
    pub required: bool,
    /// Description of the parameter.
    pub description: String,
}

impl ConfigParameter {
    /// Create a new required integer parameter.
    pub fn int(key: &str, default: i64, min: i64, max: i64, description: &str) -> Self {
        Self {
            key: key.to_string(),
            value_type: ConfigValueType::Int,
            default: default.to_string(),
            min: Some(min as f64),
            max: Some(max as f64),
            required: true,
            description: description.to_string(),
        }
    }

    /// Create a new required string parameter.
    pub fn string(key: &str, default: &str, description: &str) -> Self {
        Self {
            key: key.to_string(),
            value_type: ConfigValueType::String,
            default: default.to_string(),
            min: None,
            max: None,
            required: true,
            description: description.to_string(),
        }
    }

    /// Create a new required boolean parameter.
    pub fn bool(key: &str, default: bool, description: &str) -> Self {
        Self {
            key: key.to_string(),
            value_type: ConfigValueType::Bool,
            default: default.to_string(),
            min: None,
            max: None,
            required: true,
            description: description.to_string(),
        }
    }

    /// Create a new required float parameter.
    pub fn float(key: &str, default: f64, min: f64, max: f64, description: &str) -> Self {
        Self {
            key: key.to_string(),
            value_type: ConfigValueType::Float,
            default: default.to_string(),
            min: Some(min),
            max: Some(max),
            required: true,
            description: description.to_string(),
        }
    }

    /// Make this parameter optional.
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
}

/// A configuration section containing multiple parameters.
#[derive(Debug, Clone)]
pub struct ConfigSection {
    /// Section name (e.g., "server").
    pub name: String,
    /// Parameters in this section.
    pub parameters: Vec<ConfigParameter>,
    /// Description of the section.
    pub description: String,
}

impl ConfigSection {
    /// Create a new configuration section.
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            parameters: Vec::new(),
            description: description.to_string(),
        }
    }

    /// Add a parameter to the section.
    pub fn param(mut self, param: ConfigParameter) -> Self {
        self.parameters.push(param);
        self
    }
}

/// Errors that can occur during configuration validation.
#[derive(Debug, Error, Clone)]
pub enum ConfigValidationError {
    /// A required parameter is missing.
    #[error("Missing required parameter: {0}")]
    MissingRequired(String),

    /// A parameter has an invalid type.
    #[error("Invalid type for {key}: expected {expected}, got {actual}")]
    InvalidType {
        key: String,
        expected: String,
        actual: String,
    },

    /// A parameter value is out of range.
    #[error("Value {value} for {key} is out of range [{min}, {max}]")]
    OutOfRange {
        key: String,
        value: f64,
        min: f64,
        max: f64,
    },

    /// An unknown parameter was found.
    #[error("Unknown parameter: {0}")]
    UnknownParameter(String),
}

/// Configuration schema for validation.
#[derive(Debug, Clone)]
pub struct ConfigSchema {
    /// Configuration sections.
    sections: Vec<ConfigSection>,
    /// Parameter lookup by key.
    parameters: HashMap<String, ConfigParameter>,
}

impl ConfigSchema {
    /// Create a new configuration schema with default sections.
    pub fn new() -> Self {
        let sections = vec![
            Self::onvif_section(),
            Self::network_section(),
            Self::device_section(),
            Self::server_section(),
            Self::logging_section(),
            Self::media_section(),
            Self::ptz_section(),
            Self::imaging_section(),
            Self::stream_profile_section(1),
            Self::stream_profile_section(2),
            Self::stream_profile_section(3),
            Self::stream_profile_section(4),
        ];

        let mut parameters = HashMap::new();
        for section in &sections {
            for param in &section.parameters {
                parameters.insert(param.key.clone(), param.clone());
            }
        }

        Self {
            sections,
            parameters,
        }
    }

    /// Get a parameter definition by key.
    pub fn get_parameter(&self, key: &str) -> Option<&ConfigParameter> {
        self.parameters.get(key)
    }

    /// Get all sections.
    pub fn sections(&self) -> &[ConfigSection] {
        &self.sections
    }

    /// Validate a configuration value.
    pub fn validate(&self, key: &str, value: &str) -> Result<(), ConfigValidationError> {
        let param = self
            .parameters
            .get(key)
            .ok_or_else(|| ConfigValidationError::UnknownParameter(key.to_string()))?;

        match param.value_type {
            ConfigValueType::Int => {
                let val: i64 = value
                    .parse()
                    .map_err(|_| ConfigValidationError::InvalidType {
                        key: key.to_string(),
                        expected: "integer".to_string(),
                        actual: "non-integer".to_string(),
                    })?;

                if let (Some(min), Some(max)) = (param.min, param.max) {
                    if (val as f64) < min || (val as f64) > max {
                        return Err(ConfigValidationError::OutOfRange {
                            key: key.to_string(),
                            value: val as f64,
                            min,
                            max,
                        });
                    }
                }
            }
            ConfigValueType::Float => {
                let val: f64 = value
                    .parse()
                    .map_err(|_| ConfigValidationError::InvalidType {
                        key: key.to_string(),
                        expected: "float".to_string(),
                        actual: "non-float".to_string(),
                    })?;

                if let (Some(min), Some(max)) = (param.min, param.max) {
                    if val < min || val > max {
                        return Err(ConfigValidationError::OutOfRange {
                            key: key.to_string(),
                            value: val,
                            min,
                            max,
                        });
                    }
                }
            }
            ConfigValueType::Bool => {
                let _ = value
                    .parse::<bool>()
                    .map_err(|_| ConfigValidationError::InvalidType {
                        key: key.to_string(),
                        expected: "boolean".to_string(),
                        actual: "non-boolean".to_string(),
                    })?;
            }
            ConfigValueType::String => {
                // No validation needed for strings
            }
        }

        Ok(())
    }

    /// Get default value for a parameter.
    pub fn default_value(&self, key: &str) -> Option<&str> {
        self.parameters.get(key).map(|p| p.default.as_str())
    }

    // Section definitions

    fn onvif_section() -> ConfigSection {
        ConfigSection::new("onvif", "ONVIF protocol settings")
            .param(ConfigParameter::string(
                "onvif.device_service_path",
                "/onvif/device_service",
                "Device service endpoint path",
            ))
            .param(ConfigParameter::string(
                "onvif.media_service_path",
                "/onvif/media_service",
                "Media service endpoint path",
            ))
            .param(ConfigParameter::string(
                "onvif.ptz_service_path",
                "/onvif/ptz_service",
                "PTZ service endpoint path",
            ))
            .param(ConfigParameter::string(
                "onvif.imaging_service_path",
                "/onvif/imaging_service",
                "Imaging service endpoint path",
            ))
            .param(ConfigParameter::string(
                "onvif.profile_level",
                "S",
                "ONVIF profile level (S, G, T)",
            ))
    }

    fn network_section() -> ConfigSection {
        ConfigSection::new("network", "Network configuration")
            .param(ConfigParameter::string(
                "network.interface",
                "eth0",
                "Network interface name",
            ))
            .param(
                ConfigParameter::string(
                    "network.ip_address",
                    "",
                    "Static IP address (empty for DHCP)",
                )
                .optional(),
            )
            .param(
                ConfigParameter::string("network.netmask", "255.255.255.0", "Network mask")
                    .optional(),
            )
            .param(ConfigParameter::string("network.gateway", "", "Default gateway").optional())
            .param(
                ConfigParameter::string("network.dns_primary", "", "Primary DNS server").optional(),
            )
            .param(
                ConfigParameter::string("network.dns_secondary", "", "Secondary DNS server")
                    .optional(),
            )
            .param(ConfigParameter::bool(
                "network.dhcp_enabled",
                true,
                "Enable DHCP",
            ))
    }

    fn device_section() -> ConfigSection {
        ConfigSection::new("device", "Device information")
            .param(ConfigParameter::string(
                "device.manufacturer",
                "Anyka",
                "Device manufacturer",
            ))
            .param(ConfigParameter::string(
                "device.model",
                "AK3918 Camera",
                "Device model",
            ))
            .param(ConfigParameter::string(
                "device.firmware_version",
                "1.0.0",
                "Firmware version",
            ))
            .param(
                ConfigParameter::string(
                    "device.serial_number",
                    "",
                    "Serial number (empty to auto-generate)",
                )
                .optional(),
            )
            .param(ConfigParameter::string(
                "device.hardware_id",
                "ak3918",
                "Hardware identifier",
            ))
            .param(ConfigParameter::string(
                "device.hostname",
                "ipcam",
                "Device hostname",
            ))
            .param(
                ConfigParameter::string("device.uuid", "", "Device UUID (empty to auto-generate)")
                    .optional(),
            )
    }

    fn server_section() -> ConfigSection {
        ConfigSection::new("server", "HTTP server settings")
            .param(ConfigParameter::int(
                "server.port",
                80,
                1,
                65535,
                "HTTP server port",
            ))
            .param(ConfigParameter::string(
                "server.bind_address",
                "0.0.0.0",
                "IP address to bind to",
            ))
            .param(ConfigParameter::int(
                "server.max_connections",
                16,
                1,
                256,
                "Maximum concurrent connections",
            ))
            .param(ConfigParameter::int(
                "server.request_timeout",
                30,
                1,
                300,
                "Request timeout in seconds",
            ))
            .param(ConfigParameter::int(
                "server.max_body_size",
                1048576,
                1024,
                10485760,
                "Maximum request body size in bytes",
            ))
            .param(ConfigParameter::bool(
                "server.auth_enabled",
                true,
                "Enable authentication",
            ))
            .param(ConfigParameter::string(
                "server.realm",
                "ONVIF Camera",
                "Authentication realm",
            ))
    }

    fn logging_section() -> ConfigSection {
        ConfigSection::new("logging", "Logging configuration")
            .param(ConfigParameter::string(
                "logging.level",
                "info",
                "Log level (error, warn, info, debug, trace)",
            ))
            .param(ConfigParameter::bool(
                "logging.http_verbose",
                false,
                "Enable verbose HTTP request/response logging",
            ))
            .param(ConfigParameter::bool(
                "logging.console_enabled",
                true,
                "Enable console logging",
            ))
            .param(
                ConfigParameter::string(
                    "logging.file_path",
                    "",
                    "Log file path (empty to disable)",
                )
                .optional(),
            )
    }

    fn media_section() -> ConfigSection {
        ConfigSection::new("media", "Media stream settings")
            .param(ConfigParameter::int(
                "media.rtsp_port",
                554,
                1,
                65535,
                "RTSP server port",
            ))
            .param(ConfigParameter::int(
                "media.snapshot_port",
                0,
                0,
                65535,
                "Snapshot HTTP port (0 to use main server port)",
            ))
            .param(ConfigParameter::string(
                "media.snapshot_path",
                "/snapshot",
                "Snapshot endpoint path",
            ))
            .param(ConfigParameter::int(
                "media.max_streams",
                4,
                1,
                8,
                "Maximum concurrent streams",
            ))
    }

    fn ptz_section() -> ConfigSection {
        ConfigSection::new("ptz", "PTZ control settings")
            .param(ConfigParameter::bool(
                "ptz.enabled",
                true,
                "Enable PTZ control",
            ))
            .param(ConfigParameter::float(
                "ptz.pan_speed",
                0.5,
                0.0,
                1.0,
                "Default pan speed",
            ))
            .param(ConfigParameter::float(
                "ptz.tilt_speed",
                0.5,
                0.0,
                1.0,
                "Default tilt speed",
            ))
            .param(ConfigParameter::float(
                "ptz.zoom_speed",
                0.5,
                0.0,
                1.0,
                "Default zoom speed",
            ))
            .param(ConfigParameter::int(
                "ptz.max_presets",
                16,
                1,
                64,
                "Maximum number of presets",
            ))
            .param(ConfigParameter::bool(
                "ptz.home_on_start",
                true,
                "Move to home position on startup",
            ))
    }

    fn imaging_section() -> ConfigSection {
        ConfigSection::new("imaging", "Imaging settings")
            .param(ConfigParameter::float(
                "imaging.brightness",
                50.0,
                0.0,
                100.0,
                "Default brightness",
            ))
            .param(ConfigParameter::float(
                "imaging.contrast",
                50.0,
                0.0,
                100.0,
                "Default contrast",
            ))
            .param(ConfigParameter::float(
                "imaging.saturation",
                50.0,
                0.0,
                100.0,
                "Default saturation",
            ))
            .param(ConfigParameter::float(
                "imaging.sharpness",
                50.0,
                0.0,
                100.0,
                "Default sharpness",
            ))
            .param(ConfigParameter::bool(
                "imaging.ir_cut_filter",
                true,
                "IR cut filter enabled",
            ))
            .param(ConfigParameter::bool(
                "imaging.ir_led",
                false,
                "IR LED enabled",
            ))
    }

    fn stream_profile_section(index: u32) -> ConfigSection {
        let section_name = format!("stream_profile_{}", index);
        ConfigSection::new(&section_name, &format!("Stream profile {} settings", index))
            .param(ConfigParameter::string(
                &format!("{}.name", section_name),
                &format!("Profile{}", index),
                "Profile name",
            ))
            .param(ConfigParameter::int(
                &format!("{}.width", section_name),
                if index == 1 { 1920 } else { 640 },
                160,
                3840,
                "Video width",
            ))
            .param(ConfigParameter::int(
                &format!("{}.height", section_name),
                if index == 1 { 1080 } else { 480 },
                120,
                2160,
                "Video height",
            ))
            .param(ConfigParameter::int(
                &format!("{}.framerate", section_name),
                if index == 1 { 25 } else { 15 },
                1,
                60,
                "Frame rate",
            ))
            .param(ConfigParameter::int(
                &format!("{}.bitrate", section_name),
                if index == 1 { 4000 } else { 512 },
                64,
                16384,
                "Bitrate in kbps",
            ))
            .param(ConfigParameter::string(
                &format!("{}.encoding", section_name),
                "H264",
                "Video encoding (H264, H265, MJPEG)",
            ))
            .param(ConfigParameter::int(
                &format!("{}.gop_length", section_name),
                if index == 1 { 50 } else { 30 },
                1,
                300,
                "GOP length",
            ))
            .param(ConfigParameter::int(
                &format!("{}.quality", section_name),
                80,
                1,
                100,
                "Quality level",
            ))
    }
}

impl Default for ConfigSchema {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_schema_new() {
        let schema = ConfigSchema::new();
        assert!(!schema.sections.is_empty());
    }

    #[test]
    fn test_config_schema_get_parameter() {
        let schema = ConfigSchema::new();
        let param = schema.get_parameter("server.port");
        assert!(param.is_some());
        assert_eq!(param.unwrap().value_type, ConfigValueType::Int);
    }

    #[test]
    fn test_config_schema_validate_int() {
        let schema = ConfigSchema::new();

        // Valid value
        assert!(schema.validate("server.port", "8080").is_ok());

        // Out of range
        assert!(schema.validate("server.port", "0").is_err());
        assert!(schema.validate("server.port", "70000").is_err());

        // Invalid type
        assert!(schema.validate("server.port", "not_a_number").is_err());
    }

    #[test]
    fn test_config_schema_validate_bool() {
        let schema = ConfigSchema::new();

        // Valid values
        assert!(schema.validate("server.auth_enabled", "true").is_ok());
        assert!(schema.validate("server.auth_enabled", "false").is_ok());

        // Invalid type
        assert!(schema.validate("server.auth_enabled", "maybe").is_err());
    }

    #[test]
    fn test_config_schema_validate_float() {
        let schema = ConfigSchema::new();

        // Valid value
        assert!(schema.validate("ptz.pan_speed", "0.75").is_ok());

        // Out of range
        assert!(schema.validate("ptz.pan_speed", "-0.5").is_err());
        assert!(schema.validate("ptz.pan_speed", "1.5").is_err());
    }

    #[test]
    fn test_config_schema_default_value() {
        let schema = ConfigSchema::new();
        assert_eq!(schema.default_value("server.port"), Some("80"));
        assert_eq!(schema.default_value("device.manufacturer"), Some("Anyka"));
    }

    #[test]
    fn test_config_parameter_builders() {
        let int_param = ConfigParameter::int("test.int", 50, 0, 100, "Test integer");
        assert_eq!(int_param.value_type, ConfigValueType::Int);
        assert_eq!(int_param.default, "50");

        let string_param = ConfigParameter::string("test.string", "hello", "Test string");
        assert_eq!(string_param.value_type, ConfigValueType::String);

        let bool_param = ConfigParameter::bool("test.bool", true, "Test boolean");
        assert_eq!(bool_param.value_type, ConfigValueType::Bool);

        let float_param = ConfigParameter::float("test.float", 0.5, 0.0, 1.0, "Test float");
        assert_eq!(float_param.value_type, ConfigValueType::Float);
    }
}
