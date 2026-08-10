// =============================================================================
// Video Input Implementation
// =============================================================================
//!
//! Anyka video input implementation backed by the Anyka SDK FFI layer.
//!
//! Uses dependency injection via `Arc<dyn VideoHalTrait>` to enable mock-based
//! testing without hardware. The `VideoInputHandle` provides RAII cleanup —
//! dropping the handle automatically calls `ak_vi_close()`.
//!
//! # State Management
//!
//! - `opened: AtomicBool` — fast-path check without lock contention
//! - `handle: RwLock<Option<Arc<VideoInputHandle>>>` — thread-safe handle access
//!   with take-on-close semantics
//!
//! # Thread Safety
//!
//! All fields are `Send + Sync`. The `AtomicBool` provides lock-free reads for
//! the common `is_opened` check, while the `RwLock` protects handle mutations.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use parking_lot::RwLock;

use crate::hal::anyka::ipc::AnykaIpc;
use crate::hal::anyka::sdk::VideoDevice;
use crate::hal::common::video::{
    VideoHalTrait, VideoInputHandle, video_input_capture_off, video_input_capture_on,
    video_input_get_sensor_resolution, video_input_match_sensor, video_input_open,
    video_input_set_channel_attr, video_input_set_flip_mirror,
};
use crate::hal::common::{video_channel_attr, video_resolution};

use crate::platform::common::{
    PlatformError, PlatformResult, Resolution, VideoInput, VideoSourceConfig,
};

use super::context::ISP_CONFIG_SEARCH_PATHS;
use super::lifecycle::{align_height_to_8, align_width_to_32};

pub(super) struct AnykaVideoInput {
    pub(super) ffi: Arc<dyn VideoHalTrait>,
    pub(super) handle: RwLock<Option<Arc<VideoInputHandle>>>,
    pub(super) opened: AtomicBool,
    pub(super) capture_started: AtomicBool,
    pub(super) isp_config_path: Option<PathBuf>,
    pub(super) vpss_initialized: AtomicBool,
    pub(super) channel_layout: RwLock<(Resolution, Resolution)>,
    pub(super) rotated: AtomicBool,
}

impl AnykaVideoInput {
    /// Create a new `AnykaVideoInput` with the default (real) FFI backend.
    pub(super) fn new(isp_config_path: Option<PathBuf>) -> PlatformResult<Self> {
        let ipc = AnykaIpc::new().map_err(|e| {
            PlatformError::InitializationFailed(format!(
                "AnykaIpc connection failed (is vendor-daemon running?): {}",
                e
            ))
        })?;
        tracing::info!("AnykaVideoInput: using AnykaIpc for vendor library access");
        Ok(Self::with_ffi(Arc::new(ipc), isp_config_path))
    }

    /// Create a new `AnykaVideoInput` with a custom FFI backend.
    ///
    /// Used by tests with `MockVideoHalTrait` for hardware-free testing.
    pub(super) fn with_ffi(ffi: Arc<dyn VideoHalTrait>, isp_config_path: Option<PathBuf>) -> Self {
        Self {
            ffi,
            handle: RwLock::new(None),
            opened: AtomicBool::new(false),
            capture_started: AtomicBool::new(false),
            isp_config_path,
            vpss_initialized: AtomicBool::new(false),
            channel_layout: RwLock::new((Resolution::new(1280, 720), Resolution::new(640, 360))),
            rotated: AtomicBool::new(false),
        }
    }

    /// Get a clone of the video input handle (if opened).
    pub fn get_handle(&self) -> Option<Arc<crate::hal::common::video::VideoInputHandle>> {
        self.handle.read().clone()
    }

    pub(super) fn channel_layout(&self) -> (Resolution, Resolution) {
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
    pub(super) fn set_channel_attr(&self) -> PlatformResult<()> {
        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input not opened".to_string())
        })?;

        // Query sensor resolution to properly set crop area
        let sensor_res = video_input_get_sensor_resolution(handle, self.ffi.as_ref())?;
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
            align_width_to_32(sensor_width.max(32))
        };
        let sub_height = if sensor_height >= 360 {
            360
        } else {
            align_height_to_8(sensor_height.max(8))
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
        video_input_set_channel_attr(handle, &attr, self.ffi.as_ref())?;
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
    pub(super) fn match_sensor(&self) -> PlatformResult<()> {
        // If an explicit path was provided, try it directly
        if let Some(ref path) = self.isp_config_path {
            if path.exists() {
                tracing::info!(
                    "Loading ISP sensor config from explicit path: {}",
                    path.display()
                );
                return video_input_match_sensor(path, self.ffi.as_ref());
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
                return video_input_match_sensor(path, self.ffi.as_ref());
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
    pub(super) fn init_vpss(&self) -> PlatformResult<()> {
        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input not opened".to_string())
        })?;

        crate::hal::common::video::vpss_init(handle, VideoDevice::DEV0, self.ffi.as_ref())?;
        self.vpss_initialized.store(true, Ordering::SeqCst);
        tracing::info!("VPSS initialized successfully");
        Ok(())
    }

    /// Destroy the Video Post-Processing SubSystem (VPSS).
    ///
    /// This MUST be called BEFORE closing the video input device during cleanup.
    /// The reference implementation shows this must be done in the correct order
    /// to avoid resource leaks.
    pub(super) fn destroy_vpss(&self) -> PlatformResult<()> {
        if self
            .vpss_initialized
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            crate::hal::common::video::vpss_destroy(VideoDevice::DEV0, self.ffi.as_ref())?;
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
    pub(super) fn get_sensor_resolution(&self) -> PlatformResult<Resolution> {
        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input not opened".to_string())
        })?;

        let sensor_res = video_input_get_sensor_resolution(handle, self.ffi.as_ref())?;
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
    pub(super) fn capture_on(&self) -> PlatformResult<()> {
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
            match video_input_capture_on(handle, self.ffi.as_ref()) {
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

                    if let Err(cleanup_error) = video_input_capture_off(handle, self.ffi.as_ref()) {
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

    pub(super) fn capture_off(&self) -> PlatformResult<()> {
        if !self.capture_started.load(Ordering::SeqCst) {
            return Ok(());
        }

        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input not opened".to_string())
        })?;

        video_input_capture_off(handle, self.ffi.as_ref())?;
        self.capture_started.store(false, Ordering::SeqCst);
        tracing::info!("Video capture stopped successfully");
        Ok(())
    }

    /// Store the flip/mirror flag and, if the VI is currently open, apply it
    /// immediately via the sync IPC lane. If the VI is closed
    /// (construction-time seed, or between attach cycles), the value is
    /// stored and picked up the next time `init_video_input()` reaches this
    /// point after `capture_on()`.
    ///
    /// **Boot-time / attach-sequence use only.** Called from
    /// `AnykaPlatform::with_isp_config` (construction-time seed) and
    /// `AnykaPlatform::init_video_input` (post-`capture_on` reapply on every
    /// attach) — both contexts where a blocking SDK call is already
    /// tolerated (see `capture_on`'s own `std::thread::sleep` calls just
    /// above). Do **not** call this from an ONVIF request handler or any
    /// other tokio-worker-pool context — use the async `VideoControl::set_flip_mirror`
    /// trait method for that instead (implemented below), which routes
    /// through `spawn_blocking` specifically to avoid parking a shared
    /// worker thread.
    pub(super) fn apply_flip_mirror(&self, rotated: bool) -> PlatformResult<()> {
        self.rotated.store(rotated, Ordering::SeqCst);

        if !self.opened.load(Ordering::SeqCst) {
            return Ok(());
        }

        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input not opened".to_string())
        })?;

        video_input_set_flip_mirror(handle, rotated, rotated, self.ffi.as_ref())
    }

    /// Current flip/mirror flag (may not yet be applied if the VI is closed).
    pub(super) fn rotated(&self) -> bool {
        self.rotated.load(Ordering::SeqCst)
    }

    /// Reapply the currently-stored flip/mirror flag to hardware, without
    /// changing it. Used by `init_video_input()`'s post-`capture_on()` reapply
    /// step. Deliberately does not read-then-pass-through the way
    /// `apply_flip_mirror(self.rotated())` would — that round-trip re-stores
    /// whatever was read, which can clobber a newer value set concurrently by a
    /// live `VideoControl::set_flip_mirror` call between the read and the
    /// store. This method only ever reads `self.rotated` at the moment it
    /// applies it and never writes back, so there is no window where it can
    /// regress the atomic to a stale value.
    pub(super) fn reapply_flip_mirror(&self) -> PlatformResult<()> {
        if !self.opened.load(Ordering::SeqCst) {
            return Ok(());
        }
        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input not opened".to_string())
        })?;
        let rotated = self.rotated.load(Ordering::SeqCst);
        video_input_set_flip_mirror(handle, rotated, rotated, self.ffi.as_ref())
    }

    pub(super) fn close_blocking(&self) -> PlatformResult<()> {
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

        let vi_handle = match video_input_open(VideoDevice::DEV0, self.ffi.as_ref()) {
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

        let ffi_res = video_input_get_sensor_resolution(handle, self.ffi.as_ref())?;

        // Convert FFI Resolution (crate::hal::anyka::sdk::Resolution) to platform Resolution
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

#[async_trait]
impl crate::platform::VideoControl for AnykaVideoInput {
    /// Async-safe live apply, reachable from ONVIF request handlers.
    ///
    /// Deliberately does **not** call `apply_flip_mirror` directly — that
    /// method's `video_input_set_flip_mirror` call goes through the sync
    /// `send_request` IPC lane, which blocks the calling OS thread. Called
    /// from `apply_flip_mirror`'s two boot-time call sites that's fine (see
    /// its doc comment), but this trait method is reachable from
    /// `SetVideoSourceConfiguration`, an ONVIF SOAP handler running on the
    /// shared tokio worker pool — blocking that worker for the RPC's full
    /// timeout would stall other concurrent requests. `spawn_blocking`
    /// offloads the blocking call to the dedicated blocking-task pool
    /// instead, freeing this worker immediately. Mirrors why the imaging
    /// HAL's live-apply path uses `request_async` rather than the sync lane.
    async fn set_flip_mirror(&self, rotated: bool) -> PlatformResult<()> {
        self.rotated.store(rotated, Ordering::SeqCst);

        let handle = self.handle.read().clone();
        let Some(handle) = handle else {
            // Not open yet — value stored above, applied on next capture_on().
            return Ok(());
        };
        let ffi = self.ffi.clone();

        tokio::task::spawn_blocking(move || {
            video_input_set_flip_mirror(&handle, rotated, rotated, ffi.as_ref())
        })
        .await
        .map_err(|e| PlatformError::HardwareFailure(format!("flip/mirror task panicked: {e}")))?
    }
}
