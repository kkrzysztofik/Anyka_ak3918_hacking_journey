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
    night: Arc<super::night_mode::NightModeController>,
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
        Self::with_ffi_and_paths(
            ffi,
            super::night_mode::NodePaths::default(),
            crate::config::types::ImagingConfig::default(),
            None,
        )
    }

    /// Create imaging control with injectable GPIO/sensor paths (tests + production).
    ///
    /// `idr` fires after every day/night transition; `None` when there is no
    /// video encoder to ask for a keyframe.
    pub(super) fn with_ffi_and_paths(
        ffi: Arc<dyn crate::hal::common::imaging::ImagingHalTrait>,
        paths: super::night_mode::NodePaths,
        cfg: crate::config::types::ImagingConfig,
        idr: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> Self {
        let night = Arc::new(super::night_mode::NightModeController::new(
            paths,
            cfg.night,
            Arc::clone(&ffi),
            cfg.ir_cut_filter,
            idr,
        ));
        Self {
            ffi,
            settings: RwLock::new(ImagingSettings {
                brightness: cfg.brightness as f32,
                contrast: cfg.contrast as f32,
                saturation: cfg.saturation as f32,
                sharpness: cfg.sharpness as f32,
                ir_cut_filter: cfg.ir_cut_filter,
                ir_led: cfg.ir_led,
                wdr: false,
                backlight_compensation: false,
            }),
            video_encoder: None,
            night,
        }
    }

    /// Night-mode controller shared with auxiliary command handlers.
    pub(crate) fn night_mode(&self) -> Arc<super::night_mode::NightModeController> {
        Arc::clone(&self.night)
    }

    /// Create a new `AnykaImagingControl` with FFI backend and video encoder reference.
    ///
    /// The video encoder reference is used to trigger IDR frames when imaging
    /// settings change.
    pub(super) fn with_ffi_and_video_encoder(
        ffi: Arc<dyn crate::hal::common::imaging::ImagingHalTrait>,
        video_encoder: Arc<AnykaVideoEncoder>,
        cfg: crate::config::types::ImagingConfig,
    ) -> Self {
        let enc = Arc::clone(&video_encoder);
        let mut control = Self::with_ffi_and_paths(
            ffi,
            super::night_mode::NodePaths::default(),
            cfg,
            Some(Arc::new(move || {
                let _ = enc.request_idr_frame(true);
                let _ = enc.request_idr_frame(false);
            })),
        );
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
        Ok(self.settings.read().clone())
    }

    async fn set_settings(&self, settings: &ImagingSettings) -> PlatformResult<()> {
        use super::night_mode::DayNight;
        use crate::onvif::types::common::IrCutFilterMode;

        let start = std::time::Instant::now();
        let current = self.settings.read().clone();

        // Day/night first: GPIO transitions must not be blocked by ISP color
        // controls (which can fail independently over IPC).
        match settings.ir_cut_filter {
            IrCutFilterMode::ON => {
                self.night.set_auto_enabled(false);
                self.night.apply(DayNight::Day).await?;
            }
            IrCutFilterMode::OFF => {
                self.night.set_auto_enabled(false);
                self.night.apply(DayNight::Night).await?;
            }
            IrCutFilterMode::AUTO => {
                self.night.set_auto_enabled(true);
            }
        }

        if !Self::approximately_equal(current.brightness, settings.brightness) {
            crate::hal::common::imaging::imaging_set_brightness(
                settings.brightness,
                self.ffi.as_ref(),
            )
            .await?;
        }
        if !Self::approximately_equal(current.contrast, settings.contrast) {
            crate::hal::common::imaging::imaging_set_contrast(settings.contrast, self.ffi.as_ref())
                .await?;
        }
        if !Self::approximately_equal(current.saturation, settings.saturation) {
            crate::hal::common::imaging::imaging_set_saturation(
                settings.saturation,
                self.ffi.as_ref(),
            )
            .await?;
        }
        if !Self::approximately_equal(current.sharpness, settings.sharpness) {
            crate::hal::common::imaging::imaging_set_sharpness(
                settings.sharpness,
                self.ffi.as_ref(),
            )
            .await?;
        }

        *self.settings.write() = settings.clone();
        self.mark_imaging_update_and_request_idr("set_settings");
        tracing::info!(
            elapsed_us = start.elapsed().as_micros() as u64,
            "Applied imaging settings batch"
        );
        Ok(())
    }

    async fn get_options(&self) -> PlatformResult<ImagingOptions> {
        let caps = self.night.capabilities();
        Ok(ImagingOptions {
            ir_cut_filter_supported: caps.ircut,
            ir_led_supported: caps.ir_led,
            white_light_supported: caps.white_led,
            ..ImagingOptions::default_options()
        })
    }

    async fn set_brightness(&self, value: f32) -> PlatformResult<()> {
        if Self::approximately_equal(self.settings.read().brightness, value) {
            tracing::debug!(value, "Skipping redundant brightness update");
            return Ok(());
        }
        let start = std::time::Instant::now();
        crate::hal::common::imaging::imaging_set_brightness(value, self.ffi.as_ref()).await?;
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
        crate::hal::common::imaging::imaging_set_contrast(value, self.ffi.as_ref()).await?;
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
        crate::hal::common::imaging::imaging_set_saturation(value, self.ffi.as_ref()).await?;
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
        crate::hal::common::imaging::imaging_set_sharpness(value, self.ffi.as_ref()).await?;
        self.settings.write().sharpness = value;
        self.mark_imaging_update_and_request_idr("set_sharpness");
        tracing::info!(
            value,
            elapsed_us = start.elapsed().as_micros() as u64,
            "Sharpness updated"
        );
        Ok(())
    }

    async fn set_ir_lamp(&self, on: bool) -> PlatformResult<()> {
        use super::night_mode::{DayNight, Node};
        use crate::onvif::types::common::IrCutFilterMode;

        self.night.set_auto_enabled(false);
        self.night
            .write_lamp(Node::IrLed, on)
            .await
            .map_err(|e| crate::platform::common::PlatformError::HardwareFailure(e.to_string()))?;

        // AUTO no longer owns the filter, so reporting AUTO would be a lie.
        // The filter itself did not move: report where it actually is.
        // Resolved before taking the lock — the guard must not cross an await.
        let mode = match self.night.current_mode().await {
            DayNight::Day => IrCutFilterMode::ON,
            DayNight::Night => IrCutFilterMode::OFF,
        };
        let mut settings = self.settings.write();
        settings.ir_led = on;
        settings.ir_cut_filter = mode;
        Ok(())
    }

    async fn set_white_light(&self, on: bool) -> PlatformResult<()> {
        use super::night_mode::Node;
        self.night
            .write_lamp(Node::WhiteLed, on)
            .await
            .map_err(|e| crate::platform::common::PlatformError::HardwareFailure(e.to_string()))
    }

    async fn enable_ir_auto(&self) -> PlatformResult<()> {
        self.night.set_auto_enabled(true);
        self.settings.write().ir_cut_filter = crate::onvif::types::common::IrCutFilterMode::AUTO;
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

    #[tokio::test]
    async fn test_get_options_reports_ir_unsupported_when_nodes_are_absent() {
        use crate::hal::common::imaging::MockImagingHalTrait;

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = crate::platform::anyka::night_mode::NodePaths::rooted(dir.path(), dir.path());
        let control = AnykaImagingControl::with_ffi_and_paths(
            Arc::new(MockImagingHalTrait::new()),
            paths,
            crate::config::types::ImagingConfig::default(),
            None,
        );

        let options = control.get_options().await.unwrap();

        assert!(!options.ir_cut_filter_supported);
        assert!(!options.ir_led_supported);
        assert!(!options.white_light_supported);
    }

    #[tokio::test]
    async fn test_get_options_reports_ir_supported_when_nodes_are_present() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::platform::anyka::night_mode::{Node, NodePaths};

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        for n in [Node::IrCutA, Node::IrCutB, Node::IrLed, Node::WhiteLed] {
            std::fs::write(paths.node(n), "0").unwrap();
        }

        let control = AnykaImagingControl::with_ffi_and_paths(
            Arc::new(MockImagingHalTrait::new()),
            paths,
            crate::config::types::ImagingConfig::default(),
            None,
        );

        let options = control.get_options().await.unwrap();

        assert!(options.ir_cut_filter_supported);
        assert!(options.ir_led_supported);
        assert!(options.white_light_supported);
    }

    #[tokio::test]
    async fn test_configured_ir_cut_filter_mode_is_reported_at_startup() {
        use crate::config::types::ImagingConfig;
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = crate::platform::anyka::night_mode::NodePaths::rooted(dir.path(), dir.path());
        let control = AnykaImagingControl::with_ffi_and_paths(
            Arc::new(MockImagingHalTrait::new()),
            paths,
            ImagingConfig {
                ir_cut_filter: IrCutFilterMode::OFF,
                ..ImagingConfig::default()
            },
            None,
        );

        let settings = control.get_settings().await.unwrap();

        assert_eq!(settings.ir_cut_filter, IrCutFilterMode::OFF);
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
