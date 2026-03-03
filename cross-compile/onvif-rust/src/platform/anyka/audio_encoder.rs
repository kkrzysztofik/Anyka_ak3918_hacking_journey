// =============================================================================
// Audio Encoder Implementation
// =============================================================================
//!
//! Anyka audio encoder implementation using the Anyka SDK FFI layer.
//!
//! This module provides the `AnykaAudioEncoder` struct that implements the
//! `AudioEncoder` trait for the Anyka AK3918 platform.

use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use crate::platform::common::{AudioEncoder, AudioEncoderConfig, PlatformError, PlatformResult};

/// Anyka audio encoder implementation.
///
/// Provides audio encoding functionality for the Anyka platform using the
/// vendor daemon IPC bridge for FFI calls.
pub(super) struct AnykaAudioEncoder {
    #[allow(dead_code)]
    ffi: Arc<dyn crate::hal::common::audio::AudioHalTrait>,
    configurations: RwLock<Vec<AudioEncoderConfig>>,
}

impl AnykaAudioEncoder {
    /// Create a new `AnykaAudioEncoder` with the default FFI backend.
    ///
    /// Uses `AnykaIpc` to connect to the vendor daemon for vendor library access.
    pub(super) fn new() -> PlatformResult<Self> {
        let ffi: Arc<dyn crate::hal::common::audio::AudioHalTrait> = {
            let ipc = crate::hal::anyka::ipc::AnykaIpc::new().map_err(|e| {
                PlatformError::InitializationFailed(format!(
                    "AnykaAudioEncoder: AnykaIpc connection failed: {}",
                    e
                ))
            })?;
            tracing::info!("AnykaAudioEncoder: using AnykaIpc for vendor library access");
            Arc::new(ipc)
        };

        Ok(Self::with_ffi(ffi))
    }

    /// Create a new `AnykaAudioEncoder` with a custom FFI backend.
    ///
    /// Used by tests with `MockAudioHalTrait` for hardware-free testing.
    pub(super) fn with_ffi(ffi: Arc<dyn crate::hal::common::audio::AudioHalTrait>) -> Self {
        Self {
            ffi,
            configurations: RwLock::new(vec![AudioEncoderConfig {
                token: "AudioEncoder_1".to_string(),
                name: "Audio Stream".to_string(),
                sample_rate: 8000,
                channels: 1,
                ..Default::default()
            }]),
        }
    }
}

#[async_trait]
impl AudioEncoder for AnykaAudioEncoder {
    async fn init(&self, config: &AudioEncoderConfig) -> PlatformResult<()> {
        // TODO(github#28): Call ak_aenc_open() with actual config via FFI (functional change - out of scope for refactoring PR)
        let mut configs = self.configurations.write();
        if let Some(cfg) = configs.iter_mut().find(|c| c.token == config.token) {
            *cfg = config.clone();
        } else {
            configs.push(config.clone());
        }
        Ok(())
    }

    async fn get_configuration(&self) -> PlatformResult<AudioEncoderConfig> {
        let configs = self.configurations.read();
        configs
            .first()
            .cloned()
            .ok_or_else(|| PlatformError::HardwareUnavailable("No audio encoder".to_string()))
    }

    async fn set_configuration(&self, config: &AudioEncoderConfig) -> PlatformResult<()> {
        // TODO(github#28): Call ak_aenc_set_config() or similar via FFI (functional change - out of scope for refactoring PR)
        let mut configs = self.configurations.write();
        if let Some(cfg) = configs.iter_mut().find(|c| c.token == config.token) {
            *cfg = config.clone();
            Ok(())
        } else {
            Err(PlatformError::InvalidParameter(format!(
                "Unknown audio encoder token: {}",
                config.token
            )))
        }
    }

    async fn get_configurations(&self) -> PlatformResult<Vec<AudioEncoderConfig>> {
        Ok(self.configurations.read().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO(github#28): Replace placeholder test with real unit tests using mockall
    // (functional change - out of scope for refactoring PR)
    //
    // Note: Full tests would require mock FFI backends which are not available
    // in the test environment. The implementations are tested indirectly through
    // integration tests that exercise the Platform trait.

    #[test]
    fn test_audio_encoder_struct_exists() {
        // Basic compile check - verify the struct is properly defined
        let _ = AnykaAudioEncoder::with_ffi;
    }
}
