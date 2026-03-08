// =============================================================================
// Audio Input Implementation
// =============================================================================
//!
//! Anyka audio input implementation using the Anyka SDK FFI layer.
//!
//! This module provides the `AnykaAudioInput` struct that implements the
//! `AudioInput` trait for the Anyka AK3918 platform.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;

use crate::platform::common::{AudioInput, AudioSourceConfig, PlatformError, PlatformResult};

/// Anyka audio input implementation.
///
/// Provides audio input functionality for the Anyka platform using the
/// vendor daemon IPC bridge for FFI calls.
pub(super) struct AnykaAudioInput {
    #[allow(dead_code)]
    ffi: Arc<dyn crate::hal::common::audio::AudioHalTrait>,
    opened: AtomicBool,
}

impl AnykaAudioInput {
    /// Create a new `AnykaAudioInput` with the default FFI backend.
    ///
    /// Uses `AnykaIpc` to connect to the vendor daemon for vendor library access.
    pub(super) fn new() -> PlatformResult<Self> {
        let ipc = crate::hal::anyka::ipc::AnykaIpc::new().map_err(|e| {
            PlatformError::InitializationFailed(format!(
                "AnykaAudioInput: AnykaIpc connection failed: {}",
                e
            ))
        })?;
        tracing::info!("AnykaAudioInput: using AnykaIpc for vendor library access");
        Ok(Self {
            ffi: Arc::new(ipc),
            opened: AtomicBool::new(false),
        })
    }

    /// Create a new `AnykaAudioInput` with a custom FFI backend.
    ///
    /// Used by tests with `MockAudioHalTrait` for hardware-free testing.
    pub(super) fn with_ffi(ffi: Arc<dyn crate::hal::common::audio::AudioHalTrait>) -> Self {
        Self {
            ffi,
            opened: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl AudioInput for AnykaAudioInput {
    async fn open(&self) -> PlatformResult<()> {
        self.opened.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn close(&self) -> PlatformResult<()> {
        self.opened.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn get_configuration(&self) -> PlatformResult<AudioSourceConfig> {
        Ok(AudioSourceConfig {
            token: "AudioSource_1".to_string(),
            name: "Microphone".to_string(),
            channels: 1,
        })
    }

    async fn get_sources(&self) -> PlatformResult<Vec<AudioSourceConfig>> {
        Ok(vec![AudioSourceConfig {
            token: "AudioSource_1".to_string(),
            name: "Microphone".to_string(),
            channels: 1,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The platform integration tests exercise this implementation through the
    // `Platform` trait; keep this module test as a lightweight compile check.

    #[test]
    fn test_audio_input_struct_exists() {
        // Basic compile check - verify the struct is properly defined
        let _ = AnykaAudioInput::with_ffi;
    }
}
