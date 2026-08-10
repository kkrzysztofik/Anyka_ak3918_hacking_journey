//! Profile persistence module.
//!
//! Provides typed, serde-based persistence for ONVIF media profiles,
//! video/audio sources, and encoder configurations.
//!
//! This module intentionally uses only primitive types (no `onvif::*` imports)
//! to maintain a clean dependency direction: `onvif` → `config`, never the reverse.
//! Conversion between stored DTOs and ONVIF domain types happens in `profile_manager.rs`.

use std::fs;
use std::path::{Path, PathBuf};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// Error type
// ============================================================================

/// Errors from profile storage operations.
#[derive(Debug, Error)]
pub enum ProfileError {
    /// File I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML parsing error.
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// TOML serialization error.
    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
}

// ============================================================================
// Serde DTOs
// ============================================================================

/// Root structure for `profiles.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfilesFile {
    #[serde(default)]
    pub profiles: Vec<StoredProfile>,
    #[serde(default)]
    pub video_sources: Vec<StoredVideoSource>,
    #[serde(default)]
    pub audio_sources: Vec<StoredAudioSource>,
    #[serde(default)]
    pub video_source_configs: Vec<StoredVideoSourceConfig>,
    #[serde(default)]
    pub video_encoder_configs: Vec<StoredVideoEncoderConfig>,
    #[serde(default)]
    pub audio_source_configs: Vec<StoredAudioSourceConfig>,
    #[serde(default)]
    pub audio_encoder_configs: Vec<StoredAudioEncoderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProfile {
    pub token: String,
    pub name: String,
    #[serde(default)]
    pub fixed: bool,
    pub video_source_config: Option<String>,
    pub video_encoder_config: Option<String>,
    pub audio_source_config: Option<String>,
    pub audio_encoder_config: Option<String>,
    pub ptz_config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredVideoSource {
    pub token: String,
    #[serde(default = "default_framerate")]
    pub framerate: f64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredVideoSourceConfig {
    pub token: String,
    pub source_token: String,
    pub name: String,
    #[serde(default)]
    pub use_count: u32,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub rotated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredVideoEncoderConfig {
    pub token: String,
    pub name: String,
    /// `"H264"`, `"MJPEG"`, `"MPEG4"`
    pub encoding: String,
    pub width: u32,
    pub height: u32,
    #[serde(default = "default_quality")]
    pub quality: f64,
    pub frame_rate_limit: Option<u32>,
    pub encoding_interval: Option<u32>,
    pub bitrate_limit: Option<u32>,
    pub gov_length: Option<u32>,
    /// `"Baseline"`, `"Main"`, `"Extended"`, `"High"`
    pub h264_profile: Option<String>,
    pub session_timeout: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAudioSource {
    pub token: String,
    #[serde(default = "default_channels")]
    pub channels: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAudioSourceConfig {
    pub token: String,
    pub source_token: String,
    pub name: String,
    #[serde(default)]
    pub use_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAudioEncoderConfig {
    pub token: String,
    pub name: String,
    /// `"G711"`, `"G726"`, `"AAC"`
    pub encoding: String,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub session_timeout: Option<String>,
}

// ============================================================================
// Default helpers
// ============================================================================

fn default_framerate() -> f64 {
    30.0
}

fn default_quality() -> f64 {
    0.8
}

fn default_channels() -> u32 {
    1
}

// ============================================================================
// ProfileStorage
// ============================================================================

/// Thread-safe profile storage with TOML file persistence.
///
/// Owns its file path, following the same pattern as `ConfigStorage`.
/// Create with `new(path)`, then call `load()` / `save()` as needed.
pub struct ProfileStorage {
    data: RwLock<ProfilesFile>,
    path: PathBuf,
}

impl ProfileStorage {
    /// Create storage bound to a file path. Does not load yet.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            data: RwLock::new(ProfilesFile::default()),
            path: path.into(),
        }
    }

    /// Load profiles from the bound file.
    ///
    /// If the file does not exist, returns `Ok(())` (no-op).
    pub fn load(&self) -> Result<(), ProfileError> {
        if !self.path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.path)?;
        let profiles: ProfilesFile = toml::from_str(&content)?;
        *self.data.write() = profiles;
        Ok(())
    }

    /// Save profiles to the bound file atomically.
    pub fn save(&self) -> Result<(), ProfileError> {
        let content = toml::to_string_pretty(&*self.data.read())?;
        super::file_ops::atomic_write(&self.path, content.as_bytes(), None)?;
        Ok(())
    }

    /// Get the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if profile data is empty (no profiles loaded).
    pub fn is_empty(&self) -> bool {
        self.data.read().profiles.is_empty()
    }

    /// Get a read-only snapshot of the data.
    pub fn snapshot(&self) -> ProfilesFile {
        self.data.read().clone()
    }

    /// Replace the entire data set.
    pub fn replace(&self, data: ProfilesFile) {
        *self.data.write() = data;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_profile_storage_new_is_empty() {
        let dir = tempdir().unwrap();
        let storage = ProfileStorage::new(dir.path().join("profiles.toml"));
        assert!(storage.is_empty());
    }

    #[test]
    fn test_profile_storage_replace_and_snapshot() {
        let dir = tempdir().unwrap();
        let storage = ProfileStorage::new(dir.path().join("profiles.toml"));

        let data = ProfilesFile {
            profiles: vec![StoredProfile {
                token: "Profile_Main".to_string(),
                name: "MainStream".to_string(),
                fixed: true,
                video_source_config: Some("VSC_0".to_string()),
                video_encoder_config: Some("VEC_0".to_string()),
                audio_source_config: None,
                audio_encoder_config: None,
                ptz_config: Some("PTZ_0".to_string()),
            }],
            video_sources: vec![StoredVideoSource {
                token: "VS_0".to_string(),
                framerate: 25.0,
                width: 1920,
                height: 1080,
            }],
            ..Default::default()
        };

        storage.replace(data);
        assert!(!storage.is_empty());

        let snap = storage.snapshot();
        assert_eq!(snap.profiles.len(), 1);
        assert_eq!(snap.profiles[0].name, "MainStream");
        assert_eq!(snap.video_sources.len(), 1);
    }

    #[test]
    fn test_profile_storage_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("profiles.toml");

        // Save
        let storage1 = ProfileStorage::new(&path);
        storage1.replace(ProfilesFile {
            profiles: vec![StoredProfile {
                token: "Profile_Main".to_string(),
                name: "MainStream".to_string(),
                fixed: true,
                video_source_config: Some("VSC_0".to_string()),
                video_encoder_config: Some("VEC_0".to_string()),
                audio_source_config: None,
                audio_encoder_config: None,
                ptz_config: None,
            }],
            video_encoder_configs: vec![StoredVideoEncoderConfig {
                token: "VEC_0".to_string(),
                name: "MainEncoder".to_string(),
                encoding: "H264".to_string(),
                width: 1920,
                height: 1080,
                quality: 0.8,
                frame_rate_limit: Some(25),
                encoding_interval: Some(1),
                bitrate_limit: Some(4000),
                gov_length: Some(25),
                h264_profile: Some("Main".to_string()),
                session_timeout: Some("PT60S".to_string()),
            }],
            ..Default::default()
        });
        storage1.save().unwrap();

        // Load
        let storage2 = ProfileStorage::new(&path);
        storage2.load().unwrap();

        let snap = storage2.snapshot();
        assert_eq!(snap.profiles.len(), 1);
        assert_eq!(snap.profiles[0].token, "Profile_Main");
        assert_eq!(snap.video_encoder_configs.len(), 1);
        assert_eq!(snap.video_encoder_configs[0].encoding, "H264");
        assert_eq!(snap.video_encoder_configs[0].bitrate_limit, Some(4000));
    }

    #[test]
    fn test_profile_storage_load_nonexistent_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("does_not_exist.toml");
        let storage = ProfileStorage::new(&path);
        let result = storage.load();
        assert!(result.is_ok());
        assert!(storage.is_empty());
    }

    #[test]
    fn test_profile_storage_roundtrip_all_types() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("profiles.toml");

        let original = ProfilesFile {
            profiles: vec![StoredProfile {
                token: "Profile_Main".to_string(),
                name: "MainStream".to_string(),
                fixed: true,
                video_source_config: Some("VSC_0".to_string()),
                video_encoder_config: Some("VEC_0".to_string()),
                audio_source_config: Some("ASC_0".to_string()),
                audio_encoder_config: Some("AEC_0".to_string()),
                ptz_config: Some("PTZ_0".to_string()),
            }],
            video_sources: vec![StoredVideoSource {
                token: "VS_0".to_string(),
                framerate: 25.0,
                width: 1920,
                height: 1080,
            }],
            audio_sources: vec![StoredAudioSource {
                token: "AS_0".to_string(),
                channels: 1,
            }],
            video_source_configs: vec![StoredVideoSourceConfig {
                token: "VSC_0".to_string(),
                source_token: "VS_0".to_string(),
                name: "VideoSrcCfg0".to_string(),
                use_count: 1,
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                rotated: false,
            }],
            video_encoder_configs: vec![StoredVideoEncoderConfig {
                token: "VEC_0".to_string(),
                name: "MainEncoder".to_string(),
                encoding: "H264".to_string(),
                width: 1920,
                height: 1080,
                quality: 0.8,
                frame_rate_limit: Some(25),
                encoding_interval: Some(1),
                bitrate_limit: Some(4000),
                gov_length: Some(25),
                h264_profile: Some("Main".to_string()),
                session_timeout: Some("PT60S".to_string()),
            }],
            audio_source_configs: vec![StoredAudioSourceConfig {
                token: "ASC_0".to_string(),
                source_token: "AS_0".to_string(),
                name: "AudioSrcCfg0".to_string(),
                use_count: 1,
            }],
            audio_encoder_configs: vec![StoredAudioEncoderConfig {
                token: "AEC_0".to_string(),
                name: "AudioEnc0".to_string(),
                encoding: "G711".to_string(),
                bitrate: Some(64),
                sample_rate: Some(8000),
                session_timeout: Some("PT60S".to_string()),
            }],
        };

        let storage = ProfileStorage::new(&path);
        storage.replace(original.clone());
        storage.save().unwrap();

        let storage2 = ProfileStorage::new(&path);
        storage2.load().unwrap();

        let loaded = storage2.snapshot();
        assert_eq!(loaded.profiles.len(), original.profiles.len());
        assert_eq!(loaded.video_sources.len(), original.video_sources.len());
        assert_eq!(loaded.audio_sources.len(), original.audio_sources.len());
        assert_eq!(
            loaded.video_source_configs.len(),
            original.video_source_configs.len()
        );
        assert_eq!(
            loaded.video_encoder_configs.len(),
            original.video_encoder_configs.len()
        );
        assert_eq!(
            loaded.audio_source_configs.len(),
            original.audio_source_configs.len()
        );
        assert_eq!(
            loaded.audio_encoder_configs.len(),
            original.audio_encoder_configs.len()
        );

        // Verify detailed fields
        assert_eq!(loaded.video_sources[0].framerate, 25.0);
        assert_eq!(loaded.audio_sources[0].channels, 1);
        assert_eq!(loaded.video_source_configs[0].x, 0);
        assert_eq!(
            loaded.video_encoder_configs[0].h264_profile,
            Some("Main".to_string())
        );
        assert_eq!(loaded.audio_encoder_configs[0].bitrate, Some(64));
    }

    #[test]
    fn test_serde_defaults() {
        let toml_str = r#"
[[profiles]]
token = "P1"
name = "Test"

[[video_sources]]
token = "VS1"
width = 1920
height = 1080
"#;

        let data: ProfilesFile = toml::from_str(toml_str).unwrap();
        assert!(!data.profiles[0].fixed);
        assert_eq!(data.video_sources[0].framerate, 30.0); // default
        assert!(data.audio_sources.is_empty()); // missing = empty vec
    }

    #[test]
    fn test_stored_video_source_config_rotated_defaults_false_when_absent() {
        let toml = r#"
            token = "VideoSourceConfig_0"
            source_token = "VideoSource_1"
            name = "Main"
            width = 1920
            height = 1080
        "#;
        let cfg: StoredVideoSourceConfig = toml::from_str(toml).unwrap();
        assert!(!cfg.rotated);
    }
}
