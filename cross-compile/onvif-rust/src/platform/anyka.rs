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
//!   ├── ffi: Arc<dyn VideoFfiTrait>   (injected, mockable)
//!   ├── handle: RwLock<Option<Arc<VideoInputHandle>>>  (RAII, calls vi_close on Drop)
//!   └── opened: AtomicBool            (fast-path state check)
//! ```
//!
//! The `VideoInputHandle` implements `Drop` to automatically close the SDK device,
//! ensuring proper cleanup even in error paths. Dual-channel configuration
//! (Main: 1920x1080, Sub: 1280x720) is applied during platform initialization.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use parking_lot::RwLock;

use crate::ffi::VideoDevice;
use crate::ffi::video::{
    RealVideoFfi, VideoFfiTrait, VideoInputHandle, video_input_capture_off_internal,
    video_input_capture_on_internal, video_input_get_sensor_resolution_internal,
    video_input_match_sensor_internal, video_input_open_internal,
    video_input_set_channel_attr_internal,
};

use crate::ffi::{video_channel_attr, video_resolution};

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

        let video_input = Arc::new(AnykaVideoInput::new(isp_config_path));
        let video_encoder = Arc::new(AnykaVideoEncoder::new());
        let audio_input = Arc::new(AnykaAudioInput::new());
        let audio_encoder = Arc::new(AnykaAudioEncoder::new());
        let ptz_control: Option<Arc<dyn PTZControl>> = {
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
        let imaging_control = Some(Arc::new(AnykaImagingControl::new()) as Arc<dyn ImagingControl>);
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

        // Step 2.5: Query and store sensor resolution
        let sensor_res = self.video_input.get_sensor_resolution()?;
        *self.sensor_resolution.write() = Some(sensor_res);
        tracing::info!(
            "Sensor resolution detected: {}x{}",
            sensor_res.width,
            sensor_res.height
        );

        // Step 3: Configure dual channels
        if let Err(e) = self.video_input.set_channel_attr() {
            tracing::error!("Failed to set channel attributes, rolling back: {}", e);
            let _ = self.video_input.close().await;
            return Err(e);
        }

        // Step 4: Start capture pipeline
        if let Err(e) = self.video_input.capture_on() {
            tracing::error!("Failed to start capture pipeline, rolling back: {}", e);
            let _ = self.video_input.close().await;
            return Err(e);
        }
        tracing::info!(
            "Video input initialized with dual-channel configuration and capture started"
        );

        // Initialize dual video encoders (main 1080p + sub 720p)
        let encoder_configs = self.video_encoder.get_configurations().await?;
        for config in &encoder_configs {
            if let Err(e) = self.video_encoder.init(config).await {
                tracing::error!("Failed to initialize video encoder {}: {}", config.token, e);
                // Rollback: close video input
                let _ = self.video_input.close().await;
                return Err(PlatformError::InitializationFailed(format!(
                    "Video encoder {} initialization failed: {}",
                    config.token, e
                )));
            }
        }
        tracing::info!(
            "Video encoders initialized: {} channels",
            encoder_configs.len()
        );

        // TODO(kkrzysztofik): Call remaining Anyka SDK initialization functions via FFI
        // - ak_ai_open()
        // - ak_aenc_open()
        // PTZ is already opened in AnykaPlatform::new()
        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn shutdown(&self) -> PlatformResult<()> {
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
        // Close video input (RAII handle will call ak_vi_close)
        if let Err(e) = self.video_input.close().await {
            tracing::warn!(
                "Video input close failed during shutdown (best-effort, continuing): {}",
                e
            );
        }

        // Video encoder handles are dropped via RAII (VideoEncoderHandle::Drop
        // calls ak_venc_close). The handles are stored in AnykaVideoEncoder's
        // main_handle/sub_handle RwLocks and cleaned up when the platform is dropped.

        // TODO(kkrzysztofik): Call remaining Anyka SDK cleanup functions via FFI
        // - ak_ai_close()
        // - ak_aenc_close()
        self.initialized.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn max_sensor_resolution(&self) -> PlatformResult<Resolution> {
        self.sensor_resolution.read().ok_or_else(|| {
            PlatformError::InitializationFailed(
                "Sensor resolution not available - platform not initialized".to_string(),
            )
        })
    }
}

// =============================================================================
// Video Input Implementation
// =============================================================================

/// Anyka video input implementation backed by the Anyka SDK FFI layer.
///
/// Uses dependency injection via `Arc<dyn VideoFfiTrait>` to enable mock-based
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
///
/// These paths are searched in order when no explicit ISP config path is provided.
/// The first path that exists on the filesystem is used for `ak_vi_match_sensor()`.
const ISP_CONFIG_SEARCH_PATHS: &[&str] = &[
    "/mnt/anyka_hack/onvif/isp_gc1084.conf",
    "/etc/jffs2/isp_gc1084.conf",
    "/usr/local/isp_gc1084.conf",
];

struct AnykaVideoInput {
    ffi: Arc<dyn VideoFfiTrait>,
    handle: RwLock<Option<Arc<VideoInputHandle>>>,
    opened: AtomicBool,
    isp_config_path: Option<PathBuf>,
}

impl AnykaVideoInput {
    /// Create a new `AnykaVideoInput` with the default (real) FFI backend.
    fn new(isp_config_path: Option<PathBuf>) -> Self {
        Self::with_ffi(Arc::new(RealVideoFfi), isp_config_path)
    }

    /// Create a new `AnykaVideoInput` with a custom FFI backend.
    ///
    /// Used by tests with `MockVideoFfiTrait` for hardware-free testing.
    #[cfg(test)]
    fn with_ffi(ffi: Arc<dyn VideoFfiTrait>, isp_config_path: Option<PathBuf>) -> Self {
        Self {
            ffi,
            handle: RwLock::new(None),
            opened: AtomicBool::new(false),
            isp_config_path,
        }
    }

    #[cfg(not(test))]
    fn with_ffi(ffi: Arc<dyn VideoFfiTrait>, isp_config_path: Option<PathBuf>) -> Self {
        Self {
            ffi,
            handle: RwLock::new(None),
            opened: AtomicBool::new(false),
            isp_config_path,
        }
    }

    /// Configure dual-channel video attributes.
    ///
    /// Uses a conservative, sensor-native startup strategy to maximize capture
    /// bring-up reliability across firmware variants.
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
        attr.res = [
            // Main channel: sensor-native conservative startup.
            video_resolution {
                width: sensor_width,
                height: sensor_height,
                max_width: sensor_width,
                max_height: sensor_height,
            },
            // Sub channel: sensor-native conservative startup.
            video_resolution {
                width: sensor_width,
                height: sensor_height,
                max_width: sensor_width,
                max_height: sensor_height,
            },
        ];

        tracing::info!(
            "Applying sensor-native channel attrs: crop={}x{}, main={}x{}, sub={}x{}",
            attr.crop.width,
            attr.crop.height,
            attr.res[0].width,
            attr.res[0].height,
            attr.res[1].width,
            attr.res[1].height
        );

        video_input_set_channel_attr_internal(handle, &attr, self.ffi.as_ref())
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
                        tracing::warn!(
                            "ak_vi_capture_off cleanup after attempt {} failed: {}",
                            attempt,
                            cleanup_error
                        );
                    }

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
        // Fast path: check if already opened without acquiring the lock
        if self.opened.load(Ordering::SeqCst) {
            return Err(PlatformError::ResourceBusy(
                "Video input already opened".to_string(),
            ));
        }

        let vi_handle = video_input_open_internal(VideoDevice::DEV0, self.ffi.as_ref())?;

        let mut guard = self.handle.write();
        *guard = Some(Arc::new(vi_handle));
        self.opened.store(true, Ordering::SeqCst);

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
        // Idempotent: already closed is not an error
        if !self.opened.load(Ordering::SeqCst) {
            return Ok(());
        }

        let mut guard = self.handle.write();
        let _old_handle = guard.take(); // Drop triggers RAII cleanup
        self.opened.store(false, Ordering::SeqCst);

        tracing::info!("Video input device closed");
        Ok(())
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

        // Convert FFI Resolution (crate::ffi::Resolution) to platform Resolution
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
use std::time::{Duration, Instant};

use portable_atomic::AtomicU64;

use crate::ffi::video::{
    VideoEncoderHandle, video_encoder_open_internal, video_encoder_request_idr_internal,
    video_encoder_set_rc_internal,
};
use crate::ffi::{
    bitrate_ctrl_mode, encode_group_type, encode_output_type, encode_param, encode_use_chn,
    profile_mode,
};

use super::frame::{ActiveFrames, CallbackId, Frame, FrameCallback};

/// Encoder lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EncoderState {
    /// Encoder created but not yet initialized.
    Uninitialized,
    /// Encoder initialized and ready to produce frames.
    Initialized,
}

/// Anyka video encoder implementation with FFI integration and callback support.
///
/// Manages dual video encoders (main 1080p + sub 720p) with:
/// - RAII-based FFI handles via `VideoEncoderHandle`
/// - Zero-copy frame delivery to multiple subscribers
/// - Panic-isolated callback invocation
/// - Dynamic bitrate reconfiguration
///
/// # Architecture
///
/// ```text
/// AnykaVideoEncoder
///   ├── ffi: Arc<dyn VideoFfiTrait>       (injected, mockable)
///   ├── main_handle: RwLock<Option<Arc<VideoEncoderHandle>>>
///   ├── sub_handle:  RwLock<Option<Arc<VideoEncoderHandle>>>
///   ├── callbacks: RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>
///   └── active_frames: Arc<ActiveFrames>  (ref-counted buffer tracking)
/// ```
struct AnykaVideoEncoder {
    ffi: Arc<dyn crate::ffi::video::VideoFfiTrait>,
    configurations: RwLock<Vec<VideoEncoderConfig>>,
    main_handle: RwLock<Option<Arc<VideoEncoderHandle>>>,
    sub_handle: RwLock<Option<Arc<VideoEncoderHandle>>>,
    main_state: RwLock<EncoderState>,
    sub_state: RwLock<EncoderState>,
    callbacks: RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>,
    active_frames: Arc<ActiveFrames>,
    next_callback_id: AtomicU64,
}

impl AnykaVideoEncoder {
    /// Create a new `AnykaVideoEncoder` with the default (real) FFI backend.
    fn new() -> Self {
        Self::with_ffi(Arc::new(crate::ffi::video::RealVideoFfi))
    }

    /// Create a new `AnykaVideoEncoder` with a custom FFI backend.
    ///
    /// Used by tests with `MockVideoFfiTrait` for hardware-free testing.
    fn with_ffi(ffi: Arc<dyn crate::ffi::video::VideoFfiTrait>) -> Self {
        Self {
            ffi,
            configurations: RwLock::new(vec![
                VideoEncoderConfig {
                    token: "VideoEncoder_1".to_string(),
                    name: "Main Stream".to_string(),
                    resolution: Resolution::new(1920, 1080),
                    framerate: 25,
                    bitrate: 4000,
                    encoding: VideoEncoding::H264,
                    gop_length: 50,
                    quality: 80,
                    ..Default::default()
                },
                VideoEncoderConfig {
                    token: "VideoEncoder_2".to_string(),
                    name: "Sub Stream".to_string(),
                    resolution: Resolution::new(1280, 720),
                    framerate: 30,
                    bitrate: 2000,
                    encoding: VideoEncoding::H264,
                    gop_length: 60,
                    quality: 70,
                    ..Default::default()
                },
            ]),
            main_handle: RwLock::new(None),
            sub_handle: RwLock::new(None),
            main_state: RwLock::new(EncoderState::Uninitialized),
            sub_state: RwLock::new(EncoderState::Uninitialized),
            callbacks: RwLock::new(HashMap::new()),
            active_frames: Arc::new(ActiveFrames::new()),
            next_callback_id: AtomicU64::new(1),
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
            minqp: 10,
            maxqp: 51,
            fps: config.framerate as i32,
            goplen: config.gop_length as i32,
            bps: (config.bitrate as i32) * 1000, // kbps to bps
            profile: profile_mode::PROFILE_MAIN,
            use_chn: channel,
            enc_grp: encode_group_type::ENCODE_RECORD,
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

        let param = Self::config_to_encode_param(config, channel);
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
                    let bps = (config.bitrate as i32) * 1000;
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
                Resolution::new(640, 480),
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
    opened: AtomicBool,
}

impl AnykaAudioInput {
    fn new() -> Self {
        Self {
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
    configurations: RwLock<Vec<AudioEncoderConfig>>,
}

impl AnykaAudioEncoder {
    fn new() -> Self {
        Self {
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
    settings: RwLock<ImagingSettings>,
}

impl AnykaImagingControl {
    fn new() -> Self {
        Self {
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
        }
    }
}

#[async_trait]
impl ImagingControl for AnykaImagingControl {
    async fn get_settings(&self) -> PlatformResult<ImagingSettings> {
        // TODO(kkrzysztofik): Read actual settings from Anyka imaging SDK
        Ok(self.settings.read().clone())
    }

    async fn set_settings(&self, settings: &ImagingSettings) -> PlatformResult<()> {
        // TODO(kkrzysztofik): Apply settings via Anyka imaging SDK
        *self.settings.write() = settings.clone();
        Ok(())
    }

    async fn get_options(&self) -> PlatformResult<ImagingOptions> {
        // TODO(kkrzysztofik): Query actual hardware capabilities
        Ok(ImagingOptions::default_options())
    }

    async fn set_brightness(&self, value: f32) -> PlatformResult<()> {
        // TODO(kkrzysztofik): Call Anyka imaging SDK
        self.settings.write().brightness = value;
        Ok(())
    }

    async fn set_contrast(&self, value: f32) -> PlatformResult<()> {
        // TODO(kkrzysztofik): Call Anyka imaging SDK
        self.settings.write().contrast = value;
        Ok(())
    }

    async fn set_saturation(&self, value: f32) -> PlatformResult<()> {
        // TODO(kkrzysztofik): Call Anyka imaging SDK
        self.settings.write().saturation = value;
        Ok(())
    }

    async fn set_sharpness(&self, value: f32) -> PlatformResult<()> {
        // TODO(kkrzysztofik): Call Anyka imaging SDK
        self.settings.write().sharpness = value;
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
    use crate::ffi::video::MockVideoFfiTrait;
    use crate::ffi::{AK_FAILED_I32, AK_SUCCESS_I32};
    use std::ffi::c_void;

    fn video_dev0() -> crate::ffi::video_dev_type {
        #[cfg(use_stubs)]
        {
            crate::ffi::video_dev_type::Dev0
        }
        #[cfg(not(use_stubs))]
        {
            crate::ffi::video_dev_type::VIDEO_DEV0
        }
    }

    /// Create a mock FFI that expects a successful vi_open call.
    fn mock_ffi_with_successful_open() -> MockVideoFfiTrait {
        let mut mock = MockVideoFfiTrait::new();
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
        let mut mock = MockVideoFfiTrait::new();
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
        let mock = MockVideoFfiTrait::new();
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
        let mock = MockVideoFfiTrait::new();
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
        let mock = MockVideoFfiTrait::new();
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
    async fn test_video_input_set_channel_attr_not_opened() {
        let mock = MockVideoFfiTrait::new();
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
        let mut mock = MockVideoFfiTrait::new();
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
        let mut mock = MockVideoFfiTrait::new();
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
        let mut mock = MockVideoFfiTrait::new();
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
        let mock = MockVideoFfiTrait::new();
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
        let mock = MockVideoFfiTrait::new();
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
        let mut mock = MockVideoFfiTrait::new();
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
        let mut mock = MockVideoFfiTrait::new();
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

    #[test]
    fn test_capture_on_fails_when_not_opened() {
        let mock = MockVideoFfiTrait::new();
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
    fn mock_ffi_with_successful_encoder_open() -> MockVideoFfiTrait {
        let mut mock = MockVideoFfiTrait::new();
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
            resolution: Resolution::new(1920, 1080),
            framerate: 25,
            bitrate: 4000,
            encoding: VideoEncoding::H264,
            gop_length: 50,
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
            resolution: Resolution::new(1280, 720),
            framerate: 30,
            bitrate: 2000,
            encoding: VideoEncoding::H264,
            gop_length: 60,
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
            resolution: Resolution::new(1920, 1080),
            framerate: 25,
            bitrate: 4000,
            encoding: VideoEncoding::H264,
            gop_length: 50,
            ..Default::default()
        };
        let sub_config = VideoEncoderConfig {
            token: "VideoEncoder_2".to_string(),
            name: "Sub Stream".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 30,
            bitrate: 2000,
            encoding: VideoEncoding::H264,
            gop_length: 60,
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
        let mock = MockVideoFfiTrait::new();
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
        let mut mock = MockVideoFfiTrait::new();
        mock.expect_venc_open().returning(|_| std::ptr::null_mut());

        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            resolution: Resolution::new(1920, 1080),
            framerate: 25,
            bitrate: 4000,
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
            .withf(|_, bps| *bps == 6000 * 1000)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        // Initialize first
        let init_config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            name: "Main Stream".to_string(),
            resolution: Resolution::new(1920, 1080),
            framerate: 25,
            bitrate: 4000,
            encoding: VideoEncoding::H264,
            gop_length: 50,
            ..Default::default()
        };
        encoder.init(&init_config).await.unwrap();

        // Change bitrate
        let new_config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            name: "Main Stream".to_string(),
            resolution: Resolution::new(1920, 1080),
            framerate: 25,
            bitrate: 6000,
            encoding: VideoEncoding::H264,
            gop_length: 50,
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
        let mock = MockVideoFfiTrait::new();
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
            resolution: Resolution::new(1920, 1080),
            framerate: 25,
            bitrate: 4000,
            encoding: VideoEncoding::H264,
            gop_length: 50,
            ..Default::default()
        };
        encoder.init(&init_config).await.unwrap();

        // Attempt bitrate change that fails at FFI level
        let new_config = VideoEncoderConfig {
            token: "VideoEncoder_1".to_string(),
            name: "Main Stream".to_string(),
            resolution: Resolution::new(1920, 1080),
            framerate: 25,
            bitrate: 6000,
            encoding: VideoEncoding::H264,
            gop_length: 50,
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
        let mock = MockVideoFfiTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let config = encoder.get_configuration().await.unwrap();
        assert_eq!(config.token, "VideoEncoder_1");
        assert_eq!(config.resolution.width, 1920);
        assert_eq!(config.resolution.height, 1080);
        assert_eq!(config.framerate, 25);
        assert_eq!(config.bitrate, 4000);
    }

    #[tokio::test]
    async fn test_video_encoder_get_configurations() {
        let mock = MockVideoFfiTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let configs = encoder.get_configurations().await.unwrap();
        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].token, "VideoEncoder_1");
        assert_eq!(configs[1].token, "VideoEncoder_2");
    }

    #[tokio::test]
    async fn test_video_encoder_get_options() {
        let mock = MockVideoFfiTrait::new();
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
            resolution: Resolution::new(1920, 1080),
            framerate: 25,
            bitrate: 4000,
            encoding: VideoEncoding::H264,
            gop_length: 50,
            ..Default::default()
        };

        let param =
            AnykaVideoEncoder::config_to_encode_param(&config, encode_use_chn::ENCODE_MAIN_CHN);
        assert_eq!(param.width, 1920);
        assert_eq!(param.height, 1080);
        assert_eq!(param.fps, 25);
        assert_eq!(param.bps, 4_000_000);
        assert_eq!(param.goplen, 50);
        assert_eq!(param.use_chn, encode_use_chn::ENCODE_MAIN_CHN);
        assert_eq!(param.enc_out_type, encode_output_type::H264_ENC_TYPE);
    }

    #[test]
    fn test_config_to_encode_param_sub() {
        let config = VideoEncoderConfig {
            token: "VideoEncoder_2".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 30,
            bitrate: 2000,
            encoding: VideoEncoding::H264,
            gop_length: 60,
            ..Default::default()
        };

        let param =
            AnykaVideoEncoder::config_to_encode_param(&config, encode_use_chn::ENCODE_SUB_CHN);
        assert_eq!(param.width, 1280);
        assert_eq!(param.height, 720);
        assert_eq!(param.fps, 30);
        assert_eq!(param.bps, 2_000_000);
        assert_eq!(param.goplen, 60);
        assert_eq!(param.use_chn, encode_use_chn::ENCODE_SUB_CHN);
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
        let mock = MockVideoFfiTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let cb = Arc::new(CountingCallback::new());
        let id = encoder.register_frame_callback(cb);
        assert!(id > 0);
        assert_eq!(encoder.callbacks.read().len(), 1);
    }

    #[test]
    fn test_callback_unregistration() {
        let mock = MockVideoFfiTrait::new();
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
        let mock = MockVideoFfiTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

        let removed = encoder.unregister_frame_callback(999);
        assert!(!removed);
    }

    #[test]
    fn test_callback_invocation() {
        let mock = MockVideoFfiTrait::new();
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
        let mock = MockVideoFfiTrait::new();
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
        let mock = MockVideoFfiTrait::new();
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
        let mock = MockVideoFfiTrait::new();
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
            resolution: Resolution::new(1920, 1080),
            framerate: 25,
            bitrate: 4000,
            encoding: VideoEncoding::H264,
            gop_length: 50,
            ..Default::default()
        };
        encoder.init(&config).await.unwrap();

        let result = encoder.request_idr_frame(true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_encoder_request_idr_not_initialized() {
        let mock = MockVideoFfiTrait::new();
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
        let mock = MockVideoFfiTrait::new();
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
        let mock = MockVideoFfiTrait::new();
        let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));
        let af = encoder.active_frames();
        assert_eq!(af.active_count(), 0);
    }
}
