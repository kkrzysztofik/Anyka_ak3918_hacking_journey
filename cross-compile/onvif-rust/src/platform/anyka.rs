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
//! (Main: 1280x720, Sub: 640x360) is applied during platform initialization.

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

        // Step 2.5: VPSS init skipped — libre_anyka_app does not call ak_vpss_init()
        // and our init sequence lacks the prerequisite ak_cmd_server_register().
        // VPSS is not needed for basic video encoding; it provides post-processing
        // features (OSD overlay, video effects). Can be re-enabled later if needed
        // by calling ak_cmd_server_register() first. See akipc/main/ipc_main.c.

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
            let _ = self.video_input.close().await;
            return Err(e);
        }

        // Step 5: Start capture pipeline
        if let Err(e) = self.video_input.capture_on() {
            tracing::error!("Failed to start capture pipeline, rolling back: {}", e);
            let _ = self.video_input.capture_off();
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

        // Start frame production: bind VI+encoder and spawn polling threads
        if let Some(vi_handle) = self.video_input.get_handle() {
            let main_enc = self.video_encoder.main_handle.read().clone();
            let sub_enc = self.video_encoder.sub_handle.read().clone();

            if let Some(ref main) = main_enc
                && let Err(e) =
                    self.video_encoder
                        .start_streaming(&vi_handle, main, sub_enc.as_ref())
            {
                tracing::error!("Failed to start streaming: {}", e);
                // Non-fatal: platform can still serve ONVIF metadata without live frames
            }
        }

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

        // Stop frame production threads before closing encoders
        self.video_encoder.stop_streaming();

        if let Err(e) = self.video_encoder.close_all_encoders() {
            tracing::warn!(
                "Video encoder close failed during shutdown (best-effort, continuing): {}",
                e
            );
        }

        // Stop capture BEFORE closing video input.
        if let Err(e) = self.video_input.capture_off() {
            tracing::warn!(
                "Video capture off failed during shutdown (best-effort, continuing): {}",
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

    fn register_frame_callback(
        &self,
        callback: Arc<dyn crate::platform::frame::FrameCallback>,
    ) -> PlatformResult<()> {
        let _id = self.video_encoder.register_frame_callback(callback);
        tracing::info!("Frame callback registered (id={})", _id);
        Ok(())
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

///
/// These paths are searched in order when no explicit ISP config path is provided.
/// The first path that exists on the filesystem is used for `ak_vi_match_sensor()`.
const ISP_CONFIG_SEARCH_PATHS: &[&str] = &[
    "/mnt/anyka_hack/onvif/isp_gc1084.conf",
    "/etc/jffs2/isp_gc1084.conf",
    "/usr/local/isp_gc1084.conf",
];

const COMPAT_MAIN_MAX_WIDTH_VGA: i32 = 640;
const COMPAT_MAIN_MAX_HEIGHT_VGA: i32 = 480;
const COMPAT_SUB_MAX_WIDTH_HD: i32 = 1280;
const COMPAT_SUB_MAX_HEIGHT_HD: i32 = 720;

struct AnykaVideoInput {
    ffi: Arc<dyn VideoFfiTrait>,
    handle: RwLock<Option<Arc<VideoInputHandle>>>,
    opened: AtomicBool,
    capture_started: AtomicBool,
    isp_config_path: Option<PathBuf>,
    vpss_initialized: AtomicBool,
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
            capture_started: AtomicBool::new(false),
            isp_config_path,
            vpss_initialized: AtomicBool::new(false),
        }
    }

    #[cfg(not(test))]
    fn with_ffi(ffi: Arc<dyn VideoFfiTrait>, isp_config_path: Option<PathBuf>) -> Self {
        Self {
            ffi,
            handle: RwLock::new(None),
            opened: AtomicBool::new(false),
            capture_started: AtomicBool::new(false),
            isp_config_path,
            vpss_initialized: AtomicBool::new(false),
        }
    }

    /// Get a clone of the video input handle (if opened).
    pub fn get_handle(&self) -> Option<Arc<crate::ffi::VideoInputHandle>> {
        self.handle.read().clone()
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

        // Sub channel must be smaller than main channel — the ISP driver's internal
        // scaling pipeline stalls frame production when both are identical.
        // Reference: vi_demo uses 640x360, ipc_main defaults to 640x480.
        let sub_width = align_to_32((sensor_width / 2).max(640));
        let sub_height = align_to_8(sensor_height * sub_width / sensor_width);
        // Preserve legacy compatibility intent while guaranteeing max dimensions
        // are not below active dimensions.
        let main_max_width = sensor_width.max(COMPAT_MAIN_MAX_WIDTH_VGA);
        let main_max_height = sensor_height.max(COMPAT_MAIN_MAX_HEIGHT_VGA);
        let sub_max_width = sub_width.max(COMPAT_SUB_MAX_WIDTH_HD);
        let sub_max_height = sub_height.max(COMPAT_SUB_MAX_HEIGHT_HD);

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
            "Applying sensor-native channel attrs: crop={}x{}, main={}x{}, sub={}x{}",
            attr.crop.width,
            attr.crop.height,
            attr.res[0].width,
            attr.res[0].height,
            attr.res[1].width,
            attr.res[1].height
        );
        tracing::info!(
            "Applying Anyka compat max attrs: main_max={}x{}, sub_max={}x{}",
            attr.res[0].max_width,
            attr.res[0].max_height,
            attr.res[1].max_width,
            attr.res[1].max_height,
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

        crate::ffi::video::vpss_init_internal(handle, VideoDevice::DEV0, self.ffi.as_ref())?;
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
            crate::ffi::video::vpss_destroy_internal(VideoDevice::DEV0, self.ffi.as_ref())?;
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

#[cfg(use_stubs)]
use crate::ffi::VideoFrameType;
use crate::ffi::video::{
    VideoEncoderHandle, VideoStreamHandle, video_encoder_open_internal,
    video_encoder_request_idr_internal, video_encoder_set_rc_internal,
};
use crate::ffi::{
    bitrate_ctrl_mode, encode_group_type, encode_output_type, encode_param, encode_use_chn,
    profile_mode, video_stream,
};

use super::frame::{ActiveFrames, CallbackId, Frame, FrameCallback, FrameType, StreamId};

/// Encoder lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EncoderState {
    /// Encoder created but not yet initialized.
    Uninitialized,
    /// Encoder initialized and ready to produce frames.
    Initialized,
}

/// Convert an SDK `VideoFrameType` to our `FrameType`.
#[cfg(use_stubs)]
fn sdk_frame_type_to_frame_type(ft: VideoFrameType) -> FrameType {
    match ft {
        VideoFrameType::FrameTypeI | VideoFrameType::FrameTypePi => FrameType::VideoIFrame,
        VideoFrameType::FrameTypeP => FrameType::VideoPFrame,
        VideoFrameType::FrameTypeB => FrameType::VideoBFrame,
    }
}

#[cfg(not(use_stubs))]
fn sdk_frame_type_to_frame_type(ft: crate::ffi::video_frame_type) -> FrameType {
    match ft {
        crate::ffi::video_frame_type::FRAME_TYPE_I
        | crate::ffi::video_frame_type::FRAME_TYPE_PI => FrameType::VideoIFrame,
        crate::ffi::video_frame_type::FRAME_TYPE_P => FrameType::VideoPFrame,
        crate::ffi::video_frame_type::FRAME_TYPE_B => FrameType::VideoBFrame,
    }
}

/// Polling loop that reads encoded frames from the SDK and invokes callbacks.
///
/// This runs on a dedicated `std::thread` (NOT tokio) because `ak_venc_get_stream()`
/// is a blocking C call. The loop reads frames, converts them to `Frame`, invokes
/// all registered callbacks, then releases the SDK buffer back.
///
/// Exits cleanly when `stop_signal` is set to `true`.
fn frame_read_loop(
    stream_handle: Arc<VideoStreamHandle>,
    ffi: Arc<dyn crate::ffi::video::VideoFfiTrait>,
    stream_id: StreamId,
    callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>>,
    stop_signal: Arc<AtomicBool>,
) {
    use crate::ffi::AK_SUCCESS_I32;

    tracing::info!("Frame read loop started for {:?}", stream_id);

    // Allow the SDK's internal encoder thread to initialize before polling.
    // The C reference (platform_anyka.c:2673) uses PLATFORM_DELAY_MS_MEDIUM (100ms).
    std::thread::sleep(Duration::from_millis(100));

    let mut consecutive_errors: u32 = 0;

    while !stop_signal.load(Ordering::SeqCst) {
        let mut stream = std::mem::MaybeUninit::<video_stream>::uninit();

        // Blocking call — waits until a frame is available or error
        let ret = ffi.venc_get_stream(stream_handle.as_ptr(), stream.as_mut_ptr());

        if ret != AK_SUCCESS_I32 {
            if stop_signal.load(Ordering::SeqCst) {
                break;
            }
            // Exponential backoff: 10ms → 20ms → 40ms → 80ms → 100ms max.
            // The C reference uses base 50ms with 2x backoff up to 200ms.
            let delay = std::cmp::min(10u64 * (1u64 << consecutive_errors.min(3)), 100);
            std::thread::sleep(Duration::from_millis(delay));
            consecutive_errors += 1;
            continue;
        }

        consecutive_errors = 0;

        // SAFETY: venc_get_stream succeeded, so `stream` is fully initialized
        let mut stream_data = unsafe { stream.assume_init() };

        if !stream_data.data.is_null() && stream_data.len > 0 {
            let frame_type = sdk_frame_type_to_frame_type(stream_data.frame_type);

            let frame = Frame {
                data: stream_data.data as *const u8,
                size: stream_data.len as usize,
                // SDK timestamps are in milliseconds; Frame uses microseconds
                timestamp: stream_data.ts.wrapping_mul(1000),
                frame_type,
                stream_id,
            };

            // Invoke all callbacks (panic-isolated)
            invoke_callbacks_from_map(&callbacks, &frame);
        }

        // Release the SDK buffer back to the encoder.
        // SAFETY: We pass back the same stream struct that get_stream populated.
        // The data pointer is owned by the SDK and must be returned.
        // This MUST happen even during shutdown to avoid leaking SDK buffers.
        let _ = ffi.venc_release_stream(stream_handle.as_ptr(), &mut stream_data);
    }

    tracing::info!("Frame read loop exited for {:?}", stream_id);
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
    let mut failed = Vec::new();

    for (id, cb) in cbs.iter() {
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            cb.on_frame(frame);
        }));

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
    callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>>,
    active_frames: Arc<ActiveFrames>,
    next_callback_id: AtomicU64,
    main_stream_handle: RwLock<Option<Arc<VideoStreamHandle>>>,
    sub_stream_handle: RwLock<Option<Arc<VideoStreamHandle>>>,
    main_read_thread: RwLock<Option<std::thread::JoinHandle<()>>>,
    sub_read_thread: RwLock<Option<std::thread::JoinHandle<()>>>,
    stop_signal: Arc<AtomicBool>,
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
            active_frames: Arc::new(ActiveFrames::new()),
            next_callback_id: AtomicU64::new(1),
            main_stream_handle: RwLock::new(None),
            sub_stream_handle: RwLock::new(None),
            main_read_thread: RwLock::new(None),
            sub_read_thread: RwLock::new(None),
            stop_signal: Arc::new(AtomicBool::new(false)),
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
        vi_handle: &Arc<crate::ffi::VideoInputHandle>,
        main_enc: &Arc<VideoEncoderHandle>,
        sub_enc: Option<&Arc<VideoEncoderHandle>>,
    ) -> PlatformResult<()> {
        self.stop_signal.store(false, Ordering::SeqCst);

        // Request main stream
        let main_sh = Arc::new(VideoStreamHandle::new(
            vi_handle.as_ptr(),
            main_enc.as_ptr(),
            Arc::clone(&self.ffi),
        )?);
        *self.main_stream_handle.write() = Some(Arc::clone(&main_sh));

        // Allow the SDK encoder process thread to start up before we poll.
        // The C reference (platform_anyka.c:609) uses PLATFORM_DELAY_MS_RETRY (200ms)
        // for post-capture/post-request stabilization.
        std::thread::sleep(Duration::from_millis(200));

        // Spawn main read thread
        let main_thread = {
            let ffi = Arc::clone(&self.ffi);
            let callbacks = Arc::clone(&self.callbacks_arc());
            let stop = Arc::clone(&self.stop_signal);
            let sh = Arc::clone(&main_sh);
            std::thread::Builder::new()
                .name("venc-main-read".to_string())
                .spawn(move || {
                    frame_read_loop(sh, ffi, StreamId::VideoMain, callbacks, stop);
                })
                .map_err(|e| {
                    PlatformError::InitializationFailed(format!(
                        "Failed to spawn main read thread: {}",
                        e
                    ))
                })?
        };
        *self.main_read_thread.write() = Some(main_thread);
        tracing::info!("Main stream reader thread started");

        // Request sub stream (if encoder exists)
        if let Some(sub) = sub_enc {
            let sub_sh = Arc::new(VideoStreamHandle::new(
                vi_handle.as_ptr(),
                sub.as_ptr(),
                Arc::clone(&self.ffi),
            )?);
            *self.sub_stream_handle.write() = Some(Arc::clone(&sub_sh));

            // Same stabilization delay as main stream (see above).
            std::thread::sleep(Duration::from_millis(200));

            let sub_thread = {
                let ffi = Arc::clone(&self.ffi);
                let callbacks = Arc::clone(&self.callbacks_arc());
                let stop = Arc::clone(&self.stop_signal);
                let sh = Arc::clone(&sub_sh);
                std::thread::Builder::new()
                    .name("venc-sub-read".to_string())
                    .spawn(move || {
                        frame_read_loop(sh, ffi, StreamId::VideoSub, callbacks, stop);
                    })
                    .map_err(|e| {
                        PlatformError::InitializationFailed(format!(
                            "Failed to spawn sub read thread: {}",
                            e
                        ))
                    })?
            };
            *self.sub_read_thread.write() = Some(sub_thread);
            tracing::info!("Sub stream reader thread started");
        }

        Ok(())
    }

    /// Stop streaming: signal threads to stop, cancel streams (unblocking SDK calls),
    /// then join the reader threads.
    pub fn stop_streaming(&self) {
        self.stop_signal.store(true, Ordering::SeqCst);

        // Cancel streams to unblock ak_venc_get_stream() in reader threads.
        // Must happen BEFORE thread join — threads hold Arc clones so RAII won't fire.
        if let Some(handle) = self.main_stream_handle.read().as_ref() {
            handle.cancel();
        }
        if let Some(handle) = self.sub_stream_handle.read().as_ref() {
            handle.cancel();
        }

        // Now join threads — they will exit because venc_get_stream returns error
        // and stop_signal is set.
        if let Some(thread) = self.main_read_thread.write().take()
            && let Err(e) = thread.join()
        {
            tracing::warn!("Main read thread panicked during join: {:?}", e);
        }
        if let Some(thread) = self.sub_read_thread.write().take()
            && let Err(e) = thread.join()
        {
            tracing::warn!("Sub read thread panicked during join: {:?}", e);
        }

        // Drop stream handles (threads are done, RAII Drop is now a no-op due to cancelled flag).
        let _ = self.main_stream_handle.write().take();
        let _ = self.sub_stream_handle.write().take();

        tracing::info!("Streaming stopped");
    }

    /// Get a cloned `Arc` reference to the callbacks map for thread sharing.
    fn callbacks_arc(&self) -> Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> {
        Arc::clone(&self.callbacks)
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
        match token {
            "VideoEncoder_1" => {
                let old_handle = self.main_handle.write().take();
                if old_handle.is_some() {
                    *self.main_state.write() = EncoderState::Uninitialized;
                    tracing::info!("Closed video encoder token={}", token);
                }
                Ok(())
            }
            "VideoEncoder_2" => {
                let old_handle = self.sub_handle.write().take();
                if old_handle.is_some() {
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

    #[tokio::test]
    async fn test_capture_on_retry_aborts_when_capture_off_cleanup_fails() {
        let mut mock = MockVideoFfiTrait::new();
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
        let mock = MockVideoFfiTrait::new();
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

    // =========================================================================
    // VideoStreamHandle Tests
    // =========================================================================

    use crate::ffi::video::VideoStreamHandle;

    #[test]
    fn test_video_stream_handle_creation_success() {
        let mut mock = MockVideoFfiTrait::new();
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
        let mut mock = MockVideoFfiTrait::new();

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
        let mut mock = MockVideoFfiTrait::new();
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

    // =========================================================================
    // Frame Type Conversion Tests
    // =========================================================================

    #[cfg(use_stubs)]
    #[test]
    fn test_frame_type_conversion_i_frame() {
        use crate::ffi::VideoFrameType;
        assert_eq!(
            sdk_frame_type_to_frame_type(VideoFrameType::FrameTypeI),
            FrameType::VideoIFrame
        );
    }

    #[cfg(use_stubs)]
    #[test]
    fn test_frame_type_conversion_pi_frame() {
        use crate::ffi::VideoFrameType;
        assert_eq!(
            sdk_frame_type_to_frame_type(VideoFrameType::FrameTypePi),
            FrameType::VideoIFrame
        );
    }

    #[cfg(use_stubs)]
    #[test]
    fn test_frame_type_conversion_p_frame() {
        use crate::ffi::VideoFrameType;
        assert_eq!(
            sdk_frame_type_to_frame_type(VideoFrameType::FrameTypeP),
            FrameType::VideoPFrame
        );
    }

    #[cfg(use_stubs)]
    #[test]
    fn test_frame_type_conversion_b_frame() {
        use crate::ffi::VideoFrameType;
        assert_eq!(
            sdk_frame_type_to_frame_type(VideoFrameType::FrameTypeB),
            FrameType::VideoBFrame
        );
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
    fn test_frame_read_loop_invokes_callbacks() {
        use crate::ffi::VideoFrameType;
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

        let mut mock = MockVideoFfiTrait::new();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);

        // Frame data buffer
        let frame_data: Vec<u8> = vec![0xAB; 100];
        let frame_data_ptr = frame_data.as_ptr() as usize;

        // First call: return a frame, then signal stop
        let mut seq = mockall::Sequence::new();
        mock.expect_venc_get_stream()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, stream_ptr| {
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
            });

        mock.expect_venc_release_stream()
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        // Stream handle creation + cancel
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::ffi::video::VideoFfiTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        callbacks
            .write()
            .insert(1, Arc::new(CountingCallback(call_count_clone)));

        frame_read_loop(sh, ffi, StreamId::VideoMain, callbacks, stop);

        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        // Keep frame_data alive until after the loop
        drop(frame_data);
    }

    #[test]
    fn test_frame_read_loop_handles_error_and_retries() {
        let mut mock = MockVideoFfiTrait::new();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);
        let error_count = Arc::new(AtomicU32::new(0));
        let error_count_clone = Arc::clone(&error_count);

        // Return errors, then signal stop after 2 errors
        mock.expect_venc_get_stream().returning(move |_, _| {
            let count = error_count_clone.fetch_add(1, Ordering::SeqCst);
            if count >= 1 {
                stop_clone.store(true, Ordering::SeqCst);
            }
            crate::ffi::AK_FAILED_I32
        });

        // No release_stream calls expected (errors don't produce frames)
        // Stream handle
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::ffi::video::VideoFfiTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        frame_read_loop(sh, ffi, StreamId::VideoMain, callbacks, stop);

        // Should have retried at least twice
        assert!(error_count.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn test_stop_signal_terminates_loop() {
        let mut mock = MockVideoFfiTrait::new();
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

        let ffi: Arc<dyn crate::ffi::video::VideoFfiTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Should return immediately
        frame_read_loop(sh, ffi, StreamId::VideoMain, callbacks, stop);
    }

    #[tokio::test]
    async fn test_start_stop_streaming_lifecycle() {
        let mut mock = MockVideoFfiTrait::new();

        // Encoder open expectations
        mock.expect_venc_set_cfg_path().returning(|_| 0);
        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_open()
            .returning(move |_| test_ptr as *mut c_void);

        // Stream lifecycle expectations
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_get_stream()
            .returning(|_, _| AK_FAILED_I32); // No frames in test
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let encoder = Arc::new(AnykaVideoEncoder::with_ffi(Arc::new(mock)));

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
        let vi_handle = Arc::new(crate::ffi::VideoInputHandle::test_handle());
        let main_enc = encoder.main_handle.read().clone().unwrap();

        // Start streaming
        let result = encoder.start_streaming(&vi_handle, &main_enc, None);
        assert!(result.is_ok());

        // Verify threads are running
        assert!(encoder.main_stream_handle.read().is_some());
        assert!(encoder.main_read_thread.read().is_some());

        // Stop streaming
        encoder.stop_streaming();

        // Verify cleanup
        assert!(encoder.main_stream_handle.read().is_none());
        assert!(encoder.main_read_thread.read().is_none());
    }

    #[test]
    fn test_frame_read_loop_initial_delay() {
        // Verify frame_read_loop takes at least 100ms due to startup delay,
        // even when the stop signal is already set.
        let mut mock = MockVideoFfiTrait::new();
        let stop = Arc::new(AtomicBool::new(true)); // Pre-set stop

        mock.expect_venc_get_stream()
            .times(0)
            .returning(|_, _| AK_FAILED_I32);

        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::ffi::video::VideoFfiTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let start = std::time::Instant::now();
        frame_read_loop(sh, ffi, StreamId::VideoMain, callbacks, stop);
        let elapsed = start.elapsed();

        // Must take at least 100ms due to the startup delay
        assert!(
            elapsed >= Duration::from_millis(90),
            "Expected >= 90ms startup delay, got {:?}",
            elapsed
        );
    }

    #[test]
    fn test_consecutive_error_backoff() {
        // Verify exponential backoff: first error ~10ms, fourth error ~80ms.
        // We check that 4 consecutive errors take longer than 4 * 10ms = 40ms,
        // proving backoff is in effect.
        let mut mock = MockVideoFfiTrait::new();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);
        let error_count = Arc::new(AtomicU32::new(0));
        let error_count_clone = Arc::clone(&error_count);

        // Return 4 errors, then stop
        mock.expect_venc_get_stream().returning(move |_, _| {
            let count = error_count_clone.fetch_add(1, Ordering::SeqCst);
            if count >= 4 {
                stop_clone.store(true, Ordering::SeqCst);
            }
            crate::ffi::AK_FAILED_I32
        });

        let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
        mock.expect_venc_request_stream()
            .returning(move |_, _| test_ptr as *mut c_void);
        mock.expect_venc_cancel_stream()
            .returning(|_| AK_SUCCESS_I32);

        let ffi: Arc<dyn crate::ffi::video::VideoFfiTrait> = Arc::new(mock);
        let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

        let callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn FrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let start = std::time::Instant::now();
        frame_read_loop(sh, ffi, StreamId::VideoMain, callbacks, stop);
        let elapsed = start.elapsed();

        assert!(error_count.load(Ordering::SeqCst) >= 4);
        // 100ms initial delay + backoff: 10ms + 20ms + 40ms + 80ms = 250ms total
        // Allow some tolerance; should be at least 200ms
        assert!(
            elapsed >= Duration::from_millis(200),
            "Expected >= 200ms with backoff, got {:?}",
            elapsed
        );
    }
}
