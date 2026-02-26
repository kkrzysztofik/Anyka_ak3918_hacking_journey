//! Anyka platform implementation.
//!
//! This module provides the actual platform implementation using
//! the Anyka SDK for the AK3918 camera hardware.
//!
//! This implementation is only compiled when cross-compiling for ARM
//! (i.e., when `use_stubs` is not defined).
//!
//! # Video Input Architecture
//!
//! The video input subsystem follows the project's dependency injection pattern:
//!
//! ```text
//! AnykaVideoInput
//!   ├── ffi: Arc<dyn VideoHalTrait>   (injected, mockable)
//!   ├── handle: RwLock<Option<Arc<VideoInputHandle>>>  (RAII, calls vi_close on Drop)
//!   └── opened: AtomicBool            (fast-path state check)
//! ```
//!
//! The `VideoInputHandle` implements `Drop` to automatically close the SDK device,
//! ensuring proper cleanup even in error paths. Dual-channel configuration
//! (Main: 1280x720, Sub: 640x360) is applied during platform initialization.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use parking_lot::RwLock;

use crate::hal::VideoDevice;
use crate::hal::video::{
    VideoHalTrait, VideoInputHandle, video_input_capture_off_internal,
    video_input_capture_on_internal, video_input_get_sensor_resolution_internal,
    video_input_match_sensor_internal, video_input_open_internal,
    video_input_set_channel_attr_internal,
};

use crate::hal::vendor_ipc::VendorIpc;

use crate::streaming::bridge::BytesMutPool;

use crate::hal::{video_channel_attr, video_resolution};

use super::traits::{
    AudioEncoder, AudioEncoderConfig, AudioInput, AudioSourceConfig, DeviceInfo, DnsInfo,
    ImagingControl, ImagingOptions, ImagingSettings, NetworkInfo, NetworkInterfaceInfo,
    NetworkProtocolInfo, NtpInfo, PTZControl, Platform, PlatformError, PlatformResult, Resolution,
    VideoEncoder, VideoEncoderConfig, VideoEncoderOptions, VideoEncoding, VideoInput,
    VideoSourceConfig,
};

/// Anyka platform implementation using the actual SDK.
///
/// This implementation wraps the Anyka SDK FFI calls and provides
/// a safe Rust interface to the hardware.
pub struct AnykaPlatform {
    initialized: AtomicBool,
    device_info: DeviceInfo,
    sensor_resolution: RwLock<Option<Resolution>>,
    video_input: Arc<AnykaVideoInput>,
    video_encoder: Arc<AnykaVideoEncoder>,
    audio_input: Arc<AnykaAudioInput>,
    audio_encoder: Arc<AnykaAudioEncoder>,
    ptz_control: Option<Arc<dyn PTZControl>>,
    imaging_control: Option<Arc<dyn ImagingControl>>,
    network_info: Option<Arc<dyn NetworkInfo>>,
}

impl AnykaPlatform {
    /// Create a new Anyka platform instance.
    ///
    /// Uses auto-detection for the ISP config path. See [`with_isp_config`](Self::with_isp_config)
    /// to specify an explicit path.
    pub fn new() -> PlatformResult<Self> {
        Self::with_isp_config(None)
    }

    /// Create a new Anyka platform instance with an optional ISP config path.
    ///
    /// If `isp_config_path` is `Some`, that path is used directly for
    /// `ak_vi_match_sensor()`. If `None`, the default search paths are used.
    pub fn with_isp_config(isp_config_path: Option<PathBuf>) -> PlatformResult<Self> {
        let device_info = DeviceInfo {
            manufacturer: "Anyka".to_string(),
            model: "AK3918".to_string(),
            firmware_version: "1.0.0".to_string(),
            serial_number: "AK3918-001".to_string(),
            hardware_id: "ak3918-hw".to_string(),
        };

        let (video_input, video_encoder, audio_input, audio_encoder, imaging_control) = {
            let shared_ipc = Arc::new(VendorIpc::new().map_err(|e| {
                PlatformError::InitializationFailed(format!(
                    "VendorIpc connection failed (is vendor-daemon running?): {}",
                    e
                ))
            })?);

            tracing::info!("AnykaPlatform: using shared VendorIpc client for video/audio/imaging");

            let video_ffi: Arc<dyn VideoHalTrait> = shared_ipc.clone();
            let audio_ffi: Arc<dyn crate::hal::audio::AudioHalTrait> = shared_ipc.clone();
            let imaging_ffi: Arc<dyn crate::hal::imaging::ImagingHalTrait> = shared_ipc.clone();

            let video_input = Arc::new(AnykaVideoInput::with_ffi(
                video_ffi.clone(),
                isp_config_path.clone(),
            ));
            let video_encoder = Arc::new(AnykaVideoEncoder::with_vendor_ipc(shared_ipc.clone()));
            let audio_input = Arc::new(AnykaAudioInput::with_ffi(audio_ffi.clone()));
            let audio_encoder = Arc::new(AnykaAudioEncoder::with_ffi(audio_ffi));
            let imaging_control = Some(Arc::new(AnykaImagingControl::with_ffi_and_video_encoder(
                imaging_ffi,
                Arc::clone(&video_encoder),
            )) as Arc<dyn ImagingControl>);

            (
                video_input,
                video_encoder,
                audio_input,
                audio_encoder,
                imaging_control,
            )
        };

        let ptz_control: Option<Arc<dyn PTZControl>> = {
            tracing::info!("Initializing PTZ (native Rust driver, /dev/ak-motor0, /dev/ak-motor1)");
            let ptz = AnykaPTZControl::new();
            match ptz.open() {
                Ok(()) => {
                    tracing::info!("PTZ device opened successfully");
                    Some(Arc::new(ptz) as Arc<dyn PTZControl>)
                }
                Err(e) => {
                    tracing::error!(
                        "PTZ device failed to open, PTZ features will be unavailable: {}",
                        e
                    );
                    None
                }
            }
        };
        let network_info = Some(Arc::new(AnykaNetworkInfo::new()) as Arc<dyn NetworkInfo>);

        Ok(Self {
            initialized: AtomicBool::new(false),
            device_info,
            sensor_resolution: RwLock::new(None),
            video_input,
            video_encoder,
            audio_input,
            audio_encoder,
            ptz_control,
            imaging_control,
            network_info,
        })
    }

    // Command server functions removed - vendor-daemon IPC handles all SDK access
    fn shutdown_video_pipeline(
        video_encoder: &AnykaVideoEncoder,
        video_input: &AnykaVideoInput,
    ) -> PlatformResult<()> {
        tracing::info!("Platform shutdown: PTZ stop complete, stopping streaming...");
        video_encoder.stop_streaming()?;
        tracing::info!("Platform shutdown: streaming stopped, closing encoders...");

        video_encoder.close_all_encoders()?;
        tracing::info!("Platform shutdown: encoders closed, stopping capture...");

        // Stop capture BEFORE closing video input.
        if let Err(e) = video_input.capture_off() {
            tracing::warn!(
                "Video capture off failed during shutdown (best-effort, continuing): {}",
                e
            );
        }
        tracing::info!("Platform shutdown: capture stopped, destroying VPSS...");

        // Destroy VPSS BEFORE closing video input (required by SDK).
        if let Err(e) = video_input.destroy_vpss() {
            tracing::warn!(
                "VPSS destroy failed during shutdown (best-effort, continuing): {}",
                e
            );
        }
        tracing::info!("Platform shutdown: VPSS destroyed, closing video input...");

        // Close video input (RAII handle will call ak_vi_close)
        if let Err(e) = video_input.close_blocking() {
            tracing::warn!(
                "Video input close failed during shutdown (best-effort, continuing): {}",
                e
            );
        }
        tracing::info!("Platform shutdown: video input closed");

        Ok(())
    }
}

// Default implementation removed - use AnykaPlatform::new() for fallible initialization.
// The Default trait should never panic per Rust best practices.

#[async_trait]
impl Platform for AnykaPlatform {
    async fn get_device_info(&self) -> PlatformResult<DeviceInfo> {
        // TODO(kkrzysztofik): Read actual device info from Anyka SDK
        Ok(self.device_info.clone())
    }

    fn video_input(&self) -> Arc<dyn VideoInput> {
        self.video_input.clone()
    }

    fn video_encoder(&self) -> Arc<dyn VideoEncoder> {
        self.video_encoder.clone()
    }

    fn audio_input(&self) -> Arc<dyn AudioInput> {
        self.audio_input.clone()
    }

    fn audio_encoder(&self) -> Arc<dyn AudioEncoder> {
        self.audio_encoder.clone()
    }

    fn ptz_control(&self) -> Option<Arc<dyn PTZControl>> {
        self.ptz_control.clone()
    }

    fn imaging_control(&self) -> Option<Arc<dyn ImagingControl>> {
        self.imaging_control.clone()
    }

    fn network_info(&self) -> Option<Arc<dyn NetworkInfo>> {
        self.network_info.clone()
    }

    fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    async fn initialize(&self) -> PlatformResult<()> {
        // Step 1: Load ISP sensor configuration (must precede vi_open)
        self.video_input.match_sensor()?;
        tracing::info!("ISP sensor matched successfully");

        // Step 2: Open video input device
        self.video_input.open().await?;

        // Step 2.5: Initialize VPSS. The Anyka ONVIF reference path performs
        // ak_vpss_init() early in VI bring-up.
        if let Err(e) = self.video_input.init_vpss() {
            tracing::warn!(
                "VPSS init failed during platform init; continuing without VPSS: {}",
                e
            );
        }

        // Step 3: Query and store sensor resolution
        let sensor_res = self.video_input.get_sensor_resolution()?;
        *self.sensor_resolution.write() = Some(sensor_res);
        tracing::info!(
            "Sensor resolution detected: {}x{}",
            sensor_res.width,
            sensor_res.height
        );

        // Step 4: Configure dual channels
        if let Err(e) = self.video_input.set_channel_attr() {
            tracing::error!("Failed to set channel attributes, rolling back: {}", e);
            let _ = self.video_input.destroy_vpss();
            let _ = self.video_input.close().await;
            return Err(e);
        }
        let (main_layout, sub_layout) = self.video_input.channel_layout();
        self.video_encoder
            .sync_configurations_to_channel_layout(main_layout, sub_layout);

        // Step 5: Start capture pipeline
        if let Err(e) = self.video_input.capture_on() {
            tracing::error!("Failed to start capture pipeline, rolling back: {}", e);
            let _ = self.video_input.capture_off();
            let _ = self.video_input.destroy_vpss();
            let _ = self.video_input.close().await;
            return Err(e);
        }
        // Allow the capture pipeline to stabilize before opening encoders.
        // The C reference (platform_anyka.c:609) uses PLATFORM_DELAY_MS_RETRY (200ms).
        std::thread::sleep(Duration::from_millis(200));
        tracing::info!("Video input initialized: dual-channel config and capture started");

        // Initialize dual video encoders (main 720p + sub 360p)
        let encoder_configs = self.video_encoder.get_configurations().await?;
        let mut initialized_encoder_tokens: Vec<String> = Vec::new();
        for config in &encoder_configs {
            if let Err(e) = self.video_encoder.init(config).await {
                tracing::error!("Failed to initialize video encoder {}: {}", config.token, e);

                for token in initialized_encoder_tokens.iter().rev() {
                    if let Err(close_error) = self.video_encoder.close_encoder(token) {
                        tracing::warn!(
                            "Failed to rollback initialized encoder {}: {}",
                            token,
                            close_error
                        );
                    }
                }

                // Rollback: stop capture, close video input
                let _ = self.video_input.capture_off();
                let _ = self.video_input.destroy_vpss();
                let _ = self.video_input.close().await;
                return Err(PlatformError::InitializationFailed(format!(
                    "Video encoder {} initialization failed: {}",
                    config.token, e
                )));
            }
            initialized_encoder_tokens.push(config.token.clone());
        }
        tracing::info!(
            "Video encoders initialized: {} channels",
            encoder_configs.len()
        );

        // Step 6: Start frame production: bind VI+encoder and spawn polling threads.
        let vi_handle = match self.video_input.get_handle() {
            Some(handle) => handle,
            None => {
                let _ = self.video_encoder.close_all_encoders();
                let _ = self.video_input.capture_off();
                let _ = self.video_input.destroy_vpss();
                let _ = self.video_input.close().await;
                return Err(PlatformError::InitializationFailed(
                    "Video input handle missing after successful open".to_string(),
                ));
            }
        };
        let main_enc_candidate = {
            let guard = self.video_encoder.main_handle.read();
            guard.clone()
        };
        let main_enc = match main_enc_candidate {
            Some(handle) => handle,
            None => {
                let _ = self.video_encoder.close_all_encoders();
                let _ = self.video_input.capture_off();
                let _ = self.video_input.destroy_vpss();
                let _ = self.video_input.close().await;
                return Err(PlatformError::InitializationFailed(
                    "Main encoder handle missing after successful init".to_string(),
                ));
            }
        };
        let sub_enc = self.video_encoder.sub_handle.read().clone();
        if let Err(e) = self
            .video_encoder
            .start_streaming(&vi_handle, &main_enc, sub_enc.as_ref())
        {
            tracing::error!("Failed to start streaming, rolling back: {}", e);
            let _ = self.video_encoder.close_all_encoders();
            let _ = self.video_input.capture_off();
            let _ = self.video_input.destroy_vpss();
            let _ = self.video_input.close().await;
            return Err(PlatformError::InitializationFailed(format!(
                "Video streaming startup failed: {}",
                e
            )));
        }

        // Step 7: Validate VI/VENC pipeline readiness.
        let readiness_timeout_ms = env_var_u64("ANYKA_PIPELINE_READY_TIMEOUT_MS").unwrap_or(5000);
        let require_sub_pipeline = env_var_truthy_or("ANYKA_PIPELINE_REQUIRE_SUB", true);
        if let Err(e) = self.video_encoder.wait_for_stream_readiness(
            Duration::from_millis(readiness_timeout_ms),
            require_sub_pipeline,
        ) {
            tracing::error!(
                "VI/VENC pipeline failed readiness validation, rolling back: {}",
                e
            );
            if let Err(rollback_error) =
                Self::shutdown_video_pipeline(&self.video_encoder, &self.video_input)
            {
                tracing::error!(
                    "VI/VENC readiness rollback failed (unsafe): readiness_error='{}', rollback_error='{}'",
                    e,
                    rollback_error
                );
                return Err(rollback_error);
            }
            return Err(e);
        }

        // TODO(kkrzysztofik): Call remaining Anyka SDK initialization functions via FFI
        // - ak_ai_open()
        // - ak_aenc_open()
        // PTZ is already opened in AnykaPlatform::new()
        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn shutdown(&self) -> PlatformResult<()> {
        tracing::info!("Platform shutdown: starting PTZ stop...");
        // Best-effort PTZ stop — the PTZHandle Drop will call ptz_close.
        // We log errors but do not abort shutdown for a single subsystem failure.
        if let Some(ref ptz) = self.ptz_control
            && let Err(e) = ptz.stop().await
        {
            tracing::warn!(
                "PTZ stop failed during shutdown (best-effort, continuing): {}",
                e
            );
        }

        // Run blocking SDK teardown in a dedicated OS thread with a hard deadline.
        // This avoids async cancellation races around blocking vendor calls.
        let video_encoder = Arc::clone(&self.video_encoder);
        let video_input = Arc::clone(&self.video_input);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("anyka-shutdown-worker".to_string())
            .spawn(move || {
                let result = AnykaPlatform::shutdown_video_pipeline(&video_encoder, &video_input);
                let _ = tx.send(result);
            })
            .map_err(|e| {
                PlatformError::HardwareFailure(format!(
                    "failed to spawn anyka shutdown worker thread: {}",
                    e
                ))
            })?;

        #[cfg(test)]
        let shutdown_deadline = Duration::from_millis(200);
        #[cfg(not(test))]
        let shutdown_deadline = Duration::from_secs(12);

        let result = match rx.recv_timeout(shutdown_deadline) {
            Ok(result) => result,
            Err(_) => {
                self.video_encoder
                    .mark_unsafe_shutdown("platform shutdown worker exceeded hard deadline");
                Err(PlatformError::Timeout)
            }
        };

        // TODO(kkrzysztofik): Call remaining Anyka SDK cleanup functions via FFI
        // - ak_ai_close()
        // - ak_aenc_close()
        if result.is_ok() {
            self.initialized.store(false, Ordering::SeqCst);
            tracing::info!("Platform shutdown: complete");
        } else {
            tracing::error!("Platform shutdown ended with error: {:?}", result);
        }
        result
    }

    fn max_sensor_resolution(&self) -> PlatformResult<Resolution> {
        self.sensor_resolution.read().ok_or_else(|| {
            PlatformError::InitializationFailed(
                "Sensor resolution not available - platform not initialized".to_string(),
            )
        })
    }

    fn register_frame_callback(
        &self,
        callback: Arc<dyn crate::platform::frame::FrameCallback>,
    ) -> PlatformResult<()> {
        let _id = self.video_encoder.register_frame_callback(callback);
        tracing::info!("Frame callback registered (id={})", _id);
        Ok(())
    }

    fn register_owned_frame_callback(
        &self,
        callback: Arc<dyn crate::platform::frame::OwnedFrameCallback>,
    ) -> PlatformResult<()> {
        let _id = self.video_encoder.register_owned_frame_callback(callback);
        tracing::info!("Owned frame callback registered (id={})", _id);
        Ok(())
    }
}

// =============================================================================
// Video Input Implementation
// =============================================================================

/// Anyka video input implementation backed by the Anyka SDK FFI layer.
///
/// Uses dependency injection via `Arc<dyn VideoHalTrait>` to enable mock-based
/// testing without hardware. The `VideoInputHandle` provides RAII cleanup —
/// dropping the handle automatically calls `ak_vi_close()`.
///
/// # State Management
///
/// - `opened: AtomicBool` — fast-path check without lock contention
/// - `handle: RwLock<Option<Arc<VideoInputHandle>>>` — thread-safe handle access
///   with take-on-close semantics
///
/// # Thread Safety
///
/// All fields are `Send + Sync`. The `AtomicBool` provides lock-free reads for
/// the common `is_opened` check, while the `RwLock` protects handle mutations.
/// Default search paths for the ISP sensor configuration file.
/// Align a width value up to the 32-pixel boundary required by the video encoder.
/// Reference: `VENCODER_WIDTH_ALIGN_REQ` in `ak_vi.c`.
fn align_to_32(w: i32) -> i32 {
    (w + 31) & !31
}

/// Align a height value up to the 8-pixel boundary required by the video encoder.
/// Reference: `VENCODER_HEIGHT_ALIGN_REQ` in `ak_vi.c`.
fn align_to_8(h: i32) -> i32 {
    (h + 7) & !7
}

fn env_var_truthy(name: &str) -> bool {
    env_var_truthy_or(name, false)
}

fn env_var_truthy_or(name: &str, default: bool) -> bool {
    std::env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn env_var_u64(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
}

///
/// These paths are searched in order when no explicit ISP config path is provided.
/// The first path that exists on the filesystem is used for `ak_vi_match_sensor()`.
const ISP_CONFIG_SEARCH_PATHS: &[&str] = &[
    "/mnt/anyka_hack/onvif/isp_gc1084.conf",
    "/etc/jffs2/isp_gc1084.conf",
    "/usr/local/isp_gc1084.conf",
];

struct AnykaVideoInput {
    ffi: Arc<dyn VideoHalTrait>,
    handle: RwLock<Option<Arc<VideoInputHandle>>>,
    opened: AtomicBool,
    capture_started: AtomicBool,
    isp_config_path: Option<PathBuf>,
    vpss_initialized: AtomicBool,
    channel_layout: RwLock<(Resolution, Resolution)>,
}

impl AnykaVideoInput {
    /// Create a new `AnykaVideoInput` with the default (real) FFI backend.
    fn new(isp_config_path: Option<PathBuf>) -> PlatformResult<Self> {
        let ipc = VendorIpc::new().map_err(|e| {
            PlatformError::InitializationFailed(format!(
                "VendorIpc connection failed (is vendor-daemon running?): {}",
                e
            ))
        })?;
        tracing::info!("AnykaVideoInput: using VendorIpc for vendor library access");
        Ok(Self::with_ffi(Arc::new(ipc), isp_config_path))
    }

    /// Create a new `AnykaVideoInput` with a custom FFI backend.
    ///
    /// Used by tests with `MockVideoHalTrait` for hardware-free testing.
    fn with_ffi(ffi: Arc<dyn VideoHalTrait>, isp_config_path: Option<PathBuf>) -> Self {
        Self {
            ffi,
            handle: RwLock::new(None),
            opened: AtomicBool::new(false),
            capture_started: AtomicBool::new(false),
            isp_config_path,
            vpss_initialized: AtomicBool::new(false),
            channel_layout: RwLock::new((Resolution::new(1280, 720), Resolution::new(640, 360))),
        }
    }

    /// Get a clone of the video input handle (if opened).
    pub fn get_handle(&self) -> Option<Arc<crate::hal::VideoInputHandle>> {
        self.handle.read().clone()
    }

    fn channel_layout(&self) -> (Resolution, Resolution) {
        *self.channel_layout.read()
    }

    /// Configure dual-channel video attributes.
    ///
    /// Uses the Anyka-compatible startup strategy:
    /// main = sensor-native, sub = 640x360 (or aligned fallback for tiny sensors).
    ///
    /// # Errors
    ///
    /// Returns `PlatformError::HardwareUnavailable` if the device is not opened.
    /// Returns `PlatformError::HardwareFailure` if the SDK call fails.
    fn set_channel_attr(&self) -> PlatformResult<()> {
        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input not opened".to_string())
        })?;

        // Query sensor resolution to properly set crop area
        let sensor_res = video_input_get_sensor_resolution_internal(handle, self.ffi.as_ref())?;
        tracing::debug!(
            "Sensor resolution: {}x{}",
            sensor_res.width,
            sensor_res.height
        );

        let sensor_width = sensor_res.width as i32;
        let sensor_height = sensor_res.height as i32;

        let mut attr = video_channel_attr::default();
        attr.crop.left = 0;
        attr.crop.top = 0;
        attr.crop.width = sensor_width;
        attr.crop.height = sensor_height;

        // Keep sub at 640x360 by default (libre_anyka_app-compatible).
        let sub_width = if sensor_width >= 640 {
            640
        } else {
            align_to_32(sensor_width.max(32))
        };
        let sub_height = if sensor_height >= 360 {
            360
        } else {
            align_to_8(sensor_height.max(8))
        };

        // libre_anyka_app quirk: in vendor IPC mode, main.max_* drives sub-channel
        // validation/limits. Mirror the proven C workaround (invert max mapping).
        let (main_max_width, main_max_height, sub_max_width, sub_max_height, max_mode) = (
            sub_width,
            sub_height,
            sensor_width,
            sensor_height,
            "vendor-ipc-legacy-mapping",
        );

        attr.res = [
            // Main channel: sensor-native resolution.
            video_resolution {
                width: sensor_width,
                height: sensor_height,
                max_width: main_max_width,
                max_height: main_max_height,
            },
            // Sub channel: smaller resolution required by ISP driver.
            video_resolution {
                width: sub_width,
                height: sub_height,
                max_width: sub_max_width,
                max_height: sub_max_height,
            },
        ];

        tracing::info!(
            "Applying Anyka VI attrs (quirk-default): crop={}x{}, main={}x{}, sub={}x{}",
            attr.crop.width,
            attr.crop.height,
            attr.res[0].width,
            attr.res[0].height,
            attr.res[1].width,
            attr.res[1].height
        );
        tracing::info!(
            "Applying Anyka VI max attrs ({}): main_max={}x{}, sub_max={}x{}",
            max_mode,
            attr.res[0].max_width,
            attr.res[0].max_height,
            attr.res[1].max_width,
            attr.res[1].max_height,
        );
        video_input_set_channel_attr_internal(handle, &attr, self.ffi.as_ref())?;
        *self.channel_layout.write() = (
            Resolution::new(attr.res[0].width as u32, attr.res[0].height as u32),
            Resolution::new(attr.res[1].width as u32, attr.res[1].height as u32),
        );
        Ok(())
    }

    /// Load the ISP sensor configuration via `ak_vi_match_sensor()`.
    ///
    /// This **must** be called before `open()` so the ISP subsystem has a valid
    /// config buffer when `ak_vi_open()` internally calls `isp_init()`.
    ///
    /// If an explicit `isp_config_path` was provided at construction, that path is
    /// tried first. Otherwise, the default search paths in [`ISP_CONFIG_SEARCH_PATHS`]
    /// are checked in order.
    ///
    /// # Errors
    ///
    /// * `PlatformError::HardwareUnavailable` if no ISP config file is found
    /// * `PlatformError::HardwareFailure` if `ak_vi_match_sensor()` rejects the config
    fn match_sensor(&self) -> PlatformResult<()> {
        // If an explicit path was provided, try it directly
        if let Some(ref path) = self.isp_config_path {
            if path.exists() {
                tracing::info!(
                    "Loading ISP sensor config from explicit path: {}",
                    path.display()
                );
                return video_input_match_sensor_internal(path, self.ffi.as_ref());
            }
            tracing::warn!(
                "Explicit ISP config path does not exist: {}, falling back to search paths",
                path.display()
            );
        }

        // Search default paths
        for &search_path in ISP_CONFIG_SEARCH_PATHS {
            let path = std::path::Path::new(search_path);
            if path.exists() {
                tracing::info!("Loading ISP sensor config from: {}", search_path);
                return video_input_match_sensor_internal(path, self.ffi.as_ref());
            }
            tracing::debug!("ISP config not found at: {}", search_path);
        }

        Err(PlatformError::HardwareUnavailable(
            "No ISP sensor config file found in any search path".to_string(),
        ))
    }

    /// Initialize the Video Post-Processing SubSystem (VPSS).
    ///
    /// This MUST be called immediately after `open()` and before any other
    /// video operations. This is a critical initialization step from the
    /// reference implementation that sets up the ISP processing pipeline.
    ///
    /// # Errors
    ///
    /// Returns `PlatformError::HardwareUnavailable` if the device is not opened.
    fn init_vpss(&self) -> PlatformResult<()> {
        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input not opened".to_string())
        })?;

        crate::hal::video::vpss_init_internal(handle, VideoDevice::DEV0, self.ffi.as_ref())?;
        self.vpss_initialized.store(true, Ordering::SeqCst);
        tracing::info!("VPSS initialized successfully");
        Ok(())
    }

    /// Destroy the Video Post-Processing SubSystem (VPSS).
    ///
    /// This MUST be called BEFORE closing the video input device during cleanup.
    /// The reference implementation shows this must be done in the correct order
    /// to avoid resource leaks.
    fn destroy_vpss(&self) -> PlatformResult<()> {
        if self
            .vpss_initialized
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            crate::hal::video::vpss_destroy_internal(VideoDevice::DEV0, self.ffi.as_ref())?;
            tracing::info!("VPSS destroyed successfully");
        }
        Ok(())
    }

    /// Get the sensor resolution via `ak_vi_get_sensor_resolution()`.
    ///
    /// Returns the native resolution of the video sensor.
    /// This is used to constrain profile configurations and validate ONVIF requests.
    ///
    /// # Errors
    ///
    /// * `PlatformError::HardwareUnavailable` if the device is not opened
    /// * `PlatformError::HardwareFailure` if the SDK call fails
    fn get_sensor_resolution(&self) -> PlatformResult<Resolution> {
        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input not opened".to_string())
        })?;

        let sensor_res = video_input_get_sensor_resolution_internal(handle, self.ffi.as_ref())?;
        Ok(Resolution {
            width: sensor_res.width as u32,
            height: sensor_res.height as u32,
        })
    }

    /// Start the ISP capture pipeline via `ak_vi_capture_on()`.
    ///
    /// This should be called after `set_channel_attr()` to activate the capture
    /// pipeline. Without this call, the video input device is configured but not
    /// actually capturing frames.
    ///
    /// Uses retry mechanism with delays to allow ISP system to fully initialize,
    /// especially important for v3 ISP config files which may need extra time.
    ///
    /// # Errors
    ///
    /// * `PlatformError::HardwareUnavailable` if the device is not opened
    /// * `PlatformError::HardwareFailure` if the SDK call fails after all retries
    fn capture_on(&self) -> PlatformResult<()> {
        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input not opened".to_string())
        })?;

        // Add delay to allow VI system to fully initialize (especially for v3 configs)
        const VI_INIT_DELAY_MS: u64 = 500;
        const RETRY_DELAY_MS: u64 = 300;
        const RETRY_RESET_DELAY_MS: u64 = 120;
        const MAX_RETRIES: u32 = 3;

        std::thread::sleep(std::time::Duration::from_millis(VI_INIT_DELAY_MS));
        let capture_start = std::time::Instant::now();

        // Try to start video capture with retry mechanism
        for attempt in 1..=MAX_RETRIES {
            let attempt_start = std::time::Instant::now();
            match video_input_capture_on_internal(handle, self.ffi.as_ref()) {
                Ok(()) => {
                    self.capture_started.store(true, Ordering::SeqCst);
                    tracing::info!(
                        "Video capture started successfully on attempt {} (attempt_elapsed_ms={}, total_elapsed_ms={})",
                        attempt,
                        attempt_start.elapsed().as_millis(),
                        capture_start.elapsed().as_millis()
                    );
                    return Ok(());
                }
                Err(e) if attempt < MAX_RETRIES => {
                    tracing::warn!(
                        "ak_vi_capture_on attempt {} failed after {}ms (total={}ms): {}; running capture_off cleanup before retry",
                        attempt,
                        attempt_start.elapsed().as_millis(),
                        capture_start.elapsed().as_millis(),
                        e
                    );

                    if let Err(cleanup_error) =
                        video_input_capture_off_internal(handle, self.ffi.as_ref())
                    {
                        tracing::error!(
                            "ak_vi_capture_off cleanup after attempt {} failed, aborting retries: {}",
                            attempt,
                            cleanup_error
                        );
                        return Err(PlatformError::HardwareFailure(format!(
                            "ak_vi_capture_on retry cleanup failed on attempt {}: {}",
                            attempt, cleanup_error
                        )));
                    }
                    self.capture_started.store(false, Ordering::SeqCst);

                    std::thread::sleep(std::time::Duration::from_millis(RETRY_RESET_DELAY_MS));
                    std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
                }
                Err(e) => {
                    tracing::error!(
                        "ak_vi_capture_on failed after {} attempts (total_elapsed_ms={}): {}",
                        MAX_RETRIES,
                        capture_start.elapsed().as_millis(),
                        e
                    );
                    return Err(e);
                }
            }
        }

        Err(PlatformError::HardwareFailure(
            "ak_vi_capture_on retry loop exited unexpectedly".to_string(),
        ))
    }

    fn capture_off(&self) -> PlatformResult<()> {
        if !self.capture_started.load(Ordering::SeqCst) {
            return Ok(());
        }

        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input not opened".to_string())
        })?;

        video_input_capture_off_internal(handle, self.ffi.as_ref())?;
        self.capture_started.store(false, Ordering::SeqCst);
        tracing::info!("Video capture stopped successfully");
        Ok(())
    }

    fn close_blocking(&self) -> PlatformResult<()> {
        // Idempotent: already closed is not an error
        if !self.opened.load(Ordering::SeqCst) {
            return Ok(());
        }

        if let Err(e) = self.capture_off() {
            tracing::warn!(
                "Video capture off failed during close (best-effort, continuing): {}",
                e
            );
        }

        let mut guard = self.handle.write();
        let _old_handle = guard.take(); // Drop triggers RAII cleanup
        self.opened.store(false, Ordering::SeqCst);
        self.capture_started.store(false, Ordering::SeqCst);

        tracing::info!("Video input device closed");
        Ok(())
    }
}

#[async_trait]
impl VideoInput for AnykaVideoInput {
    /// Open the video input device via the Anyka SDK.
    ///
    /// Calls `ak_vi_open(VIDEO_DEV0)` through the FFI layer and stores the
    /// resulting handle for subsequent operations.
    ///
    /// # Errors
    ///
    /// - `PlatformError::ResourceBusy` if the device is already opened.
    /// - `PlatformError::HardwareUnavailable` if the SDK returns a null handle.
    async fn open(&self) -> PlatformResult<()> {
        if self
            .opened
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(PlatformError::ResourceBusy(
                "Video input already opened".to_string(),
            ));
        }

        let vi_handle = match video_input_open_internal(VideoDevice::DEV0, self.ffi.as_ref()) {
            Ok(handle) => handle,
            Err(error) => {
                self.opened.store(false, Ordering::SeqCst);
                return Err(error);
            }
        };

        let mut guard = self.handle.write();
        *guard = Some(Arc::new(vi_handle));

        tracing::info!("Video input device opened successfully");
        Ok(())
    }

    /// Close the video input device.
    ///
    /// Takes the handle from the `RwLock`, dropping it to trigger RAII cleanup
    /// (`ak_vi_close()` via the `Drop` implementation on `VideoInputHandle`).
    ///
    /// This operation is idempotent — calling close on an already-closed device
    /// returns `Ok(())`.
    async fn close(&self) -> PlatformResult<()> {
        self.close_blocking()
    }

    /// Get the native sensor resolution from the hardware.
    ///
    /// Queries the Anyka SDK via `ak_vi_get_sensor_resolution()` and converts
    /// the FFI `Resolution` type to the platform `Resolution` type.
    ///
    /// # Errors
    ///
    /// - `PlatformError::HardwareUnavailable` if the device is not opened.
    /// - `PlatformError::HardwareFailure` if the SDK call fails.
    async fn get_resolution(&self) -> PlatformResult<Resolution> {
        if !self.opened.load(Ordering::SeqCst) {
            return Err(PlatformError::HardwareUnavailable(
                "Video input not opened".to_string(),
            ));
        }

        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input handle not available".to_string())
        })?;

        let ffi_res = video_input_get_sensor_resolution_internal(handle, self.ffi.as_ref())?;

        // Convert FFI Resolution (crate::hal::Resolution) to platform Resolution
        Ok(Resolution::new(ffi_res.width, ffi_res.height))
    }

    /// Get the video source configurations.
    ///
    /// Returns a single video source for the AK3918's camera sensor.
    /// The resolution is queried from hardware if the device is opened,
    /// otherwise defaults to 1920x1080.
    async fn get_sources(&self) -> PlatformResult<Vec<VideoSourceConfig>> {
        let resolution = if self.opened.load(Ordering::SeqCst) {
            self.get_resolution()
                .await
                .unwrap_or(Resolution::new(1920, 1080))
        } else {
            Resolution::new(1920, 1080)
        };

        Ok(vec![VideoSourceConfig {
            token: "VideoSource_1".to_string(),
            name: "Main Camera".to_string(),
            resolution,
            max_framerate: 30.0,
        }])
    }
}

// =============================================================================
// Video Encoder Implementation
// =============================================================================

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::time::Instant;

use portable_atomic::AtomicU64;

use crate::hal::VideoFrameType;
use crate::hal::video::{
    VideoEncoderHandle, VideoStreamHandle, video_encoder_open_internal,
    video_encoder_request_idr_internal, video_encoder_set_rc_internal,
};
#[cfg(test)]
use crate::hal::video_stream;
use crate::hal::{
    bitrate_ctrl_mode, encode_group_type, encode_output_type, encode_param, encode_use_chn,
    profile_mode,
};

use super::frame::{
    ActiveFrames, CallbackId, Frame, FrameCallback, FrameType, OwnedFrame, OwnedFrameCallback,
    StreamId,
};

/// Encoder lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EncoderState {
    /// Encoder created but not yet initialized.
    Uninitialized,
    /// Encoder initialized and ready to produce frames.
    Initialized,
}

/// Convert an SDK `VideoFrameType` to our `FrameType`.
fn sdk_frame_type_to_frame_type(ft: VideoFrameType) -> FrameType {
    match ft {
        VideoFrameType::FrameTypeI => FrameType::VideoIFrame,
        VideoFrameType::FrameTypePi => FrameType::VideoPiFrame,
        VideoFrameType::FrameTypeP => FrameType::VideoPFrame,
        VideoFrameType::FrameTypeB => FrameType::VideoBFrame,
    }
}

const SDK_ERROR_NO_DATA: i32 = 23;
#[cfg(test)]
const NO_DATA_IDR_RECOVERY_EVERY_ERRORS: u32 = 3;
#[cfg(not(test))]
const NO_DATA_IDR_RECOVERY_EVERY_ERRORS: u32 = 100;
const PIPELINE_READINESS_POLL_MS: u64 = 25;
const CALLBACK_HISTOGRAM_LOG_INTERVAL: u64 = 1000;
const CALLBACK_BUCKET_LIMITS_US: [u64; 6] = [250, 500, 1000, 2000, 5000, u64::MAX];
const CALLBACK_SLOW_WARN_THRESHOLD_US: u64 = 5000;
const CALLBACK_SLOW_LOG_INTERVAL: u64 = 50;

static CALLBACK_DURATION_TOTAL: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_MAX_US: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_BUCKET_0: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_BUCKET_1: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_BUCKET_2: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_BUCKET_3: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_BUCKET_4: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_BUCKET_5: AtomicU64 = AtomicU64::new(0);
static CALLBACK_SLOW_TOTAL: AtomicU64 = AtomicU64::new(0);
static LAST_IMAGING_UPDATE_SEQ: AtomicU64 = AtomicU64::new(0);
static LAST_IMAGING_UPDATE_UNIX_MS: AtomicU64 = AtomicU64::new(0);

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn callback_bucket_counter(index: usize) -> &'static AtomicU64 {
    match index {
        0 => &CALLBACK_DURATION_BUCKET_0,
        1 => &CALLBACK_DURATION_BUCKET_1,
        2 => &CALLBACK_DURATION_BUCKET_2,
        3 => &CALLBACK_DURATION_BUCKET_3,
        4 => &CALLBACK_DURATION_BUCKET_4,
        _ => &CALLBACK_DURATION_BUCKET_5,
    }
}

fn record_callback_duration(elapsed_us: u64) {
    CALLBACK_DURATION_TOTAL.fetch_add(1, Ordering::Relaxed);
    CALLBACK_DURATION_MAX_US.fetch_max(elapsed_us, Ordering::Relaxed);

    for (index, limit) in CALLBACK_BUCKET_LIMITS_US.iter().enumerate() {
        if elapsed_us <= *limit {
            callback_bucket_counter(index).fetch_add(1, Ordering::Relaxed);
            break;
        }
    }
}

fn histogram_percentile_bucket_us(percentile: f64) -> u64 {
    let total = CALLBACK_DURATION_TOTAL.load(Ordering::Relaxed);
    if total == 0 {
        return 0;
    }

    let threshold = (total as f64 * percentile).ceil() as u64;
    let mut cumulative = 0u64;
    for (index, limit) in CALLBACK_BUCKET_LIMITS_US.iter().enumerate() {
        cumulative += callback_bucket_counter(index).load(Ordering::Relaxed);
        if cumulative >= threshold {
            return *limit;
        }
    }

    CALLBACK_BUCKET_LIMITS_US[CALLBACK_BUCKET_LIMITS_US.len() - 1]
}

fn maybe_log_callback_histogram() {
    let total = CALLBACK_DURATION_TOTAL.load(Ordering::Relaxed);
    if total == 0 || !total.is_multiple_of(CALLBACK_HISTOGRAM_LOG_INTERVAL) {
        return;
    }

    tracing::debug!(
        callback_samples = total,
        callback_slow_over_5ms = CALLBACK_SLOW_TOTAL.load(Ordering::Relaxed),
        callback_p50_us = histogram_percentile_bucket_us(0.50),
        callback_p95_us = histogram_percentile_bucket_us(0.95),
        callback_p99_us = histogram_percentile_bucket_us(0.99),
        callback_max_us = CALLBACK_DURATION_MAX_US.load(Ordering::Relaxed),
        callback_bucket_le_250us = CALLBACK_DURATION_BUCKET_0.load(Ordering::Relaxed),
        callback_bucket_le_500us = CALLBACK_DURATION_BUCKET_1.load(Ordering::Relaxed),
        callback_bucket_le_1ms = CALLBACK_DURATION_BUCKET_2.load(Ordering::Relaxed),
        callback_bucket_le_2ms = CALLBACK_DURATION_BUCKET_3.load(Ordering::Relaxed),
        callback_bucket_le_5ms = CALLBACK_DURATION_BUCKET_4.load(Ordering::Relaxed),
        callback_bucket_gt_5ms = CALLBACK_DURATION_BUCKET_5.load(Ordering::Relaxed),
        "Frame callback duration histogram"
    );
}

fn maybe_log_slow_callback(callback_id: u64, elapsed_us: u64, callback_kind: &'static str) {
    let slow_total = CALLBACK_SLOW_TOTAL.fetch_add(1, Ordering::Relaxed) + 1;
    if slow_total == 1 || slow_total.is_multiple_of(CALLBACK_SLOW_LOG_INTERVAL) {
        tracing::warn!(
            callback_id,
            elapsed_us,
            slow_count = slow_total,
            threshold_us = CALLBACK_SLOW_WARN_THRESHOLD_US,
            callback_kind,
            "Frame callback exceeded latency threshold"
        );
    }
}

fn maybe_log_slow_owned_callback(callback_id: u64, elapsed_us: u64) {
    maybe_log_slow_callback(callback_id, elapsed_us, "owned");
}

#[cfg(test)]
fn compute_no_data_recovery_interval_errors(trigger_ms: u64, cycle_sleep_ms: u64) -> u32 {
    let trigger_ms = trigger_ms.max(1);
    let cycle_sleep_ms = cycle_sleep_ms.max(1);
    let errors = trigger_ms.div_ceil(cycle_sleep_ms);
    errors.max(1).min(u64::from(u32::MAX)) as u32
}

#[inline]
fn push_mode_enabled(has_sub_stream: bool) -> bool {
    !has_sub_stream
}

#[inline]
fn is_push_mode_transient_error(error: &PlatformError) -> bool {
    matches!(
        error,
        PlatformError::Timeout | PlatformError::ResourceBusy(_)
    )
}

#[derive(Default)]
struct StreamHealthCounters {
    main_frames: AtomicU64,
    sub_frames: AtomicU64,
    main_no_data_errors: AtomicU64,
    sub_no_data_errors: AtomicU64,
}

#[derive(Debug, Clone, Copy)]
struct StreamHealthSnapshot {
    main_frames: u64,
    sub_frames: u64,
    main_no_data_errors: u64,
    sub_no_data_errors: u64,
}

impl StreamHealthCounters {
    fn reset(&self) {
        self.main_frames.store(0, Ordering::SeqCst);
        self.sub_frames.store(0, Ordering::SeqCst);
        self.main_no_data_errors.store(0, Ordering::SeqCst);
        self.sub_no_data_errors.store(0, Ordering::SeqCst);
    }

    fn record_frame(&self, stream_id: StreamId) {
        match stream_id {
            StreamId::VideoMain => {
                self.main_frames.fetch_add(1, Ordering::SeqCst);
            }
            StreamId::VideoSub => {
                self.sub_frames.fetch_add(1, Ordering::SeqCst);
            }
            StreamId::Audio => {}
        }
    }

    fn record_no_data_error(&self, stream_id: StreamId) {
        match stream_id {
            StreamId::VideoMain => {
                self.main_no_data_errors.fetch_add(1, Ordering::SeqCst);
            }
            StreamId::VideoSub => {
                self.sub_no_data_errors.fetch_add(1, Ordering::SeqCst);
            }
            StreamId::Audio => {}
        }
    }

    fn snapshot(&self) -> StreamHealthSnapshot {
        StreamHealthSnapshot {
            main_frames: self.main_frames.load(Ordering::SeqCst),
            sub_frames: self.sub_frames.load(Ordering::SeqCst),
            main_no_data_errors: self.main_no_data_errors.load(Ordering::SeqCst),
            sub_no_data_errors: self.sub_no_data_errors.load(Ordering::SeqCst),
        }
    }
}

/// Unified frame reader loop for video callbacks.
///
/// Production mode is push-only: the loop blocks on `VendorIpc::recv_pushed_frame()`,
/// routes frames by stream id, and invokes callbacks.
///
/// In unit tests (when `VendorIpc` is unavailable), a test-only fallback polls
/// `venc_get_stream()` to preserve existing mock-based coverage.
///
/// # Unified reader thread
///
/// This function drains frames from **both** main and sub streams in a single
/// thread, alternating between them each cycle.  This eliminates the IPC mutex
/// contention that occurred when two independent threads (`venc-main-read` and
/// `venc-sub-read`) competed for the same `Mutex<UnixStream>` to the vendor
/// daemon.  The vendor daemon is single-threaded, so serialising requests from
/// one thread matches its dispatch model perfectly.
///
/// Each stream has independent per-stream counters (frame count, no-data
/// streak, adaptive sleep state) so IDR recovery and health tracking remain
/// per-channel.
#[allow(clippy::too_many_arguments)]
fn unified_frame_read_loop(
    main_stream_handle: Arc<VideoStreamHandle>,
    sub_stream_handle: Option<Arc<VideoStreamHandle>>,
    _ffi: Arc<dyn crate::hal::video::VideoHalTrait>,
    callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>>,
    owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>>,
    stop_signal: Arc<AtomicBool>,
    stream_health: Arc<StreamHealthCounters>,
    main_enc_addr: Option<usize>,
    sub_enc_addr: Option<usize>,
    vendor_ipc: Option<Arc<VendorIpc>>,
    frame_pool: Option<Arc<BytesMutPool>>,
) {
    #[cfg(test)]
    use crate::hal::AK_SUCCESS_I32;

    /// Per-stream state for the unified reader loop.
    struct StreamState {
        stream_id: StreamId,
        consecutive_no_data: u32,
        frame_count: u64,
        total_bytes: u64,
        iframe_count: u64,
        error_count: u64,
        last_error_was_no_data: bool,
        recovery_encoder_handle_addr: Option<usize>,
        last_imaging_seq_frame_logged: u64,
        last_imaging_seq_iframe_logged: u64,
    }

    impl StreamState {
        fn new(stream_id: StreamId, recovery_encoder_handle_addr: Option<usize>) -> Self {
            Self {
                stream_id,
                consecutive_no_data: 0,
                frame_count: 0,
                total_bytes: 0,
                iframe_count: 0,
                error_count: 0,
                last_error_was_no_data: true,
                recovery_encoder_handle_addr,
                last_imaging_seq_frame_logged: 0,
                last_imaging_seq_iframe_logged: 0,
            }
        }
    }

    /// Drain all available frames from a single stream handle.
    ///
    /// Returns the number of frames drained this cycle.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn drain_stream(
        handle: &VideoStreamHandle,
        ffi: &dyn crate::hal::video::VideoHalTrait,
        state: &mut StreamState,
        callbacks: &RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>,
        stop_signal: &AtomicBool,
        stream_health: &StreamHealthCounters,
        no_data_recovery_interval_errors: u32,
    ) -> u32 {
        let mut frames_this_cycle: u32 = 0;

        loop {
            if stop_signal.load(Ordering::SeqCst) {
                break;
            }

            // ── Owned (zero-copy) path: fetch directly into BytesMut ──
            if let Some(ipc) = vendor_ipc {
                match ipc.fetch_frame_owned(handle.as_ptr(), state.stream_id, frame_pool) {
                    Ok(owned_frame) => {
                        // Reset no-data counter on successful frame retrieval
                        state.consecutive_no_data = 0;
                        state.last_error_was_no_data = true;

                        let frame_type = owned_frame.frame_type;
                        let frame_size = owned_frame.data.len();

                        tracing::trace!(
                            stream = ?state.stream_id,
                            size = frame_size,
                            timestamp_us = owned_frame.timestamp,
                            frame_type = ?frame_type,
                            "Frame retrieved via owned path (zero-copy)"
                        );

                        state.frame_count += 1;
                        frames_this_cycle += 1;
                        stream_health.record_frame(state.stream_id);
                        state.total_bytes += frame_size as u64;
                        if matches!(frame_type, FrameType::VideoIFrame) {
                            state.iframe_count += 1;
                        }

                        // Imaging update tracking
                        let latest_imaging_seq = LAST_IMAGING_UPDATE_SEQ.load(Ordering::Relaxed);
                        if latest_imaging_seq > state.last_imaging_seq_frame_logged {
                            let applied_ms = LAST_IMAGING_UPDATE_UNIX_MS.load(Ordering::Relaxed);
                            let latency_ms = current_unix_ms().saturating_sub(applied_ms);
                            tracing::info!(
                                stream = ?state.stream_id,
                                imaging_seq = latest_imaging_seq,
                                latency_ms,
                                frame_type = ?frame_type,
                                "First encoded frame observed after imaging update"
                            );
                            state.last_imaging_seq_frame_logged = latest_imaging_seq;
                        }
                        if matches!(frame_type, FrameType::VideoIFrame)
                            && latest_imaging_seq > state.last_imaging_seq_iframe_logged
                        {
                            let applied_ms = LAST_IMAGING_UPDATE_UNIX_MS.load(Ordering::Relaxed);
                            let latency_ms = current_unix_ms().saturating_sub(applied_ms);
                            tracing::info!(
                                stream = ?state.stream_id,
                                imaging_seq = latest_imaging_seq,
                                latency_ms,
                                "First IDR observed after imaging update"
                            );
                            state.last_imaging_seq_iframe_logged = latest_imaging_seq;
                        }

                        // Periodic summary every 300 frames
                        if state.frame_count.is_multiple_of(300) {
                            let (overflow, eviction, fallback, dropped) =
                                if let Some(ipc_ref) = vendor_ipc {
                                    ipc_ref.shm_diagnostic_counters()
                                } else {
                                    (0, 0, 0, 0)
                                };
                            tracing::debug!(
                                stream = ?state.stream_id,
                                frames = state.frame_count,
                                total_bytes = state.total_bytes,
                                iframes = state.iframe_count,
                                errors = state.error_count,
                                shm_overflow = overflow,
                                shm_eviction = eviction,
                                shm_fallback = fallback,
                                shm_dropped = dropped,
                                "Frame read loop progress (owned path)"
                            );
                        }

                        // Try owned callbacks first (zero-copy path).
                        // If no owned callbacks are registered, fall back to
                        // legacy FrameCallback with a borrowed Frame.
                        if let Some(remaining) =
                            invoke_owned_callbacks_from_map(owned_callbacks, owned_frame)
                        {
                            // No owned callbacks consumed the frame — fall back to legacy path.
                            // Create a temporary Frame borrowing from the OwnedFrame's
                            // BytesMut buffer for backward-compatible callback invocation.
                            // SAFETY: remaining.data is a valid BytesMut allocation.
                            // The pointer remains valid for the duration of callback
                            // invocation because remaining lives on this stack frame
                            // and is not dropped until after invoke_callbacks_from_map.
                            let frame = Frame {
                                data: remaining.data.as_ptr(),
                                size: frame_size,
                                timestamp: remaining.timestamp,
                                frame_type,
                                stream_id: state.stream_id,
                            };
                            invoke_callbacks_from_map(callbacks, &frame);
                        } // else: owned callbacks consumed the frame — no legacy fallback needed

                        // Release the frame (sends remote_token to daemon).
                        if let Err(e) = ipc.release_frame_owned(handle.as_ptr()) {
                            tracing::warn!(
                                stream = ?state.stream_id,
                                error = %e,
                                "release_frame_owned failed"
                            );
                        }
                        tracing::trace!(stream = ?state.stream_id, "Owned frame released");

                        continue; // Continue draining
                    }
                    Err(_e) => {
                        // Treat errors from fetch_frame_owned similar to legacy
                        // venc_get_stream failures. The owned path returns errors
                        // for both IPC failures and daemon-side no-data.
                        state.consecutive_no_data += 1;
                        state.last_error_was_no_data = true;
                        stream_health.record_no_data_error(state.stream_id);

                        if state
                            .consecutive_no_data
                            .is_multiple_of(no_data_recovery_interval_errors)
                        {
                            if state.frame_count > 0 {
                                if let Some(handle_addr) = state.recovery_encoder_handle_addr {
                                    let idr_ret =
                                        ffi.venc_set_iframe(handle_addr as *mut std::ffi::c_void);
                                    if idr_ret == AK_SUCCESS_I32 {
                                        tracing::warn!(
                                            stream = ?state.stream_id,
                                            consecutive_no_data = state.consecutive_no_data,
                                            recovery_interval = no_data_recovery_interval_errors,
                                            "Sustained no-data detected; requested IDR recovery frame"
                                        );
                                    } else {
                                        tracing::warn!(
                                            stream = ?state.stream_id,
                                            consecutive_no_data = state.consecutive_no_data,
                                            recovery_interval = no_data_recovery_interval_errors,
                                            idr_error_code = idr_ret,
                                            "Sustained no-data detected; IDR recovery request failed"
                                        );
                                    }
                                }
                            } else if state.recovery_encoder_handle_addr.is_some() {
                                tracing::debug!(
                                    stream = ?state.stream_id,
                                    consecutive_no_data = state.consecutive_no_data,
                                    recovery_interval = no_data_recovery_interval_errors,
                                    "Skipping no-data IDR recovery before first frame"
                                );
                            }
                        }

                        break; // Exit inner drain loop
                    }
                }
            }

            // ── Legacy path (no VendorIpc available, e.g. in tests) ──
            let mut stream = std::mem::MaybeUninit::<video_stream>::uninit();
            let stream_ptr = stream.as_mut_ptr();
            let ret = ffi.venc_get_stream(handle.as_ptr(), stream_ptr);

            if ret != AK_SUCCESS_I32 {
                // Optimistic fast-path: skip IPC get_error_no() call when we
                // expect no-data (the common case). Probe on first call to
                // establish baseline, then periodically to detect changes.
                let probe_interval = 50u32;
                let should_probe = state.consecutive_no_data == 0
                    || !state.last_error_was_no_data
                    || state
                        .consecutive_no_data
                        .wrapping_add(1)
                        .is_multiple_of(probe_interval);

                let is_no_data = if should_probe {
                    let sdk_errno = ffi.get_error_no();
                    sdk_errno == SDK_ERROR_NO_DATA
                } else {
                    true // Assume no-data (optimistic)
                };

                if is_no_data {
                    state.consecutive_no_data += 1;
                    state.last_error_was_no_data = true;
                    stream_health.record_no_data_error(state.stream_id);

                    if state
                        .consecutive_no_data
                        .is_multiple_of(no_data_recovery_interval_errors)
                    {
                        if state.frame_count > 0 {
                            if let Some(handle_addr) = state.recovery_encoder_handle_addr {
                                let idr_ret =
                                    ffi.venc_set_iframe(handle_addr as *mut std::ffi::c_void);
                                if idr_ret == AK_SUCCESS_I32 {
                                    tracing::warn!(
                                        stream = ?state.stream_id,
                                        consecutive_no_data = state.consecutive_no_data,
                                        recovery_interval = no_data_recovery_interval_errors,
                                        "Sustained no-data detected; requested IDR recovery frame"
                                    );
                                } else {
                                    tracing::warn!(
                                        stream = ?state.stream_id,
                                        consecutive_no_data = state.consecutive_no_data,
                                        recovery_interval = no_data_recovery_interval_errors,
                                        idr_error_code = idr_ret,
                                        "Sustained no-data detected; IDR recovery request failed"
                                    );
                                }
                            }
                        } else if state.recovery_encoder_handle_addr.is_some() {
                            tracing::debug!(
                                stream = ?state.stream_id,
                                consecutive_no_data = state.consecutive_no_data,
                                recovery_interval = no_data_recovery_interval_errors,
                                "Skipping no-data IDR recovery before first frame"
                            );
                        }
                    }
                } else {
                    state.last_error_was_no_data = false;
                    state.error_count += 1;
                    // Log non-no-data errors on first occurrence and every 50th
                    if state.error_count == 1 || state.error_count.is_multiple_of(50) {
                        let sdk_errstr = ffi.get_error_str();
                        tracing::warn!(
                            stream = ?state.stream_id,
                            error_code = ret,
                            "venc_get_stream failed (non-no-data error): {}",
                            sdk_errstr
                        );
                    }
                }
                break; // Exit inner drain loop
            }

            // Reset no-data counter on successful frame retrieval
            state.consecutive_no_data = 0;
            state.last_error_was_no_data = true;

            // SAFETY: venc_get_stream succeeded, so `stream` is fully initialized.
            let stream_data = unsafe { stream.assume_init_mut() };

            if !stream_data.data.is_null() && stream_data.len > 0 {
                let frame_type = sdk_frame_type_to_frame_type(stream_data.frame_type);
                let frame_size = stream_data.len as usize;

                tracing::trace!(
                    stream = ?state.stream_id,
                    size = frame_size,
                    timestamp_ms = stream_data.ts,
                    frame_type = ?frame_type,
                    "Frame retrieved from SDK"
                );

                state.frame_count += 1;
                frames_this_cycle += 1;
                stream_health.record_frame(state.stream_id);
                state.total_bytes += frame_size as u64;
                if matches!(frame_type, FrameType::VideoIFrame) {
                    state.iframe_count += 1;
                }

                let latest_imaging_seq = LAST_IMAGING_UPDATE_SEQ.load(Ordering::Relaxed);
                if latest_imaging_seq > state.last_imaging_seq_frame_logged {
                    let applied_ms = LAST_IMAGING_UPDATE_UNIX_MS.load(Ordering::Relaxed);
                    let latency_ms = current_unix_ms().saturating_sub(applied_ms);
                    tracing::info!(
                        stream = ?state.stream_id,
                        imaging_seq = latest_imaging_seq,
                        latency_ms,
                        frame_type = ?frame_type,
                        "First encoded frame observed after imaging update"
                    );
                    state.last_imaging_seq_frame_logged = latest_imaging_seq;
                }

                if matches!(frame_type, FrameType::VideoIFrame)
                    && latest_imaging_seq > state.last_imaging_seq_iframe_logged
                {
                    let applied_ms = LAST_IMAGING_UPDATE_UNIX_MS.load(Ordering::Relaxed);
                    let latency_ms = current_unix_ms().saturating_sub(applied_ms);
                    tracing::info!(
                        stream = ?state.stream_id,
                        imaging_seq = latest_imaging_seq,
                        latency_ms,
                        "First IDR observed after imaging update"
                    );
                    state.last_imaging_seq_iframe_logged = latest_imaging_seq;
                }

                // Periodic summary every 300 frames (~10s at 30fps)
                if state.frame_count.is_multiple_of(300) {
                    tracing::debug!(
                        stream = ?state.stream_id,
                        frames = state.frame_count,
                        total_bytes = state.total_bytes,
                        iframes = state.iframe_count,
                        errors = state.error_count,
                        "Frame read loop progress"
                    );
                }

                let frame = Frame {
                    data: stream_data.data as *const u8,
                    size: frame_size,
                    // SDK timestamps are in milliseconds; Frame uses microseconds
                    timestamp: stream_data.ts.wrapping_mul(1000),
                    frame_type,
                    stream_id: state.stream_id,
                };

                // Invoke all callbacks (panic-isolated)
                invoke_callbacks_from_map(callbacks, &frame);
            } else {
                tracing::trace!(
                    stream = ?state.stream_id,
                    data_null = stream_data.data.is_null(),
                    len = stream_data.len,
                    "Frame skipped: null data or zero length"
                );
            }

            // Release the SDK buffer back to the encoder.
            // SAFETY: We pass back the same stream struct that get_stream populated.
            // The data pointer is owned by the SDK and must be returned.
            // This MUST happen even during shutdown to avoid leaking SDK buffers.
            let _ = ffi.venc_release_stream(handle.as_ptr(), stream_data);
            tracing::trace!(stream = ?state.stream_id, "SDK buffer released");
        }

        frames_this_cycle
    }

    let has_sub = sub_stream_handle.is_some();

    tracing::info!(
        has_sub_stream = has_sub,
        "Unified frame read loop started (push-only mode)"
    );

    let mut main_state = StreamState::new(StreamId::VideoMain, main_enc_addr);
    let mut sub_state = StreamState::new(StreamId::VideoSub, sub_enc_addr);

    if vendor_ipc.is_none() {
        #[cfg(test)]
        {
            let idle_poll_sleep_ms = env_var_u64("ANYKA_FRAME_POLL_SLEEP_MS")
                .unwrap_or(50)
                .max(1);
            let active_poll_sleep_ms = env_var_u64("ONVIF_ACTIVE_POLL_SLEEP_MS")
                .unwrap_or(8)
                .max(1);
            let default_no_data_idr_trigger_ms =
                u64::from(NO_DATA_IDR_RECOVERY_EVERY_ERRORS) * idle_poll_sleep_ms;
            let no_data_idr_trigger_ms = env_var_u64("ONVIF_NO_DATA_IDR_TRIGGER_MS")
                .unwrap_or(default_no_data_idr_trigger_ms);
            let mut current_sleep_ms: u64 = idle_poll_sleep_ms;

            while !stop_signal.load(Ordering::SeqCst) {
                let has_active_callbacks = !callbacks.read().is_empty();
                let cycle_sleep_ms = if has_active_callbacks {
                    active_poll_sleep_ms
                } else {
                    idle_poll_sleep_ms
                };
                let no_data_recovery_interval_errors = compute_no_data_recovery_interval_errors(
                    no_data_idr_trigger_ms,
                    cycle_sleep_ms,
                );

                let main_frames = drain_stream(
                    &main_stream_handle,
                    _ffi.as_ref(),
                    &mut main_state,
                    &callbacks,
                    &stop_signal,
                    &stream_health,
                    no_data_recovery_interval_errors,
                );

                let sub_frames = if let Some(ref sub_sh) = sub_stream_handle {
                    drain_stream(
                        sub_sh,
                        _ffi.as_ref(),
                        &mut sub_state,
                        &callbacks,
                        &stop_signal,
                        &stream_health,
                        no_data_recovery_interval_errors,
                    )
                } else {
                    0
                };

                let total_frames_this_cycle = main_frames + sub_frames;
                if has_active_callbacks || total_frames_this_cycle > 0 {
                    current_sleep_ms = cycle_sleep_ms;
                } else {
                    current_sleep_ms = (current_sleep_ms * 2).min(cycle_sleep_ms * 4);
                }
                std::thread::sleep(Duration::from_millis(current_sleep_ms));
            }

            tracing::info!("Unified frame read loop exited (test fallback mode)");
            return;
        }

        #[cfg(not(test))]
        {
            tracing::error!("Push-only mode requires VendorIpc; unified reader exiting");
            return;
        }
    }

    let ipc = if let Some(ipc) = vendor_ipc.as_ref() {
        ipc
    } else {
        tracing::error!("Push-only mode requires VendorIpc; unified reader exiting");
        return;
    };

    if let Err(e) = ipc.start_push(main_stream_handle.as_ptr(), StreamId::VideoMain) {
        tracing::error!("Failed to start push mode for main stream: {}", e);
        return;
    }
    let mut sub_push_started = false;
    if let Some(ref sub_handle) = sub_stream_handle {
        if let Err(e) = ipc.start_push(sub_handle.as_ptr(), StreamId::VideoSub) {
            tracing::error!("Failed to start push mode for sub stream: {}", e);
            let _ = ipc.stop_push(Some(StreamId::VideoMain));
            return;
        }
        sub_push_started = true;
    }

    tracing::info!(
        has_sub_stream = sub_push_started,
        "Push-based frame delivery active"
    );

    // ── Push-mode fast path: daemon polls SDK, pushes frames proactively ──
    // Eliminates ~105 wasted IPC round-trips/sec (only 15 of ~120 carry frames).
    if let Some(ref ipc) = vendor_ipc
        && push_mode_enabled(has_sub)
    {
        match ipc.start_push(main_stream_handle.as_ptr()) {
            Err(e) => {
                tracing::warn!("Push mode not available, falling back to polling: {}", e);
            }
            Ok(()) => {
                tracing::info!("Push-based frame delivery active");
                while !stop_signal.load(Ordering::SeqCst) {
                    match ipc.recv_pushed_frame(StreamId::VideoMain, frame_pool.as_deref()) {
                        Ok(owned_frame) => {
                            main_state.consecutive_no_data = 0;
                            main_state.last_error_was_no_data = true;

                            let frame_type = owned_frame.frame_type;
                            let frame_size = owned_frame.data.len();

                            main_state.frame_count += 1;
                            stream_health.record_frame(StreamId::VideoMain);
                            main_state.total_bytes += frame_size as u64;
                            if matches!(frame_type, FrameType::VideoIFrame) {
                                main_state.iframe_count += 1;
                            }

                            if main_state.frame_count.is_multiple_of(300) {
                                let (overflow, eviction, fallback, dropped) =
                                    ipc.shm_diagnostic_counters();
                                tracing::debug!(
                                    frames = main_state.frame_count,
                                    total_bytes = main_state.total_bytes,
                                    iframes = main_state.iframe_count,
                                    shm_overflow = overflow,
                                    shm_eviction = eviction,
                                    shm_fallback = fallback,
                                    shm_dropped = dropped,
                                    "Push-mode frame delivery progress"
                                );
                            }

                            if let Some(remaining) =
                                invoke_owned_callbacks_from_map(&owned_callbacks, owned_frame)
                            {
                                let frame = Frame {
                                    data: remaining.data.as_ptr(),
                                    size: frame_size,
                                    timestamp: remaining.timestamp,
                                    frame_type,
                                    stream_id: StreamId::VideoMain,
                                };
                                invoke_callbacks_from_map(&callbacks, &frame);
                            }
                        }
                        Err(e) => {
                            tracing::debug!("Push recv error: {}", e);
                            main_state.consecutive_no_data += 1;
                            // Timeouts and dropped-frame notifications are normal in push mode.
                            if is_push_mode_transient_error(&e) {
                                continue;
                            }
                            // IO/disconnect/protocol errors require fallback to polling.
                            tracing::warn!("Push mode interrupted, falling back to polling: {}", e);
                            break;
                        }
                    }
                }
                let _ = ipc.stop_push();
                tracing::info!(
                    push_frames = main_state.frame_count,
                    push_bytes = main_state.total_bytes,
                    "Push mode ended"
                );
            }
        }
    } else if vendor_ipc.is_some() && has_sub {
        tracing::info!(
            "Push mode disabled because sub stream is active; using unified polling for both streams"
        );
    }

    while !stop_signal.load(Ordering::SeqCst) {
        match ipc.recv_pushed_frame(frame_pool.as_deref()) {
            Ok(owned_frame) => {
                let state = match owned_frame.stream_id {
                    StreamId::VideoMain => &mut main_state,
                    StreamId::VideoSub => &mut sub_state,
                    StreamId::Audio => {
                        tracing::trace!("Ignoring unexpected audio frame in video loop");
                        continue;
                    }
                };

                state.consecutive_no_data = 0;
                state.last_error_was_no_data = true;
                let frame_type = owned_frame.frame_type;
                let frame_size = owned_frame.data.len();

                state.frame_count += 1;
                stream_health.record_frame(owned_frame.stream_id);
                state.total_bytes += frame_size as u64;
                if matches!(frame_type, FrameType::VideoIFrame) {
                    state.iframe_count += 1;
                }

                if state.frame_count.is_multiple_of(300) {
                    let (overflow, eviction, fallback, dropped) = ipc.shm_diagnostic_counters();
                    tracing::debug!(
                        stream = ?owned_frame.stream_id,
                        frames = state.frame_count,
                        total_bytes = state.total_bytes,
                        iframes = state.iframe_count,
                        shm_overflow = overflow,
                        shm_eviction = eviction,
                        shm_fallback = fallback,
                        shm_dropped = dropped,
                        "Push-mode frame delivery progress"
                    );
                }

                if let Some(remaining) =
                    invoke_owned_callbacks_from_map(&owned_callbacks, owned_frame)
                {
                    let frame = Frame {
                        data: remaining.data.as_ptr(),
                        size: frame_size,
                        timestamp: remaining.timestamp,
                        frame_type,
                        stream_id: remaining.stream_id,
                    };
                    invoke_callbacks_from_map(&callbacks, &frame);
                }
            }
            Err(e) => {
                tracing::debug!("Push recv error: {}", e);
                if is_push_mode_transient_error(&e) {
                    continue;
                }
                tracing::error!("Push mode interrupted by non-transient error: {}", e);
                break;
            }
        }
    }

    if sub_push_started {
        let _ = ipc.stop_push(Some(StreamId::VideoSub));
    }
    let _ = ipc.stop_push(Some(StreamId::VideoMain));
    tracing::info!("Push mode ended");

    tracing::info!(
        main_frames = main_state.frame_count,
        main_bytes = main_state.total_bytes,
        main_iframes = main_state.iframe_count,
        main_errors = main_state.error_count,
        sub_frames = sub_state.frame_count,
        sub_bytes = sub_state.total_bytes,
        sub_iframes = sub_state.iframe_count,
        sub_errors = sub_state.error_count,
        "Unified frame read loop exited"
    );
}

/// Invoke all registered callbacks with a frame, isolating panics.
///
/// This is a standalone function (not a method) so it can be used from
/// the `frame_read_loop` thread without holding a reference to `AnykaVideoEncoder`.
fn invoke_callbacks_from_map(
    callbacks: &RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>,
    frame: &Frame,
) {
    let cbs = callbacks.read();
    let cb_count = cbs.len();
    tracing::trace!(
        callback_count = cb_count,
        stream = ?frame.stream_id,
        "Invoking frame callbacks"
    );
    let mut failed = Vec::new();

    for (id, cb) in cbs.iter() {
        let start = std::time::Instant::now();
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            cb.on_frame(frame);
        }));
        let elapsed = start.elapsed();
        let elapsed_us = elapsed.as_micros() as u64;
        record_callback_duration(elapsed_us);

        if elapsed_us > CALLBACK_SLOW_WARN_THRESHOLD_US {
            maybe_log_slow_callback(*id, elapsed_us, "borrowed");
        }

        if result.is_err() {
            tracing::error!("Frame callback {} panicked, marking for removal", id);
            failed.push(*id);
        }
    }

    if !failed.is_empty() {
        drop(cbs);
        let mut cbs_write = callbacks.write();
        for id in failed {
            cbs_write.remove(&id);
        }
    }

    maybe_log_callback_histogram();
}

/// Invoke all registered owned-frame callbacks, transferring ownership.
///
/// If there are no owned callbacks, returns `Some(owned_frame)` so the caller
/// can fall back to the legacy `FrameCallback` path.
///
/// If there is exactly one callback (common case — just `StreamingBridge`),
/// the `OwnedFrame` is moved directly — true zero-copy.
///
/// If there are multiple callbacks, each except the last receives a clone.
fn invoke_owned_callbacks_from_map(
    owned_callbacks: &RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>,
    owned_frame: OwnedFrame,
) -> Option<OwnedFrame> {
    let cbs = owned_callbacks.read();
    let cb_count = cbs.len();

    if cb_count == 0 {
        return Some(owned_frame);
    }

    tracing::trace!(
        callback_count = cb_count,
        stream = ?owned_frame.stream_id,
        "Invoking owned frame callbacks (zero-copy)"
    );

    // Collect Arc refs so we can drop the read lock before invoking
    let callbacks: Vec<(CallbackId, Arc<dyn OwnedFrameCallback>)> =
        cbs.iter().map(|(id, cb)| (*id, Arc::clone(cb))).collect();
    drop(cbs);

    let mut failed = Vec::new();
    let last_idx = callbacks.len() - 1;

    for (i, (id, cb)) in callbacks.iter().enumerate() {
        let start = std::time::Instant::now();

        let frame_to_send = if i < last_idx {
            // Not the last callback — clone the data
            OwnedFrame {
                data: owned_frame.data.clone(),
                timestamp: owned_frame.timestamp,
                frame_type: owned_frame.frame_type,
                stream_id: owned_frame.stream_id,
            }
        } else {
            // Last callback — will get the moved frame below
            break;
        };

        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            cb.on_owned_frame(frame_to_send);
        }));

        let elapsed = start.elapsed();
        let elapsed_us = elapsed.as_micros() as u64;
        record_callback_duration(elapsed_us);

        if elapsed_us > CALLBACK_SLOW_WARN_THRESHOLD_US {
            maybe_log_slow_owned_callback(*id, elapsed_us);
        }

        if result.is_err() {
            tracing::error!("Owned frame callback {} panicked, marking for removal", id);
            failed.push(*id);
        }
    }

    // Invoke the last callback with the original owned_frame (moved)
    let (last_id, last_cb) = &callbacks[last_idx];
    let start = std::time::Instant::now();
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        last_cb.on_owned_frame(owned_frame);
    }));
    let elapsed = start.elapsed();
    let elapsed_us = elapsed.as_micros() as u64;
    record_callback_duration(elapsed_us);
    if elapsed_us > CALLBACK_SLOW_WARN_THRESHOLD_US {
        maybe_log_slow_owned_callback(*last_id, elapsed_us);
    }
    if result.is_err() {
        tracing::error!(
            "Owned frame callback {} panicked, marking for removal",
            last_id
        );
        failed.push(*last_id);
    }

    // Remove failed callbacks
    if !failed.is_empty() {
        let mut cbs_write = owned_callbacks.write();
        for id in failed {
            cbs_write.remove(&id);
        }
    }

    maybe_log_callback_histogram();

    None // Frame was consumed by owned callbacks
}

/// Anyka video encoder implementation with FFI integration and callback support.
///
/// Manages dual video encoders (main 720p + sub 360p) with:
/// - RAII-based FFI handles via `VideoEncoderHandle`
/// - Zero-copy frame delivery to multiple subscribers
/// - Panic-isolated callback invocation
/// - Dynamic bitrate reconfiguration
///
/// # Architecture
///
/// ```text
/// AnykaVideoEncoder
///   ├── ffi: Arc<dyn VideoHalTrait>       (injected, mockable)
///   ├── main_handle: RwLock<Option<Arc<VideoEncoderHandle>>>
///   ├── sub_handle:  RwLock<Option<Arc<VideoEncoderHandle>>>
///   ├── callbacks: RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>
///   └── active_frames: Arc<ActiveFrames>  (ref-counted buffer tracking)
/// ```
struct AnykaVideoEncoder {
    ffi: Arc<dyn crate::hal::video::VideoHalTrait>,
    /// Optional VendorIpc reference for the zero-copy owned frame path.
    /// This is the same object as `ffi` (when using IPC mode), stored
    /// separately because we can't downcast `dyn VideoHalTrait` to `VendorIpc`.
    vendor_ipc: Option<Arc<VendorIpc>>,
    configurations: RwLock<Vec<VideoEncoderConfig>>,
    main_handle: RwLock<Option<Arc<VideoEncoderHandle>>>,
    sub_handle: RwLock<Option<Arc<VideoEncoderHandle>>>,
    main_state: RwLock<EncoderState>,
    sub_state: RwLock<EncoderState>,
    callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>>,
    owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>>,
    active_frames: Arc<ActiveFrames>,
    next_callback_id: AtomicU64,
    main_stream_handle: RwLock<Option<Arc<VideoStreamHandle>>>,
    sub_stream_handle: RwLock<Option<Arc<VideoStreamHandle>>>,
    read_thread: RwLock<Option<std::thread::JoinHandle<()>>>,
    stop_signal: Arc<AtomicBool>,
    stream_health: Arc<StreamHealthCounters>,
    unsafe_shutdown_required: AtomicBool,
}

#[cfg(test)]
const STREAM_THREAD_JOIN_TIMEOUT: Duration = Duration::from_millis(50);
#[cfg(not(test))]
const STREAM_THREAD_JOIN_TIMEOUT: Duration = Duration::from_secs(3);
#[cfg(test)]
const STREAM_CANCEL_TIMEOUT: Duration = Duration::from_millis(20);
#[cfg(not(test))]
const STREAM_CANCEL_TIMEOUT: Duration = Duration::from_secs(2);
/// Grace period after setting `stop_signal` before cancelling streams.
/// Gives non-stuck reader threads time to check `stop_signal` and exit the
/// drain loop naturally. Dana vendor uses 20ms; we use 50ms for ~1.5 poll
/// cycles at the default 30ms sleep interval.
#[cfg(test)]
const CANCEL_GRACE_PERIOD: Duration = Duration::from_millis(5);
#[cfg(not(test))]
const CANCEL_GRACE_PERIOD: Duration = Duration::from_millis(50);

/// Join a thread with a timeout. Returns `true` if the thread completed (success or
/// panic), otherwise returns the original join handle so caller can retry after an
/// emergency unblock action.
fn join_thread_with_timeout(
    thread: std::thread::JoinHandle<()>,
    name: &str,
    timeout: Duration,
) -> Result<(), std::thread::JoinHandle<()>> {
    let start = std::time::Instant::now();
    let thread = thread;
    while !thread.is_finished() {
        if start.elapsed() >= timeout {
            tracing::error!(
                "Thread '{}' join timed out after {:?} — thread may be stuck in kernel I/O",
                name,
                timeout
            );
            return Err(thread);
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    match thread.join() {
        Ok(()) => {
            tracing::info!("Thread '{}' joined successfully", name);
        }
        Err(e) => {
            tracing::warn!("Thread '{}' panicked: {:?}", name, e);
        }
    }

    Ok(())
}

impl AnykaVideoEncoder {
    /// Create a new `AnykaVideoEncoder` with the default (real) FFI backend.
    ///
    /// Uses `VendorIpc` to connect to the vendor daemon for vendor library access.
    fn new() -> PlatformResult<Self> {
        let ipc = crate::hal::vendor_ipc::VendorIpc::new().map_err(|e| {
            PlatformError::InitializationFailed(format!(
                "AnykaVideoEncoder: VendorIpc connection failed: {}",
                e
            ))
        })?;
        tracing::info!("AnykaVideoEncoder: using VendorIpc for vendor library access");
        Ok(Self::with_ffi(Arc::new(ipc)))
    }

    /// Create a new `AnykaVideoEncoder` with a custom FFI backend.
    ///
    /// Used by tests with `MockVideoHalTrait` for hardware-free testing.
    fn with_ffi(ffi: Arc<dyn crate::hal::video::VideoHalTrait>) -> Self {
        Self {
            ffi,
            vendor_ipc: None, // No VendorIpc available when using custom FFI
            configurations: RwLock::new(vec![
                VideoEncoderConfig {
                    token: "VideoEncoder_1".to_string(),
                    name: "Main Stream".to_string(),
                    resolution: Resolution::new(1280, 720),
                    framerate: 15,
                    bitrate: 2000,
                    encoding: VideoEncoding::H264,
                    gop_length: 30,
                    quality: 80,
                    ..Default::default()
                },
                VideoEncoderConfig {
                    token: "VideoEncoder_2".to_string(),
                    name: "Sub Stream".to_string(),
                    resolution: Resolution::new(640, 360),
                    framerate: 15,
                    bitrate: 300,
                    encoding: VideoEncoding::H264,
                    gop_length: 30,
                    quality: 70,
                    ..Default::default()
                },
            ]),
            main_handle: RwLock::new(None),
            sub_handle: RwLock::new(None),
            main_state: RwLock::new(EncoderState::Uninitialized),
            sub_state: RwLock::new(EncoderState::Uninitialized),
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            owned_callbacks: Arc::new(RwLock::new(HashMap::new())),
            active_frames: Arc::new(ActiveFrames::new()),
            next_callback_id: AtomicU64::new(1),
            main_stream_handle: RwLock::new(None),
            sub_stream_handle: RwLock::new(None),
            read_thread: RwLock::new(None),
            stop_signal: Arc::new(AtomicBool::new(false)),
            stream_health: Arc::new(StreamHealthCounters::default()),
            unsafe_shutdown_required: AtomicBool::new(false),
        }
    }

    /// Create a new `AnykaVideoEncoder` with a shared VendorIpc instance.
    ///
    /// The VendorIpc is stored both as the `dyn VideoHalTrait` backend and as a
    /// concrete reference for the zero-copy frame fetch path.
    fn with_vendor_ipc(ipc: Arc<VendorIpc>) -> Self {
        let mut encoder = Self::with_ffi(ipc.clone() as Arc<dyn crate::hal::video::VideoHalTrait>);
        encoder.vendor_ipc = Some(ipc);
        encoder
    }

    fn mark_unsafe_shutdown(&self, reason: &str) {
        let first = !self.unsafe_shutdown_required.swap(true, Ordering::SeqCst);
        if first {
            tracing::error!(
                reason = reason,
                "Unsafe video teardown detected; hard process termination required"
            );
        } else {
            tracing::error!(
                reason = reason,
                "Unsafe video teardown already active; preserving hard-exit requirement"
            );
        }
    }

    fn leak_stream_handles_for_hard_shutdown(&self) {
        if let Some(handle) = self.main_stream_handle.write().take() {
            std::mem::forget(handle);
        }
        if let Some(handle) = self.sub_stream_handle.write().take() {
            std::mem::forget(handle);
        }
    }

    fn leak_encoder_handles_for_hard_shutdown(&self) {
        if let Some(handle) = self.main_handle.write().take() {
            *self.main_state.write() = EncoderState::Uninitialized;
            std::mem::forget(handle);
        }
        if let Some(handle) = self.sub_handle.write().take() {
            *self.sub_state.write() = EncoderState::Uninitialized;
            std::mem::forget(handle);
        }
    }

    fn fail_fast_to_hard_shutdown(&self, reason: impl Into<String>) -> PlatformError {
        let reason = reason.into();
        self.mark_unsafe_shutdown(&reason);
        self.leak_stream_handles_for_hard_shutdown();
        self.leak_encoder_handles_for_hard_shutdown();
        PlatformError::HardwareFailure(format!("unsafe teardown required: {}", reason))
    }

    fn requires_hard_shutdown(&self) -> bool {
        self.unsafe_shutdown_required.load(Ordering::SeqCst)
    }

    fn sync_configurations_to_channel_layout(&self, main: Resolution, sub: Resolution) {
        let mut configs = self.configurations.write();
        for cfg in configs.iter_mut() {
            match cfg.token.as_str() {
                "VideoEncoder_1" => cfg.resolution = main,
                "VideoEncoder_2" => cfg.resolution = sub,
                _ => {}
            }
        }
        tracing::info!(
            "Aligned encoder configurations to VI layout: main={}x{}, sub={}x{}",
            main.width,
            main.height,
            sub.width,
            sub.height
        );
    }

    fn wait_for_stream_readiness(
        &self,
        timeout: Duration,
        require_sub: bool,
    ) -> PlatformResult<()> {
        let started = Instant::now();
        loop {
            let health = self.stream_health.snapshot();
            if health.main_frames > 0 && (!require_sub || health.sub_frames > 0) {
                if health.sub_frames == 0 {
                    tracing::warn!(
                        "VI/VENC readiness check passed on main stream only (sub stream has no frames)"
                    );
                } else {
                    tracing::info!(
                        "VI/VENC readiness check passed: main_frames={}, sub_frames={}",
                        health.main_frames,
                        health.sub_frames
                    );
                }
                return Ok(());
            }
            if started.elapsed() >= timeout {
                return Err(PlatformError::InitializationFailed(format!(
                    "VI/VENC pipeline readiness timeout after {:?}: main_frames={}, sub_frames={}, main_no_data_errors={}, sub_no_data_errors={}",
                    timeout,
                    health.main_frames,
                    health.sub_frames,
                    health.main_no_data_errors,
                    health.sub_no_data_errors
                )));
            }
            std::thread::sleep(Duration::from_millis(PIPELINE_READINESS_POLL_MS));
        }
    }

    /// Map a `VideoEncoderConfig` to FFI `encode_param`.
    fn config_to_encode_param(
        config: &VideoEncoderConfig,
        channel: encode_use_chn,
    ) -> encode_param {
        let enc_out_type = match config.encoding {
            VideoEncoding::H264 => encode_output_type::H264_ENC_TYPE,
            VideoEncoding::H265 => encode_output_type::HEVC_ENC_TYPE,
            VideoEncoding::Mjpeg => encode_output_type::MJPEG_ENC_TYPE,
        };
        let br_mode = match config.bitrate_mode {
            crate::platform::BitrateMode::Cbr => bitrate_ctrl_mode::BR_MODE_CBR,
            crate::platform::BitrateMode::Vbr => bitrate_ctrl_mode::BR_MODE_VBR,
        };
        encode_param {
            width: config.resolution.width,
            height: config.resolution.height,
            minqp: 20,
            maxqp: 51,
            fps: config.framerate as i32,
            goplen: config.gop_length as i32,
            bps: config.bitrate as i32, // kbps (vendor SDK expects kbps despite field name)
            profile: profile_mode::PROFILE_MAIN,
            use_chn: channel,
            enc_grp: match channel {
                encode_use_chn::ENCODE_MAIN_CHN => encode_group_type::ENCODE_MAINCHN_NET,
                encode_use_chn::ENCODE_SUB_CHN => encode_group_type::ENCODE_SUBCHN_NET,
            },
            br_mode,
            enc_out_type,
        }
    }

    /// Register a frame callback.
    ///
    /// Returns a `CallbackId` that can be used to unregister the callback.
    /// Multiple callbacks can be registered (e.g., RTSP + HTTP-FLV).
    pub fn register_frame_callback(&self, callback: Arc<dyn FrameCallback>) -> CallbackId {
        let id = self.next_callback_id.fetch_add(1, Ordering::SeqCst);
        self.callbacks.write().insert(id, callback);
        id
    }

    /// Unregister a previously registered frame callback.
    pub fn unregister_frame_callback(&self, id: CallbackId) -> bool {
        self.callbacks.write().remove(&id).is_some()
    }

    /// Register an owned frame callback (zero-copy path).
    ///
    /// Returns a `CallbackId` that can be used to unregister the callback.
    pub fn register_owned_frame_callback(
        &self,
        callback: Arc<dyn OwnedFrameCallback>,
    ) -> CallbackId {
        let id = self.next_callback_id.fetch_add(1, Ordering::SeqCst);
        self.owned_callbacks.write().insert(id, callback);
        id
    }

    /// Unregister a previously registered owned frame callback.
    pub fn unregister_owned_frame_callback(&self, id: CallbackId) -> bool {
        self.owned_callbacks.write().remove(&id).is_some()
    }

    /// Invoke all registered callbacks with a frame, isolating panics.
    ///
    /// Each callback is invoked in a `catch_unwind` boundary. If a callback
    /// panics, it is logged and marked for removal. Callbacks that take
    /// longer than 2ms generate a warning log.
    pub fn invoke_callbacks(&self, frame: &Frame) {
        let callbacks = self.callbacks.read();
        let mut failed_callbacks = Vec::new();

        for (id, callback) in callbacks.iter() {
            let start = Instant::now();

            let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                callback.on_frame(frame);
            }));

            let duration = start.elapsed();

            if duration > Duration::from_millis(2) {
                tracing::warn!(
                    "Frame callback {} took {:?} (exceeds 2ms threshold)",
                    id,
                    duration
                );
            }

            if result.is_err() {
                tracing::error!("Frame callback {} panicked, marking for removal", id);
                failed_callbacks.push(*id);
            }
        }

        // Remove panicked callbacks outside the read lock
        if !failed_callbacks.is_empty() {
            drop(callbacks);
            let mut callbacks_write = self.callbacks.write();
            for id in failed_callbacks {
                callbacks_write.remove(&id);
            }
        }
    }

    /// Get a reference to the active frames tracker.
    pub fn active_frames(&self) -> &Arc<ActiveFrames> {
        &self.active_frames
    }

    /// Start streaming from the encoder by requesting stream handles and spawning
    /// dedicated reader threads that poll the SDK for encoded frames.
    ///
    /// # Arguments
    ///
    /// * `vi_handle` - Video input handle (provides raw sensor data)
    /// * `main_enc` - Main encoder handle (720p)
    /// * `sub_enc` - Optional sub encoder handle (360p)
    pub fn start_streaming(
        &self,
        vi_handle: &Arc<crate::hal::VideoInputHandle>,
        main_enc: &Arc<VideoEncoderHandle>,
        sub_enc: Option<&Arc<VideoEncoderHandle>>,
    ) -> PlatformResult<()> {
        self.stop_signal.store(false, Ordering::SeqCst);
        self.stream_health.reset();

        // ── Phase 1: Request all streams (no reader threads yet) ─────────
        //
        // The vendor reference (ak_onvif_demo.c:498-541) requests ALL streams
        // before reading from any of them. This avoids a race condition where
        // venc_get_stream() (called by a reader thread) iterates the SDK's
        // internal venc_list under `cancel_mutex` while venc_request_stream()
        // modifies that same list under a different lock (`cancel_lock`).

        // 1a. Request main stream
        let main_sh = Arc::new(VideoStreamHandle::new(
            vi_handle.as_ptr(),
            main_enc.as_ptr(),
            Arc::clone(&self.ffi),
        )?);
        *self.main_stream_handle.write() = Some(Arc::clone(&main_sh));

        // 1b. Request sub stream (if encoder exists)
        let sub_sh = if let Some(sub) = sub_enc {
            let sh = Arc::new(VideoStreamHandle::new(
                vi_handle.as_ptr(),
                sub.as_ptr(),
                Arc::clone(&self.ffi),
            )?);
            *self.sub_stream_handle.write() = Some(Arc::clone(&sh));
            Some(sh)
        } else {
            None
        };

        // 1c. Kick initial IDR on requested streams after both requests are complete.
        // This keeps request ordering deterministic while still forcing fast decoder sync.
        if let Err(e) = video_encoder_request_idr_internal(main_enc, self.ffi.as_ref()) {
            tracing::warn!("Failed to set initial I-frame for main stream: {}", e);
        }
        if let Some(sub) = sub_enc
            && let Err(e) = video_encoder_request_idr_internal(sub, self.ffi.as_ref())
        {
            tracing::warn!("Failed to set initial I-frame for sub stream: {}", e);
        }
        tracing::debug!("Video streams requested and IDR kicks issued");

        // ── Phase 2: Single stabilization delay ─────────────────────────
        //
        // Both encoders are now requested and IDR-kicked. Give the ISP/encoder
        // pipeline time to produce the first frames. Configurable via env var
        // for on-device tuning; default 300ms exceeds the vendor's recommended
        // PLATFORM_DELAY_MS_RETRY (200ms) stabilization window.
        let stabilization_ms = env_var_u64("ANYKA_STREAM_STABILIZATION_MS").unwrap_or(300);
        std::thread::sleep(Duration::from_millis(stabilization_ms));
        tracing::debug!(stabilization_ms, "Stream stabilization delay complete");

        // ── Phase 3: Spawn unified reader thread ─────────────────────────
        //
        // A single thread drains both main and sub streams in alternating
        // fashion.  This eliminates IPC mutex contention that occurred when
        // two independent threads competed for the shared UnixStream to the
        // vendor daemon.

        let reader_thread = {
            let ffi = Arc::clone(&self.ffi);
            let callbacks = Arc::clone(&self.callbacks_arc());
            let stop = Arc::clone(&self.stop_signal);
            let stream_health = Arc::clone(&self.stream_health);
            let main_sh_clone = Arc::clone(&main_sh);
            let sub_sh_clone = sub_sh.as_ref().map(Arc::clone);
            let main_enc_addr = main_enc.as_ptr() as usize;
            let sub_enc_addr = sub_enc.map(|h| h.as_ptr() as usize);
            let vendor_ipc = self.vendor_ipc.clone();
            let owned_callbacks = Arc::clone(&self.owned_callbacks_arc());
            let frame_pool = vendor_ipc
                .as_ref()
                .map(|_| Arc::new(BytesMutPool::default_frame_pool()));
            std::thread::Builder::new()
                .name("venc-read".to_string())
                .spawn(move || {
                    unified_frame_read_loop(
                        main_sh_clone,
                        sub_sh_clone,
                        ffi,
                        callbacks,
                        owned_callbacks,
                        stop,
                        stream_health,
                        Some(main_enc_addr),
                        sub_enc_addr,
                        vendor_ipc,
                        frame_pool,
                    );
                })
                .map_err(|e| {
                    PlatformError::InitializationFailed(format!(
                        "Failed to spawn reader thread: {}",
                        e
                    ))
                })?
        };
        *self.read_thread.write() = Some(reader_thread);
        tracing::info!(
            has_sub_stream = sub_sh.is_some(),
            "Unified stream reader thread started"
        );

        Ok(())
    }

    /// Stop all frame-read threads and cancel the active video streams.
    ///
    /// **Cancel-first ordering** (matching vendor SDK pattern):
    ///   1. Set `stop_signal` — cooperative exit for non-stuck threads
    ///   2. Sleep `CANCEL_GRACE_PERIOD` — non-stuck threads exit naturally
    ///   3. `cancel_stream` — stops SDK internal threads, unblocks `get_stream`
    ///   4. Join reader threads — should complete quickly after cancel
    ///   5. Drop stream handles
    ///
    /// If cancel fails or join times out after cancel, we fail fast into
    /// unsafe teardown mode requiring process termination.
    pub fn stop_streaming(&self) -> PlatformResult<()> {
        if self.requires_hard_shutdown() {
            return Err(PlatformError::HardwareFailure(
                "unsafe teardown required: previous shutdown failure".to_string(),
            ));
        }

        // Phase 1: Signal stop and give non-stuck threads a grace period to exit.
        tracing::info!("stop_streaming: signalling stop...");
        self.stop_signal.store(true, Ordering::SeqCst);
        std::thread::sleep(CANCEL_GRACE_PERIOD);

        let mut main_stream_handle = self.main_stream_handle.read().clone();
        let mut sub_stream_handle = self.sub_stream_handle.read().clone();
        let mut failures: Vec<String> = Vec::new();
        let mut cancel_failed = false;

        // Phase 2: Cancel streams — stops SDK internal threads and unblocks
        // any reader stuck in ak_venc_get_stream().
        if let Some(ref handle) = main_stream_handle {
            tracing::info!("stop_streaming: cancelling main stream...");
            if let Err(e) = handle.cancel_checked_with_timeout(STREAM_CANCEL_TIMEOUT) {
                tracing::error!("stop_streaming: main stream cancel failed: {}", e);
                failures.push(format!("main stream cancel failed: {}", e));
                cancel_failed = true;
            } else {
                tracing::info!("stop_streaming: main stream cancelled");
            }
        }

        if cancel_failed {
            if sub_stream_handle.is_some() {
                tracing::warn!(
                    "stop_streaming: skipping sub stream cancel after main cancel failure \
                     to avoid lock contention with an in-flight vendor cancel call"
                );
            }
        } else if let Some(ref handle) = sub_stream_handle {
            tracing::info!("stop_streaming: cancelling sub stream...");
            if let Err(e) = handle.cancel_checked_with_timeout(STREAM_CANCEL_TIMEOUT) {
                tracing::error!("stop_streaming: sub stream cancel failed: {}", e);
                failures.push(format!("sub stream cancel failed: {}", e));
                cancel_failed = true;
            } else {
                tracing::info!("stop_streaming: sub stream cancelled");
            }
        }

        if cancel_failed {
            if let Some(handle) = main_stream_handle.take() {
                std::mem::forget(handle);
            }
            if let Some(handle) = sub_stream_handle.take() {
                std::mem::forget(handle);
            }
            return Err(self.fail_fast_to_hard_shutdown(failures.join("; ")));
        }

        // Phase 3: Join the unified reader thread — should complete quickly
        // now that cancel has unblocked any stuck SDK calls.
        if let Some(thread) = self.read_thread.write().take() {
            tracing::info!("stop_streaming: joining reader thread...");
            if let Err(_thread) =
                join_thread_with_timeout(thread, "venc-read", STREAM_THREAD_JOIN_TIMEOUT)
            {
                failures.push(
                    "reader thread join timeout after cancel (possible blocked kernel I/O)"
                        .to_string(),
                );
            }
        }

        if !failures.is_empty() {
            if let Some(handle) = main_stream_handle.take() {
                std::mem::forget(handle);
            }
            if let Some(handle) = sub_stream_handle.take() {
                std::mem::forget(handle);
            }
            return Err(self.fail_fast_to_hard_shutdown(failures.join("; ")));
        }

        // Phase 4: Drop stream handles (cancel already completed).
        tracing::info!("stop_streaming: dropping stream handles...");
        let _ = self.main_stream_handle.write().take();
        let _ = self.sub_stream_handle.write().take();

        tracing::info!("Streaming stopped");
        Ok(())
    }

    /// Get a cloned `Arc` reference to the callbacks map for thread sharing.
    fn callbacks_arc(&self) -> Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> {
        Arc::clone(&self.callbacks)
    }

    /// Get a cloned `Arc` reference to the owned callbacks map for thread sharing.
    fn owned_callbacks_arc(&self) -> Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> {
        Arc::clone(&self.owned_callbacks)
    }

    /// Request an IDR (I-frame) from the specified encoder channel.
    ///
    /// # Arguments
    ///
    /// * `main` - If true, request IDR from main encoder; otherwise from sub encoder.
    pub fn request_idr_frame(&self, main: bool) -> PlatformResult<()> {
        let handle_guard = if main {
            self.main_handle.read()
        } else {
            self.sub_handle.read()
        };

        let handle = handle_guard.as_ref().ok_or_else(|| {
            let channel = if main { "main" } else { "sub" };
            PlatformError::HardwareUnavailable(format!("{} encoder not initialized", channel))
        })?;

        video_encoder_request_idr_internal(handle, self.ffi.as_ref())
    }

    /// Close a single encoder by token.
    ///
    /// This is used for initialization rollback when one encoder fails after
    /// previous encoders have been successfully opened.
    fn close_encoder(&self, token: &str) -> PlatformResult<()> {
        if self.requires_hard_shutdown() {
            return Err(PlatformError::HardwareFailure(
                "unsafe teardown required: skipping encoder close".to_string(),
            ));
        }

        match token {
            "VideoEncoder_1" => {
                let old_handle = self.main_handle.write().take();
                if let Some(handle) = old_handle {
                    if let Err(e) = handle.close_blocking_with_ffi(self.ffi.as_ref()) {
                        return Err(self.fail_fast_to_hard_shutdown(format!(
                            "main encoder close failed: {}",
                            e
                        )));
                    }
                    *self.main_state.write() = EncoderState::Uninitialized;
                    tracing::info!("Closed video encoder token={}", token);
                }
                Ok(())
            }
            "VideoEncoder_2" => {
                let old_handle = self.sub_handle.write().take();
                if let Some(handle) = old_handle {
                    if let Err(e) = handle.close_blocking_with_ffi(self.ffi.as_ref()) {
                        return Err(self.fail_fast_to_hard_shutdown(format!(
                            "sub encoder close failed: {}",
                            e
                        )));
                    }
                    *self.sub_state.write() = EncoderState::Uninitialized;
                    tracing::info!("Closed video encoder token={}", token);
                }
                Ok(())
            }
            _ => Err(PlatformError::InvalidParameter(format!(
                "Unknown encoder token: {}",
                token
            ))),
        }
    }

    fn close_all_encoders(&self) -> PlatformResult<()> {
        if self.requires_hard_shutdown() {
            return Err(PlatformError::HardwareFailure(
                "unsafe teardown required: skipping encoder close".to_string(),
            ));
        }
        self.close_encoder("VideoEncoder_2")?;
        self.close_encoder("VideoEncoder_1")?;
        Ok(())
    }
}

#[async_trait]
impl VideoEncoder for AnykaVideoEncoder {
    async fn init(&self, config: &VideoEncoderConfig) -> PlatformResult<()> {
        let (channel, handle_lock, state_lock) = match config.token.as_str() {
            "VideoEncoder_1" => (
                encode_use_chn::ENCODE_MAIN_CHN,
                &self.main_handle,
                &self.main_state,
            ),
            "VideoEncoder_2" => (
                encode_use_chn::ENCODE_SUB_CHN,
                &self.sub_handle,
                &self.sub_state,
            ),
            _ => {
                return Err(PlatformError::InvalidParameter(format!(
                    "Unknown encoder token: {}. Expected VideoEncoder_1 or VideoEncoder_2",
                    config.token
                )));
            }
        };

        // Validate encoder resolution before opening
        if config.resolution.width == 0 || config.resolution.height == 0 {
            return Err(PlatformError::InvalidParameter(
                "Encoder resolution must be non-zero".to_string(),
            ));
        }
        if !config.resolution.width.is_multiple_of(4) || !config.resolution.height.is_multiple_of(4)
        {
            return Err(PlatformError::InvalidParameter(format!(
                "Encoder resolution {}x{} must be divisible by 4",
                config.resolution.width, config.resolution.height
            )));
        }

        let param = Self::config_to_encode_param(config, channel);

        tracing::debug!(
            "Opening encoder {}: {}x{} @ {}fps, {}kbps, goplen={}, enc_grp={:?}, use_chn={:?}, param_size={}",
            config.token,
            param.width,
            param.height,
            param.fps,
            param.bps,
            param.goplen,
            param.enc_grp,
            param.use_chn,
            std::mem::size_of::<encode_param>(),
        );

        // Point the vendor library at our SD card venc.cfg before opening.
        // The V2 encoder doesn't read this file but ak_venc_open requires it to exist.
        let cfg_path = c"/mnt/anyka_hack/onvif/venc.cfg";
        self.ffi.venc_set_cfg_path(cfg_path.as_ptr());

        let enc_handle = video_encoder_open_internal(&param, self.ffi.as_ref())?;

        *handle_lock.write() = Some(Arc::new(enc_handle));
        *state_lock.write() = EncoderState::Initialized;

        // Update stored configuration
        let mut configs = self.configurations.write();
        if let Some(cfg) = configs.iter_mut().find(|c| c.token == config.token) {
            *cfg = config.clone();
        } else {
            configs.push(config.clone());
        }

        tracing::info!(
            "Video encoder {} initialized: {}x{} @ {}fps, {}kbps",
            config.token,
            config.resolution.width,
            config.resolution.height,
            config.framerate,
            config.bitrate
        );

        Ok(())
    }

    async fn get_configuration(&self) -> PlatformResult<VideoEncoderConfig> {
        let configs = self.configurations.read();
        configs
            .first()
            .cloned()
            .ok_or_else(|| PlatformError::HardwareUnavailable("No encoder configured".to_string()))
    }

    async fn set_configuration(&self, config: &VideoEncoderConfig) -> PlatformResult<()> {
        let handle_guard = match config.token.as_str() {
            "VideoEncoder_1" => self.main_handle.read(),
            "VideoEncoder_2" => self.sub_handle.read(),
            _ => {
                return Err(PlatformError::InvalidParameter(format!(
                    "Unknown encoder token: {}",
                    config.token
                )));
            }
        };

        // If the encoder handle exists, apply bitrate change via FFI
        if let Some(handle) = handle_guard.as_ref() {
            let current_config = {
                let configs = self.configurations.read();
                configs.iter().find(|c| c.token == config.token).cloned()
            };

            if let Some(ref current) = current_config {
                if current.bitrate != config.bitrate {
                    let bps = config.bitrate as i32; // kbps (vendor SDK expects kbps)
                    video_encoder_set_rc_internal(handle, bps, self.ffi.as_ref())?;
                    tracing::info!(
                        "Encoder {} bitrate changed: {}kbps → {}kbps",
                        config.token,
                        current.bitrate,
                        config.bitrate
                    );
                }

                // Warn about changes that require encoder restart
                if current.resolution != config.resolution
                    || current.framerate != config.framerate
                    || current.gop_length != config.gop_length
                    || current.encoding != config.encoding
                {
                    tracing::warn!(
                        "Encoder {} configuration change requires restart for: resolution/fps/gop/encoding",
                        config.token
                    );
                }
            }
        }
        drop(handle_guard);

        // Update stored configuration
        let mut configs = self.configurations.write();
        if let Some(cfg) = configs.iter_mut().find(|c| c.token == config.token) {
            *cfg = config.clone();
            Ok(())
        } else {
            Err(PlatformError::InvalidParameter(format!(
                "Unknown encoder token: {}",
                config.token
            )))
        }
    }

    async fn get_configurations(&self) -> PlatformResult<Vec<VideoEncoderConfig>> {
        Ok(self.configurations.read().clone())
    }

    async fn get_options(&self) -> PlatformResult<VideoEncoderOptions> {
        Ok(VideoEncoderOptions {
            resolutions: vec![
                Resolution::new(1920, 1080),
                Resolution::new(1280, 720),
                Resolution::new(640, 360),
            ],
            encodings: vec![VideoEncoding::H264],
            framerate_range: (1, 30),
            bitrate_range: (128, 8000),
            gop_range: (1, 300),
            quality_range: (0, 100),
        })
    }
}

// =============================================================================
// Audio Input Implementation
// =============================================================================

/// Anyka audio input implementation.
struct AnykaAudioInput {
    #[allow(dead_code)]
    ffi: Arc<dyn crate::hal::audio::AudioHalTrait>,
    opened: AtomicBool,
}

impl AnykaAudioInput {
    /// Create a new `AnykaAudioInput`.
    ///
    /// Uses `VendorIpc` to connect to the vendor daemon for vendor library access.
    fn new() -> PlatformResult<Self> {
        let ipc = crate::hal::vendor_ipc::VendorIpc::new().map_err(|e| {
            PlatformError::InitializationFailed(format!(
                "AnykaAudioInput: VendorIpc connection failed: {}",
                e
            ))
        })?;
        tracing::info!("AnykaAudioInput: using VendorIpc for vendor library access");
        Ok(Self {
            ffi: Arc::new(ipc),
            opened: AtomicBool::new(false),
        })
    }

    fn with_ffi(ffi: Arc<dyn crate::hal::audio::AudioHalTrait>) -> Self {
        Self {
            ffi,
            opened: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl AudioInput for AnykaAudioInput {
    async fn open(&self) -> PlatformResult<()> {
        // TODO(kkrzysztofik): Call ak_ai_open() via FFI
        self.opened.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn close(&self) -> PlatformResult<()> {
        // TODO(kkrzysztofik): Call ak_ai_close() via FFI
        self.opened.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn get_configuration(&self) -> PlatformResult<AudioSourceConfig> {
        // TODO(kkrzysztofik): Get actual audio config from Anyka SDK
        Ok(AudioSourceConfig {
            token: "AudioSource_1".to_string(),
            name: "Microphone".to_string(),
            channels: 1,
        })
    }

    async fn get_sources(&self) -> PlatformResult<Vec<AudioSourceConfig>> {
        // TODO(kkrzysztofik): Query actual audio sources
        Ok(vec![AudioSourceConfig {
            token: "AudioSource_1".to_string(),
            name: "Microphone".to_string(),
            channels: 1,
        }])
    }
}

// =============================================================================
// Audio Encoder Implementation
// =============================================================================

/// Anyka audio encoder implementation.
struct AnykaAudioEncoder {
    #[allow(dead_code)]
    ffi: Arc<dyn crate::hal::audio::AudioHalTrait>,
    configurations: RwLock<Vec<AudioEncoderConfig>>,
}

impl AnykaAudioEncoder {
    /// Create a new `AnykaAudioEncoder`.
    ///
    /// Uses `VendorIpc` to connect to the vendor daemon for vendor library access.
    fn new() -> PlatformResult<Self> {
        let ffi: Arc<dyn crate::hal::audio::AudioHalTrait> = {
            let ipc = crate::hal::vendor_ipc::VendorIpc::new().map_err(|e| {
                PlatformError::InitializationFailed(format!(
                    "AnykaAudioEncoder: VendorIpc connection failed: {}",
                    e
                ))
            })?;
            tracing::info!("AnykaAudioEncoder: using VendorIpc for vendor library access");
            Arc::new(ipc)
        };

        Ok(Self::with_ffi(ffi))
    }

    fn with_ffi(ffi: Arc<dyn crate::hal::audio::AudioHalTrait>) -> Self {
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
        // TODO(kkrzysztofik): Call ak_aenc_open() with actual config via FFI
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
        // TODO(kkrzysztofik): Call ak_aenc_set_config() or similar via FFI
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

// =============================================================================
// PTZ Control Implementation
// =============================================================================

/// Anyka PTZ control — delegates to `HardwarePTZControl` which calls the FFI layer.
///
/// The PTZ stub has been replaced with a real hardware implementation
/// (see `hw_ptz.rs`) that controls the physical stepper motors via FFI.
type AnykaPTZControl = super::hw_ptz::HardwarePTZControl;

// =============================================================================
// Imaging Control Implementation
// =============================================================================

/// Anyka imaging control implementation.
struct AnykaImagingControl {
    ffi: Arc<dyn crate::hal::imaging::ImagingHalTrait>,
    settings: RwLock<ImagingSettings>,
    video_encoder: Option<Weak<AnykaVideoEncoder>>,
}

impl AnykaImagingControl {
    /// Create a new `AnykaImagingControl`.
    ///
    /// Uses `VendorIpc` to connect to the vendor daemon for vendor library access.
    fn new() -> PlatformResult<Self> {
        let ffi: Arc<dyn crate::hal::imaging::ImagingHalTrait> = {
            let ipc = crate::hal::vendor_ipc::VendorIpc::new().map_err(|e| {
                PlatformError::InitializationFailed(format!(
                    "AnykaImagingControl: VendorIpc connection failed: {}",
                    e
                ))
            })?;
            tracing::info!("AnykaImagingControl: using VendorIpc for vendor library access");
            Arc::new(ipc)
        };

        Ok(Self::with_ffi(ffi))
    }

    fn with_ffi(ffi: Arc<dyn crate::hal::imaging::ImagingHalTrait>) -> Self {
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

    fn with_ffi_and_video_encoder(
        ffi: Arc<dyn crate::hal::imaging::ImagingHalTrait>,
        video_encoder: Arc<AnykaVideoEncoder>,
    ) -> Self {
        let mut control = Self::with_ffi(ffi);
        control.video_encoder = Some(Arc::downgrade(&video_encoder));
        control
    }

    fn approximately_equal(a: f32, b: f32) -> bool {
        (a - b).abs() <= 0.001
    }

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
        crate::hal::imaging::imaging_set_brightness_internal(
            settings.brightness,
            self.ffi.as_ref(),
        )?;
        crate::hal::imaging::imaging_set_contrast_internal(settings.contrast, self.ffi.as_ref())?;
        crate::hal::imaging::imaging_set_saturation_internal(
            settings.saturation,
            self.ffi.as_ref(),
        )?;
        crate::hal::imaging::imaging_set_sharpness_internal(settings.sharpness, self.ffi.as_ref())?;
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
        crate::hal::imaging::imaging_set_brightness_internal(value, self.ffi.as_ref())?;
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
        crate::hal::imaging::imaging_set_contrast_internal(value, self.ffi.as_ref())?;
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
        crate::hal::imaging::imaging_set_saturation_internal(value, self.ffi.as_ref())?;
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
        crate::hal::imaging::imaging_set_sharpness_internal(value, self.ffi.as_ref())?;
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

// =============================================================================
// Network Info Implementation
// =============================================================================

/// Anyka network information implementation.
///
/// Reads network configuration from the Linux system. Falls back to empty
/// values if system files cannot be read.
struct AnykaNetworkInfo;

impl AnykaNetworkInfo {
    fn new() -> Self {
        Self
    }

    /// Read network interfaces from /sys/class/net and /proc/net/route.
    fn read_interfaces() -> Vec<NetworkInterfaceInfo> {
        use std::fs;
        use std::path::Path;

        let net_dir = Path::new("/sys/class/net");
        let mut interfaces = Vec::new();

        // Try to read available interfaces
        if let Ok(entries) = fs::read_dir(net_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip loopback
                if name == "lo" {
                    continue;
                }

                // Read MAC address
                let mac_path = entry.path().join("address");
                let mac_address = fs::read_to_string(&mac_path)
                    .ok()
                    .map(|s| s.trim().to_uppercase());

                // Read operational state
                let operstate_path = entry.path().join("operstate");
                let enabled = fs::read_to_string(&operstate_path)
                    .map(|s| s.trim() == "up")
                    .unwrap_or(false);

                // Read link speed (in Mbps)
                let speed_path = entry.path().join("speed");
                let link_speed = fs::read_to_string(&speed_path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok());

                // Try to get IP address via ip command output parsing
                // This is a simplified approach - real implementation might use netlink
                let (ipv4_address, ipv4_prefix_length, ipv4_dhcp) = Self::read_interface_ip(&name);

                interfaces.push(NetworkInterfaceInfo {
                    token: name.clone(),
                    name,
                    enabled,
                    ipv4_address,
                    ipv4_prefix_length,
                    ipv4_dhcp,
                    mac_address,
                    link_speed,
                });
            }
        }

        interfaces
    }

    /// Read IP address for an interface.
    fn read_interface_ip(interface: &str) -> (Option<String>, Option<u8>, bool) {
        use std::fs;

        // Try to read from /etc/network/interfaces or similar
        // This is a simplified check - in real embedded Linux, DHCP state
        // might be determined differently

        // Check if DHCP is used (look for dhclient lease)
        let dhcp_lease_path = format!("/var/lib/dhcp/dhclient.{}.leases", interface);
        let from_dhcp = std::path::Path::new(&dhcp_lease_path).exists();

        // Try reading from /proc/net/fib_trie or parsing ip addr output
        // For now, try a simple approach via /proc/net/route
        if let Ok(route_content) = fs::read_to_string("/proc/net/route") {
            for line in route_content.lines().skip(1) {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 8 && fields[0] == interface {
                    // Parse gateway destination to find interface IP
                    // This is a simplified approach
                    if fields[1] == "00000000" {
                        // Default route - interface has connectivity
                        // Would need more sophisticated parsing for actual IP
                    }
                }
            }
        }

        // For a more complete implementation, we'd use netlink or parse
        // /proc/net/fib_trie, but for now return None (empty will be reported)
        (None, None, from_dhcp)
    }

    /// Read DNS configuration from /etc/resolv.conf.
    fn read_dns_config() -> DnsInfo {
        use std::fs;

        let mut dns_info = DnsInfo::default();

        if let Ok(content) = fs::read_to_string("/etc/resolv.conf") {
            for line in content.lines() {
                let line = line.trim();

                // Skip comments
                if line.starts_with('#') {
                    continue;
                }

                if let Some(domain) = line.strip_prefix("search ") {
                    dns_info
                        .search_domains
                        .extend(domain.split_whitespace().map(String::from));
                } else if let Some(domain) = line.strip_prefix("domain ") {
                    dns_info.search_domains.push(domain.trim().to_string());
                } else if let Some(nameserver) = line.strip_prefix("nameserver ") {
                    let ns = nameserver.trim().to_string();
                    // Assume manual unless we detect DHCP
                    dns_info.dns_manual.push(ns);
                }
            }
        }

        // Check if DNS was obtained via DHCP
        // Simple heuristic: if /etc/resolv.conf was modified by dhclient
        if std::path::Path::new("/var/lib/dhcp/dhclient.leases").exists() {
            dns_info.from_dhcp = true;
            // Move servers to dhcp list
            dns_info.dns_from_dhcp = std::mem::take(&mut dns_info.dns_manual);
        }

        dns_info
    }

    /// Read NTP configuration from /etc/ntp.conf or similar.
    fn read_ntp_config() -> NtpInfo {
        let mut ntp_info = NtpInfo::default();

        if let Some(servers) = Self::parse_ntp_conf() {
            ntp_info.ntp_manual = servers;
        } else if let Some(servers) = Self::parse_timesyncd_conf() {
            ntp_info.ntp_manual = servers;
        }

        ntp_info
    }

    /// Parse /etc/ntp.conf file.
    fn parse_ntp_conf() -> Option<Vec<String>> {
        use std::fs;

        let content = fs::read_to_string("/etc/ntp.conf").ok()?;
        let mut servers = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }

            if let Some(server) = line.strip_prefix("server ") {
                let server = server.split_whitespace().next()?.to_string();
                if !server.is_empty() {
                    servers.push(server);
                }
            }
        }

        if servers.is_empty() {
            None
        } else {
            Some(servers)
        }
    }

    /// Parse /etc/systemd/timesyncd.conf file.
    fn parse_timesyncd_conf() -> Option<Vec<String>> {
        use std::fs;

        let content = fs::read_to_string("/etc/systemd/timesyncd.conf").ok()?;
        let mut servers = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if let Some(servers_str) = line.strip_prefix("NTP=") {
                servers.extend(servers_str.split_whitespace().map(String::from));
            }
        }

        if servers.is_empty() {
            None
        } else {
            Some(servers)
        }
    }
}

#[async_trait]
impl NetworkInfo for AnykaNetworkInfo {
    async fn get_network_interfaces(&self) -> PlatformResult<Vec<NetworkInterfaceInfo>> {
        Ok(Self::read_interfaces())
    }

    async fn get_dns_info(&self) -> PlatformResult<DnsInfo> {
        Ok(Self::read_dns_config())
    }

    async fn get_ntp_info(&self) -> PlatformResult<NtpInfo> {
        Ok(Self::read_ntp_config())
    }

    async fn get_network_protocols(&self) -> PlatformResult<Vec<NetworkProtocolInfo>> {
        // Return the protocols this ONVIF server supports
        // These are typically configured at build/runtime, not read from system
        Ok(vec![
            NetworkProtocolInfo {
                name: "HTTP".to_string(),
                enabled: true,
                ports: vec![80],
            },
            NetworkProtocolInfo {
                name: "RTSP".to_string(),
                enabled: true,
                ports: vec![554],
            },
        ])
    }
}

// =============================================================================
// Unit Tests for AnykaVideoInput
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::video::MockVideoHalTrait;
    use crate::hal::{AK_FAILED_I32, AK_SUCCESS_I32};
    use std::ffi::c_void;

    fn video_dev0() -> crate::hal::video_dev_type {
        crate::hal::video_dev_type::Dev0
    }

    /// Create a mock FFI that expects a successful vi_open call.
    fn mock_ffi_with_successful_open() -> MockVideoHalTrait {
        let mut mock = MockVideoHalTrait::new();
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_vi_open()
            .with(mockall::predicate::eq(video_dev0()))
            .times(1)
            .returning(move |_| test_ptr as *mut c_void);
        mock
    }

    #[tokio::test]
    async fn test_video_input_open_success() {
        let mock = mock_ffi_with_successful_open();
        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

        let result = vi.open().await;
        assert!(result.is_ok());
        assert!(vi.opened.load(Ordering::SeqCst));
        assert!(vi.handle.read().is_some());
    }

    #[tokio::test]
    async fn test_video_input_open_already_opened() {
        let mock = mock_ffi_with_successful_open();
        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

        // First open succeeds
        vi.open().await.unwrap();

        // Second open returns ResourceBusy
        let result = vi.open().await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::ResourceBusy(msg)) => {
                assert!(msg.contains("already opened"));
            }
            other => panic!("Expected ResourceBusy, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_video_input_open_hardware_failure() {
        let mut mock = MockVideoHalTrait::new();
        mock.expect_vi_open()
            .with(mockall::predicate::eq(video_dev0()))
            .times(1)
            .returning(|_| std::ptr::null_mut());

        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

        let result = vi.open().await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(_)) => {}
            other => panic!("Expected HardwareUnavailable, got {:?}", other),
        }
        // State should remain false on failure
        assert!(!vi.opened.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_video_input_close_success() {
        let mock = mock_ffi_with_successful_open();
        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

        vi.open().await.unwrap();
        assert!(vi.opened.load(Ordering::SeqCst));

        let result = vi.close().await;
        assert!(result.is_ok());
        assert!(!vi.opened.load(Ordering::SeqCst));
        assert!(vi.handle.read().is_none());
    }

    #[tokio::test]
    async fn test_video_input_close_idempotent() {
        let mock = MockVideoHalTrait::new();
        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

        // Close without ever opening — should succeed (idempotent)
        let result = vi.close().await;
        assert!(result.is_ok());

        // Close again — still idempotent
        let result = vi.close().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_video_input_get_resolution_success() {
        let mut mock = mock_ffi_with_successful_open();
        mock.expect_vi_get_sensor_resolution()
            .times(1)
            .returning(|_, res| {
                unsafe {
                    (*res).width = 1920;
                    (*res).height = 1080;
                    (*res).max_width = 1920;
                    (*res).max_height = 1080;
                }
                AK_SUCCESS_I32
            });

        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
        vi.open().await.unwrap();

        let result = vi.get_resolution().await;
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.width, 1920);
        assert_eq!(res.height, 1080);
    }

    #[tokio::test]
    async fn test_video_input_get_resolution_not_opened() {
        let mock = MockVideoHalTrait::new();
        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

        let result = vi.get_resolution().await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(msg)) => {
                assert!(msg.contains("not opened"));
            }
            other => panic!("Expected HardwareUnavailable, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_video_input_get_resolution_ffi_error() {
        let mut mock = mock_ffi_with_successful_open();
        mock.expect_vi_get_sensor_resolution()
            .times(1)
            .returning(|_, _| AK_FAILED_I32);

        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
        vi.open().await.unwrap();

        let result = vi.get_resolution().await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_get_sensor_resolution"));
            }
            other => panic!("Expected HardwareFailure, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_video_input_get_sources_returns_config() {
        let mock = MockVideoHalTrait::new();
        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

        // get_sources works even when not opened (uses default resolution)
        let result = vi.get_sources().await;
        assert!(result.is_ok());
        let sources = result.unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].token, "VideoSource_1");
        assert_eq!(sources[0].name, "Main Camera");
        assert_eq!(sources[0].resolution.width, 1920);
        assert_eq!(sources[0].resolution.height, 1080);
        assert!((sources[0].max_framerate - 30.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_video_input_get_sources_with_hardware_resolution() {
        let mut mock = mock_ffi_with_successful_open();
        mock.expect_vi_get_sensor_resolution()
            .times(1)
            .returning(|_, res| {
                unsafe {
                    (*res).width = 2560;
                    (*res).height = 1440;
                    (*res).max_width = 2560;
                    (*res).max_height = 1440;
                }
                AK_SUCCESS_I32
            });

        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
        vi.open().await.unwrap();

        let sources = vi.get_sources().await.unwrap();
        assert_eq!(sources[0].resolution.width, 2560);
        assert_eq!(sources[0].resolution.height, 1440);
    }

    #[tokio::test]
    async fn test_video_input_set_channel_attr_success() {
        let mut mock = mock_ffi_with_successful_open();
        mock.expect_vi_get_sensor_resolution()
            .times(1)
            .returning(|_, res| {
                unsafe {
                    (*res).width = 1280;
                    (*res).height = 720;
                    (*res).max_width = 1280;
                    (*res).max_height = 720;
                }
                AK_SUCCESS_I32
            });
        mock.expect_vi_set_channel_attr()
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
        vi.open().await.unwrap();

        let result = vi.set_channel_attr();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_video_input_set_channel_attr_matches_anyka_quirk_default() {
        let mut mock = mock_ffi_with_successful_open();
        mock.expect_vi_get_sensor_resolution()
            .times(1)
            .returning(|_, res| {
                unsafe {
                    (*res).width = 1920;
                    (*res).height = 1080;
                    (*res).max_width = 1920;
                    (*res).max_height = 1080;
                }
                AK_SUCCESS_I32
            });
        mock.expect_vi_set_channel_attr()
            .times(1)
            .returning(|_, attr| {
                unsafe {
                    // Anyka default: main=sensor, sub=640x360.
                    assert_eq!((*attr).res[0].width, 1920);
                    assert_eq!((*attr).res[0].height, 1080);
                    assert_eq!((*attr).res[1].width, 640);
                    assert_eq!((*attr).res[1].height, 360);
                    // libre_anyka_app quirk (via vendor IPC):
                    // main.max_* drives sub-channel limits.
                    assert_eq!((*attr).res[0].max_width, 640);
                    assert_eq!((*attr).res[0].max_height, 360);
                    assert_eq!((*attr).res[1].max_width, 1920);
                    assert_eq!((*attr).res[1].max_height, 1080);
                }
                AK_SUCCESS_I32
            });

        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
        vi.open().await.unwrap();

        let result = vi.set_channel_attr();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_video_input_set_channel_attr_not_opened() {
        let mock = MockVideoHalTrait::new();
        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

        let result = vi.set_channel_attr();
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(msg)) => {
                assert!(msg.contains("not opened"));
            }
            other => panic!("Expected HardwareUnavailable, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_video_input_set_channel_attr_ffi_error() {
        let mut mock = mock_ffi_with_successful_open();
        mock.expect_vi_get_sensor_resolution()
            .times(1)
            .returning(|_, res| {
                unsafe {
                    (*res).width = 1280;
                    (*res).height = 720;
                    (*res).max_width = 1280;
                    (*res).max_height = 720;
                }
                AK_SUCCESS_I32
            });
        mock.expect_vi_set_channel_attr()
            .times(1)
            .returning(|_, _| AK_FAILED_I32);

        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
        vi.open().await.unwrap();

        let result = vi.set_channel_attr();
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_set_channel_attr"));
            }
            other => panic!("Expected HardwareFailure, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_video_input_concurrent_operations() {
        let mut mock = MockVideoHalTrait::new();
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_vi_open()
            .with(mockall::predicate::eq(video_dev0()))
            .times(1)
            .returning(move |_| test_ptr as *mut c_void);
        mock.expect_vi_get_sensor_resolution()
            .times(2)
            .returning(|_, res| {
                unsafe {
                    (*res).width = 1920;
                    (*res).height = 1080;
                    (*res).max_width = 1920;
                    (*res).max_height = 1080;
                }
                AK_SUCCESS_I32
            });

        let vi = Arc::new(AnykaVideoInput::with_ffi(Arc::new(mock), None));
        vi.open().await.unwrap();

        // Spawn concurrent resolution queries
        let vi1 = vi.clone();
        let vi2 = vi.clone();

        let (r1, r2) = tokio::join!(
            tokio::spawn(async move { vi1.get_resolution().await }),
            tokio::spawn(async move { vi2.get_resolution().await }),
        );

        assert!(r1.unwrap().is_ok());
        assert!(r2.unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_video_input_open_close_reopen() {
        let mut mock = MockVideoHalTrait::new();
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_vi_open()
            .with(mockall::predicate::eq(video_dev0()))
            .times(2) // open, close, open again
            .returning(move |_| test_ptr as *mut c_void);

        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

        // Open -> close -> reopen cycle
        vi.open().await.unwrap();
        assert!(vi.opened.load(Ordering::SeqCst));

        vi.close().await.unwrap();
        assert!(!vi.opened.load(Ordering::SeqCst));

        vi.open().await.unwrap();
        assert!(vi.opened.load(Ordering::SeqCst));
    }

    // =========================================================================
    // match_sensor + capture_on Tests
    // =========================================================================

    #[test]
    fn test_match_sensor_with_explicit_existing_path() {
        let mut mock = MockVideoHalTrait::new();
        mock.expect_vi_match_sensor()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        // Use a path that exists — Cargo.toml always exists in the project root
        let vi = AnykaVideoInput::with_ffi(
            Arc::new(mock),
            Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")),
        );
        let result = vi.match_sensor();
        assert!(result.is_ok());
    }

    #[test]
    fn test_match_sensor_explicit_path_not_found_falls_back() {
        // With an explicit path that doesn't exist AND no search paths existing,
        // match_sensor should return an error.
        let mock = MockVideoHalTrait::new();
        let vi = AnykaVideoInput::with_ffi(
            Arc::new(mock),
            Some(PathBuf::from("/nonexistent/isp_config.conf")),
        );
        let result = vi.match_sensor();
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(msg)) => {
                assert!(msg.contains("No ISP sensor config file found"));
            }
            _ => panic!("Expected HardwareUnavailable error"),
        }
    }

    #[test]
    fn test_match_sensor_no_config_found() {
        // No explicit path, no default search paths exist
        let mock = MockVideoHalTrait::new();
        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
        let result = vi.match_sensor();
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(msg)) => {
                assert!(msg.contains("No ISP sensor config file found"));
            }
            _ => panic!("Expected HardwareUnavailable error"),
        }
    }

    #[tokio::test]
    async fn test_capture_on_success() {
        let mut mock = MockVideoHalTrait::new();
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_vi_open()
            .returning(move |_| test_ptr as *mut c_void);
        mock.expect_vi_capture_on()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
        vi.open().await.unwrap();

        let result = vi.capture_on();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_capture_on_retry_runs_capture_off_cleanup() {
        let mut mock = MockVideoHalTrait::new();
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;

        mock.expect_vi_open()
            .times(1)
            .returning(move |_| test_ptr as *mut c_void);

        let mut attempts = 0;
        mock.expect_vi_capture_on().times(2).returning(move |_| {
            attempts += 1;
            if attempts == 1 {
                AK_FAILED_I32
            } else {
                AK_SUCCESS_I32
            }
        });

        mock.expect_vi_capture_off()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
        vi.open().await.unwrap();

        let result = vi.capture_on();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_capture_on_retry_aborts_when_capture_off_cleanup_fails() {
        let mut mock = MockVideoHalTrait::new();
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;

        mock.expect_vi_open()
            .times(1)
            .returning(move |_| test_ptr as *mut c_void);

        mock.expect_vi_capture_on()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        mock.expect_vi_capture_off()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
        vi.open().await.unwrap();

        let result = vi.capture_on();
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("retry cleanup failed"));
            }
            other => panic!("Expected HardwareFailure, got {:?}", other),
        }
    }

    #[test]
    fn test_capture_on_fails_when_not_opened() {
        let mock = MockVideoHalTrait::new();
        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

        let result = vi.capture_on();
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(msg)) => {
                assert!(msg.contains("Video input not opened"));
            }
            _ => panic!("Expected HardwareUnavailable error"),
        }
    }

    // =========================================================================
    // Video Encoder Tests
    // =========================================================================

    use crate::platform::frame::{Frame, FrameCallback, FrameType, StreamId};
    use std::sync::atomic::AtomicU32;

    /// Create a mock FFI that expects a successful venc_open call.
    fn mock_ffi_with_successful_encoder_open() -> MockVideoHalTrait {
        let mut mock = MockVideoHalTrait::new();
        mock.expect_venc_set_cfg_path().returning(|_| 0);
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_open()
            .returning(move |_| test_ptr as *mut c_void);
        mock
    }

    #[tokio::test]
    async fn test_video_encoder_init_main_stream() {
        let mock = mock_ffi_with_successful_encoder_open();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            name: "Main Stream".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 15,
            bitrate: 2000,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            quality: 80,
            ..Default::default()
        };

        let result = encoder.init(&config).await;
        assert!(result.is_ok());
        assert!(encoder.main_handle.read().is_some());
        assert_eq!(*encoder.main_state.read(), EncoderState::Initialized);
    }

    #[tokio::test]
    async fn test_video_encoder_init_sub_stream() {
        let mock = mock_ffi_with_successful_encoder_open();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let config = VideoEncoderConfig {
            token: "VideoEncoder_2".to_string(),
            name: "Sub Stream".to_string(),
            resolution: Resolution::new(640, 360),
            framerate: 15,
            bitrate: 300,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            quality: 70,
            ..Default::default()
        };

        let result = encoder.init(&config).await;
        assert!(result.is_ok());
        assert!(encoder.sub_handle.read().is_some());
        assert_eq!(*encoder.sub_state.read(), EncoderState::Initialized);
    }

    #[tokio::test]
    async fn test_video_encoder_init_dual_streams() {
        let mock = mock_ffi_with_successful_encoder_open();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let main_config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            name: "Main Stream".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 15,
            bitrate: 2000,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            ..Default::default()
        };
        let sub_config = VideoEncoderConfig {
            token: "VideoEncoder_2".to_string(),
            name: "Sub Stream".to_string(),
            resolution: Resolution::new(640, 360),
            framerate: 15,
            bitrate: 300,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            ..Default::default()
        };

        encoder.init(&main_config).await.unwrap();
        encoder.init(&sub_config).await.unwrap();

        assert!(encoder.main_handle.read().is_some());
        assert!(encoder.sub_handle.read().is_some());

        let configs = encoder.get_configurations().await.unwrap();
        assert_eq!(configs.len(), 2);
    }

    #[tokio::test]
    async fn test_video_encoder_init_invalid_token() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let config = VideoEncoderConfig {
            token: "VideoEncoder_99".to_string(),
            ..Default::default()
        };

        let result = encoder.init(&config).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("VideoEncoder_99"));
            }
            other => panic!("Expected InvalidParameter, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_video_encoder_init_ffi_failure() {
        let mut mock = MockVideoHalTrait::new();
        mock.expect_venc_set_cfg_path().returning(|_| 0);
        mock.expect_venc_open().returning(|_| std::ptr::null_mut());

        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 15,
            bitrate: 2000,
            encoding: VideoEncoding::H264,
            ..Default::default()
        };

        let result = encoder.init(&config).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(_)) => {}
            other => panic!("Expected HardwareUnavailable, got {:?}", other),
        }
        // Handle should remain None on failure
        assert!(encoder.main_handle.read().is_none());
    }

    #[tokio::test]
    async fn test_video_encoder_set_configuration_bitrate_change() {
        let mut mock = mock_ffi_with_successful_encoder_open();
        mock.expect_venc_set_rc()
            .withf(|_, bps| *bps == 6000) // kbps passed directly to SDK
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        // Initialize first
        let init_config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            name: "Main Stream".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 15,
            bitrate: 2000,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            ..Default::default()
        };
        encoder.init(&init_config).await.unwrap();

        // Change bitrate
        let new_config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            name: "Main Stream".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 15,
            bitrate: 6000,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            ..Default::default()
        };
        let result = encoder.set_configuration(&new_config).await;
        assert!(result.is_ok());

        // Verify configuration updated
        let config = encoder.get_configuration().await.unwrap();
        assert_eq!(config.bitrate, 6000);
    }

    #[tokio::test]
    async fn test_video_encoder_set_configuration_invalid_token() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let config = VideoEncoderConfig {
            token: "VideoEncoder_99".to_string(),
            ..Default::default()
        };

        let result = encoder.set_configuration(&config).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("VideoEncoder_99"));
            }
            other => panic!("Expected InvalidParameter, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_video_encoder_set_configuration_ffi_error() {
        let mut mock = mock_ffi_with_successful_encoder_open();
        mock.expect_venc_set_rc().returning(|_, _| AK_FAILED_I32);

        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        // Initialize first
        let init_config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            name: "Main Stream".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 15,
            bitrate: 2000,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            ..Default::default()
        };
        encoder.init(&init_config).await.unwrap();

        // Attempt bitrate change that fails at FFI level
        let new_config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            name: "Main Stream".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 15,
            bitrate: 6000,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            ..Default::default()
        };
        let result = encoder.set_configuration(&new_config).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(_)) => {}
            other => panic!("Expected HardwareFailure, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_video_encoder_get_configuration() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let config = encoder.get_configuration().await.unwrap();
        assert_eq!(config.token, "VideoEncoder_1");
        assert_eq!(config.resolution.width, 1280);
        assert_eq!(config.resolution.height, 720);
        assert_eq!(config.framerate, 15);
        assert_eq!(config.bitrate, 2000);
    }

    #[tokio::test]
    async fn test_video_encoder_get_configurations() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let configs = encoder.get_configurations().await.unwrap();
        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].token, "VideoEncoder_1");
        assert_eq!(configs[1].token, "VideoEncoder_2");
    }

    #[tokio::test]
    async fn test_video_encoder_get_options() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let options = encoder.get_options().await.unwrap();
        assert_eq!(options.resolutions.len(), 3);
        assert_eq!(options.framerate_range, (1, 30));
        assert_eq!(options.bitrate_range, (128, 8000));
    }

    #[test]
    fn test_config_to_encode_param_main() {
        let config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 15,
            bitrate: 2000,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            ..Default::default()
        };

        let param =
            AnykaVideoEncoder::config_to_encode_param(&config, encode_use_chn::ENCODE_MAIN_CHN);
        assert_eq!(param.width, 1280);
        assert_eq!(param.height, 720);
        assert_eq!(param.fps, 15);
        assert_eq!(param.bps, 2000); // kbps passed directly
        assert_eq!(param.goplen, 30);
        assert_eq!(param.minqp, 20);
        assert_eq!(param.use_chn, encode_use_chn::ENCODE_MAIN_CHN);
        assert_eq!(param.enc_grp, encode_group_type::ENCODE_MAINCHN_NET);
        assert_eq!(param.enc_out_type, encode_output_type::H264_ENC_TYPE);
    }

    #[test]
    fn test_config_to_encode_param_sub() {
        let config = VideoEncoderConfig {
            token: "VideoEncoder_2".to_string(),
            resolution: Resolution::new(640, 360),
            framerate: 15,
            bitrate: 300,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            ..Default::default()
        };

        let param =
            AnykaVideoEncoder::config_to_encode_param(&config, encode_use_chn::ENCODE_SUB_CHN);
        assert_eq!(param.width, 640);
        assert_eq!(param.height, 360);
        assert_eq!(param.fps, 15);
        assert_eq!(param.bps, 300); // kbps passed directly
        assert_eq!(param.goplen, 30);
        assert_eq!(param.use_chn, encode_use_chn::ENCODE_SUB_CHN);
        assert_eq!(param.enc_grp, encode_group_type::ENCODE_SUBCHN_NET);
    }

    #[test]
    fn test_config_to_encode_param_h265() {
        let config = VideoEncoderConfig {
            encoding: VideoEncoding::H265,
            ..Default::default()
        };

        let param =
            AnykaVideoEncoder::config_to_encode_param(&config, encode_use_chn::ENCODE_MAIN_CHN);
        assert_eq!(param.enc_out_type, encode_output_type::HEVC_ENC_TYPE);
    }

    #[test]
    fn test_config_to_encode_param_vbr_mode() {
        let config = VideoEncoderConfig {
            bitrate_mode: crate::platform::BitrateMode::Vbr,
            ..Default::default()
        };

        let param =
            AnykaVideoEncoder::config_to_encode_param(&config, encode_use_chn::ENCODE_MAIN_CHN);
        assert_eq!(param.br_mode, bitrate_ctrl_mode::BR_MODE_VBR);
    }

    // =========================================================================
    // Frame Callback Tests
    // =========================================================================

    /// Test callback that counts invocations.
    struct CountingCallback {
        count: AtomicU32,
    }

    impl CountingCallback {
        fn new() -> Self {
            Self {
                count: AtomicU32::new(0),
            }
        }

        fn count(&self) -> u32 {
            self.count.load(Ordering::SeqCst)
        }
    }

    impl FrameCallback for CountingCallback {
        fn on_frame(&self, _frame: &Frame) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Test callback that deliberately panics.
    struct PanickingCallback;

    impl FrameCallback for PanickingCallback {
        fn on_frame(&self, _frame: &Frame) {
            panic!("Intentional panic for testing");
        }
    }

    /// Test owned callback that counts invocations.
    struct CountingOwnedCallback {
        count: AtomicU64,
        last_size: AtomicU64,
    }

    impl CountingOwnedCallback {
        fn new() -> Self {
            Self {
                count: AtomicU64::new(0),
                last_size: AtomicU64::new(0),
            }
        }

        fn call_count(&self) -> u64 {
            self.count.load(Ordering::SeqCst)
        }

        fn last_size(&self) -> u64 {
            self.last_size.load(Ordering::SeqCst)
        }
    }

    impl OwnedFrameCallback for CountingOwnedCallback {
        fn on_owned_frame(&self, frame: OwnedFrame) {
            self.count.fetch_add(1, Ordering::SeqCst);
            self.last_size
                .store(frame.data.len() as u64, Ordering::SeqCst);
        }
    }

    /// Test owned callback that deliberately panics.
    struct PanickingOwnedCallback;

    impl OwnedFrameCallback for PanickingOwnedCallback {
        fn on_owned_frame(&self, _frame: OwnedFrame) {
            panic!("intentional panic in owned callback test");
        }
    }

    fn make_test_frame() -> Frame {
        static TEST_DATA: [u8; 4] = [0x00, 0x00, 0x00, 0x01];
        Frame {
            data: TEST_DATA.as_ptr(),
            size: 4,
            timestamp: 1_000_000,
            frame_type: FrameType::VideoIFrame,
            stream_id: StreamId::VideoMain,
        }
    }

    #[test]
    fn test_callback_registration() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let cb = Arc::new(CountingCallback::new());
        let id = encoder.register_frame_callback(cb);
        assert!(id > 0);
        assert_eq!(encoder.callbacks.read().len(), 1);
    }

    #[test]
    fn test_callback_unregistration() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let cb = Arc::new(CountingCallback::new());
        let id = encoder.register_frame_callback(cb);
        assert_eq!(encoder.callbacks.read().len(), 1);

        let removed = encoder.unregister_frame_callback(id);
        assert!(removed);
        assert_eq!(encoder.callbacks.read().len(), 0);
    }

    #[test]
    fn test_callback_unregister_nonexistent() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let removed = encoder.unregister_frame_callback(999);
        assert!(!removed);
    }

    #[test]
    fn test_callback_invocation() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let cb = Arc::new(CountingCallback::new());
        let cb_ref = Arc::clone(&cb);
        encoder.register_frame_callback(cb);

        let frame = make_test_frame();
        encoder.invoke_callbacks(&frame);

        assert_eq!(cb_ref.count(), 1);
    }

    #[test]
    fn test_multiple_callbacks_invocation() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let cb1 = Arc::new(CountingCallback::new());
        let cb2 = Arc::new(CountingCallback::new());
        let cb1_ref = Arc::clone(&cb1);
        let cb2_ref = Arc::clone(&cb2);

        encoder.register_frame_callback(cb1);
        encoder.register_frame_callback(cb2);

        let frame = make_test_frame();
        encoder.invoke_callbacks(&frame);

        assert_eq!(cb1_ref.count(), 1);
        assert_eq!(cb2_ref.count(), 1);
    }

    #[test]
    fn test_callback_panic_isolation() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        // Register a panicking callback
        let panicking = Arc::new(PanickingCallback);
        encoder.register_frame_callback(panicking);

        // Register a normal callback
        let normal = Arc::new(CountingCallback::new());
        let normal_ref = Arc::clone(&normal);
        encoder.register_frame_callback(normal);

        let frame = make_test_frame();

        // invoke_callbacks should not panic even though one callback does
        encoder.invoke_callbacks(&frame);

        // Panicked callback should be removed
        assert_eq!(encoder.callbacks.read().len(), 1);

        // Normal callback may or may not have been invoked depending on
        // iteration order (HashMap is unordered), but encoder survived
        let _ = normal_ref.count();
    }

    #[test]
    fn test_callback_panicked_removed_on_second_invocation() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let panicking = Arc::new(PanickingCallback);
        encoder.register_frame_callback(panicking);

        let frame = make_test_frame();

        // First invocation removes panicked callback
        encoder.invoke_callbacks(&frame);
        assert_eq!(encoder.callbacks.read().len(), 0);

        // Second invocation with no callbacks is fine
        encoder.invoke_callbacks(&frame);
    }

    // ===== Owned Frame Callback Tests =====

    /// Helper to register an owned callback directly in tests (bypasses AnykaVideoEncoder).
    fn register_owned_callback_for_test(
        callbacks: &Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>>,
        callback: Arc<dyn OwnedFrameCallback>,
    ) -> CallbackId {
        static NEXT_ID: portable_atomic::AtomicU64 = portable_atomic::AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, portable_atomic::Ordering::SeqCst);
        callbacks.write().insert(id, callback);
        id
    }

    #[test]
    fn test_register_owned_frame_callback() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let cb = Arc::new(CountingOwnedCallback::new());
        let id = encoder.register_owned_frame_callback(cb);
        assert!(id > 0);
        assert_eq!(encoder.owned_callbacks.read().len(), 1);
    }

    #[test]
    fn test_unregister_owned_frame_callback() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let cb = Arc::new(CountingOwnedCallback::new());
        let id = encoder.register_owned_frame_callback(cb);
        assert_eq!(encoder.owned_callbacks.read().len(), 1);

        let removed = encoder.unregister_owned_frame_callback(id);
        assert!(removed);
        assert_eq!(encoder.owned_callbacks.read().len(), 0);
    }

    #[test]
    fn test_invoke_owned_callbacks_from_map_empty() {
        use parking_lot::RwLock;
        use std::collections::HashMap;

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let owned_frame = OwnedFrame {
            data: bytes::BytesMut::from(&b"test data"[..]),
            timestamp: 1000,
            frame_type: FrameType::VideoIFrame,
            stream_id: StreamId::VideoMain,
        };

        // Should return the frame since there are no callbacks
        let result = invoke_owned_callbacks_from_map(&owned_callbacks, owned_frame);
        assert!(result.is_some());
    }

    #[test]
    fn test_invoke_owned_callbacks_from_map_single() {
        use parking_lot::RwLock;
        use std::collections::HashMap;

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let cb = Arc::new(CountingOwnedCallback::new());
        let cb_ref = Arc::clone(&cb);
        let _id = register_owned_callback_for_test(&owned_callbacks, cb);

        let owned_frame = OwnedFrame {
            data: bytes::BytesMut::from(&b"test data for single callback"[..]),
            timestamp: 1000,
            frame_type: FrameType::VideoIFrame,
            stream_id: StreamId::VideoMain,
        };

        // Should consume the frame (return None)
        let result = invoke_owned_callbacks_from_map(&owned_callbacks, owned_frame);
        assert!(result.is_none());
        assert_eq!(cb_ref.call_count(), 1);
    }

    #[test]
    fn test_invoke_owned_callbacks_from_map_multiple() {
        use parking_lot::RwLock;
        use std::collections::HashMap;

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let cb1 = Arc::new(CountingOwnedCallback::new());
        let cb2 = Arc::new(CountingOwnedCallback::new());
        let cb1_ref = Arc::clone(&cb1);
        let cb2_ref = Arc::clone(&cb2);
        register_owned_callback_for_test(&owned_callbacks, cb1);
        register_owned_callback_for_test(&owned_callbacks, cb2);

        let owned_frame = OwnedFrame {
            data: bytes::BytesMut::from(&b"test data for multiple callbacks"[..]),
            timestamp: 1000,
            frame_type: FrameType::VideoIFrame,
            stream_id: StreamId::VideoMain,
        };

        // Should consume the frame (return None)
        let result = invoke_owned_callbacks_from_map(&owned_callbacks, owned_frame);
        assert!(result.is_none());
        assert_eq!(cb1_ref.call_count(), 1);
        assert_eq!(cb2_ref.call_count(), 1);
    }

    #[test]
    fn test_invoke_owned_callbacks_from_map_panic_recovery() {
        use parking_lot::RwLock;
        use std::collections::HashMap;

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Register a panicking callback
        let panicking = Arc::new(PanickingOwnedCallback);
        register_owned_callback_for_test(&owned_callbacks, panicking);

        // Register a normal callback
        let normal = Arc::new(CountingOwnedCallback::new());
        let normal_ref = Arc::clone(&normal);
        register_owned_callback_for_test(&owned_callbacks, normal);

        let owned_frame = OwnedFrame {
            data: bytes::BytesMut::from(&b"test panic recovery"[..]),
            timestamp: 1000,
            frame_type: FrameType::VideoIFrame,
            stream_id: StreamId::VideoMain,
        };

        // Should not panic - panic should be caught
        let result = invoke_owned_callbacks_from_map(&owned_callbacks, owned_frame);
        assert!(result.is_none());

        // Panicking callback should be removed, normal should remain
        assert_eq!(owned_callbacks.read().len(), 1);

        // Normal callback may or may not have been invoked depending on iteration order
        let _ = normal_ref.call_count();
    }

    #[tokio::test]
    async fn test_video_encoder_request_idr_main() {
        let mut mock = mock_ffi_with_successful_encoder_open();
        mock.expect_venc_set_iframe()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        // Initialize main encoder
        let config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 15,
            bitrate: 2000,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            ..Default::default()
        };
        encoder.init(&config).await.unwrap();

        let result = encoder.request_idr_frame(true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_encoder_request_idr_not_initialized() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let result = encoder.request_idr_frame(true);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(msg)) => {
                assert!(msg.contains("main encoder not initialized"));
            }
            other => panic!("Expected HardwareUnavailable, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_video_encoder_concurrent_config_access() {
        let mock = MockVideoHalTrait::new();
        let encoder = Arc::new(AnykaVideoEncoder::with_ffi(Arc::new(mock)));

        let e1 = encoder.clone();
        let e2 = encoder.clone();

        let (r1, r2) = tokio::join!(
            tokio::spawn(async move { e1.get_configurations().await }),
            tokio::spawn(async move { e2.get_configurations().await }),
        );

        assert!(r1.unwrap().is_ok());
        assert!(r2.unwrap().is_ok());
    }

    #[test]
    fn test_encoder_active_frames_accessible() {
        let mock = MockVideoHalTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));
        let af = encoder.active_frames();
        assert_eq!(af.active_count(), 0);
    }

    // =========================================================================
    // VideoStreamHandle Tests
    // =========================================================================

    use crate::hal::video::VideoStreamHandle;

    #[test]
    fn test_video_stream_handle_creation_success() {
        let mut mock = MockVideoHalTrait::new();
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;

        mock.expect_venc_request_stream()
            .times(1)
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let vi_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();

        let result = VideoStreamHandle::new(vi_handle, venc_handle, Arc::new(mock));
        assert!(result.is_ok());
        // Drop triggers venc_cancel_stream
    }

    #[test]
    fn test_video_stream_handle_creation_null_returns_error() {
        let mut mock = MockVideoHalTrait::new();

        mock.expect_venc_request_stream()
            .times(1)
            .returning(|_, _| std::ptr::null_mut());

        let vi_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();

        let result = VideoStreamHandle::new(vi_handle, venc_handle, Arc::new(mock));
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_venc_request_stream"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_stream_handle_drop_calls_cancel() {
        let mut mock = MockVideoHalTrait::new();
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;

        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .withf(move |handle| *handle == test_ptr as *mut c_void)
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let vi_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();

        let sh = VideoStreamHandle::new(vi_handle, venc_handle, Arc::new(mock)).unwrap();
        drop(sh); // Should call venc_cancel_stream exactly once
    }

    #[test]
    fn test_video_stream_handle_explicit_cancel_is_idempotent() {
        let mut mock = MockVideoHalTrait::new();
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;

        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .withf(move |handle| *handle == test_ptr as *mut c_void)
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let vi_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();

        let sh = VideoStreamHandle::new(vi_handle, venc_handle, Arc::new(mock)).unwrap();
        assert!(sh.cancel());
        assert!(sh.cancel()); // Second cancel is a no-op success
        drop(sh); // Drop must not invoke cancel again
    }

    // =========================================================================
    // Frame Type Conversion Tests
    // =========================================================================

    #[cfg(use_stubs)]
    #[test]
    fn test_frame_type_conversion_i_frame() {
        use crate::hal::VideoFrameType;
        assert_eq!(
            sdk_frame_type_to_frame_type(VideoFrameType::FrameTypeI),
            FrameType::VideoIFrame
        );
    }

    #[cfg(use_stubs)]
    #[test]
    fn test_frame_type_conversion_pi_frame() {
        use crate::hal::VideoFrameType;
        assert_eq!(
            sdk_frame_type_to_frame_type(VideoFrameType::FrameTypePi),
            FrameType::VideoPiFrame
        );
    }

    #[cfg(use_stubs)]
    #[test]
    fn test_frame_type_conversion_p_frame() {
        use crate::hal::VideoFrameType;
        assert_eq!(
            sdk_frame_type_to_frame_type(VideoFrameType::FrameTypeP),
            FrameType::VideoPFrame
        );
    }

    #[cfg(use_stubs)]
    #[test]
    fn test_frame_type_conversion_b_frame() {
        use crate::hal::VideoFrameType;
        assert_eq!(
            sdk_frame_type_to_frame_type(VideoFrameType::FrameTypeB),
            FrameType::VideoBFrame
        );
    }

    #[test]
    fn test_push_mode_enabled_main_only() {
        assert!(push_mode_enabled(false));
    }

    #[test]
    fn test_push_mode_disabled_when_sub_stream_present() {
        assert!(!push_mode_enabled(true));
    }

    #[test]
    fn test_push_mode_transient_error_classification() {
        assert!(is_push_mode_transient_error(&PlatformError::Timeout));
        assert!(is_push_mode_transient_error(&PlatformError::ResourceBusy(
            "frame dropped".to_string()
        )));
        assert!(!is_push_mode_transient_error(
            &PlatformError::HardwareFailure("socket disconnected".to_string())
        ));
    }

    // =========================================================================
    // Timestamp Conversion Tests
    // =========================================================================

    #[test]
    fn test_timestamp_conversion_ms_to_us() {
        // SDK timestamps are in ms, Frame uses µs
        let sdk_ts_ms: u64 = 12345;
        let frame_ts_us = sdk_ts_ms.wrapping_mul(1000);
        assert_eq!(frame_ts_us, 12_345_000);
    }

    #[test]
    fn test_timestamp_conversion_zero() {
        let sdk_ts_ms: u64 = 0;
        let frame_ts_us = sdk_ts_ms.wrapping_mul(1000);
        assert_eq!(frame_ts_us, 0);
    }

    #[test]
    fn test_timestamp_conversion_wrapping() {
        // Verify wrapping_mul won't panic on large values
        let sdk_ts_ms: u64 = u64::MAX;
        let frame_ts_us = sdk_ts_ms.wrapping_mul(1000);
        // Just verify it doesn't panic; exact value isn't important
        let _ = frame_ts_us;
    }

    // =========================================================================
    // Frame Read Loop Tests
    // =========================================================================

    #[test]
    #[cfg(use_stubs)]
    fn test_frame_read_loop_invokes_callbacks() {
        use crate::hal::VideoFrameType;
        use std::sync::atomic::AtomicUsize;

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = Arc::clone(&call_count);

        struct CountingCallback(Arc<AtomicUsize>);
        impl FrameCallback for CountingCallback {
            fn on_frame(&self, frame: &Frame) {
                assert_eq!(frame.stream_id, StreamId::VideoMain);
                assert_eq!(frame.frame_type, FrameType::VideoIFrame);
                assert_eq!(frame.size, 100);
                // SDK ts=5000ms → Frame ts=5_000_000µs
                assert_eq!(frame.timestamp, 5_000_000);
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut mock = MockVideoHalTrait::new();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);

        // Frame data buffer
        let frame_data: Vec<u8> = vec![0xAB; 100];
        let frame_data_ptr = frame_data.as_ptr() as usize;
        let get_stream_ptr = Arc::new(AtomicUsize::new(0));
        let get_stream_ptr_clone = Arc::clone(&get_stream_ptr);

        // Drain-loop pattern: first call returns a frame (sets stop signal),
        // second call returns failure to break inner drain loop.
        let call_idx = Arc::new(AtomicUsize::new(0));
        let call_idx_clone = Arc::clone(&call_idx);
        mock.expect_venc_get_stream()
            .returning(move |_, stream_ptr| {
                get_stream_ptr_clone.store(stream_ptr as usize, Ordering::SeqCst);
                let idx = call_idx_clone.fetch_add(1, Ordering::SeqCst);
                if idx == 0 {
                    // First call: return a frame
                    unsafe {
                        let stream = &mut *stream_ptr;
                        stream.data = frame_data_ptr as *mut u8;
                        stream.len = 100;
                        stream.ts = 5000; // ms
                        stream.seq_no = 1;
                        stream.frame_type = VideoFrameType::FrameTypeI;
                    }
                    stop_clone.store(true, Ordering::SeqCst);
                    AK_SUCCESS_I32
                } else {
                    // Subsequent calls: no more frames (breaks drain loop)
                    crate::hal::AK_FAILED_I32
                }
            });
        mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);

        mock.expect_venc_release_stream()
            .times(1)
            .returning(move |_, stream_ptr| {
                assert_eq!(
                    stream_ptr as usize,
                    get_stream_ptr.load(Ordering::SeqCst),
                    "release must use same video_stream pointer as get"
                );
                AK_SUCCESS_I32
            });

        // Stream handle creation + cancel
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        callbacks
            .write()
            .insert(1, Arc::new(CountingCallback(call_count_clone)));

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        unified_frame_read_loop(
            sh,
            None,
            ffi,
            callbacks,
            owned_callbacks,
            stop,
            Arc::new(StreamHealthCounters::default()),
            None,
            None,
            None,
            None,
        );

        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        // Keep frame_data alive until after the loop
        drop(frame_data);
    }

    #[test]
    fn test_frame_read_loop_handles_no_data_and_retries() {
        let mut mock = MockVideoHalTrait::new();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);
        let error_count = Arc::new(AtomicU32::new(0));
        let error_count_clone = Arc::clone(&error_count);

        // Return no-data errors, then signal stop after 2 errors.
        // In the drain-loop pattern, get_stream returning non-success breaks
        // the inner loop, then the outer loop sleeps and retries.
        mock.expect_venc_get_stream().returning(move |_, _| {
            let count = error_count_clone.fetch_add(1, Ordering::SeqCst);
            if count >= 1 {
                stop_clone.store(true, Ordering::SeqCst);
            }
            crate::hal::AK_FAILED_I32
        });
        // No-data errors don't call get_error_str (only non-no-data errors do)
        mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);

        // No release_stream calls expected (errors don't produce frames)
        // Stream handle
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        unified_frame_read_loop(
            sh,
            None,
            ffi,
            callbacks,
            owned_callbacks,
            stop,
            Arc::new(StreamHealthCounters::default()),
            None,
            None,
            None,
            None,
        );

        // Should have retried at least twice
        assert!(error_count.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn test_stop_signal_terminates_loop() {
        let mut mock = MockVideoHalTrait::new();
        let stop = Arc::new(AtomicBool::new(true)); // Pre-set stop

        // get_stream should never be called since stop is already set
        // (but allow 0 calls in case of timing)
        mock.expect_venc_get_stream()
            .times(0)
            .returning(|_, _| AK_FAILED_I32);

        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Should return immediately
        unified_frame_read_loop(
            sh,
            None,
            ffi,
            callbacks,
            owned_callbacks,
            stop,
            Arc::new(StreamHealthCounters::default()),
            None,
            None,
            None,
            None,
        );
    }

    #[tokio::test]
    async fn test_start_stop_streaming_lifecycle() {
        let mut mock = MockVideoHalTrait::new();

        // Encoder open expectations
        mock.expect_venc_set_cfg_path().returning(|_| 0);
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_open()
            .returning(move |_| test_ptr as *mut c_void);

        // Stream lifecycle expectations
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_set_iframe()
            .times(1..)
            .returning(|_| AK_SUCCESS_I32);
        mock.expect_venc_get_stream()
            .returning(|_, _| AK_FAILED_I32); // No frames in test
        mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);
        let encoder = Arc::new(AnykaVideoEncoder::with_ffi(Arc::new(mock)));
        // Keep the reader loop on active poll cadence in this lifecycle test
        // so stop/join timing is deterministic under host CI scheduling.
        let _callback_id = encoder.register_frame_callback(Arc::new(CountingCallback::new()));

        // Initialize main encoder
        let config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            name: "Main Stream".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 15,
            bitrate: 2000,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            quality: 80,
            ..Default::default()
        };
        encoder.init(&config).await.unwrap();

        // Create a dummy VI handle for testing
        let vi_handle = Arc::new(crate::hal::VideoInputHandle::test_handle());
        let main_enc = encoder.main_handle.read().clone().unwrap();

        // Start streaming
        let result = encoder.start_streaming(&vi_handle, &main_enc, None);
        assert!(result.is_ok());

        // Verify threads are running
        assert!(encoder.main_stream_handle.read().is_some());
        assert!(encoder.read_thread.read().is_some());

        // Stop streaming
        let stop_result = encoder.stop_streaming();
        assert!(
            stop_result.is_ok(),
            "stop_streaming failed: {:?}",
            stop_result
        );

        // Verify cleanup
        assert!(encoder.main_stream_handle.read().is_none());
        assert!(encoder.read_thread.read().is_none());
    }

    #[test]
    fn test_stop_streaming_join_timeout_after_cancel_marks_unsafe() {
        let mut mock = MockVideoHalTrait::new();
        let stream_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .times(1)
            .returning(move |_, _| stream_ptr as *mut c_void);
        // Cancel is called first in the new ordering.
        mock.expect_venc_cancel_stream()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let encoder = AnykaVideoEncoder::with_ffi(Arc::clone(&ffi));
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());
        *encoder.main_stream_handle.write() = Some(sh);

        // Simulate a reader thread stuck in kernel I/O that doesn't unblock
        // even after cancel (e.g. blocked in a kernel ioctl).
        let blocked = std::thread::spawn(move || {
            // Force the thread to outlive STREAM_THREAD_JOIN_TIMEOUT in tests.
            std::thread::sleep(Duration::from_millis(200));
        });
        *encoder.read_thread.write() = Some(blocked);

        let result = encoder.stop_streaming();
        assert!(result.is_err());
        assert!(encoder.requires_hard_shutdown());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("unsafe teardown required"));
                assert!(msg.contains("join timeout"));
            }
            other => panic!("Expected unsafe teardown HardwareFailure, got {:?}", other),
        }
    }

    #[test]
    fn test_stop_streaming_cancel_failure_still_attempts_second_channel() {
        let mut mock = MockVideoHalTrait::new();
        let mut request_seq = mockall::Sequence::new();
        let stream_ptr_a = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        let stream_ptr_b = (stream_ptr_a.wrapping_add(16)) as *mut c_void as usize;

        mock.expect_venc_request_stream()
            .times(1)
            .in_sequence(&mut request_seq)
            .returning(move |_, _| stream_ptr_a as *mut c_void);
        mock.expect_venc_request_stream()
            .times(1)
            .in_sequence(&mut request_seq)
            .returning(move |_, _| stream_ptr_b as *mut c_void);

        // First channel cancel fails; sub-channel cancel should still be attempted
        // to avoid leaving SDK threads running.
        mock.expect_venc_cancel_stream()
            .times(1)
            .returning(|_| AK_FAILED_I32);
        mock.expect_venc_cancel_stream()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let encoder = AnykaVideoEncoder::with_ffi(Arc::clone(&ffi));
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr_main = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr_sub = (venc_ptr_main as usize).wrapping_add(32) as *mut c_void;
        let main = Arc::new(
            VideoStreamHandle::new(vi_ptr, venc_ptr_main, Arc::clone(&ffi)).expect("main stream"),
        );
        let sub = Arc::new(
            VideoStreamHandle::new(vi_ptr, venc_ptr_sub, Arc::clone(&ffi)).expect("sub stream"),
        );
        *encoder.main_stream_handle.write() = Some(main);
        *encoder.sub_stream_handle.write() = Some(sub);

        let result = encoder.stop_streaming();
        assert!(result.is_err());
        assert!(encoder.requires_hard_shutdown());
    }

    #[test]
    fn test_stop_streaming_cancel_timeout_marks_unsafe_and_attempts_both() {
        let mut mock = MockVideoHalTrait::new();
        let stream_ptr_a = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        let stream_ptr_b = (stream_ptr_a.wrapping_add(16)) as *mut c_void as usize;
        mock.expect_venc_request_stream()
            .times(1)
            .returning(move |_, _| stream_ptr_a as *mut c_void);
        mock.expect_venc_request_stream()
            .times(1)
            .returning(move |_, _| stream_ptr_b as *mut c_void);

        // Main cancel blocks past STREAM_CANCEL_TIMEOUT (20ms in tests).
        mock.expect_venc_cancel_stream().times(1).returning(|_| {
            std::thread::sleep(Duration::from_millis(200));
            AK_SUCCESS_I32
        });
        // Sub cancel still attempted.
        mock.expect_venc_cancel_stream()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let encoder = AnykaVideoEncoder::with_ffi(Arc::clone(&ffi));
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr_main = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr_sub = (venc_ptr_main as usize).wrapping_add(32) as *mut c_void;
        let main = Arc::new(
            VideoStreamHandle::new(vi_ptr, venc_ptr_main, Arc::clone(&ffi)).expect("main stream"),
        );
        let sub = Arc::new(
            VideoStreamHandle::new(vi_ptr, venc_ptr_sub, Arc::clone(&ffi)).expect("sub stream"),
        );
        *encoder.main_stream_handle.write() = Some(main);
        *encoder.sub_stream_handle.write() = Some(sub);

        let result = encoder.stop_streaming();
        assert!(result.is_err());
        assert!(encoder.requires_hard_shutdown());
    }

    #[tokio::test]
    async fn test_shutdown_order_is_cancel_then_join_then_close() {
        let mut mock = MockVideoHalTrait::new();
        let mut seq = mockall::Sequence::new();
        let enc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        let stream_ptr = (enc_ptr.wrapping_add(64)) as *mut c_void as usize;

        mock.expect_venc_set_cfg_path().times(1).returning(|_| 0);
        mock.expect_venc_open()
            .times(1)
            .returning(move |_| enc_ptr as *mut c_void);
        mock.expect_venc_request_stream()
            .times(1)
            .returning(move |_, _| stream_ptr as *mut c_void);

        // Cancel must happen before close (cancel-first pattern).
        mock.expect_venc_cancel_stream()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| AK_SUCCESS_I32);
        mock.expect_venc_close()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| AK_SUCCESS_I32);

        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));
        let config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            name: "Main Stream".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 15,
            bitrate: 2000,
            encoding: VideoEncoding::H264,
            gop_length: 30,
            quality: 80,
            ..Default::default()
        };
        encoder.init(&config).await.unwrap();

        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = encoder
            .main_handle
            .read()
            .as_ref()
            .expect("main encoder")
            .as_ptr();
        let sh = Arc::new(
            VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&encoder.ffi))
                .expect("stream handle"),
        );
        *encoder.main_stream_handle.write() = Some(sh);

        // Reader thread exits on stop_signal — simulates a non-stuck reader.
        let stop = Arc::clone(&encoder.stop_signal);
        let joinable_reader = std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(1));
            }
        });
        *encoder.read_thread.write() = Some(joinable_reader);

        encoder.stop_streaming().expect("stop_streaming");
        encoder.close_all_encoders().expect("close_all_encoders");
    }

    #[test]
    fn test_stop_streaming_cancel_unblocks_reader_thread() {
        // Verify that the cancel-first pattern allows a reader thread that
        // exits on stop_signal to join successfully.
        let mut mock = MockVideoHalTrait::new();
        let stream_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .times(1)
            .returning(move |_, _| stream_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let encoder = AnykaVideoEncoder::with_ffi(Arc::clone(&ffi));
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());
        *encoder.main_stream_handle.write() = Some(sh);

        // Reader thread waits for stop_signal, then exits (simulates a thread
        // that would be stuck in get_stream until cancel fires).
        let stop = Arc::clone(&encoder.stop_signal);
        let reader = std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(1));
            }
        });
        *encoder.read_thread.write() = Some(reader);

        let result = encoder.stop_streaming();
        assert!(result.is_ok(), "cancel-first should allow clean join");
        assert!(!encoder.requires_hard_shutdown());
        assert!(encoder.main_stream_handle.read().is_none());
    }

    #[test]
    fn test_frame_read_loop_no_initial_delay() {
        // Verify frame_read_loop exits quickly when stop signal is pre-set
        // (no 100ms initial delay — removed in drain-loop refactor).
        let mut mock = MockVideoHalTrait::new();
        let stop = Arc::new(AtomicBool::new(true)); // Pre-set stop

        mock.expect_venc_get_stream()
            .times(0)
            .returning(|_, _| AK_FAILED_I32);

        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let start = std::time::Instant::now();
        unified_frame_read_loop(
            sh,
            None,
            ffi,
            callbacks,
            owned_callbacks,
            stop,
            Arc::new(StreamHealthCounters::default()),
            None,
            None,
            None,
            None,
        );
        let elapsed = start.elapsed();

        // Should exit very quickly — no initial delay
        assert!(
            elapsed < Duration::from_millis(50),
            "Expected fast exit with pre-set stop, got {:?}",
            elapsed
        );
    }

    #[test]
    fn test_drain_loop_adaptive_sleep() {
        // Verify the drain-loop uses adaptive sleep (50ms default) between cycles.
        // With no-data errors, each cycle backs off: 50 + 100 + 200 + 200 = 550ms (capped at 4x).
        let mut mock = MockVideoHalTrait::new();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);
        let error_count = Arc::new(AtomicU32::new(0));
        let error_count_clone = Arc::clone(&error_count);

        // Return 4 no-data errors, then stop
        mock.expect_venc_get_stream().returning(move |_, _| {
            let count = error_count_clone.fetch_add(1, Ordering::SeqCst);
            if count >= 3 {
                stop_clone.store(true, Ordering::SeqCst);
            }
            crate::hal::AK_FAILED_I32
        });
        mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);

        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let start = std::time::Instant::now();
        unified_frame_read_loop(
            sh,
            None,
            ffi,
            callbacks,
            owned_callbacks,
            stop,
            Arc::new(StreamHealthCounters::default()),
            None,
            None,
            None,
            None,
        );
        let elapsed = start.elapsed();

        assert!(error_count.load(Ordering::SeqCst) >= 4);
        // 4 drain cycles × adaptive sleep: 50 + 100 + 200 + 200 = 550ms (capped at 4x base).
        // Allow tolerance: should be between 400ms and 800ms.
        assert!(
            elapsed >= Duration::from_millis(400) && elapsed <= Duration::from_millis(800),
            "Expected ~550ms with adaptive sleep over 4 cycles, got {:?}",
            elapsed
        );
    }

    #[test]
    fn test_drain_loop_resets_sleep_on_frame() {
        // Verify adaptive sleep resets to base when frames are available.
        // Cycle 1: no-data (sleep 100ms), cycle 2: frame + no-data (sleep 50ms), cycle 3: no-data (sleep 100ms).
        // Total: ~250ms (100 + 50 + 100).
        let mut mock = MockVideoHalTrait::new();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);
        let call_idx = Arc::new(AtomicU32::new(0));
        let call_idx_clone = Arc::clone(&call_idx);

        let frame_data: Vec<u8> = vec![0xCD; 64];
        let frame_data_ptr = frame_data.as_ptr() as usize;

        // Cycle 1: no-data (sleep backs off)
        // Cycle 2: frame found then no-data (sleep resets to base, then backs off)
        // Cycle 3: no-data (sleep backs off again)
        // Cycle 4: no-data and stop
        mock.expect_venc_get_stream()
            .returning(move |_, stream_ptr| {
                let idx = call_idx_clone.fetch_add(1, Ordering::SeqCst);
                match idx {
                    0 => crate::hal::AK_FAILED_I32, // no-data: sleep will back off to 100ms
                    1 => {
                        // First call in cycle 2: return frame (sleep resets to 50ms)
                        unsafe {
                            let stream = &mut *stream_ptr;
                            stream.data = frame_data_ptr as *mut u8;
                            stream.len = 64;
                            stream.ts = 1000;
                            stream.seq_no = 1;
                            stream.frame_type = VideoFrameType::FrameTypeP;
                        }
                        crate::hal::AK_SUCCESS_I32
                    }
                    2 => crate::hal::AK_FAILED_I32, // Second call in cycle 2: no-data (sleep 100ms)
                    3 => crate::hal::AK_FAILED_I32, // no-data (sleep 200ms capped)
                    _ => {
                        stop_clone.store(true, Ordering::SeqCst);
                        crate::hal::AK_FAILED_I32
                    }
                }
            });
        mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);
        mock.expect_venc_release_stream()
            .returning(|_, _| AK_SUCCESS_I32);

        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let start = std::time::Instant::now();
        unified_frame_read_loop(
            sh,
            None,
            ffi,
            callbacks,
            owned_callbacks,
            stop,
            Arc::new(StreamHealthCounters::default()),
            None,
            None,
            None,
            None,
        );
        let elapsed = start.elapsed();

        // Adaptive timing: 100 + 50 + 100 = ~250ms total
        // Allow tolerance: 180ms to 600ms (generous for CI environments)
        assert!(
            elapsed >= Duration::from_millis(180) && elapsed <= Duration::from_millis(600),
            "Expected ~250ms with reset on frame, got {:?}",
            elapsed
        );
    }

    #[test]
    fn test_drain_loop_skips_get_error_no_on_fast_path() {
        // Verify get_error_no() is NOT called on every no-data cycle (optimistic fast path).
        // With the probe interval of 50, it should only probe occasionally.
        // First cycle always probes (to establish baseline), subsequent cycles skip until probe_interval.
        let mut mock = MockVideoHalTrait::new();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);
        let call_idx = Arc::new(AtomicU32::new(0));
        let call_idx_clone = Arc::clone(&call_idx);

        // Run many no-data cycles and count how many times get_error_no is called
        mock.expect_venc_get_stream().returning(move |_, _| {
            let idx = call_idx_clone.fetch_add(1, Ordering::SeqCst);
            if idx >= 10 {
                stop_clone.store(true, Ordering::SeqCst);
            }
            crate::hal::AK_FAILED_I32
        });
        mock.expect_get_error_no()
            .times(1) // Should be called only once (first cycle probes, rest skip)
            .returning(|| SDK_ERROR_NO_DATA);

        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        unified_frame_read_loop(
            sh,
            None,
            ffi,
            callbacks,
            owned_callbacks,
            stop,
            Arc::new(StreamHealthCounters::default()),
            None,
            None,
            None,
            None,
        );

        // get_error_no should have been called only 1 time, not 10 times
        // (first cycle always probes, rest skip until probe_interval=50)
    }

    #[tokio::test]
    async fn test_video_input_set_channel_attr_small_sensor_fallback() {
        let mut mock = mock_ffi_with_successful_open();
        mock.expect_vi_get_sensor_resolution()
            .times(1)
            .returning(|_, res| {
                unsafe {
                    (*res).width = 320;
                    (*res).height = 240;
                    (*res).max_width = 320;
                    (*res).max_height = 240;
                }
                AK_SUCCESS_I32
            });
        mock.expect_vi_set_channel_attr()
            .times(1)
            .returning(|_, attr| {
                unsafe {
                    assert_eq!((*attr).res[0].width, 320);
                    assert_eq!((*attr).res[0].height, 240);
                    assert_eq!((*attr).res[1].width, 320);
                    assert_eq!((*attr).res[1].height, 240);
                    assert_eq!((*attr).res[0].max_width, 320);
                    assert_eq!((*attr).res[0].max_height, 240);
                    assert_eq!((*attr).res[1].max_width, 320);
                    assert_eq!((*attr).res[1].max_height, 240);
                }
                AK_SUCCESS_I32
            });

        let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
        vi.open().await.unwrap();

        let result = vi.set_channel_attr();
        assert!(result.is_ok());
    }

    #[test]
    fn test_frame_read_loop_skips_idr_before_first_frame() {
        let mut mock = MockVideoHalTrait::new();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);
        let error_count = Arc::new(AtomicU32::new(0));
        let error_count_clone = Arc::clone(&error_count);

        mock.expect_venc_get_stream().returning(move |_, _| {
            let count = error_count_clone.fetch_add(1, Ordering::SeqCst);
            if count >= NO_DATA_IDR_RECOVERY_EVERY_ERRORS {
                stop_clone.store(true, Ordering::SeqCst);
            }
            AK_FAILED_I32
        });
        mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);
        // No-data errors don't call get_error_str in drain-loop pattern
        mock.expect_venc_set_iframe().times(0);

        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        unified_frame_read_loop(
            sh,
            None,
            ffi,
            callbacks,
            owned_callbacks,
            stop,
            Arc::new(StreamHealthCounters::default()),
            Some(venc_ptr as usize),
            None,
            None,
            None,
        );

        assert!(error_count.load(Ordering::SeqCst) >= NO_DATA_IDR_RECOVERY_EVERY_ERRORS);
    }

    #[test]
    fn test_frame_read_loop_requests_idr_after_frames_then_sustained_no_data() {
        let mut mock = MockVideoHalTrait::new();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);
        let no_data_count = Arc::new(AtomicU32::new(0));
        let no_data_count_clone = Arc::clone(&no_data_count);

        let frame_data: Vec<u8> = vec![0xCD; 64];
        let frame_data_ptr = frame_data.as_ptr() as usize;
        let call_idx = Arc::new(AtomicU32::new(0));
        let call_idx_clone = Arc::clone(&call_idx);

        mock.expect_venc_get_stream()
            .returning(move |_, stream_ptr| {
                let idx = call_idx_clone.fetch_add(1, Ordering::SeqCst);
                if idx == 0 {
                    unsafe {
                        let stream = &mut *stream_ptr;
                        stream.data = frame_data_ptr as *mut u8;
                        stream.len = 64;
                        stream.ts = 9000;
                        stream.seq_no = 1;
                        stream.frame_type = VideoFrameType::FrameTypeI;
                    }
                    AK_SUCCESS_I32
                } else {
                    let errs = no_data_count_clone.fetch_add(1, Ordering::SeqCst) + 1;
                    if errs > NO_DATA_IDR_RECOVERY_EVERY_ERRORS {
                        stop_clone.store(true, Ordering::SeqCst);
                    }
                    AK_FAILED_I32
                }
            });
        mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);
        mock.expect_venc_set_iframe()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);
        mock.expect_venc_release_stream()
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::hal::video::VideoHalTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        unified_frame_read_loop(
            sh,
            None,
            ffi,
            callbacks,
            owned_callbacks,
            stop,
            Arc::new(StreamHealthCounters::default()),
            Some(venc_ptr as usize),
            None,
            None,
            None,
        );

        assert!(no_data_count.load(Ordering::SeqCst) > NO_DATA_IDR_RECOVERY_EVERY_ERRORS);
        drop(frame_data);
    }
}
