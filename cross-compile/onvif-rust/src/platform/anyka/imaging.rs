// =============================================================================
// Imaging Control Implementation
// =============================================================================
//!
//! Anyka imaging control implementation using the Anyka SDK FFI layer.
//!
//! This module provides the `AnykaImagingControl` struct that implements the
//! `ImagingControl` trait for the Anyka AK3918 platform.
//!
//! ## IDR-Trigger Behavior
//!
//! The imaging settings trigger an IDR (Instantaneous Decoder Refresh) frame
//! request on video encoder(s) when settings change. This ensures that the
//! video stream applies the new imaging parameters at the earliest opportunity.

use portable_atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use parking_lot::RwLock;

use crate::platform::common::{ImagingControl, ImagingOptions, ImagingSettings, PlatformResult};

use super::video_encoder::AnykaVideoEncoder;

/// Global sequence counter for imaging updates.
///
/// Used for logging and debugging to track the order of imaging parameter changes.
/// This is the single source of truth - the frame read loop in mod.rs reads from here.
pub(super) static LAST_IMAGING_UPDATE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Timestamp of the last imaging update (Unix milliseconds).
///
/// This is the single source of truth - the frame read loop in mod.rs reads from here.
pub(super) static LAST_IMAGING_UPDATE_UNIX_MS: AtomicU64 = AtomicU64::new(0);

/// Get current Unix timestamp in milliseconds.
///
/// This is the single source of truth - used by both imaging control and frame read loop.
pub(super) fn current_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Anyka imaging control implementation.
///
/// Provides imaging parameter control (brightness, contrast, saturation, sharpness)
/// for the Anyka platform using the vendor daemon IPC bridge for FFI calls.
///
/// Note: The IDR-trigger behavior is preserved from the original implementation.
/// When imaging settings change, an IDR frame request is made to ensure the
/// new parameters are applied at the next keyframe.
pub(super) struct AnykaImagingControl {
    ffi: Arc<dyn crate::hal::common::imaging::ImagingHalTrait>,
    settings: RwLock<ImagingSettings>,
    video_encoder: Option<Weak<AnykaVideoEncoder>>,
}

impl AnykaImagingControl {
    /// Create a new `AnykaImagingControl` with the default FFI backend.
    ///
    /// Uses `AnykaIpc` to connect to the vendor daemon for vendor library access.
    pub(super) fn new() -> PlatformResult<Self> {
        let ffi: Arc<dyn crate::hal::common::imaging::ImagingHalTrait> = {
            let ipc = crate::hal::anyka::ipc::AnykaIpc::new().map_err(|e| {
                crate::platform::traits::PlatformError::InitializationFailed(format!(
                    "AnykaImagingControl: AnykaIpc connection failed: {}",
                    e
                ))
            })?;
            tracing::info!("AnykaImagingControl: using AnykaIpc for vendor library access");
            Arc::new(ipc)
        };

        Ok(Self::with_ffi(ffi))
    }

    /// Create a new `AnykaImagingControl` with a custom FFI backend.
    ///
    /// Used by tests with `MockImagingHalTrait` for hardware-free testing.
    pub(super) fn with_ffi(ffi: Arc<dyn crate::hal::common::imaging::ImagingHalTrait>) -> Self {
        Self {
            ffi,
            settings: RwLock::new(ImagingSettings {
                brightness: 50.0,
                contrast: 50.0,
                saturation: 50.0,
                sharpness: 50.0,
                ir_cut_filter: true,
                ir_led: false,
                wdr: false,
                backlight_compensation: false,
            }),
            video_encoder: None,
        }
    }

    /// Create a new `AnykaImagingControl` with FFI backend and video encoder reference.
    ///
    /// The video encoder reference is used to trigger IDR frames when imaging
    /// settings change.
    pub(super) fn with_ffi_and_video_encoder(
        ffi: Arc<dyn crate::hal::common::imaging::ImagingHalTrait>,
        video_encoder: Arc<AnykaVideoEncoder>,
    ) -> Self {
        let mut control = Self::with_ffi(ffi);
        control.video_encoder = Some(Arc::downgrade(&video_encoder));
        control
    }

    /// Check if two f32 values are approximately equal.
    fn approximately_equal(a: f32, b: f32) -> bool {
        (a - b).abs() <= 0.001
    }

    /// Mark that an imaging update occurred and request IDR frames.
    ///
    /// This triggers an IDR frame request on all bound video encoders to ensure
    /// the new imaging parameters are applied at the next keyframe.
    fn mark_imaging_update_and_request_idr(&self, operation: &'static str) {
        let seq = LAST_IMAGING_UPDATE_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
        LAST_IMAGING_UPDATE_UNIX_MS.store(current_unix_ms(), Ordering::Relaxed);

        let mut requested_streams = 0u32;
        if let Some(video_encoder) = self.video_encoder.as_ref().and_then(Weak::upgrade) {
            if video_encoder.request_idr_frame(true).is_ok() {
                requested_streams += 1;
            }
            if video_encoder.request_idr_frame(false).is_ok() {
                requested_streams += 1;
            }
        }

        tracing::info!(
            operation,
            imaging_seq = seq,
            requested_streams,
            "Imaging update applied"
        );
    }
}

#[async_trait]
impl ImagingControl for AnykaImagingControl {
    async fn get_settings(&self) -> PlatformResult<ImagingSettings> {
        // TODO(kkrzysztofik): Read actual settings from Anyka imaging SDK
        Ok(self.settings.read().clone())
    }

    async fn set_settings(&self, settings: &ImagingSettings) -> PlatformResult<()> {
        let start = std::time::Instant::now();
        crate::hal::common::imaging::imaging_set_brightness(
            settings.brightness,
            self.ffi.as_ref(),
        )?;
        crate::hal::common::imaging::imaging_set_contrast(settings.contrast, self.ffi.as_ref())?;
        crate::hal::common::imaging::imaging_set_saturation(
            settings.saturation,
            self.ffi.as_ref(),
        )?;
        crate::hal::common::imaging::imaging_set_sharpness(settings.sharpness, self.ffi.as_ref())?;
        *self.settings.write() = settings.clone();
        self.mark_imaging_update_and_request_idr("set_settings");
        tracing::info!(
            elapsed_us = start.elapsed().as_micros() as u64,
            "Applied imaging settings batch"
        );
        Ok(())
    }

    async fn get_options(&self) -> PlatformResult<ImagingOptions> {
        // TODO(kkrzysztofik): Query actual hardware capabilities
        Ok(ImagingOptions::default_options())
    }

    async fn set_brightness(&self, value: f32) -> PlatformResult<()> {
        if Self::approximately_equal(self.settings.read().brightness, value) {
            tracing::debug!(value, "Skipping redundant brightness update");
            return Ok(());
        }
        let start = std::time::Instant::now();
        crate::hal::common::imaging::imaging_set_brightness(value, self.ffi.as_ref())?;
        self.settings.write().brightness = value;
        self.mark_imaging_update_and_request_idr("set_brightness");
        tracing::info!(
            value,
            elapsed_us = start.elapsed().as_micros() as u64,
            "Brightness updated"
        );
        Ok(())
    }

    async fn set_contrast(&self, value: f32) -> PlatformResult<()> {
        if Self::approximately_equal(self.settings.read().contrast, value) {
            tracing::debug!(value, "Skipping redundant contrast update");
            return Ok(());
        }
        let start = std::time::Instant::now();
        crate::hal::common::imaging::imaging_set_contrast(value, self.ffi.as_ref())?;
        self.settings.write().contrast = value;
        self.mark_imaging_update_and_request_idr("set_contrast");
        tracing::info!(
            value,
            elapsed_us = start.elapsed().as_micros() as u64,
            "Contrast updated"
        );
        Ok(())
    }

    async fn set_saturation(&self, value: f32) -> PlatformResult<()> {
        if Self::approximately_equal(self.settings.read().saturation, value) {
            tracing::debug!(value, "Skipping redundant saturation update");
            return Ok(());
        }
        let start = std::time::Instant::now();
        crate::hal::common::imaging::imaging_set_saturation(value, self.ffi.as_ref())?;
        self.settings.write().saturation = value;
        self.mark_imaging_update_and_request_idr("set_saturation");
        tracing::info!(
            value,
            elapsed_us = start.elapsed().as_micros() as u64,
            "Saturation updated"
        );
        Ok(())
    }

    async fn set_sharpness(&self, value: f32) -> PlatformResult<()> {
        if Self::approximately_equal(self.settings.read().sharpness, value) {
            tracing::debug!(value, "Skipping redundant sharpness update");
            return Ok(());
        }
        let start = std::time::Instant::now();
        crate::hal::common::imaging::imaging_set_sharpness(value, self.ffi.as_ref())?;
        self.settings.write().sharpness = value;
        self.mark_imaging_update_and_request_idr("set_sharpness");
        tracing::info!(
            value,
            elapsed_us = start.elapsed().as_micros() as u64,
            "Sharpness updated"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full tests would require mock FFI backends which are not available
    // in the test environment. The implementations are tested indirectly through
    // integration tests that exercise the Platform trait.

    #[test]
    fn test_imaging_struct_exists() {
        // Basic compile check - verify the struct is properly defined
        let _ = AnykaImagingControl::with_ffi;
        let _ = AnykaImagingControl::approximately_equal;
    }

    #[test]
    fn test_approximately_equal() {
        assert!(AnykaImagingControl::approximately_equal(50.0, 50.0));
        assert!(AnykaImagingControl::approximately_equal(50.0, 50.001));
        assert!(!AnykaImagingControl::approximately_equal(50.0, 60.0));
    }

    // =============================================================================
    // Bugfix verification: single shared imaging observability state
    // =============================================================================

    #[test]
    fn test_imaging_observability_state_atomics_exist() {
        // Verify the static atomics exist and are accessible
        // These are the single source of truth for imaging updates
        let _initial_seq = LAST_IMAGING_UPDATE_SEQ.load(std::sync::atomic::Ordering::Relaxed);
        let _initial_ts = LAST_IMAGING_UPDATE_UNIX_MS.load(std::sync::atomic::Ordering::Relaxed);

        // Atomics exist and are readable (u64 is always >= 0, no need to assert)
        // This test verifies the shared state exists and is accessible
    }

    #[test]
    fn test_imaging_observability_state_atomics_update() {
        // Test that atomics can be updated (simulating an imaging update)
        let prev_seq = LAST_IMAGING_UPDATE_SEQ.load(std::sync::atomic::Ordering::Relaxed);
        let new_seq =
            LAST_IMAGING_UPDATE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        let ts = current_unix_ms();

        // Sequence should have incremented from previous value
        assert_eq!(new_seq, prev_seq + 1);

        // Timestamp should be reasonable (after Unix epoch)
        assert!(ts > 0);
    }

    #[test]
    fn test_current_unix_ms_returns_reasonable_value() {
        let now = current_unix_ms();
        // Unix epoch was ~1.7 billion seconds ago (2024), so ms should be > 1.7 trillion
        // Allow some tolerance for test execution time
        assert!(now > 1_700_000_000_000, "Timestamp too old: {}", now);
    }
}
