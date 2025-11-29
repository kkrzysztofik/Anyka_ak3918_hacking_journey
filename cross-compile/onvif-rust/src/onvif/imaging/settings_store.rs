//! Imaging settings storage and validation.
//!
//! This module provides thread-safe storage for imaging settings with
//! schema validation against supported options.

use parking_lot::RwLock;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

use crate::onvif::types::common::{FloatRange, ImagingSettings20, ImagingStatus20};
use crate::onvif::types::imaging::ImagingOptions20;
use crate::platform::{ImagingControl, ImagingOptions, ImagingSettings, PlatformError};

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur in imaging settings operations.
#[derive(Debug, Error, Clone)]
pub enum ImagingSettingsError {
    /// Parameter value is out of valid range.
    #[error("Parameter '{parameter}' value {value} is out of range ({min} - {max})")]
    OutOfRange {
        parameter: String,
        value: f32,
        min: f32,
        max: f32,
    },

    /// Video source token is invalid.
    #[error("Invalid video source token: {0}")]
    InvalidToken(String),

    /// Platform error.
    #[error("Platform error: {0}")]
    PlatformError(String),

    /// Settings validation failed.
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}

impl From<PlatformError> for ImagingSettingsError {
    fn from(err: PlatformError) -> Self {
        ImagingSettingsError::PlatformError(err.to_string())
    }
}

/// Result type for imaging settings operations.
pub type ImagingSettingsResult<T> = Result<T, ImagingSettingsError>;

// ============================================================================
// Settings Store
// ============================================================================

/// Thread-safe storage for imaging settings.
///
/// The `ImagingSettingsStore` provides concurrent access to imaging settings
/// with validation against supported options from the platform.
pub struct ImagingSettingsStore {
    /// Current imaging settings per video source.
    settings: RwLock<std::collections::HashMap<String, ImagingSettings20>>,

    /// Supported imaging options per video source.
    options: RwLock<std::collections::HashMap<String, ImagingOptions20>>,

    /// Platform imaging control (optional).
    platform_control: Option<Arc<dyn ImagingControl>>,

    /// Known video source tokens.
    video_source_tokens: RwLock<Vec<String>>,

    /// Path to persistence file (optional).
    persistence_path: Option<PathBuf>,
}

/// TOML file structure for persisting imaging settings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ImagingSettingsFile {
    /// Settings per video source.
    #[serde(default)]
    video_sources: std::collections::HashMap<String, PersistedImagingSettings>,
}

/// Persisted imaging settings structure.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PersistedImagingSettings {
    brightness: Option<f32>,
    contrast: Option<f32>,
    color_saturation: Option<f32>,
    sharpness: Option<f32>,
    ir_cut_filter: Option<String>,
    wide_dynamic_range_mode: Option<String>,
    wide_dynamic_range_level: Option<f32>,
    backlight_compensation_mode: Option<String>,
    backlight_compensation_level: Option<f32>,
}

impl ImagingSettingsStore {
    /// Create a new imaging settings store.
    pub fn new() -> Self {
        Self {
            settings: RwLock::new(std::collections::HashMap::new()),
            options: RwLock::new(std::collections::HashMap::new()),
            platform_control: None,
            video_source_tokens: RwLock::new(vec!["VideoSource_1".to_string()]),
            persistence_path: None,
        }
    }

    /// Create a new imaging settings store with persistence path.
    pub fn with_persistence(path: impl AsRef<Path>) -> Self {
        let mut store = Self::new();
        store.persistence_path = Some(path.as_ref().to_path_buf());
        store
    }

    /// Create a new imaging settings store with platform control.
    pub fn with_platform(platform_control: Arc<dyn ImagingControl>) -> Self {
        Self {
            settings: RwLock::new(std::collections::HashMap::new()),
            options: RwLock::new(std::collections::HashMap::new()),
            platform_control: Some(platform_control),
            video_source_tokens: RwLock::new(vec!["VideoSource_1".to_string()]),
            persistence_path: None,
        }
    }

    /// Create with both platform control and persistence.
    pub fn with_platform_and_persistence(
        platform_control: Arc<dyn ImagingControl>,
        path: impl AsRef<Path>,
    ) -> Self {
        Self {
            settings: RwLock::new(std::collections::HashMap::new()),
            options: RwLock::new(std::collections::HashMap::new()),
            platform_control: Some(platform_control),
            video_source_tokens: RwLock::new(vec!["VideoSource_1".to_string()]),
            persistence_path: Some(path.as_ref().to_path_buf()),
        }
    }

    /// Set the persistence path.
    pub fn set_persistence_path(&mut self, path: impl AsRef<Path>) {
        self.persistence_path = Some(path.as_ref().to_path_buf());
    }

    /// Load settings from persistence file.
    pub fn load_from_file(&self) -> ImagingSettingsResult<()> {
        let path = match &self.persistence_path {
            Some(p) => p,
            None => {
                tracing::debug!("No persistence path configured, skipping load");
                return Ok(());
            }
        };

        if !path.exists() {
            tracing::info!("Imaging settings file does not exist: {:?}", path);
            return Ok(());
        }

        let content = fs::read_to_string(path).map_err(|e| {
            ImagingSettingsError::ValidationFailed(format!("Failed to read file: {}", e))
        })?;

        let file: ImagingSettingsFile = toml::from_str(&content).map_err(|e| {
            ImagingSettingsError::ValidationFailed(format!("Failed to parse TOML: {}", e))
        })?;

        let mut settings_cache = self.settings.write();
        for (token, persisted) in file.video_sources {
            let settings = Self::persisted_to_onvif(&persisted);
            settings_cache.insert(token, settings);
        }

        tracing::info!(
            "Loaded imaging settings for {} video sources from {:?}",
            settings_cache.len(),
            path
        );
        Ok(())
    }

    /// Check if a video source token is valid.
    pub fn is_valid_token(&self, token: &str) -> bool {
        let tokens = self.video_source_tokens.read();
        tokens.contains(&token.to_string())
    }

    /// Get the list of video source tokens.
    pub fn get_video_source_tokens(&self) -> Vec<String> {
        self.video_source_tokens.read().clone()
    }

    /// Add a video source token.
    pub fn add_video_source_token(&self, token: String) {
        let mut tokens = self.video_source_tokens.write();
        if !tokens.contains(&token) {
            tokens.push(token);
        }
    }

    /// Get current imaging settings for a video source.
    ///
    /// If platform control is available, settings are fetched from hardware.
    /// Otherwise, cached settings or defaults are returned.
    pub async fn get_settings(
        &self,
        video_source_token: &str,
    ) -> ImagingSettingsResult<ImagingSettings20> {
        // Validate token
        if !self.is_valid_token(video_source_token) {
            return Err(ImagingSettingsError::InvalidToken(
                video_source_token.to_string(),
            ));
        }

        // Try to get from platform first
        if let Some(ref control) = self.platform_control {
            match control.get_settings().await {
                Ok(platform_settings) => {
                    let settings = Self::platform_to_onvif_settings(&platform_settings);
                    // Cache the settings
                    let mut cache = self.settings.write();
                    cache.insert(video_source_token.to_string(), settings.clone());
                    return Ok(settings);
                }
                Err(e) => {
                    tracing::warn!("Failed to get settings from platform: {}", e);
                    // Fall through to cache
                }
            }
        }

        // Return cached or default settings
        let cache = self.settings.read();
        Ok(cache
            .get(video_source_token)
            .cloned()
            .unwrap_or_else(Self::default_settings))
    }

    /// Set imaging settings for a video source.
    ///
    /// Settings are validated against supported options before being applied.
    pub async fn set_settings(
        &self,
        video_source_token: &str,
        settings: &ImagingSettings20,
        force_persistence: bool,
    ) -> ImagingSettingsResult<()> {
        // Validate token
        if !self.is_valid_token(video_source_token) {
            return Err(ImagingSettingsError::InvalidToken(
                video_source_token.to_string(),
            ));
        }

        // Validate settings against options
        self.validate_settings(video_source_token, settings).await?;

        // Apply to platform if available
        if let Some(ref control) = self.platform_control {
            let platform_settings = Self::onvif_to_platform_settings(settings);
            control.set_settings(&platform_settings).await?;
        }

        // Update cache
        {
            let mut cache = self.settings.write();
            cache.insert(video_source_token.to_string(), settings.clone());
        }

        // Persist if requested
        if force_persistence {
            self.persist_settings(video_source_token, settings)?;
        }

        Ok(())
    }

    /// Validate settings against supported options.
    pub async fn validate_settings(
        &self,
        video_source_token: &str,
        settings: &ImagingSettings20,
    ) -> ImagingSettingsResult<()> {
        let options = self.get_options(video_source_token).await?;

        // Validate brightness
        if let Some(brightness) = settings.brightness {
            if let Some(ref range) = options.brightness {
                if brightness < range.min || brightness > range.max {
                    return Err(ImagingSettingsError::OutOfRange {
                        parameter: "Brightness".to_string(),
                        value: brightness,
                        min: range.min,
                        max: range.max,
                    });
                }
            }
        }

        // Validate contrast
        if let Some(contrast) = settings.contrast {
            if let Some(ref range) = options.contrast {
                if contrast < range.min || contrast > range.max {
                    return Err(ImagingSettingsError::OutOfRange {
                        parameter: "Contrast".to_string(),
                        value: contrast,
                        min: range.min,
                        max: range.max,
                    });
                }
            }
        }

        // Validate color saturation
        if let Some(saturation) = settings.color_saturation {
            if let Some(ref range) = options.color_saturation {
                if saturation < range.min || saturation > range.max {
                    return Err(ImagingSettingsError::OutOfRange {
                        parameter: "ColorSaturation".to_string(),
                        value: saturation,
                        min: range.min,
                        max: range.max,
                    });
                }
            }
        }

        // Validate sharpness
        if let Some(sharpness) = settings.sharpness {
            if let Some(ref range) = options.sharpness {
                if sharpness < range.min || sharpness > range.max {
                    return Err(ImagingSettingsError::OutOfRange {
                        parameter: "Sharpness".to_string(),
                        value: sharpness,
                        min: range.min,
                        max: range.max,
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate a single parameter against options.
    pub fn validate_parameter(
        &self,
        parameter: &str,
        value: f32,
        options: &ImagingOptions20,
    ) -> ImagingSettingsResult<()> {
        let range = match parameter {
            "Brightness" => options.brightness.as_ref(),
            "Contrast" => options.contrast.as_ref(),
            "ColorSaturation" => options.color_saturation.as_ref(),
            "Sharpness" => options.sharpness.as_ref(),
            _ => None,
        };

        if let Some(range) = range {
            if value < range.min || value > range.max {
                return Err(ImagingSettingsError::OutOfRange {
                    parameter: parameter.to_string(),
                    value,
                    min: range.min,
                    max: range.max,
                });
            }
        }

        Ok(())
    }

    /// Get supported imaging options for a video source.
    pub async fn get_options(
        &self,
        video_source_token: &str,
    ) -> ImagingSettingsResult<ImagingOptions20> {
        // Validate token
        if !self.is_valid_token(video_source_token) {
            return Err(ImagingSettingsError::InvalidToken(
                video_source_token.to_string(),
            ));
        }

        // Try to get from platform first
        if let Some(ref control) = self.platform_control {
            match control.get_options().await {
                Ok(platform_options) => {
                    let options = Self::platform_to_onvif_options(&platform_options);
                    // Cache the options
                    let mut cache = self.options.write();
                    cache.insert(video_source_token.to_string(), options.clone());
                    return Ok(options);
                }
                Err(e) => {
                    tracing::warn!("Failed to get options from platform: {}", e);
                    // Fall through to cache
                }
            }
        }

        // Return cached or default options
        let cache = self.options.read();
        Ok(cache
            .get(video_source_token)
            .cloned()
            .unwrap_or_else(Self::default_options))
    }

    /// Get imaging status for a video source.
    pub async fn get_status(
        &self,
        video_source_token: &str,
    ) -> ImagingSettingsResult<ImagingStatus20> {
        // Validate token
        if !self.is_valid_token(video_source_token) {
            return Err(ImagingSettingsError::InvalidToken(
                video_source_token.to_string(),
            ));
        }

        // Return default status (focus status is optional)
        Ok(ImagingStatus20::default())
    }

    /// Persist settings to configuration file.
    fn persist_settings(
        &self,
        video_source_token: &str,
        settings: &ImagingSettings20,
    ) -> ImagingSettingsResult<()> {
        let path = match &self.persistence_path {
            Some(p) => p,
            None => {
                tracing::debug!("No persistence path configured, skipping persist");
                return Ok(());
            }
        };

        // Load existing file or create new
        let mut file = if path.exists() {
            let content = fs::read_to_string(path).map_err(|e| {
                ImagingSettingsError::ValidationFailed(format!("Failed to read file: {}", e))
            })?;
            toml::from_str(&content).unwrap_or_else(|_| ImagingSettingsFile {
                video_sources: std::collections::HashMap::new(),
            })
        } else {
            ImagingSettingsFile {
                video_sources: std::collections::HashMap::new(),
            }
        };

        // Update settings for this video source
        let persisted = Self::onvif_to_persisted(settings);
        file.video_sources
            .insert(video_source_token.to_string(), persisted);

        // Serialize to TOML
        let content = toml::to_string_pretty(&file).map_err(|e| {
            ImagingSettingsError::ValidationFailed(format!("Failed to serialize TOML: {}", e))
        })?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ImagingSettingsError::ValidationFailed(format!("Failed to create directory: {}", e))
            })?;
        }

        // Atomic write: write to temp file, then rename
        let temp_path = path.with_extension("toml.tmp");
        {
            let mut file = fs::File::create(&temp_path).map_err(|e| {
                ImagingSettingsError::ValidationFailed(format!("Failed to create temp file: {}", e))
            })?;
            file.write_all(content.as_bytes()).map_err(|e| {
                ImagingSettingsError::ValidationFailed(format!("Failed to write file: {}", e))
            })?;
            file.sync_all().map_err(|e| {
                ImagingSettingsError::ValidationFailed(format!("Failed to sync file: {}", e))
            })?;
        }

        fs::rename(&temp_path, path).map_err(|e| {
            ImagingSettingsError::ValidationFailed(format!("Failed to rename temp file: {}", e))
        })?;

        tracing::debug!(
            "Persisted imaging settings for {} to {:?}",
            video_source_token,
            path
        );
        Ok(())
    }

    /// Convert persisted settings to ONVIF format.
    fn persisted_to_onvif(persisted: &PersistedImagingSettings) -> ImagingSettings20 {
        use crate::onvif::types::common::{
            BacklightCompensation20, BacklightCompensationMode, IrCutFilterMode, WideDynamicMode,
            WideDynamicRange20,
        };

        ImagingSettings20 {
            brightness: persisted.brightness,
            contrast: persisted.contrast,
            color_saturation: persisted.color_saturation,
            sharpness: persisted.sharpness,
            ir_cut_filter: persisted.ir_cut_filter.as_ref().map(|s| match s.as_str() {
                "ON" => IrCutFilterMode::ON,
                "OFF" => IrCutFilterMode::OFF,
                _ => IrCutFilterMode::AUTO,
            }),
            wide_dynamic_range: persisted.wide_dynamic_range_mode.as_ref().map(|mode| {
                WideDynamicRange20 {
                    mode: match mode.as_str() {
                        "ON" => WideDynamicMode::ON,
                        _ => WideDynamicMode::OFF,
                    },
                    level: persisted.wide_dynamic_range_level,
                }
            }),
            backlight_compensation: persisted.backlight_compensation_mode.as_ref().map(|mode| {
                BacklightCompensation20 {
                    mode: match mode.as_str() {
                        "ON" => BacklightCompensationMode::ON,
                        _ => BacklightCompensationMode::OFF,
                    },
                    level: persisted.backlight_compensation_level,
                }
            }),
            ..Default::default()
        }
    }

    /// Convert ONVIF settings to persisted format.
    fn onvif_to_persisted(settings: &ImagingSettings20) -> PersistedImagingSettings {
        PersistedImagingSettings {
            brightness: settings.brightness,
            contrast: settings.contrast,
            color_saturation: settings.color_saturation,
            sharpness: settings.sharpness,
            ir_cut_filter: settings.ir_cut_filter.as_ref().map(|m| format!("{:?}", m)),
            wide_dynamic_range_mode: settings
                .wide_dynamic_range
                .as_ref()
                .map(|w| format!("{:?}", w.mode)),
            wide_dynamic_range_level: settings.wide_dynamic_range.as_ref().and_then(|w| w.level),
            backlight_compensation_mode: settings
                .backlight_compensation
                .as_ref()
                .map(|b| format!("{:?}", b.mode)),
            backlight_compensation_level: settings
                .backlight_compensation
                .as_ref()
                .and_then(|b| b.level),
        }
    }

    /// Convert platform settings to ONVIF format.
    fn platform_to_onvif_settings(settings: &ImagingSettings) -> ImagingSettings20 {
        use crate::onvif::types::common::{
            BacklightCompensation20, BacklightCompensationMode, IrCutFilterMode, WideDynamicMode,
            WideDynamicRange20,
        };

        ImagingSettings20 {
            brightness: Some(settings.brightness),
            contrast: Some(settings.contrast),
            color_saturation: Some(settings.saturation),
            sharpness: Some(settings.sharpness),
            ir_cut_filter: Some(if settings.ir_cut_filter {
                IrCutFilterMode::ON
            } else {
                IrCutFilterMode::OFF
            }),
            wide_dynamic_range: if settings.wdr {
                Some(WideDynamicRange20 {
                    mode: WideDynamicMode::ON,
                    level: Some(50.0),
                })
            } else {
                Some(WideDynamicRange20 {
                    mode: WideDynamicMode::OFF,
                    level: None,
                })
            },
            backlight_compensation: if settings.backlight_compensation {
                Some(BacklightCompensation20 {
                    mode: BacklightCompensationMode::ON,
                    level: Some(50.0),
                })
            } else {
                Some(BacklightCompensation20 {
                    mode: BacklightCompensationMode::OFF,
                    level: None,
                })
            },
            exposure: None,
            focus: None,
            white_balance: None,
            extension: None,
        }
    }

    /// Convert ONVIF settings to platform format.
    fn onvif_to_platform_settings(settings: &ImagingSettings20) -> ImagingSettings {
        use crate::onvif::types::common::{IrCutFilterMode, WideDynamicMode};

        ImagingSettings {
            brightness: settings.brightness.unwrap_or(50.0),
            contrast: settings.contrast.unwrap_or(50.0),
            saturation: settings.color_saturation.unwrap_or(50.0),
            sharpness: settings.sharpness.unwrap_or(50.0),
            ir_cut_filter: settings
                .ir_cut_filter
                .as_ref()
                .map(|m| matches!(m, IrCutFilterMode::ON | IrCutFilterMode::AUTO))
                .unwrap_or(true),
            ir_led: false, // Not exposed in ImagingSettings20
            wdr: settings
                .wide_dynamic_range
                .as_ref()
                .map(|wdr| matches!(wdr.mode, WideDynamicMode::ON))
                .unwrap_or(false),
            backlight_compensation: settings
                .backlight_compensation
                .as_ref()
                .map(|blc| {
                    matches!(
                        blc.mode,
                        crate::onvif::types::common::BacklightCompensationMode::ON
                    )
                })
                .unwrap_or(false),
        }
    }

    /// Convert platform options to ONVIF format.
    fn platform_to_onvif_options(options: &ImagingOptions) -> ImagingOptions20 {
        use crate::onvif::types::imaging::{
            BacklightCompensationMode, BacklightCompensationOptions20, IrCutFilterMode,
            WideDynamicMode, WideDynamicRangeOptions20,
        };

        ImagingOptions20 {
            brightness: Some(FloatRange {
                min: options.brightness_range.0,
                max: options.brightness_range.1,
            }),
            contrast: Some(FloatRange {
                min: options.contrast_range.0,
                max: options.contrast_range.1,
            }),
            color_saturation: Some(FloatRange {
                min: options.saturation_range.0,
                max: options.saturation_range.1,
            }),
            sharpness: Some(FloatRange {
                min: options.sharpness_range.0,
                max: options.sharpness_range.1,
            }),
            ir_cut_filter_modes: if options.ir_cut_filter_supported {
                vec![
                    IrCutFilterMode::ON,
                    IrCutFilterMode::OFF,
                    IrCutFilterMode::AUTO,
                ]
            } else {
                vec![]
            },
            wide_dynamic_range: if options.wdr_supported {
                Some(WideDynamicRangeOptions20 {
                    mode: vec![WideDynamicMode::OFF, WideDynamicMode::ON],
                    level: Some(FloatRange {
                        min: 0.0,
                        max: 100.0,
                    }),
                })
            } else {
                None
            },
            backlight_compensation: if options.backlight_compensation_supported {
                Some(BacklightCompensationOptions20 {
                    mode: vec![
                        BacklightCompensationMode::OFF,
                        BacklightCompensationMode::ON,
                    ],
                    level: Some(FloatRange {
                        min: 0.0,
                        max: 100.0,
                    }),
                })
            } else {
                None
            },
            exposure: None,
            focus: None,
            white_balance: None,
            extension: None,
        }
    }

    /// Get default imaging settings.
    fn default_settings() -> ImagingSettings20 {
        use crate::onvif::types::common::IrCutFilterMode;

        ImagingSettings20 {
            brightness: Some(50.0),
            contrast: Some(50.0),
            color_saturation: Some(50.0),
            sharpness: Some(50.0),
            ir_cut_filter: Some(IrCutFilterMode::AUTO),
            wide_dynamic_range: None,
            backlight_compensation: None,
            exposure: None,
            focus: None,
            white_balance: None,
            extension: None,
        }
    }

    /// Get default imaging options.
    fn default_options() -> ImagingOptions20 {
        use crate::onvif::types::imaging::{
            BacklightCompensationMode, BacklightCompensationOptions20, IrCutFilterMode,
            WideDynamicMode, WideDynamicRangeOptions20,
        };

        ImagingOptions20 {
            brightness: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            contrast: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            color_saturation: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            sharpness: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            ir_cut_filter_modes: vec![
                IrCutFilterMode::ON,
                IrCutFilterMode::OFF,
                IrCutFilterMode::AUTO,
            ],
            wide_dynamic_range: Some(WideDynamicRangeOptions20 {
                mode: vec![WideDynamicMode::OFF, WideDynamicMode::ON],
                level: Some(FloatRange {
                    min: 0.0,
                    max: 100.0,
                }),
            }),
            backlight_compensation: Some(BacklightCompensationOptions20 {
                mode: vec![
                    BacklightCompensationMode::OFF,
                    BacklightCompensationMode::ON,
                ],
                level: Some(FloatRange {
                    min: 0.0,
                    max: 100.0,
                }),
            }),
            exposure: None,
            focus: None,
            white_balance: None,
            extension: None,
        }
    }
}

impl Default for ImagingSettingsStore {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_settings_store() {
        let store = ImagingSettingsStore::new();
        assert!(store.is_valid_token("VideoSource_1"));
        assert!(!store.is_valid_token("InvalidToken"));
    }

    #[test]
    fn test_add_video_source_token() {
        let store = ImagingSettingsStore::new();
        assert!(!store.is_valid_token("VideoSource_2"));
        store.add_video_source_token("VideoSource_2".to_string());
        assert!(store.is_valid_token("VideoSource_2"));
    }

    #[test]
    fn test_get_video_source_tokens() {
        let store = ImagingSettingsStore::new();
        let tokens = store.get_video_source_tokens();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], "VideoSource_1");
    }

    #[test]
    fn test_default_settings() {
        let settings = ImagingSettingsStore::default_settings();
        assert_eq!(settings.brightness, Some(50.0));
        assert_eq!(settings.contrast, Some(50.0));
        assert_eq!(settings.color_saturation, Some(50.0));
        assert_eq!(settings.sharpness, Some(50.0));
    }

    #[test]
    fn test_default_options() {
        let options = ImagingSettingsStore::default_options();
        assert!(options.brightness.is_some());
        let brightness = options.brightness.unwrap();
        assert_eq!(brightness.min, 0.0);
        assert_eq!(brightness.max, 100.0);
    }

    #[test]
    fn test_validate_parameter_in_range() {
        let store = ImagingSettingsStore::new();
        let options = ImagingSettingsStore::default_options();
        let result = store.validate_parameter("Brightness", 50.0, &options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_parameter_out_of_range() {
        let store = ImagingSettingsStore::new();
        let options = ImagingSettingsStore::default_options();
        let result = store.validate_parameter("Brightness", 150.0, &options);
        assert!(result.is_err());
        match result {
            Err(ImagingSettingsError::OutOfRange {
                parameter, value, ..
            }) => {
                assert_eq!(parameter, "Brightness");
                assert_eq!(value, 150.0);
            }
            _ => panic!("Expected OutOfRange error"),
        }
    }

    #[tokio::test]
    async fn test_get_settings_invalid_token() {
        let store = ImagingSettingsStore::new();
        let result = store.get_settings("InvalidToken").await;
        assert!(result.is_err());
        match result {
            Err(ImagingSettingsError::InvalidToken(token)) => {
                assert_eq!(token, "InvalidToken");
            }
            _ => panic!("Expected InvalidToken error"),
        }
    }

    #[tokio::test]
    async fn test_get_settings_default() {
        let store = ImagingSettingsStore::new();
        let result = store.get_settings("VideoSource_1").await;
        assert!(result.is_ok());
        let settings = result.unwrap();
        assert_eq!(settings.brightness, Some(50.0));
    }

    #[tokio::test]
    async fn test_get_options_default() {
        let store = ImagingSettingsStore::new();
        let result = store.get_options("VideoSource_1").await;
        assert!(result.is_ok());
        let options = result.unwrap();
        assert!(options.brightness.is_some());
    }

    #[tokio::test]
    async fn test_get_status() {
        let store = ImagingSettingsStore::new();
        let result = store.get_status("VideoSource_1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_settings_valid() {
        let store = ImagingSettingsStore::new();
        let mut settings = ImagingSettingsStore::default_settings();
        settings.brightness = Some(75.0);
        let result = store.set_settings("VideoSource_1", &settings, false).await;
        assert!(result.is_ok());

        // Verify settings were updated
        let retrieved = store.get_settings("VideoSource_1").await.unwrap();
        assert_eq!(retrieved.brightness, Some(75.0));
    }

    #[tokio::test]
    async fn test_set_settings_invalid_value() {
        let store = ImagingSettingsStore::new();
        let mut settings = ImagingSettingsStore::default_settings();
        settings.brightness = Some(150.0); // Out of range
        let result = store.set_settings("VideoSource_1", &settings, false).await;
        assert!(result.is_err());
    }
}
