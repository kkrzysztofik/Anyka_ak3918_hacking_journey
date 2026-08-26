//! Anyka platform implementation.
//!
//! This module provides the actual platform implementation using
//! the Anyka SDK for the AK3918 camera hardware.
//!
//! This implementation is only compiled when cross-compiling for ARM
//! (i.e., when `use_stubs` is not defined).

// Submodules
pub mod audio_encoder;
pub mod audio_input;
pub mod context;
pub mod imaging;
pub mod lifecycle;
pub mod network_info;
pub(crate) mod night_mode;
pub mod ptz_actor;
pub mod ptz_control;
pub mod sound;
pub mod supervisor;
mod video_encoder;
pub use video_encoder::StreamOpenParams;
mod video_input;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::RwLock;

use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;

use crate::hal::anyka::ipc::AnykaIpc;
use crate::hal::common::video::VideoHalTrait;
use supervisor::{AttachTarget, Availability, PlatformAttachTarget, run_supervisor};

use super::common::{
    DeviceInfo, ImagingControl, NetworkInfo, PTZControl, Platform, PlatformError, PlatformResult,
    PtzDiagnostics, Resolution, VideoControl, VideoEncoder, VideoInput,
};
use crate::config::types::PtzConfig;
use crate::lifecycle::startup::OptionalInitResult;

// Types used by tests
#[cfg(test)]
use super::common::{VideoEncoderConfig, VideoEncoding};
use video_encoder::AnykaVideoEncoder;

/// Bridges the streaming layer's "I need parameter sets" to the vendor encoder.
struct EncoderIdrRequester {
    encoder: Arc<AnykaVideoEncoder>,
}

impl crate::platform::frame::IdrRequester for EncoderIdrRequester {
    fn request_idr(&self, is_main: bool) {
        if let Err(e) = self.encoder.request_idr_frame(is_main) {
            // error!: the caller only asks when a stream has no cached SPS/PPS,
            // so a failure here means RTSP DESCRIBE keeps 404ing. The shipped
            // log level is "error".
            tracing::error!(
                is_main,
                error = %e,
                "IDR request failed; a stream with no cached SPS/PPS stays undescribable"
            );
        }
    }
}
use video_input::AnykaVideoInput;

// Re-export helpers from submodules for internal use
use context::{pipeline_ready_timeout_ms, pipeline_require_sub};

use lifecycle::{
    capture_stabilization_delay, execute_shutdown_with_timeout, shutdown_video_pipeline,
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
    /// Full PTZ bring-up outcome, including the open failure reason when present.
    ptz_init: OptionalInitResult<Arc<AnykaPTZControl>>,
    imaging_control: Option<Arc<dyn ImagingControl>>,
    network_info: Option<Arc<dyn NetworkInfo>>,
    /// The shared IPC client, kept so the supervisor can attach and detach it.
    ///
    /// Construction leaves it detached: every request is refused until the
    /// supervisor completes an attach. That is what lets cold start and recovery
    /// share one path.
    ipc: Arc<AnykaIpc>,
    /// Shutdown signal + task handle for the night-mode AUTO loop, so
    /// `shutdown` can stop it before video teardown closes the daemon VI.
    night_loop: std::sync::Mutex<
        Option<(
            tokio::sync::broadcast::Sender<()>,
            tokio::task::JoinHandle<()>,
        )>,
    >,
    /// 1 Hz OSD overlay task — stopped before VI teardown / on reattach.
    osd_loop: std::sync::Mutex<
        Option<(
            tokio::sync::broadcast::Sender<()>,
            tokio::task::JoinHandle<()>,
        )>,
    >,
    osd_cfg: Arc<parking_lot::RwLock<crate::config::types::OsdConfig>>,
    osd_device_name: String,
    /// Path the async init path reads to restore the saved PTZ position.
    /// `None` means persistence is disabled.
    ptz_position_path: Option<std::path::PathBuf>,
}

/// Bring PTZ up, or skip it entirely when `ptz_config.enabled` is false.
///
/// Bring-up is not free: it spawns the `ptz-actor` thread, opens `/dev/ak-motor{0,1}` and runs
/// `ptz_check_self`, a physical calibration sweep. On the AK3918 that sweep costs ~2.1 s of every
/// startup and then fails with -1 before the vertical motor is attempted, so `enabled = false` is
/// a real saving and not just a service toggle.
///
/// Returning [`OptionalInitResult`] keeps the open failure reason (device path + errno)
/// reachable from diagnostics instead of discarding it into a bare `None`.
///
/// `position_path` is the on-disk location the actor will write the tracked position to
/// after each completed command. `None` disables persistence.
fn init_ptz_control(
    ptz_config: &PtzConfig,
    position_path: Option<std::path::PathBuf>,
) -> OptionalInitResult<Arc<AnykaPTZControl>> {
    if !ptz_config.enabled {
        tracing::info!("PTZ disabled by config (ptz.enabled = false); skipping motor bring-up");
        return OptionalInitResult::Disabled;
    }

    tracing::info!("Initializing PTZ (native Rust driver, /dev/ak-motor0, /dev/ak-motor1)");
    let ptz = AnykaPTZControl::new(PtzRates::from_config(ptz_config), position_path);
    match ptz.open() {
        Ok(()) => {
            tracing::info!("PTZ device opened successfully");
            OptionalInitResult::Success(Arc::new(ptz))
        }
        Err(e) => {
            tracing::error!(
                "PTZ device failed to open, PTZ features will be unavailable: {}",
                e
            );
            OptionalInitResult::Failed {
                component: "PTZ".to_string(),
                // The device path and errno live in this string. Losing it is what made
                // PTZ bring-up undiagnosable from anywhere but a telnet session.
                error: e.to_string(),
            }
        }
    }
}

/// Named construction inputs for [`AnykaPlatform::with_isp_config`].
///
/// Field names keep the two `Option<PathBuf>` paths and the boolean seed from being
/// swapped at call sites.
pub struct AnykaPlatformIspConfig {
    pub isp_config_path: Option<PathBuf>,
    pub ptz_config: PtzConfig,
    pub main_encoder: StreamOpenParams,
    pub sub_encoder: StreamOpenParams,
    pub imaging_cfg: crate::config::types::ImagingConfig,
    pub initial_rotated: bool,
    pub position_path: Option<PathBuf>,
    pub osd_cfg: crate::config::types::OsdConfig,
    /// Fallback camera label when `[osd.name].text` is empty.
    pub osd_device_name: String,
}

impl AnykaPlatform {
    /// Shared live OSD settings (for ONVIF `SetOSD` to update the running renderer).
    pub fn osd_config(&self) -> Arc<parking_lot::RwLock<crate::config::types::OsdConfig>> {
        Arc::clone(&self.osd_cfg)
    }

    /// Create a new Anyka platform instance.
    ///
    /// Uses auto-detection for the ISP config path and brings PTZ up. See
    /// [`with_isp_config`](Self::with_isp_config) to specify an explicit path or to disable PTZ.
    pub fn new() -> PlatformResult<Self> {
        Self::with_isp_config(AnykaPlatformIspConfig {
            isp_config_path: None,
            ptz_config: crate::config::types::PtzConfig::default(),
            main_encoder: StreamOpenParams::default(),
            sub_encoder: StreamOpenParams::default(),
            imaging_cfg: crate::config::types::ImagingConfig::default(),
            initial_rotated: false,
            position_path: None,
            osd_cfg: crate::config::types::OsdConfig::default(),
            osd_device_name: "ipcam".to_string(),
        })
    }

    /// Static hardware descriptor reported by `get_device_info`.
    fn device_descriptor() -> DeviceInfo {
        DeviceInfo {
            manufacturer: "Anyka".to_string(),
            model: "AK3918".to_string(),
            firmware_version: "1.0.0".to_string(),
            serial_number: "AK3918-001".to_string(),
            hardware_id: "ak3918-hw".to_string(),
        }
    }

    /// Build a platform backed by caller-supplied mock HALs (tests only).
    ///
    /// Skips the `AnykaIpc` connection that `with_isp_config` requires, so the
    /// bring-up and teardown orchestration can be exercised without hardware.
    /// PTZ/imaging/network are left absent — they are independently tested.
    ///
    /// `initial_rotated` mirrors `with_isp_config`'s same-named parameter, so
    /// the construction-time seed path is exercisable under test too.
    #[cfg(test)]
    pub(super) fn with_mocked_hal(
        video_ffi: Arc<dyn VideoHalTrait>,
        audio_ffi: Arc<dyn crate::hal::common::audio::AudioHalTrait>,
        isp_config_path: Option<PathBuf>,
        initial_rotated: bool,
    ) -> Self {
        let video_input = Arc::new(AnykaVideoInput::with_ffi(
            video_ffi.clone(),
            isp_config_path,
        ));
        video_input.rotated.store(initial_rotated, Ordering::SeqCst);

        Self {
            initialized: AtomicBool::new(false),
            device_info: Self::device_descriptor(),
            sensor_resolution: RwLock::new(None),
            video_input,
            video_encoder: Arc::new(AnykaVideoEncoder::with_ffi(video_ffi)),
            audio_input: Arc::new(AnykaAudioInput::with_ffi(audio_ffi.clone())),
            audio_encoder: Arc::new(AnykaAudioEncoder::with_ffi(audio_ffi)),
            ptz_control: None,
            ptz_init: OptionalInitResult::Disabled,
            imaging_control: None,
            network_info: None,
            // Detached: these tests drive the mocked HALs directly and never
            // route through the real IPC client.
            ipc: Arc::new(AnykaIpc::new_detached().expect("detached IPC needs no daemon")),
            night_loop: std::sync::Mutex::new(None),
            osd_loop: std::sync::Mutex::new(None),
            osd_cfg: Arc::new(parking_lot::RwLock::new(
                crate::config::types::OsdConfig::default(),
            )),
            osd_device_name: "ipcam".to_string(),
            ptz_position_path: None,
        }
    }

    /// Create a new Anyka platform instance with an optional ISP config path.
    ///
    /// If `isp_config_path` is `Some`, that path is used directly for
    /// `ak_vi_match_sensor()`. If `None`, the default search paths are used.
    ///
    /// `ptz_config` carries the full `[ptz]` section; the `enabled` flag and the
    /// dead-reckoning rates are read from it. See [`init_ptz_control`] for what skipping
    /// the bring-up avoids.
    ///
    /// `main_encoder`/`sub_encoder` carry the parameters that only `ak_venc_open` can apply, so
    /// they must arrive here rather than through `set_configuration`. See [`StreamOpenParams`].
    ///
    /// `initial_rotated` seeds the persisted flip/mirror flag before the VI is even open, so it
    /// is a no-op on the FFI at this point — `AnykaVideoInput::rotated` just remembers it until
    /// `init_video_input()`'s post-`capture_on()` reapply step actually applies it to hardware.
    ///
    /// `position_path` is the on-disk file the actor writes to after every completed
    /// command. `None` disables persistence.
    pub fn with_isp_config(cfg: AnykaPlatformIspConfig) -> PlatformResult<Self> {
        let AnykaPlatformIspConfig {
            isp_config_path,
            ptz_config,
            main_encoder,
            sub_encoder,
            imaging_cfg,
            initial_rotated,
            position_path,
            osd_cfg,
            osd_device_name,
        } = cfg;
        let device_info = Self::device_descriptor();

        // Start the AUTO day/night loop with its own shutdown channel; the
        // handle is stored so `shutdown` can stop it before teardown.
        let (night_loop_tx, night_loop_rx) = tokio::sync::broadcast::channel(1);

        let (
            video_input,
            video_encoder,
            audio_input,
            audio_encoder,
            imaging_control,
            ipc,
            night_loop_task,
        ) = {
            let shared_ipc = Arc::new(AnykaIpc::new_detached().map_err(|e| {
                PlatformError::InitializationFailed(format!("AnykaIpc construction failed: {}", e))
            })?);

            tracing::info!(
                "AnykaPlatform: using shared AnykaIpc client (detached; supervisor will attach)"
            );

            let video_ffi: Arc<dyn VideoHalTrait> = shared_ipc.clone();
            let audio_ffi: Arc<dyn crate::hal::common::audio::AudioHalTrait> = shared_ipc.clone();
            let imaging_ffi: Arc<dyn crate::hal::common::imaging::ImagingHalTrait> =
                shared_ipc.clone();

            let video_input = Arc::new(AnykaVideoInput::with_ffi(
                video_ffi.clone(),
                isp_config_path.clone(),
            ));
            video_input.rotated.store(initial_rotated, Ordering::SeqCst);
            let video_encoder = Arc::new(AnykaVideoEncoder::with_ipc(
                shared_ipc.clone(),
                main_encoder,
                sub_encoder,
            ));
            let audio_input = Arc::new(AnykaAudioInput::with_ffi(audio_ffi.clone()));
            let audio_encoder = Arc::new(AnykaAudioEncoder::with_ffi(audio_ffi));
            let imaging = Arc::new(AnykaImagingControl::with_ffi_and_video_encoder(
                imaging_ffi,
                Arc::clone(&video_encoder),
                imaging_cfg,
            ));
            let night_loop_task = imaging.night_mode().spawn_auto_loop(night_loop_rx);
            let imaging_control = Some(imaging as Arc<dyn ImagingControl>);

            (
                video_input,
                video_encoder,
                audio_input,
                audio_encoder,
                imaging_control,
                shared_ipc,
                night_loop_task,
            )
        };

        let ptz_init = init_ptz_control(&ptz_config, position_path.clone());
        let ptz_control = match &ptz_init {
            OptionalInitResult::Success(ptz) => Some(Arc::clone(ptz) as Arc<dyn PTZControl>),
            _ => None,
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
            ptz_init,
            imaging_control,
            network_info,
            ipc,
            night_loop: std::sync::Mutex::new(Some((night_loop_tx, night_loop_task))),
            osd_loop: std::sync::Mutex::new(None),
            osd_cfg: Arc::new(parking_lot::RwLock::new(osd_cfg)),
            osd_device_name,
            ptz_position_path: position_path,
        })
    }

    /// Start the attach supervisor and return the availability channel plus task handle.
    ///
    /// The supervisor is the sole owner of the attachment: it drives the same
    /// `attach → initialize → serve → rollback → detach` path for cold start and for
    /// recovery, so a daemon that is absent at boot is not a special case.
    ///
    /// `shutdown` cancels attach / wait / backoff so Application can await the
    /// JoinHandle before platform teardown.
    pub fn spawn_supervisor(
        self: &Arc<Self>,
        shutdown: broadcast::Receiver<()>,
    ) -> PlatformResult<(watch::Receiver<Availability>, JoinHandle<()>)> {
        let (tx, rx) = watch::channel(Availability::Unavailable);
        let target: Arc<dyn AttachTarget> = Arc::new(PlatformAttachTarget::new(Arc::clone(self))?);
        let handle = tokio::spawn(async move {
            run_supervisor(target, &tx, shutdown).await;
        });
        Ok((rx, handle))
    }

    /// Best-effort unwind of a partial or complete bring-up, for the supervisor.
    pub(super) async fn rollback_for_supervisor(&self) {
        self.stop_osd_overlay().await;
        self.rollback_video_pipeline().await;
        self.initialized.store(false, Ordering::SeqCst);
    }

    /// The shared IPC client, for the supervisor.
    pub(super) fn ipc(&self) -> &Arc<AnykaIpc> {
        &self.ipc
    }

    /// Stop any running OSD tick and bring up a fresh one against the live VI.
    ///
    /// Failures are logged and ignored — OSD is optional decoration.
    async fn start_osd_overlay(&self) {
        self.stop_osd_overlay().await;

        if !self.osd_cfg.read().enabled {
            tracing::info!("OSD disabled by config; skipping overlay");
            return;
        }

        let Some(vi) = self.video_input.get_handle() else {
            tracing::warn!("OSD skipped: VI handle missing after capture_on");
            return;
        };

        let dims = match self.ipc.osd_init(vi.as_ptr() as u64).await {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(error = %e, "osd_init failed; continuing without overlay");
                return;
            }
        };
        tracing::info!(
            main_w = dims[0].width,
            main_h = dims[0].height,
            sub_w = dims[1].width,
            sub_h = dims[1].height,
            "OSD initialised"
        );

        let (tx, rx) = broadcast::channel(1);
        let handle =
            crate::osd::renderer::spawn_osd_renderer(crate::osd::renderer::OsdRendererArgs {
                ipc: Arc::clone(&self.ipc),
                vi,
                dims,
                config: Arc::clone(&self.osd_cfg),
                device_name: self.osd_device_name.clone(),
                shutdown: rx,
            });
        match self.osd_loop.lock() {
            Ok(mut guard) => *guard = Some((tx, handle)),
            Err(poisoned) => *poisoned.into_inner() = Some((tx, handle)),
        }
    }

    async fn stop_osd_overlay(&self) {
        let taken = match self.osd_loop.lock() {
            Ok(mut guard) => guard.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };
        if let Some((tx, task)) = taken {
            stop_broadcast_loop(tx, task, "OSD overlay task").await;
        }
    }

    /// Initialize steps 1-5 of platform bring-up: sensor match, VI open, VPSS,
    /// sensor resolution, dual-channel config and capture start.
    ///
    /// Each failure path rolls back only what this stage brought up.
    async fn init_video_input(&self) -> PlatformResult<()> {
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

        // Step 5.5: Reapply flip/mirror. VI state does not survive
        // close/reopen, so a vendor-daemon crash-and-reattach needs this
        // too, not just cold boot — which is why it lives here rather than
        // only in the constructor. Soft-fail: an upside-down stream is still
        // a working stream, so this must not abort the whole bring-up.
        if let Err(e) = self.video_input.reapply_flip_mirror() {
            tracing::warn!("Failed to reapply flip/mirror after capture_on: {}", e);
        }

        // Step 5.6: OSD overlay. Soft-fail — a missing burn-in must never take
        // the video stream down. Re-runs on every attach (daemon restart).
        self.start_osd_overlay().await;

        // Allow the capture pipeline to stabilize before opening encoders.
        tokio::time::sleep(capture_stabilization_delay()).await;
        tracing::info!("Video input initialized: dual-channel config and capture started");

        Ok(())
    }

    /// Initialize the dual video encoders (main 720p + sub 360p).
    ///
    /// On failure, tears down the whole video pipeline (any encoders opened by
    /// this stage plus the video input brought up by `init_video_input`).
    async fn init_video_encoders(&self) -> PlatformResult<()> {
        let encoder_configs = self.video_encoder.get_configurations().await?;
        for config in &encoder_configs {
            if let Err(e) = self.video_encoder.init(config).await {
                tracing::error!("Failed to initialize video encoder {}: {}", config.token, e);

                self.rollback_video_pipeline().await;
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

        Ok(())
    }

    /// Step 6: bind VI + encoder handles and spawn the frame polling threads.
    async fn start_frame_production(&self) -> PlatformResult<()> {
        let vi_handle = match self.video_input.get_handle() {
            Some(handle) => handle,
            None => {
                self.rollback_video_pipeline().await;
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
                self.rollback_video_pipeline().await;
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
            self.rollback_video_pipeline().await;
            return Err(PlatformError::InitializationFailed(format!(
                "Video streaming startup failed: {}",
                e
            )));
        }

        Ok(())
    }

    /// Best-effort teardown of encoders + video input during step 6 rollback.
    async fn rollback_video_pipeline(&self) {
        // Stop OSD before tearing down VI — every video rollback path must not
        // leave the renderer holding a dead handle. Idempotent with the call in
        // `rollback_for_supervisor`.
        self.stop_osd_overlay().await;
        let _ = self.video_encoder.close_all_encoders();
        let _ = self.video_input.capture_off();
        let _ = self.video_input.destroy_vpss();
        let _ = self.video_input.close().await;
    }

    /// Step 7: validate VI/VENC pipeline readiness, rolling back on failure.
    fn validate_pipeline_readiness(&self) -> PlatformResult<()> {
        let readiness_timeout_ms = pipeline_ready_timeout_ms();
        let require_sub_pipeline = pipeline_require_sub();
        if let Err(e) = self.video_encoder.wait_for_stream_readiness(
            Duration::from_millis(readiness_timeout_ms),
            require_sub_pipeline,
        ) {
            tracing::error!(
                "VI/VENC pipeline failed readiness validation, rolling back: {}",
                e
            );
            if let Err(rollback_error) =
                shutdown_video_pipeline(&self.video_encoder, &self.video_input)
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

        Ok(())
    }
}

// Default implementation removed - use AnykaPlatform::new() for fallible initialization.
// The Default trait should never panic per Rust best practices.

#[async_trait]
impl Platform for AnykaPlatform {
    async fn get_device_info(&self) -> PlatformResult<DeviceInfo> {
        Ok(self.device_info.clone())
    }

    crate::impl_platform_accessors!();

    fn video_control(&self) -> Option<Arc<dyn VideoControl>> {
        Some(self.video_input.clone() as Arc<dyn VideoControl>)
    }

    fn stream_frame_age_ms(&self) -> Option<u64> {
        self.video_encoder.stream_frame_age_ms()
    }

    fn ptz_diagnostics(&self) -> Option<PtzDiagnostics> {
        Some(match &self.ptz_init {
            OptionalInitResult::Disabled => PtzDiagnostics::disabled(),
            OptionalInitResult::Failed { error, .. } => PtzDiagnostics::failed(error.clone()),
            OptionalInitResult::Success(ptz) => ptz.diagnostics(),
        })
    }

    fn apply_osd_config(&self, cfg: crate::config::types::OsdConfig) {
        *self.osd_cfg.write() = cfg;
    }

    async fn initialize(&self) -> PlatformResult<()> {
        self.init_video_input().await?;
        self.init_video_encoders().await?;
        self.start_frame_production().await?;
        self.validate_pipeline_readiness()?;

        // Restore the saved PTZ position *after* `check_self` has driven the
        // motors to true center, so the dead-reckoned move starts from the
        // most accurate origin available. Every failure is non-fatal: a state
        // file must never fail boot.
        if let (Some(path), OptionalInitResult::Success(ptz)) =
            (&self.ptz_position_path, &self.ptz_init)
        {
            // Discard the `Result`: an unwrap here would convert a corrupt
            // state file into a boot failure, which is exactly what the design
            // says must not happen.
            let _ = ptz.restore_position_from_disk(path).await;
        }

        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn shutdown(&self) -> PlatformResult<()> {
        tracing::info!("Platform shutdown: starting PTZ stop...");

        // Stop the night-mode AUTO loop before video teardown: it holds an Arc
        // to the IPC client and must not tick against a closed VI handle.
        // The guard is dropped here so no std mutex guard spans the await.
        let night_loop = match self.night_loop.lock() {
            Ok(mut guard) => guard.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };
        if let Some((tx, task)) = night_loop {
            stop_night_loop(tx, task).await;
        }
        self.stop_osd_overlay().await;

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

        // Use the lifecycle module's timeout-aware shutdown helper
        let result = execute_shutdown_with_timeout(
            Arc::clone(&self.video_encoder),
            Arc::clone(&self.video_input),
        );

        if result.is_ok() {
            self.initialized.store(false, Ordering::SeqCst);
            tracing::info!("Platform shutdown: complete");
        } else {
            tracing::error!("Platform shutdown ended with error: {:?}", result);
        }
        result
    }

    fn requires_hard_shutdown(&self) -> bool {
        self.video_encoder.requires_hard_shutdown()
    }

    fn max_sensor_resolution(&self) -> PlatformResult<Resolution> {
        self.sensor_resolution.read().ok_or_else(|| {
            PlatformError::InitializationFailed(
                "Sensor resolution not available - platform not initialized".to_string(),
            )
        })
    }

    fn register_owned_frame_callback(
        &self,
        callback: Arc<dyn crate::platform::frame::OwnedFrameCallback>,
    ) -> PlatformResult<()> {
        let _id = self.video_encoder.register_owned_frame_callback(callback);
        tracing::info!("Owned frame callback registered (id={})", _id);
        Ok(())
    }

    fn idr_requester(&self) -> Option<Arc<dyn crate::platform::frame::IdrRequester>> {
        Some(Arc::new(EncoderIdrRequester {
            encoder: Arc::clone(&self.video_encoder),
        }))
    }
}

// =============================================================================
// PTZ Control Implementation
// =============================================================================

/// Ask a broadcast-shutdown task to stop and join it, aborting after a timeout.
///
/// `tokio::time::timeout` borrowing the handle (not consuming it) means the
/// task stays joinable if it ignores the shutdown signal; abort and await it
/// so it cannot keep ticking against a torn-down VI handle.
async fn stop_broadcast_loop(
    tx: tokio::sync::broadcast::Sender<()>,
    mut task: tokio::task::JoinHandle<()>,
    label: &str,
) {
    let _ = tx.send(());
    if tokio::time::timeout(Duration::from_secs(2), &mut task)
        .await
        .is_err()
    {
        tracing::warn!("{label} did not stop within 2s; aborting");
        task.abort();
        let _ = task.await;
    }
}

/// Ask the night-mode AUTO loop to stop and join it, aborting after a timeout.
async fn stop_night_loop(
    tx: tokio::sync::broadcast::Sender<()>,
    task: tokio::task::JoinHandle<()>,
) {
    stop_broadcast_loop(tx, task, "night-mode AUTO loop").await;
}

use ptz_actor::PtzRates;
/// Anyka PTZ control — delegates to `AnykaPTZControl` which calls the FFI layer.
///
/// The PTZ stub has been replaced with a real hardware implementation
/// (see `ptz_control.rs`) that controls the physical stepper motors via FFI.
use ptz_control::AnykaPTZControl;

// Re-export from submodules for internal use
use audio_encoder::AnykaAudioEncoder;
use audio_input::AnykaAudioInput;
use imaging::AnykaImagingControl;
use network_info::AnykaNetworkInfo;

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests;
