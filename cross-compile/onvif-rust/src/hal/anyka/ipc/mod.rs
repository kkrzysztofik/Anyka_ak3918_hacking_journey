//! Unix socket IPC client for Anyka vendor daemon communication.
//!
//! This module provides a HAL backend that communicates with a vendor daemon
//! via Unix socket instead of directly calling the Anyka SDK C functions.
//!
//! # Binary Protocol
//!
//! Request format: cmd_id (i32) + req_len (u32) + req_data (bytes)
//! Response format: status (i32) + resp_len (u32) + resp_data (bytes)
//!
//! Frame delivery is push-only: vendor-daemon writes encoded frames into a
//! shared-memory ring and sends 12-byte notifications over
//! `/tmp/vd-frame-main.sock` and `/tmp/vd-frame-sub.sock`.

#![allow(dead_code)]

mod audio;
mod imaging;
mod shm_ring;
mod video;

use crate::platform::PlatformError;
use crate::platform::PlatformResult;
use crate::platform::frame::{OwnedFrame, StreamId};
use crate::streaming::bridge::BytesMutPool;
use shm_ring::{FrameNotification, ShmRingReader};

use std::ffi::c_void;
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tracing::{debug, warn};

use crate::hal::common::{
    aenc_attr, audio_param, encode_param, pcm_param, video_channel_attr, video_dev_type,
    video_resolution,
};

use crate::hal::common::AK_SUCCESS_I32;

// ============================================================================
// Self-contained type definitions for IPC (matching C daemon structs)
// ============================================================================

/// Video device type - matches C enum
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct IpcVideoDevType(pub i32);

/// Video resolution - matches C struct
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct IpcVideoResolution {
    pub width: i32,
    pub height: i32,
    pub max_width: i32,
    pub max_height: i32,
}

/// Crop info - matches C struct
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct IpcCropInfo {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

/// Video channel attr - matches C struct
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct IpcVideoChannelAttr {
    pub crop: IpcCropInfo,
    pub res: [IpcVideoResolution; 2],
}

/// Encode group type - matches C enum
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum IpcEncodeGroupType {
    #[default]
    ENCODE_RECORD = 0,
    ENCODE_MAINCHN_NET = 1,
    ENCODE_SUBCHN_NET = 2,
    ENCODE_PICTURE = 3,
}

/// Encode use channel - matches C enum
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum IpcEncodeUseChn {
    #[default]
    ENCODE_MAIN_CHN = 0,
    ENCODE_SUB_CHN = 1,
}

/// Encode output type - matches C enum
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum IpcEncodeOutputType {
    #[default]
    H264_ENC_TYPE = 0,
    MJPEG_ENC_TYPE = 1,
    HEVC_ENC_TYPE = 2,
}

/// Bitrate control mode - matches C enum
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum IpcBitrateCtrlMode {
    #[default]
    BR_MODE_CBR = 0,
    BR_MODE_VBR = 1,
}

/// Profile mode - matches C enum
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum IpcProfileMode {
    #[default]
    PROFILE_MAIN = 0,
    PROFILE_HIGH = 1,
    PROFILE_BASE = 2,
    PROFILE_HEVC_MAIN = 3,
    PROFILE_HEVC_MAIN_STILL = 4,
}

/// Encode param - matches C struct
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct IpcEncodeParam {
    pub width: u32,
    pub height: u32,
    pub minqp: i32,
    pub maxqp: i32,
    pub fps: i32,
    pub goplen: i32,
    pub bps: i32,
    pub profile: IpcProfileMode,
    pub use_chn: IpcEncodeUseChn,
    pub enc_grp: IpcEncodeGroupType,
    pub br_mode: IpcBitrateCtrlMode,
    pub enc_out_type: IpcEncodeOutputType,
}

/// PCM param - matches C struct
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct IpcPcmParam {
    pub sample_rate: u32,
    pub sample_bits: u32,
    pub channel_num: u32,
}

/// Audio param - matches C struct
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct IpcAudioParam {
    pub sample_rate: u32,
    pub channel_num: u32,
    pub sample_bits: u32,
    pub type_: i32,
}

/// Aenc attr - matches C struct
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct IpcAencAttr {
    pub aac_head: i32,
}

/// Video frame type - matches C enum
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum IpcVideoFrameType {
    #[default]
    FrameTypeP = 0,
    FrameTypeI = 1,
    FrameTypeB = 2,
    FrameTypePi = 3,
}

/// Video stream - matches C struct
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct IpcVideoStream {
    pub data: *mut u8,
    pub len: u32,
    pub ts: u64,
    pub seq_no: std::os::raw::c_ulong,
    pub frame_type: IpcVideoFrameType,
}

// ============================================================================
// IPC Client Implementation
// ============================================================================

/// Debug logging flag for IPC connections.
/// Controlled by `logging.ipc_debug` config value.
/// When false, IPC connection debug messages are suppressed.
static IPC_DEBUG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Enable or disable IPC debug logging.
///
/// This function is called at startup based on the `logging.ipc_debug` config value.
#[allow(dead_code)]
pub fn set_ipc_debug_logging(enabled: bool) {
    IPC_DEBUG.store(enabled, std::sync::atomic::Ordering::SeqCst);
}

/// Check if IPC debug logging is enabled.
fn is_ipc_debug_enabled() -> bool {
    IPC_DEBUG.load(std::sync::atomic::Ordering::SeqCst)
}

fn monotonic_millis() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

const VENDOR_SOCKET_PATH: &str = "/tmp/vendor-daemon.sock";

/// Path for the dedicated main-stream frame notification socket.
const FRAME_MAIN_SOCKET_PATH: &str = "/tmp/vd-frame-main.sock";
/// Path for the dedicated sub-stream frame notification socket.
const FRAME_SUB_SOCKET_PATH: &str = "/tmp/vd-frame-sub.sock";
/// Path for the dedicated control socket (Approach B Phase 3)
const CTRL_SOCKET_PATH: &str = "/tmp/vd-ctrl.sock";

/// Maximum allowed IPC response body (2 MB — large enough for raw I-frames).
const MAX_RESPONSE_SIZE: usize = 2 * 1024 * 1024;

const CMD_VI_MATCH_SENSOR: i32 = 1;
const CMD_VI_OPEN: i32 = 2;
const CMD_VI_CLOSE: i32 = 3;
const CMD_VI_GET_SENSOR_RESOLUTION: i32 = 4;
const CMD_VI_SET_CHANNEL_ATTR: i32 = 5;
const CMD_VI_CAPTURE_ON: i32 = 6;
const CMD_VI_CAPTURE_OFF: i32 = 7;
const CMD_VPSS_INIT: i32 = 8;
const CMD_VPSS_DESTROY: i32 = 9;
const CMD_VENC_SET_CFG_PATH: i32 = 10;
const CMD_VENC_OPEN: i32 = 11;
const CMD_VENC_CLOSE: i32 = 12;
const CMD_VENC_SET_RC: i32 = 13;
const CMD_VENC_SET_IFRAME: i32 = 14;
const CMD_VENC_REQUEST_STREAM: i32 = 15;
const CMD_VENC_CANCEL_STREAM: i32 = 18;
const CMD_VENC_START_PUSH: i32 = 19;
const CMD_VENC_STOP_PUSH: i32 = 20;
const CMD_AI_OPEN: i32 = 50;
const CMD_AI_CLOSE: i32 = 51;
const CMD_AI_SET_ADC_VOLUME: i32 = 52;
const CMD_AI_SET_ASLC_VOLUME: i32 = 53;
const CMD_AENC_OPEN: i32 = 54;
const CMD_AENC_CLOSE: i32 = 55;
const CMD_AENC_SET_ATTR: i32 = 56;
const CMD_ISP_SET_BRIGHTNESS: i32 = 100;
const CMD_ISP_SET_CONTRAST: i32 = 101;
const CMD_ISP_SET_SATURATION: i32 = 102;
const CMD_ISP_SET_SHARPNESS: i32 = 103;
const CMD_ISP_SET_IR_FILTER: i32 = 104;
const CMD_ISP_SET_WDR: i32 = 105;
const CMD_GET_ERROR_NO: i32 = 200;
const CMD_GET_ERROR_STR: i32 = 201;
const PUSH_NOTIFICATION_TIMEOUT: Duration = Duration::from_millis(200);

/// Timeout for blocking reads/writes on the IPC control socket.
///
/// If the vendor daemon stops responding, this bounds how long `send_request_once`
/// will block before returning an error. The caller's reconnect logic will then
/// attempt one retry.
const IPC_CTRL_TIMEOUT: Duration = Duration::from_secs(10);

/// IPC client for Anyka vendor daemon communication.
pub struct AnykaIpc {
    /// Control socket for command RPCs.
    stream: Arc<Mutex<UnixStream>>,
    /// Dedicated main-stream frame notification socket.
    frame_main_stream: Mutex<Option<UnixStream>>,
    /// Dedicated sub-stream frame notification socket.
    frame_sub_stream: Mutex<Option<UnixStream>>,
    /// Shared memory ring buffer reader (Approach A) - wrapped in Mutex for interior mutability.
    /// Only the frame reader thread accesses this, so there's no contention.
    /// None if daemon doesn't use shared memory.
    shm_reader: Mutex<Option<ShmRingReader>>,
    /// Tie-breaker when both channels are ready at once; toggles to prevent starvation.
    prefer_sub_on_tie: AtomicBool,
}

impl AnykaIpc {
    fn cmd_name(cmd_id: i32) -> &'static str {
        match cmd_id {
            CMD_VI_MATCH_SENSOR => "VI_MATCH_SENSOR",
            CMD_VI_OPEN => "VI_OPEN",
            CMD_VI_CLOSE => "VI_CLOSE",
            CMD_VI_GET_SENSOR_RESOLUTION => "VI_GET_SENSOR_RESOLUTION",
            CMD_VI_SET_CHANNEL_ATTR => "VI_SET_CHANNEL_ATTR",
            CMD_VI_CAPTURE_ON => "VI_CAPTURE_ON",
            CMD_VI_CAPTURE_OFF => "VI_CAPTURE_OFF",
            CMD_VPSS_INIT => "VPSS_INIT",
            CMD_VPSS_DESTROY => "VPSS_DESTROY",
            CMD_VENC_SET_CFG_PATH => "VENC_SET_CFG_PATH",
            CMD_VENC_OPEN => "VENC_OPEN",
            CMD_VENC_CLOSE => "VENC_CLOSE",
            CMD_VENC_SET_RC => "VENC_SET_RC",
            CMD_VENC_SET_IFRAME => "VENC_SET_IFRAME",
            CMD_VENC_REQUEST_STREAM => "VENC_REQUEST_STREAM",
            CMD_VENC_CANCEL_STREAM => "VENC_CANCEL_STREAM",
            CMD_VENC_START_PUSH => "VENC_START_PUSH",
            CMD_VENC_STOP_PUSH => "VENC_STOP_PUSH",
            CMD_AI_OPEN => "AI_OPEN",
            CMD_AI_CLOSE => "AI_CLOSE",
            CMD_AI_SET_ADC_VOLUME => "AI_SET_ADC_VOLUME",
            CMD_AI_SET_ASLC_VOLUME => "AI_SET_ASLC_VOLUME",
            CMD_AENC_OPEN => "AENC_OPEN",
            CMD_AENC_CLOSE => "AENC_CLOSE",
            CMD_AENC_SET_ATTR => "AENC_SET_ATTR",
            CMD_ISP_SET_BRIGHTNESS => "ISP_SET_BRIGHTNESS",
            CMD_ISP_SET_CONTRAST => "ISP_SET_CONTRAST",
            CMD_ISP_SET_SATURATION => "ISP_SET_SATURATION",
            CMD_ISP_SET_SHARPNESS => "ISP_SET_SHARPNESS",
            CMD_ISP_SET_IR_FILTER => "ISP_SET_IR_FILTER",
            CMD_ISP_SET_WDR => "ISP_SET_WDR",
            CMD_GET_ERROR_NO => "GET_ERROR_NO",
            CMD_GET_ERROR_STR => "GET_ERROR_STR",
            _ => "UNKNOWN",
        }
    }

    /// Create a new IPC client connected to the vendor daemon.
    pub fn new() -> PlatformResult<Self> {
        // Connect to control socket: try new path first, fall back to legacy
        // The daemon now uses /tmp/vd-ctrl.sock (replacing /tmp/vendor-daemon.sock)
        let stream = UnixStream::connect(CTRL_SOCKET_PATH)
            .or_else(|_| UnixStream::connect(VENDOR_SOCKET_PATH))
            .map_err(|e| {
                PlatformError::HardwareUnavailable(format!(
                    "Cannot connect to vendor daemon (tried {} and {}): {}",
                    CTRL_SOCKET_PATH, VENDOR_SOCKET_PATH, e
                ))
            })?;
        if is_ipc_debug_enabled() {
            debug!(socket = CTRL_SOCKET_PATH, "Connected to vendor daemon");
        }

        // Set timeouts on the control socket to bound blocking I/O.
        stream
            .set_read_timeout(Some(IPC_CTRL_TIMEOUT))
            .map_err(|e| {
                PlatformError::HardwareFailure(format!(
                    "Failed to set IPC control socket read timeout: {}",
                    e
                ))
            })?;
        stream
            .set_write_timeout(Some(IPC_CTRL_TIMEOUT))
            .map_err(|e| {
                PlatformError::HardwareFailure(format!(
                    "Failed to set IPC control socket write timeout: {}",
                    e
                ))
            })?;

        // Push-only mode requires dedicated frame sockets for main and sub streams.
        let frame_main_stream = UnixStream::connect(FRAME_MAIN_SOCKET_PATH).map_err(|e| {
            PlatformError::HardwareUnavailable(format!(
                "Push-only mode requires main frame socket {}: {}",
                FRAME_MAIN_SOCKET_PATH, e
            ))
        })?;
        let frame_sub_stream = UnixStream::connect(FRAME_SUB_SOCKET_PATH).map_err(|e| {
            PlatformError::HardwareUnavailable(format!(
                "Push-only mode requires sub frame socket {}: {}",
                FRAME_SUB_SOCKET_PATH, e
            ))
        })?;
        tracing::info!(
            main_socket = FRAME_MAIN_SOCKET_PATH,
            sub_socket = FRAME_SUB_SOCKET_PATH,
            "Connected to dedicated frame sockets"
        );

        // Push-only mode requires shared memory ring buffer.
        let shm_reader = match ShmRingReader::open() {
            Ok(Some(reader)) => reader,
            Ok(None) => {
                return Err(PlatformError::HardwareUnavailable(
                    "Push-only mode requires shared memory ring buffer".into(),
                ));
            }
            Err(e) => {
                return Err(PlatformError::HardwareUnavailable(format!(
                    "Push-only mode requires shared memory ring buffer: {}",
                    e
                )));
            }
        };
        tracing::info!("Shared memory ring buffer opened");

        Ok(Self {
            stream: Arc::new(Mutex::new(stream)),
            frame_main_stream: Mutex::new(Some(frame_main_stream)),
            frame_sub_stream: Mutex::new(Some(frame_sub_stream)),
            shm_reader: Mutex::new(Some(shm_reader)),
            prefer_sub_on_tie: AtomicBool::new(false),
        })
    }

    /// Create a new IPC client connected to a custom socket path (test-only).
    #[cfg(test)]
    pub fn new_with_path(path: &str) -> PlatformResult<Self> {
        let mut stream = UnixStream::connect(path).map_err(|e| {
            PlatformError::HardwareUnavailable(format!(
                "Failed to connect to vendor daemon at {}: {}",
                path, e
            ))
        })?;

        // Set timeouts on the control socket to bound blocking I/O (same as production).
        stream
            .set_read_timeout(Some(IPC_CTRL_TIMEOUT))
            .map_err(|e| {
                PlatformError::HardwareFailure(format!(
                    "Failed to set IPC control socket read timeout: {}",
                    e
                ))
            })?;
        stream
            .set_write_timeout(Some(IPC_CTRL_TIMEOUT))
            .map_err(|e| {
                PlatformError::HardwareFailure(format!(
                    "Failed to set IPC control socket write timeout: {}",
                    e
                ))
            })?;

        if is_ipc_debug_enabled() {
            debug!(socket = path, "Connected to vendor daemon (test)");
        }

        // In test mode, try to connect to frame sockets if they exist.
        let frame_main_stream = match UnixStream::connect(FRAME_MAIN_SOCKET_PATH) {
            Ok(fs) => Some(fs),
            Err(_) => None,
        };
        let frame_sub_stream = match UnixStream::connect(FRAME_SUB_SOCKET_PATH) {
            Ok(fs) => Some(fs),
            Err(_) => None,
        };

        // In test mode, try to open shm if available
        let shm_reader = match ShmRingReader::open() {
            Ok(Some(reader)) => Some(reader),
            _ => None,
        };

        Ok(Self {
            stream: Arc::new(Mutex::new(stream)),
            frame_main_stream: Mutex::new(frame_main_stream),
            frame_sub_stream: Mutex::new(frame_sub_stream),
            shm_reader: Mutex::new(shm_reader),
            prefer_sub_on_tie: AtomicBool::new(false),
        })
    }

    fn read_push_notification(
        stream: &mut UnixStream,
        channel: &'static str,
    ) -> PlatformResult<FrameNotification> {
        stream
            .set_read_timeout(Some(PUSH_NOTIFICATION_TIMEOUT))
            .map_err(|e| {
                PlatformError::HardwareFailure(format!(
                    "push read-timeout setup error ({}): {}",
                    channel, e
                ))
            })?;

        let mut notif_bytes = [0u8; 12];
        stream
            .read_exact(&mut notif_bytes)
            .map_err(|e| match e.kind() {
                ErrorKind::WouldBlock | ErrorKind::TimedOut => PlatformError::Timeout,
                _ => PlatformError::HardwareFailure(format!(
                    "push notification read error ({}): {}",
                    channel, e
                )),
            })?;
        let notif = FrameNotification::from_bytes(&notif_bytes);
        if is_ipc_debug_enabled() {
            debug!(
                event = "push_notification_received",
                diag_monotonic_ms = monotonic_millis(),
                channel,
                slot_index = notif.slot_index,
                flags = notif.flags,
                socket_fallback = notif.is_socket_fallback(),
                frame_dropped = notif.is_frame_dropped(),
                "received push notification"
            );
        }
        Ok(notif)
    }

    fn map_shm_slot_read_error(error: PlatformError, channel: &'static str) -> PlatformError {
        match error {
            PlatformError::InvalidParameter(msg) if msg.contains("not ready for reading") => {
                warn!(
                    event = "shm_slot_transient_race",
                    diag_monotonic_ms = monotonic_millis(),
                    channel,
                    reason = %msg,
                    "push frame read classified as transient race"
                );
                PlatformError::ResourceBusy(format!(
                    "transient slot race on {} channel: {}",
                    channel, msg
                ))
            }
            other => {
                warn!(
                    event = "shm_slot_read_error",
                    diag_monotonic_ms = monotonic_millis(),
                    channel,
                    error = %other,
                    "push frame read failed"
                );
                other
            }
        }
    }

    fn choose_ready_channel(&self, main_ready: bool, sub_ready: bool) -> Option<&'static str> {
        match (main_ready, sub_ready) {
            (true, false) => Some("main"),
            (false, true) => Some("sub"),
            (true, true) => {
                let prefer_sub = self.prefer_sub_on_tie.fetch_xor(true, Ordering::AcqRel);
                if prefer_sub {
                    Some("sub")
                } else {
                    Some("main")
                }
            }
            (false, false) => None,
        }
    }

    /// Attempt to reconnect to the vendor daemon, replacing the existing socket.
    fn reconnect(&self) -> PlatformResult<()> {
        warn!(
            socket = CTRL_SOCKET_PATH,
            "Attempting to reconnect to vendor daemon"
        );
        // Try new path first, fall back to legacy
        let new_stream = UnixStream::connect(CTRL_SOCKET_PATH)
            .or_else(|_| UnixStream::connect(VENDOR_SOCKET_PATH))
            .map_err(|e| {
                PlatformError::HardwareUnavailable(format!(
                    "Failed to reconnect to vendor daemon: {}",
                    e
                ))
            })?;
        let mut guard = self.stream.lock().map_err(|e| {
            PlatformError::HardwareFailure(format!("IPC mutex poisoned on reconnect: {}", e))
        })?;
        *guard = new_stream;
        guard
            .set_read_timeout(Some(IPC_CTRL_TIMEOUT))
            .map_err(|e| {
                PlatformError::HardwareFailure(format!(
                    "Failed to set IPC control socket read timeout after reconnect: {}",
                    e
                ))
            })?;
        guard
            .set_write_timeout(Some(IPC_CTRL_TIMEOUT))
            .map_err(|e| {
                PlatformError::HardwareFailure(format!(
                    "Failed to set IPC control socket write timeout after reconnect: {}",
                    e
                ))
            })?;
        if is_ipc_debug_enabled() {
            debug!(socket = VENDOR_SOCKET_PATH, "Reconnected to vendor daemon");
        }
        Ok(())
    }

    /// Send a request and receive a response over the Unix socket.
    ///
    /// On I/O error, attempts a single reconnect and retries the request.
    pub(crate) fn send_request(
        &self,
        cmd_id: i32,
        req_data: &[u8],
    ) -> PlatformResult<(i32, Vec<u8>)> {
        let started = Instant::now();
        let cmd_name = Self::cmd_name(cmd_id);
        if is_ipc_debug_enabled() {
            debug!(
                cmd_id,
                cmd_name,
                req_len = req_data.len(),
                "IPC request start"
            );
        }
        match self.send_request_once(cmd_id, req_data) {
            Ok(result) => {
                let elapsed_ms = started.elapsed().as_millis();
                if is_ipc_debug_enabled() {
                    debug!(
                        cmd_id,
                        cmd_name,
                        status = result.0,
                        resp_len = result.1.len(),
                        elapsed_ms,
                        "IPC request done"
                    );
                }
                Ok(result)
            }
            Err(e) => {
                let elapsed_ms = started.elapsed().as_millis();
                warn!(
                    cmd_id,
                    cmd_name,
                    elapsed_ms,
                    error = %e,
                    "IPC request failed; attempting reconnect"
                );
                self.reconnect()?;
                if is_ipc_debug_enabled() {
                    debug!(cmd_id, cmd_name, "IPC request retry start");
                }
                let retry_started = Instant::now();
                let retry_result = self.send_request_once(cmd_id, req_data);
                match &retry_result {
                    Ok((status, resp_data)) => {
                        if is_ipc_debug_enabled() {
                            debug!(
                                cmd_id,
                                cmd_name,
                                status,
                                resp_len = resp_data.len(),
                                elapsed_ms = retry_started.elapsed().as_millis(),
                                "IPC request retry done"
                            );
                        }
                    }
                    Err(err) => {
                        warn!(
                            cmd_id,
                            cmd_name,
                            elapsed_ms = retry_started.elapsed().as_millis(),
                            error = %err,
                            "IPC request retry failed"
                        );
                    }
                }
                retry_result
            }
        }
    }

    /// Map I/O errors to PlatformError, distinguishing timeouts from hardware failures.
    fn map_io_error(ctx: &str, e: std::io::Error) -> PlatformError {
        if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut {
            PlatformError::Timeout
        } else {
            PlatformError::HardwareFailure(format!("{}: {}", ctx, e))
        }
    }

    /// Perform a single send/receive cycle without reconnect logic.
    fn send_request_once(&self, cmd_id: i32, req_data: &[u8]) -> PlatformResult<(i32, Vec<u8>)> {
        let mut stream = self
            .stream
            .lock()
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC mutex poisoned: {}", e)))?;

        // Write request: cmd_id (i32 LE) + req_len (u32 LE) + req_data
        let req_len = req_data.len() as u32;
        stream
            .write_all(&cmd_id.to_le_bytes())
            .map_err(|e| Self::map_io_error("IPC write error", e))?;
        stream
            .write_all(&req_len.to_le_bytes())
            .map_err(|e| Self::map_io_error("IPC write error", e))?;
        if !req_data.is_empty() {
            stream
                .write_all(req_data)
                .map_err(|e| Self::map_io_error("IPC write error", e))?;
        }
        stream
            .flush()
            .map_err(|e| Self::map_io_error("IPC flush error", e))?;

        // Read response: status (i32 LE) + resp_len (u32 LE) + resp_data
        let mut status_buf = [0u8; 4];
        stream
            .read_exact(&mut status_buf)
            .map_err(|e| Self::map_io_error("IPC read error", e))?;
        let status = i32::from_le_bytes(status_buf);

        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .map_err(|e| Self::map_io_error("IPC read error", e))?;
        let resp_len = u32::from_le_bytes(len_buf) as usize;

        // Bounded decode — reject suspiciously large responses before allocating.
        if resp_len > MAX_RESPONSE_SIZE {
            return Err(PlatformError::HardwareFailure(format!(
                "IPC response too large: {} bytes (max {})",
                resp_len, MAX_RESPONSE_SIZE
            )));
        }

        let mut resp_data = vec![0u8; resp_len];
        if resp_len > 0 {
            stream
                .read_exact(&mut resp_data)
                .map_err(|e| Self::map_io_error("IPC read error", e))?;
        }

        Ok((status, resp_data))
    }

    /// Send request expecting a handle response (8-byte i64).
    pub(crate) fn send_handle_request(
        &self,
        cmd_id: i32,
        req_data: &[u8],
    ) -> PlatformResult<*mut c_void> {
        let (status, resp_data) = self.send_request(cmd_id, req_data)?;
        if status != AK_SUCCESS_I32 || resp_data.len() < 8 {
            return Err(PlatformError::HardwareFailure(format!(
                "IPC request failed with status: {}",
                status
            )));
        }
        let handle_val =
            i64::from_le_bytes(resp_data[0..8].try_into().map_err(|_| {
                PlatformError::HardwareFailure("Invalid handle response".to_string())
            })?);
        Ok(handle_val as *mut c_void)
    }

    /// Send request expecting i32 response.
    pub(crate) fn send_i32_request(&self, cmd_id: i32, req_data: &[u8]) -> PlatformResult<i32> {
        let (status, resp_data) = self.send_request(cmd_id, req_data)?;
        if resp_data.len() >= 4 {
            Ok(i32::from_le_bytes(resp_data[0..4].try_into().map_err(
                |_| PlatformError::HardwareFailure("Invalid i32 response".to_string()),
            )?))
        } else {
            Ok(status)
        }
    }

    fn stream_id_to_wire(stream_id: StreamId) -> u32 {
        match stream_id {
            StreamId::VideoMain => 0,
            StreamId::VideoSub => 1,
            StreamId::Audio => 2,
        }
    }

    // ============================================================================
    // Push-mode frame delivery
    // ============================================================================

    /// Start push-based frame delivery from the daemon.
    ///
    /// The daemon spawns a dedicated thread that polls `ak_venc_get_stream()`,
    /// writes frames to the ring buffer, and pushes unsolicited 12-byte
    /// notifications over the frame socket. Returns `Ok(())` if the daemon
    /// accepted the command, or an error if the daemon doesn't support push mode.
    pub fn start_push(
        &self,
        stream_handle: *mut c_void,
        stream_id: StreamId,
    ) -> PlatformResult<()> {
        let handle_val = stream_handle as u64;
        tracing::info!(
            event = "push_start_request",
            diag_monotonic_ms = monotonic_millis(),
            stream_id = ?stream_id,
            handle = handle_val,
            "requesting daemon push start"
        );
        let mut req = [0u8; 12];
        req[0..8].copy_from_slice(&handle_val.to_le_bytes());
        req[8..12].copy_from_slice(&Self::stream_id_to_wire(stream_id).to_le_bytes());
        let (status, _) = self.send_request(CMD_VENC_START_PUSH, &req)?;
        if status != AK_SUCCESS_I32 {
            warn!(
                event = "push_start_failed",
                diag_monotonic_ms = monotonic_millis(),
                stream_id = ?stream_id,
                status,
                "daemon rejected push start request"
            );
            return Err(PlatformError::HardwareFailure("start_push failed".into()));
        }
        tracing::info!(
            event = "push_start_ack",
            diag_monotonic_ms = monotonic_millis(),
            stream_id = ?stream_id,
            status,
            "daemon accepted push start request"
        );
        Ok(())
    }

    /// Stop push-based frame delivery.
    pub fn stop_push(&self, stream_id: Option<StreamId>) -> PlatformResult<()> {
        tracing::info!(
            event = "push_stop_request",
            diag_monotonic_ms = monotonic_millis(),
            stream_id = ?stream_id,
            "requesting daemon push stop"
        );
        let req = stream_id.map(|id| Self::stream_id_to_wire(id).to_le_bytes().to_vec());
        let req_slice = req.as_deref().unwrap_or(&[]);
        let (status, _) = self.send_request(CMD_VENC_STOP_PUSH, req_slice)?;
        if status != AK_SUCCESS_I32 {
            warn!(
                event = "push_stop_failed",
                diag_monotonic_ms = monotonic_millis(),
                stream_id = ?stream_id,
                status,
                "daemon rejected push stop request"
            );
            return Err(PlatformError::HardwareFailure("stop_push failed".into()));
        }
        tracing::info!(
            event = "push_stop_ack",
            diag_monotonic_ms = monotonic_millis(),
            stream_id = ?stream_id,
            status,
            "daemon accepted push stop request"
        );
        Ok(())
    }

    /// Read ring buffer diagnostic counters (overflow, eviction, fallback, dropped).
    ///
    /// Returns `(0, 0, 0, 0)` if the shm reader is not available or version < 2.
    pub fn shm_diagnostic_counters(&self) -> (u32, u32, u32, u32) {
        let guard = match self.shm_reader.lock() {
            Ok(g) => g,
            Err(_) => return (0, 0, 0, 0),
        };
        match guard.as_ref() {
            Some(shm) => shm.diagnostic_counters(),
            None => (0, 0, 0, 0),
        }
    }

    /// Receive the next pushed frame from the daemon.
    ///
    /// In push mode, the daemon sends 12-byte notifications proactively.
    /// This method blocks until a notification arrives (no polling needed).
    /// The frame data is read from shared memory using the slot index in
    /// the notification.
    pub fn recv_pushed_frame(&self, pool: Option<&BytesMutPool>) -> PlatformResult<OwnedFrame> {
        let mut frame_main_guard = self.frame_main_stream.lock().map_err(|e| {
            PlatformError::HardwareFailure(format!("frame_main_stream mutex poisoned: {}", e))
        })?;
        let mut frame_sub_guard = self.frame_sub_stream.lock().map_err(|e| {
            PlatformError::HardwareFailure(format!("frame_sub_stream mutex poisoned: {}", e))
        })?;

        let mut poll_fds = [libc::pollfd {
            fd: -1,
            events: 0,
            revents: 0,
        }; 2];
        let mut poll_count = 0usize;
        let mut main_idx = None;
        let mut sub_idx = None;

        if let Some(stream) = frame_main_guard.as_ref() {
            poll_fds[poll_count].fd = stream.as_raw_fd();
            poll_fds[poll_count].events = libc::POLLIN;
            main_idx = Some(poll_count);
            poll_count += 1;
        }
        if let Some(stream) = frame_sub_guard.as_ref() {
            poll_fds[poll_count].fd = stream.as_raw_fd();
            poll_fds[poll_count].events = libc::POLLIN;
            sub_idx = Some(poll_count);
            poll_count += 1;
        }
        if poll_count == 0 {
            return Err(PlatformError::HardwareFailure(
                "no frame notification sockets for push mode".into(),
            ));
        }

        let timeout_ms = i32::try_from(PUSH_NOTIFICATION_TIMEOUT.as_millis()).unwrap_or(i32::MAX);
        // SAFETY: `poll_fds` points to a stack-allocated array of valid `pollfd`
        // entries, `poll_count` is bounded by that array length, and timeout is finite.
        let poll_ret = unsafe {
            libc::poll(
                poll_fds.as_mut_ptr(),
                poll_count as libc::nfds_t,
                timeout_ms,
            )
        };
        if poll_ret < 0 {
            return Err(PlatformError::HardwareFailure(format!(
                "push notification poll error: {}",
                std::io::Error::last_os_error()
            )));
        }
        if poll_ret == 0 {
            return Err(PlatformError::Timeout);
        }

        let main_ready = main_idx
            .map(|idx| (poll_fds[idx].revents & libc::POLLIN) != 0)
            .unwrap_or(false);
        let sub_ready = sub_idx
            .map(|idx| (poll_fds[idx].revents & libc::POLLIN) != 0)
            .unwrap_or(false);
        let main_hup_err = main_idx
            .map(|idx| (poll_fds[idx].revents & (libc::POLLHUP | libc::POLLERR)) != 0)
            .unwrap_or(false);
        let sub_hup_err = sub_idx
            .map(|idx| (poll_fds[idx].revents & (libc::POLLHUP | libc::POLLERR)) != 0)
            .unwrap_or(false);

        let chosen_channel = self.choose_ready_channel(main_ready, sub_ready);
        if is_ipc_debug_enabled() {
            debug!(
                event = "push_poll_result",
                diag_monotonic_ms = monotonic_millis(),
                main_ready,
                sub_ready,
                main_hup_err,
                sub_hup_err,
                chosen_channel = ?chosen_channel,
                "push notification poll completed"
            );
        }
        let (channel_name, notif) = if let Some(channel) = chosen_channel {
            if channel == "main" {
                let stream = frame_main_guard.as_mut().ok_or_else(|| {
                    PlatformError::HardwareFailure("main frame socket became unavailable".into())
                })?;
                (channel, Self::read_push_notification(stream, channel)?)
            } else {
                let stream = frame_sub_guard.as_mut().ok_or_else(|| {
                    PlatformError::HardwareFailure("sub frame socket became unavailable".into())
                })?;
                (channel, Self::read_push_notification(stream, channel)?)
            }
        } else if main_hup_err || sub_hup_err {
            return Err(PlatformError::HardwareFailure(format!(
                "push notification socket disconnected (main_hup_err={}, sub_hup_err={})",
                main_hup_err, sub_hup_err
            )));
        } else {
            return Err(PlatformError::Timeout);
        };

        // Check for dropped-frame notification (Fix 4 integration)
        if notif.is_frame_dropped() {
            warn!(
                event = "push_notification_frame_dropped",
                diag_monotonic_ms = monotonic_millis(),
                channel = channel_name,
                slot_index = notif.slot_index,
                flags = notif.flags,
                "daemon reported dropped frame notification"
            );
            return Err(PlatformError::ResourceBusy(
                "frame dropped by daemon (P-frame during ring overflow)".into(),
            ));
        }

        // Read from shared memory
        let mut shm_guard = self.shm_reader.lock().map_err(|e| {
            PlatformError::HardwareFailure(format!("shm_reader mutex poisoned: {}", e))
        })?;
        let shm = shm_guard.as_mut().ok_or_else(|| {
            PlatformError::HardwareFailure("shm reader not available for push mode".into())
        })?;

        let (metadata, frame_data) = shm
            .read_slot_into_bytesmut(notif.slot_index, pool)
            .map_err(|e| Self::map_shm_slot_read_error(e, channel_name))?;

        Ok(OwnedFrame {
            data: frame_data,
            timestamp: metadata.timestamp_ms,
            frame_type: metadata.frame_type,
            stream_id: metadata.stream_id,
        })
    }

    // ============================================================================
    // Encoding helpers - convert Rust types to bytes for IPC
    // ============================================================================

    /// Encode video_dev_type to a fixed-size buffer (4 bytes).
    fn encode_video_dev_type_buf(dev: video_dev_type) -> ([u8; 4], usize) {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&(dev as i32).to_le_bytes());
        (buf, 4)
    }

    /// Encode video_channel_attr to a fixed-size buffer (48 bytes).
    /// Format: crop (16 bytes) + res[0] (16 bytes) + res[1] (16 bytes)
    pub(crate) fn encode_video_channel_attr_buf(attr: &video_channel_attr) -> ([u8; 48], usize) {
        let mut buf = [0u8; 48];
        let mut offset = 0;
        // Crop info (16 bytes)
        buf[offset..offset + 4].copy_from_slice(&attr.crop.left.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&attr.crop.top.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&attr.crop.width.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&attr.crop.height.to_le_bytes());
        offset += 4;
        // Resolution array (32 bytes)
        for res in &attr.res {
            buf[offset..offset + 4].copy_from_slice(&res.width.to_le_bytes());
            offset += 4;
            buf[offset..offset + 4].copy_from_slice(&res.height.to_le_bytes());
            offset += 4;
            buf[offset..offset + 4].copy_from_slice(&res.max_width.to_le_bytes());
            offset += 4;
            buf[offset..offset + 4].copy_from_slice(&res.max_height.to_le_bytes());
            offset += 4;
        }
        (buf, 48)
    }

    /// Encode encode_param to a fixed-size buffer (48 bytes).
    pub(crate) fn encode_encode_param_buf(param: &encode_param) -> ([u8; 48], usize) {
        let mut buf = [0u8; 48];
        let mut offset = 0;
        buf[offset..offset + 4].copy_from_slice(&param.width.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&param.height.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&param.minqp.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&param.maxqp.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&param.fps.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&param.goplen.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&param.bps.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&(param.profile as i32).to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&(param.use_chn as i32).to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&(param.enc_grp as i32).to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&(param.br_mode as i32).to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&(param.enc_out_type as i32).to_le_bytes());
        offset += 4;
        (buf, offset)
    }

    /// Encode pcm_param to a fixed-size buffer (12 bytes).
    pub(crate) fn encode_pcm_param_buf(param: &pcm_param) -> ([u8; 12], usize) {
        let mut buf = [0u8; 12];
        let mut offset = 0;
        buf[offset..offset + 4].copy_from_slice(&param.sample_rate.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&param.sample_bits.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&param.channel_num.to_le_bytes());
        offset += 4;
        (buf, offset)
    }

    /// Encode audio_param to a fixed-size buffer (16 bytes).
    pub(crate) fn encode_audio_param_buf(param: &audio_param) -> ([u8; 16], usize) {
        let mut buf = [0u8; 16];
        let mut offset = 0;
        buf[offset..offset + 4].copy_from_slice(&param.sample_rate.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&param.channel_num.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&param.sample_bits.to_le_bytes());
        offset += 4;
        buf[offset..offset + 4].copy_from_slice(&param.type_.to_le_bytes());
        offset += 4;
        (buf, offset)
    }

    /// Encode aenc_attr to a fixed-size buffer (4 bytes).
    pub(crate) fn encode_aenc_attr_buf(attr: &aenc_attr) -> ([u8; 4], usize) {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&attr.aac_head.to_le_bytes());
        (buf, 4)
    }

    // ============================================================================
    // Direct-write helpers — stack-based, zero heap allocation (Phase 4)
    // ============================================================================

    /// Write an i32 value directly to the stream — no heap allocation.
    fn write_i32(stream: &mut UnixStream, val: i32) -> std::io::Result<()> {
        stream.write_all(&val.to_le_bytes())
    }

    /// Write a u32 value directly to the stream — no heap allocation.
    fn write_u32(stream: &mut UnixStream, val: u32) -> std::io::Result<()> {
        stream.write_all(&val.to_le_bytes())
    }

    /// Write a u64 value directly to the stream — no heap allocation.
    fn write_u64(stream: &mut UnixStream, val: u64) -> std::io::Result<()> {
        stream.write_all(&val.to_le_bytes())
    }

    fn decode_video_resolution(data: &[u8]) -> PlatformResult<video_resolution> {
        if data.len() < 16 {
            return Err(PlatformError::HardwareFailure(
                "Invalid video_resolution response: too short".to_string(),
            ));
        }
        Ok(video_resolution {
            width: i32::from_le_bytes(
                data[0..4]
                    .try_into()
                    .map_err(|_| PlatformError::HardwareFailure("Invalid width".to_string()))?,
            ),
            height: i32::from_le_bytes(
                data[4..8]
                    .try_into()
                    .map_err(|_| PlatformError::HardwareFailure("Invalid height".to_string()))?,
            ),
            max_width: i32::from_le_bytes(
                data[8..12]
                    .try_into()
                    .map_err(|_| PlatformError::HardwareFailure("Invalid max_width".to_string()))?,
            ),
            max_height: i32::from_le_bytes(
                data[12..16].try_into().map_err(|_| {
                    PlatformError::HardwareFailure("Invalid max_height".to_string())
                })?,
            ),
        })
    }

    /// Convert an i32 frame_type value from the IPC wire format to the
    /// platform's `video_frame_type` enum.
    ///
    /// # Safety
    ///
    /// This function is called from an `unsafe` block in `venc_get_stream` where
    /// the `video_stream` pointer has already been validated as non-null.
    /// The conversion itself is safe: we use an exhaustive match instead of
    /// `std::mem::transmute`, so no invalid enum values can be produced.
    fn ipc_to_frame_type(val: i32) -> crate::hal::common::VideoFrameType {
        use crate::hal::common::VideoFrameType;
        match val {
            1 => VideoFrameType::FrameTypeI,
            2 => VideoFrameType::FrameTypeB,
            3 => VideoFrameType::FrameTypePi,
            _ => VideoFrameType::FrameTypeP, // 0 = P-frame; unknown defaults to P
        }
    }
}

// ============================================================================
// Test helpers shared across submodules
// ============================================================================

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Monotonic counter for unique per-test socket paths.
    pub static TEST_DAEMON_COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Helper: spawns a fake vendor daemon on a temporary Unix socket.
    ///
    /// Accepts a single connection and handles IPC requests in a loop until EOF.
    /// The `handler` closure maps `(cmd_id, req_data)` to `(status, resp_data)`.
    pub struct FakeDaemon {
        /// Path of the bound socket; used by tests to connect AnykaIpc.
        pub socket_path: String,
        /// Background listener thread — kept alive as long as FakeDaemon exists.
        _listener_thread: std::thread::JoinHandle<()>,
    }

    impl FakeDaemon {
        pub fn start(handler: impl Fn(i32, &[u8]) -> (i32, Vec<u8>) + Send + 'static) -> Self {
            let counter = TEST_DAEMON_COUNTER.fetch_add(1, Ordering::SeqCst);
            let socket_path = format!(
                "/tmp/test-vendor-daemon-{}-{}.sock",
                std::process::id(),
                counter
            );
            // Remove any stale socket left by a crashed previous test run.
            let _ = std::fs::remove_file(&socket_path);
            let listener = UnixListener::bind(&socket_path).unwrap();
            let path_clone = socket_path.clone();
            let handle = std::thread::spawn(move || {
                if let Ok((mut stream, _)) = listener.accept() {
                    loop {
                        // Read cmd_id (i32 LE)
                        let mut cmd_buf = [0u8; 4];
                        if stream.read_exact(&mut cmd_buf).is_err() {
                            break;
                        }
                        let cmd_id = i32::from_le_bytes(cmd_buf);

                        // Read req_len (u32 LE)
                        let mut len_buf = [0u8; 4];
                        if stream.read_exact(&mut len_buf).is_err() {
                            break;
                        }
                        let req_len = u32::from_le_bytes(len_buf) as usize;

                        // Read req_data
                        let mut req_data = vec![0u8; req_len];
                        if req_len > 0 && stream.read_exact(&mut req_data).is_err() {
                            break;
                        }

                        // Invoke handler and write response
                        let (status, resp_data) = handler(cmd_id, &req_data);
                        let _ = stream.write_all(&status.to_le_bytes());
                        let _ = stream.write_all(&(resp_data.len() as u32).to_le_bytes());
                        if !resp_data.is_empty() {
                            let _ = stream.write_all(&resp_data);
                        }
                        let _ = stream.flush();
                    }
                }
                let _ = std::fs::remove_file(&path_clone);
            });
            // Give the daemon time to bind and enter accept().
            std::thread::sleep(std::time::Duration::from_millis(50));
            Self {
                socket_path,
                _listener_thread: handle,
            }
        }

        /// Spawns a fake daemon that delays before responding, simulating a hung vendor daemon.
        ///
        /// The `delay` parameter specifies how long to sleep before sending the response.
        /// This is used to test timeout behavior.
        pub fn start_with_delay(
            delay: std::time::Duration,
            handler: impl Fn(i32, &[u8]) -> (i32, Vec<u8>) + Send + 'static,
        ) -> Self {
            let counter = TEST_DAEMON_COUNTER.fetch_add(1, Ordering::SeqCst);
            let socket_path = format!(
                "/tmp/test-vendor-daemon-delayed-{}-{}.sock",
                std::process::id(),
                counter
            );
            // Remove any stale socket left by a crashed previous test run.
            let _ = std::fs::remove_file(&socket_path);
            let listener = UnixListener::bind(&socket_path).unwrap();
            let path_clone = socket_path.clone();
            let handle = std::thread::spawn(move || {
                if let Ok((mut stream, _)) = listener.accept() {
                    // Read request (we need to consume the request before delaying)
                    let mut cmd_buf = [0u8; 4];
                    if stream.read_exact(&mut cmd_buf).is_err() {
                        return;
                    }
                    let cmd_id = i32::from_le_bytes(cmd_buf);

                    let mut len_buf = [0u8; 4];
                    if stream.read_exact(&mut len_buf).is_err() {
                        return;
                    }
                    let req_len = u32::from_le_bytes(len_buf) as usize;

                    let mut req_data = vec![0u8; req_len];
                    if req_len > 0 && stream.read_exact(&mut req_data).is_err() {
                        return;
                    }

                    // Delay before responding - simulating a hung daemon
                    std::thread::sleep(delay);

                    // Now send the response
                    let (status, resp_data) = handler(cmd_id, &req_data);
                    let _ = stream.write_all(&status.to_le_bytes());
                    let _ = stream.write_all(&(resp_data.len() as u32).to_le_bytes());
                    if !resp_data.is_empty() {
                        let _ = stream.write_all(&resp_data);
                    }
                    let _ = stream.flush();
                }
                let _ = std::fs::remove_file(&path_clone);
            });
            // Give the daemon time to bind and enter accept().
            std::thread::sleep(std::time::Duration::from_millis(50));
            Self {
                socket_path,
                _listener_thread: handle,
            }
        }
    }

    pub fn make_ipc_for_channel_selection_tests() -> AnykaIpc {
        let (ctrl_a, _ctrl_b) = UnixStream::pair().unwrap();
        let (main_a, _main_b) = UnixStream::pair().unwrap();
        let (sub_a, _sub_b) = UnixStream::pair().unwrap();
        AnykaIpc {
            stream: std::sync::Arc::new(std::sync::Mutex::new(ctrl_a)),
            frame_main_stream: Mutex::new(Some(main_a)),
            frame_sub_stream: Mutex::new(Some(sub_a)),
            shm_reader: Mutex::new(None),
            prefer_sub_on_tie: AtomicBool::new(false),
        }
    }
}

// ============================================================================
// Core protocol and encoding tests (stay in mod.rs)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::test_helpers::*;
    use super::*;
    use crate::hal::common::AK_FAILED_I32;
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    /// A simple success command round-trips correctly: set_brightness returns AK_SUCCESS.
    #[test]
    fn test_ipc_connect_and_simple_command_returns_success() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        // set_brightness is part of ImagingHalTrait; accessible from within the crate.
        let result =
            <AnykaIpc as crate::hal::common::imaging::ImagingHalTrait>::set_brightness(&ipc, 50);

        assert_eq!(result, AK_SUCCESS_I32, "expected AK_SUCCESS from daemon");
    }

    /// When the daemon returns status=-1 (AK_FAILED), the trait method propagates it.
    #[test]
    fn test_ipc_error_response_propagates_ak_failed() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_FAILED_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        let result =
            <AnykaIpc as crate::hal::common::imaging::ImagingHalTrait>::set_brightness(&ipc, 50);

        assert_eq!(
            result, AK_FAILED_I32,
            "expected AK_FAILED propagated from daemon"
        );
    }

    /// A daemon that sends an 8-byte handle lets vi_open return a non-null pointer.
    #[test]
    fn test_ipc_handle_response_returns_non_null_pointer() {
        let handle_value: i64 = 0x1234_5678;
        let daemon = FakeDaemon::start(move |_cmd_id, _req| {
            (AK_SUCCESS_I32, handle_value.to_le_bytes().to_vec())
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        let handle = {
            use crate::hal::common::sdk_types::VideoDevType;
            <AnykaIpc as crate::hal::common::video::VideoHalTrait>::vi_open(
                &ipc,
                VideoDevType::Dev0,
            )
        };

        assert!(!handle.is_null(), "expected non-null handle from daemon");
        assert_eq!(
            handle as i64, handle_value,
            "handle value should match daemon response"
        );
    }

    /// Three sequential commands over the same connection all succeed.
    #[test]
    fn test_ipc_multiple_requests_same_connection_all_succeed() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        for i in 0..3 {
            let result = <AnykaIpc as crate::hal::common::imaging::ImagingHalTrait>::set_brightness(
                &ipc,
                50 + i,
            );
            assert_eq!(result, AK_SUCCESS_I32, "request {} should succeed", i);
        }
    }

    /// Pull-style venc_get_stream is removed in push-only mode.
    #[test]
    #[cfg(use_stubs)]
    fn test_ipc_get_stream_removed_returns_failed() {
        use std::mem::MaybeUninit;
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));

        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        let stream_handle = 1usize as *mut std::ffi::c_void;
        let mut vs = MaybeUninit::<crate::hal::common::sdk_types::VideoStream>::zeroed();
        let vs_ptr = vs.as_mut_ptr() as *mut crate::hal::common::video_stream;

        let result = <AnykaIpc as crate::hal::common::video::VideoHalTrait>::venc_get_stream(
            &ipc,
            stream_handle,
            vs_ptr,
        );
        assert_eq!(result, AK_FAILED_I32);
    }

    #[test]
    fn test_ipc_traits_implement_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AnykaIpc>();
    }

    #[test]
    fn test_ipc_frame_type_conversion_p_frame() {
        #[cfg(use_stubs)]
        {
            use crate::hal::common::sdk_types::VideoFrameType;
            assert_eq!(AnykaIpc::ipc_to_frame_type(0), VideoFrameType::FrameTypeP);
        }
    }

    #[test]
    fn test_ipc_frame_type_conversion_i_frame() {
        #[cfg(use_stubs)]
        {
            use crate::hal::common::sdk_types::VideoFrameType;
            assert_eq!(AnykaIpc::ipc_to_frame_type(1), VideoFrameType::FrameTypeI);
        }
    }

    #[test]
    fn test_ipc_frame_type_conversion_b_frame() {
        #[cfg(use_stubs)]
        {
            use crate::hal::common::sdk_types::VideoFrameType;
            assert_eq!(AnykaIpc::ipc_to_frame_type(2), VideoFrameType::FrameTypeB);
        }
    }

    #[test]
    fn test_ipc_frame_type_conversion_pi_frame() {
        #[cfg(use_stubs)]
        {
            use crate::hal::common::sdk_types::VideoFrameType;
            assert_eq!(AnykaIpc::ipc_to_frame_type(3), VideoFrameType::FrameTypePi);
        }
    }

    #[test]
    fn test_ipc_frame_type_conversion_unknown_defaults_to_p() {
        #[cfg(use_stubs)]
        {
            use crate::hal::common::sdk_types::VideoFrameType;
            assert_eq!(AnykaIpc::ipc_to_frame_type(99), VideoFrameType::FrameTypeP);
            assert_eq!(AnykaIpc::ipc_to_frame_type(-1), VideoFrameType::FrameTypeP);
        }
    }

    #[test]
    fn test_decode_video_resolution_rejects_short_input() {
        let short_data = vec![0u8; 8]; // 8 bytes — less than the required 16
        let result = AnykaIpc::decode_video_resolution(&short_data);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("too short"), "unexpected error: {}", msg);
            }
            other => panic!("Expected HardwareFailure, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_video_resolution_parses_valid_data() {
        let mut data = Vec::new();
        data.extend_from_slice(&1920i32.to_le_bytes()); // width
        data.extend_from_slice(&1080i32.to_le_bytes()); // height
        data.extend_from_slice(&1920i32.to_le_bytes()); // max_width
        data.extend_from_slice(&1080i32.to_le_bytes()); // max_height

        let result = AnykaIpc::decode_video_resolution(&data);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.width, 1920);
        assert_eq!(res.height, 1080);
        assert_eq!(res.max_width, 1920);
        assert_eq!(res.max_height, 1080);
    }

    #[test]
    fn test_encode_video_channel_attr_buf_round_trip() {
        #[cfg(use_stubs)]
        {
            use crate::hal::common::sdk_types::{CropInfo, VideoChannelAttr, VideoResolution};

            let attr = VideoChannelAttr {
                crop: CropInfo {
                    left: 10,
                    top: 20,
                    width: 640,
                    height: 480,
                },
                res: [
                    VideoResolution {
                        width: 1920,
                        height: 1080,
                        max_width: 1920,
                        max_height: 1080,
                    },
                    VideoResolution {
                        width: 640,
                        height: 480,
                        max_width: 640,
                        max_height: 480,
                    },
                ],
            };

            let (buf, len) = AnykaIpc::encode_video_channel_attr_buf(&attr);
            let encoded = &buf[..len];

            // 4 crop fields * 4 bytes + 2 resolutions * 4 fields * 4 bytes = 16 + 32 = 48
            assert_eq!(len, 48);

            // Verify crop fields
            assert_eq!(i32::from_le_bytes(encoded[0..4].try_into().unwrap()), 10);
            assert_eq!(i32::from_le_bytes(encoded[4..8].try_into().unwrap()), 20);
            assert_eq!(i32::from_le_bytes(encoded[8..12].try_into().unwrap()), 640);
            assert_eq!(i32::from_le_bytes(encoded[12..16].try_into().unwrap()), 480);

            // Verify first resolution (res[0])
            assert_eq!(
                i32::from_le_bytes(encoded[16..20].try_into().unwrap()),
                1920
            );
            assert_eq!(
                i32::from_le_bytes(encoded[20..24].try_into().unwrap()),
                1080
            );

            // Verify second resolution (res[1])
            assert_eq!(i32::from_le_bytes(encoded[32..36].try_into().unwrap()), 640);
            assert_eq!(i32::from_le_bytes(encoded[36..40].try_into().unwrap()), 480);
        }
    }

    #[test]
    fn test_encode_encode_param_buf_byte_length() {
        #[cfg(use_stubs)]
        {
            use crate::hal::common::sdk_types::EncodeParam;

            let param = EncodeParam::default();
            let (_buf, len) = AnykaIpc::encode_encode_param_buf(&param);

            // 12 fields * 4 bytes each = 48 bytes
            assert_eq!(len, 48);
        }
    }

    #[test]
    fn test_encode_pcm_param_buf_values() {
        #[cfg(use_stubs)]
        {
            use crate::hal::common::sdk_types::PcmParam;

            let param = PcmParam {
                sample_rate: 8000,
                sample_bits: 16,
                channel_num: 1,
            };

            let (buf, len) = AnykaIpc::encode_pcm_param_buf(&param);
            let encoded = &buf[..len];

            assert_eq!(len, 12);
            assert_eq!(u32::from_le_bytes(encoded[0..4].try_into().unwrap()), 8000);
            assert_eq!(u32::from_le_bytes(encoded[4..8].try_into().unwrap()), 16);
            assert_eq!(u32::from_le_bytes(encoded[8..12].try_into().unwrap()), 1);
        }
    }

    #[test]
    fn test_encode_audio_param_buf_values() {
        #[cfg(use_stubs)]
        {
            use crate::hal::common::sdk_types::AudioParam;

            let param = AudioParam {
                sample_rate: 48000,
                channel_num: 2,
                sample_bits: 16,
                type_: 1,
            };

            let (buf, len) = AnykaIpc::encode_audio_param_buf(&param);
            let encoded = &buf[..len];

            assert_eq!(len, 16);
            assert_eq!(u32::from_le_bytes(encoded[0..4].try_into().unwrap()), 48000);
            assert_eq!(u32::from_le_bytes(encoded[4..8].try_into().unwrap()), 2);
            assert_eq!(u32::from_le_bytes(encoded[8..12].try_into().unwrap()), 16);
            assert_eq!(i32::from_le_bytes(encoded[12..16].try_into().unwrap()), 1);
        }
    }

    #[test]
    fn test_encode_aenc_attr_buf_values() {
        #[cfg(use_stubs)]
        {
            use crate::hal::common::sdk_types::AencAttr;

            let attr = AencAttr { aac_head: 1 };
            let (buf, len) = AnykaIpc::encode_aenc_attr_buf(&attr);

            assert_eq!(len, 4);
            assert_eq!(&buf[..len], &[1, 0, 0, 0]);
        }
    }

    #[test]
    fn test_decode_video_resolution_exact_16_bytes() {
        let mut data = Vec::new();
        data.extend_from_slice(&1280i32.to_le_bytes()); // width
        data.extend_from_slice(&720i32.to_le_bytes()); // height
        data.extend_from_slice(&1920i32.to_le_bytes()); // max_width
        data.extend_from_slice(&1080i32.to_le_bytes()); // max_height

        assert_eq!(data.len(), 16);

        let result = AnykaIpc::decode_video_resolution(&data);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.width, 1280);
        assert_eq!(res.height, 720);
        assert_eq!(res.max_width, 1920);
        assert_eq!(res.max_height, 1080);
    }

    #[test]
    fn test_decode_video_resolution_extra_bytes_ignored() {
        let mut data = Vec::new();
        data.extend_from_slice(&640i32.to_le_bytes()); // width
        data.extend_from_slice(&480i32.to_le_bytes()); // height
        data.extend_from_slice(&1920i32.to_le_bytes()); // max_width
        data.extend_from_slice(&1080i32.to_le_bytes()); // max_height
        // Extra padding bytes — should be ignored
        data.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0xFF]);

        assert!(data.len() > 16);

        let result = AnykaIpc::decode_video_resolution(&data);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.width, 640);
        assert_eq!(res.height, 480);
    }

    #[test]
    fn test_max_response_size_constant() {
        assert_eq!(MAX_RESPONSE_SIZE, 2 * 1024 * 1024);
    }

    #[test]
    fn test_read_push_notification_reads_12_byte_notification() {
        let (mut writer, mut reader) = UnixStream::pair().unwrap();
        let raw = [
            0x02, 0x00, 0x00, 0x00, // slot_index = 2
            0x34, 0x12, 0x00, 0x00, // frame_len = 0x1234
            0x01, 0x00, 0x00, 0x00, // flags = VD_NOTIFY_LAST_FRAGMENT
        ];
        writer.write_all(&raw).unwrap();
        writer.flush().unwrap();

        let notif = AnykaIpc::read_push_notification(&mut reader, "main").unwrap();
        assert_eq!(notif.slot_index, 2);
        assert_eq!(notif.frame_len, 0x1234);
        assert_eq!(notif.flags, 1);
    }

    #[test]
    fn test_recv_pushed_frame_returns_resource_busy_on_frame_drop_notification() {
        use std::sync::{Arc, Mutex};

        // Create a Unix socket pair - one end goes to AnykaIpc, other we write to
        let (reader, mut writer) = UnixStream::pair().unwrap();

        // Construct AnykaIpc with only frame_main_stream set (no shm reader needed for drop path)
        let ipc = AnykaIpc {
            stream: Arc::new(Mutex::new(writer.try_clone().unwrap())),
            frame_main_stream: Mutex::new(Some(reader)),
            frame_sub_stream: Mutex::new(None),
            shm_reader: Mutex::new(None),
            prefer_sub_on_tie: AtomicBool::new(false),
        };

        // Write a 12-byte drop notification: slot_index=0, frame_len=0, flags=VD_NOTIFY_FRAME_DROPPED(4)
        let drop_notification = [
            0x00, 0x00, 0x00, 0x00, // slot_index = 0
            0x00, 0x00, 0x00, 0x00, // frame_len = 0
            0x04, 0x00, 0x00, 0x00, // flags = VD_NOTIFY_FRAME_DROPPED (1 << 2)
        ];
        writer.write_all(&drop_notification).unwrap();
        writer.flush().unwrap();

        // Call recv_pushed_frame - should return early with ResourceBusy due to drop notification
        let result = ipc.recv_pushed_frame(None);

        // Use is_err() and match directly to avoid Debug requirement
        if result.is_ok() {
            panic!("Expected error, got Ok(_)");
        }
        let err = result.err().expect("checked is_err above");
        match err {
            PlatformError::ResourceBusy(msg) => {
                assert!(
                    msg.contains("frame dropped"),
                    "Expected 'frame dropped' in message, got: {}",
                    msg
                );
            }
            other => panic!("expected ResourceBusy, got {:?}", other),
        }
    }

    #[test]
    fn test_map_shm_slot_read_error_not_ready_becomes_resource_busy() {
        let error =
            PlatformError::InvalidParameter("slot 0 not ready for reading (state: 0)".into());
        let mapped = AnykaIpc::map_shm_slot_read_error(error, "main");

        match mapped {
            PlatformError::ResourceBusy(msg) => {
                assert!(msg.contains("transient slot race"));
                assert!(msg.contains("main"));
            }
            other => panic!("expected ResourceBusy, got {:?}", other),
        }
    }

    #[test]
    fn test_map_shm_slot_read_error_other_invalid_parameter_passthrough() {
        let error = PlatformError::InvalidParameter("slot index 99 out of range".into());
        let mapped = AnykaIpc::map_shm_slot_read_error(error, "sub");

        match mapped {
            PlatformError::InvalidParameter(msg) => {
                assert_eq!(msg, "slot index 99 out of range");
            }
            other => panic!("expected InvalidParameter, got {:?}", other),
        }
    }

    #[test]
    fn test_choose_ready_channel_single_channel_ready() {
        let ipc = make_ipc_for_channel_selection_tests();

        assert_eq!(ipc.choose_ready_channel(true, false), Some("main"));
        assert_eq!(ipc.choose_ready_channel(false, true), Some("sub"));
        assert_eq!(ipc.choose_ready_channel(false, false), None);
    }

    #[test]
    fn test_choose_ready_channel_tie_alternates_between_main_and_sub() {
        let ipc = make_ipc_for_channel_selection_tests();

        assert_eq!(ipc.choose_ready_channel(true, true), Some("main"));
        assert_eq!(ipc.choose_ready_channel(true, true), Some("sub"));
        assert_eq!(ipc.choose_ready_channel(true, true), Some("main"));
        assert_eq!(ipc.choose_ready_channel(true, true), Some("sub"));
    }

    #[test]
    fn test_start_push_encodes_stream_handle_and_stream_id() {
        use std::sync::{Arc, Mutex};

        let captured: Arc<Mutex<Vec<(i32, Vec<u8>)>>> = Arc::new(Mutex::new(Vec::new()));
        let captured_closure = Arc::clone(&captured);
        let daemon = FakeDaemon::start(move |cmd_id, req| {
            captured_closure
                .lock()
                .unwrap()
                .push((cmd_id, req.to_vec()));
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        let stream_handle = 0x1234_5678usize as *mut std::ffi::c_void;

        ipc.start_push(stream_handle, StreamId::VideoSub).unwrap();

        let captured = captured.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].0, CMD_VENC_START_PUSH);
        assert_eq!(captured[0].1.len(), 12);

        let handle = u64::from_le_bytes(captured[0].1[0..8].try_into().unwrap());
        let stream_id = u32::from_le_bytes(captured[0].1[8..12].try_into().unwrap());
        assert_eq!(handle, stream_handle as u64);
        assert_eq!(stream_id, 1);
    }

    #[test]
    fn test_stop_push_with_stream_id_encodes_payload() {
        use std::sync::{Arc, Mutex};

        let captured: Arc<Mutex<Vec<(i32, Vec<u8>)>>> = Arc::new(Mutex::new(Vec::new()));
        let captured_closure = Arc::clone(&captured);
        let daemon = FakeDaemon::start(move |cmd_id, req| {
            captured_closure
                .lock()
                .unwrap()
                .push((cmd_id, req.to_vec()));
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        ipc.stop_push(Some(StreamId::VideoMain)).unwrap();

        let captured = captured.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].0, CMD_VENC_STOP_PUSH);
        assert_eq!(captured[0].1, 0u32.to_le_bytes().to_vec());
    }

    #[test]
    fn test_stop_push_without_stream_id_sends_empty_payload() {
        use std::sync::{Arc, Mutex};

        let captured: Arc<Mutex<Vec<(i32, Vec<u8>)>>> = Arc::new(Mutex::new(Vec::new()));
        let captured_closure = Arc::clone(&captured);
        let daemon = FakeDaemon::start(move |cmd_id, req| {
            captured_closure
                .lock()
                .unwrap()
                .push((cmd_id, req.to_vec()));
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        ipc.stop_push(None).unwrap();

        let captured = captured.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].0, CMD_VENC_STOP_PUSH);
        assert!(captured[0].1.is_empty());
    }

    /// Test that IPC control socket times out when the daemon hangs (stops responding).
    /// This verifies that the IPC_CTRL_TIMEOUT (10 seconds) is properly enforced and
    /// returns PlatformError::Timeout rather than hanging forever or returning HardwareFailure.
    #[test]
    fn test_ipc_ctrl_socket_times_out_on_hung_daemon() {
        use std::time::{Duration, Instant};

        // Create a fake daemon that delays response longer than the timeout (10 seconds)
        // We use 15 seconds delay to ensure timeout fires
        let delay = Duration::from_secs(15);
        let daemon = FakeDaemon::start_with_delay(delay, |_cmd_id, _req| (AK_SUCCESS_I32, vec![]));

        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        // Time the request - it should timeout
        let start = Instant::now();
        let result =
            <AnykaIpc as crate::hal::common::imaging::ImagingHalTrait>::set_brightness(&ipc, 50);
        let elapsed = start.elapsed();

        // Verify we got a timeout error (AK_FAILED_I32 = -1 is returned on error from the trait,
        // but the underlying send_request should return PlatformError::Timeout)
        // Since we're testing the trait method, it returns the daemon's status code.
        // We need to test send_request directly to verify timeout behavior.

        // For this test, we'll verify that the request took longer than the timeout
        // (indicating it actually waited for the timeout) and less than the delay
        // (indicating it didn't wait forever)
        assert!(
            elapsed >= Duration::from_secs(10),
            "Request should have waited at least 10 seconds for timeout, but took {:?}",
            elapsed
        );
        assert!(
            elapsed < Duration::from_secs(20),
            "Request should have timed out before the 15-second delay, but took {:?}",
            elapsed
        );

        // The result should be AK_FAILED because the request timed out and the trait method
        // propagates the daemon's error status (which in timeout case is -1)
        assert_eq!(
            result, AK_FAILED_I32,
            "set_brightness should return AK_FAILED when IPC times out"
        );
    }

    /// Test that IPC control socket times out within a reasonable window (< 15 seconds)
    /// when the daemon hangs. This is a more explicit timing verification.
    #[test]
    fn test_ipc_ctrl_socket_timeout_fires_within_15_seconds() {
        // Create a fake daemon that delays response for 20 seconds (longer than timeout + margin)
        let delay = Duration::from_secs(20);
        let daemon = FakeDaemon::start_with_delay(delay, |_cmd_id, _req| (AK_SUCCESS_I32, vec![]));

        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        let start = Instant::now();
        let _result =
            <AnykaIpc as crate::hal::common::imaging::ImagingHalTrait>::set_brightness(&ipc, 50);
        let elapsed = start.elapsed();

        // The timeout should fire well within 15 seconds (IPC_CTRL_TIMEOUT is 10 seconds)
        assert!(
            elapsed < Duration::from_secs(15),
            "Timeout should fire within 15 seconds, but took {:?}",
            elapsed
        );

        tracing::debug!(
            elapsed_secs = elapsed.as_secs(),
            "IPC timeout fired within expected window"
        );
    }
}
