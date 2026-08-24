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

    async fn start(
        &self,
        bridge: &Arc<crate::streaming::bridge::StreamingBridge>,
        sample_rate: u32,
        channels: u32,
    ) -> PlatformResult<()> {
        // Start microphone capture and advertise the audio track.
        //
        // Ordering matters: the ASC is published only after the daemon accepts
        // the push request, so the SDP never promises a track the camera is
        // not actually sending.
        self.ffi.start_audio_push(sample_rate, channels)?;

        bridge.set_audio_config(crate::streaming::helpers::aac_audio_specific_config(
            sample_rate,
            channels,
        ));

        tracing::info!(sample_rate, channels, "Audio capture started");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hal::common::audio::MockAudioHalTrait;
    use crate::streaming::bridge::{LowLatencyFrameQueue, StreamingBridge};
    use mockall::predicate::eq;

    fn make_test_bridge() -> Arc<StreamingBridge> {
        Arc::new(StreamingBridge::new(
            LowLatencyFrameQueue::new("test-main", 4),
            LowLatencyFrameQueue::new("test-sub", 4),
            8000,
        ))
    }

    #[tokio::test]
    async fn test_start_publishes_audio_config_and_requests_push() {
        // Audio must not be advertised until the daemon has accepted the push
        // request: an SDP that promises a track the camera never sends leaves
        // clients waiting on RTP that never arrives.
        let mut mock = MockAudioHalTrait::new();
        mock.expect_start_audio_push()
            .with(eq(8000), eq(1))
            .times(1)
            .returning(|_, _| Ok(()));

        let encoder = AnykaAudioEncoder::with_ffi(Arc::new(mock));
        let bridge = make_test_bridge();

        encoder.start(&bridge, 8000, 1).await.unwrap();

        assert_eq!(
            bridge.audio_config.read().as_deref(),
            Some([0x15, 0x88].as_slice())
        );
    }

    #[tokio::test]
    async fn test_start_leaves_audio_config_none_when_daemon_rejects() {
        // Audio is strictly additive; a failed mic must never take video down
        // and must never advertise a track.
        let mut mock = MockAudioHalTrait::new();
        mock.expect_start_audio_push()
            .times(1)
            .returning(|_, _| Err(PlatformError::HardwareFailure("mic".into())));

        let encoder = AnykaAudioEncoder::with_ffi(Arc::new(mock));
        let bridge = make_test_bridge();

        let result = encoder.start(&bridge, 8000, 1).await;
        assert!(result.is_err());
        assert!(bridge.audio_config.read().is_none());
    }

    // The platform integration tests exercise this implementation through the
    // `Platform` trait; keep this module test as a lightweight compile check.

    #[test]
    fn test_audio_encoder_struct_exists() {
        // Basic compile check - verify the struct is properly defined
        let _ = AnykaAudioEncoder::with_ffi;
    }
}
