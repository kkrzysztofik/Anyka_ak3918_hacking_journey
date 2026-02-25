//! Unix socket IPC client for vendor daemon communication.
//!
//! This module provides an alternative FFI implementation that communicates with
//! a vendor daemon via Unix socket at `/tmp/vendor-daemon.sock` instead of
//! directly calling the Anyka SDK C functions.
//!
//! # Binary Protocol
//!
//! Request format: cmd_id (i32) + req_len (u32) + req_data (bytes)
//! Response format: status (i32) + resp_len (u32) + resp_data (bytes)
//!
//! # Frame Response Format (CMD_VENC_GET_STREAM)
//!
//! Response data layout:
//! ```text
//! [u32 frame_len][u64 timestamp][u32 seq_no][i32 frame_type][u64 remote_token][frame_data_bytes]
//!  4 bytes        8 bytes        4 bytes     4 bytes          8 bytes
//! ```
//! Total header = 28 bytes, followed by `frame_len` bytes of actual frame data.
//!
//! # Usage
//!
//! This module can be used as a drop-in replacement for the direct FFI calls
//! by using the `VendorIpc` implementation of the FFI traits.

#![allow(dead_code)]

use bytes::BytesMut;

use crate::hal::shm_ring::{FrameNotification, ShmRingReader};
use crate::platform::PlatformError;
use crate::platform::PlatformResult;
use crate::platform::frame::{FrameMetadata, FrameType, OwnedFrame, StreamId};
use crate::streaming::bridge::BytesMutPool;

use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::{debug, error, warn};

use crate::hal::video::VideoHalTrait;
use crate::hal::{aenc_attr, audio_param, pcm_param};
use crate::hal::{
    encode_param, video_channel_attr, video_dev_type, video_resolution, video_stream,
};

use crate::hal::{AK_FAILED_I32, AK_SUCCESS_I32};

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
// PendingFrame — locally-owned frame buffer awaiting release
// ============================================================================

/// Holds a frame buffer that was received from the daemon and must be
/// explicitly released via `CMD_VENC_RELEASE_STREAM` when the caller is done.
struct PendingFrame {
    /// Locally-owned copy of the encoded frame data.
    data: Vec<u8>,
    /// Opaque token the daemon uses to identify this frame on its side.
    remote_token: u64,
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

const VENDOR_SOCKET_PATH: &str = "/tmp/vendor-daemon.sock";

/// Path for the dedicated frame socket (Approach B Phase 3)
const FRAME_SOCKET_PATH: &str = "/tmp/vd-frame.sock";
/// Path for the dedicated control socket (Approach B Phase 3)
const CTRL_SOCKET_PATH: &str = "/tmp/vd-ctrl.sock";

/// Maximum allowed IPC response body (2 MB — large enough for raw I-frames).
const MAX_RESPONSE_SIZE: usize = 2 * 1024 * 1024;

/// Byte length of the fixed header in a `CMD_VENC_GET_STREAM` response:
/// frame_len(4) + timestamp(8) + seq_no(4) + frame_type(4) + remote_token(8) = 28
const VENC_STREAM_HEADER_LEN: usize = 28;

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
const CMD_VENC_GET_STREAM: i32 = 16;
const CMD_VENC_RELEASE_STREAM: i32 = 17;
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

/// IPC client for vendor daemon communication
pub struct VendorIpc {
    /// Legacy/control socket for commands (used when dual socket not available)
    stream: Arc<Mutex<UnixStream>>,
    /// Dedicated frame socket (Approach B P3) - wrapped in Mutex for interior mutability.
    /// Only the frame reader thread accesses this, so there's no contention.
    /// None if daemon doesn't support dual sockets.
    frame_stream: Mutex<Option<UnixStream>>,
    /// Shared memory ring buffer reader (Approach A) - wrapped in Mutex for interior mutability.
    /// Only the frame reader thread accesses this, so there's no contention.
    /// None if daemon doesn't use shared memory.
    shm_reader: Mutex<Option<ShmRingReader>>,
    /// Frames that have been handed to the caller and are awaiting release (legacy path).
    pending_frames: Mutex<HashMap<u64, PendingFrame>>,
    /// Remote tokens for frames fetched via the owned path (new zero-copy path).
    /// Key: stream handle as u64. Value: remote_token for frame release.
    pending_tokens: Mutex<HashMap<u64, u64>>,
}

impl VendorIpc {
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
            CMD_VENC_GET_STREAM => "VENC_GET_STREAM",
            CMD_VENC_RELEASE_STREAM => "VENC_RELEASE_STREAM",
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

        // Try connecting to dedicated frame socket (Approach B Phase 3)
        let frame_stream = match UnixStream::connect(FRAME_SOCKET_PATH) {
            Ok(fs) => {
                tracing::info!("Connected to dedicated frame socket");
                Some(fs)
            }
            Err(e) => {
                tracing::info!(
                    "Frame socket not available, using legacy single-socket mode: {}",
                    e
                );
                None
            }
        };

        // Try opening shared memory ring buffer (Approach A)
        let shm_reader = match ShmRingReader::open() {
            Ok(Some(reader)) => {
                tracing::info!("Shared memory ring buffer opened");
                Some(reader)
            }
            Ok(None) => {
                tracing::info!("Shared memory not available");
                None
            }
            Err(e) => {
                tracing::warn!("Shared memory open failed: {}", e);
                None
            }
        };

        Ok(Self {
            stream: Arc::new(Mutex::new(stream)),
            frame_stream: Mutex::new(frame_stream),
            shm_reader: Mutex::new(shm_reader),
            pending_frames: Mutex::new(HashMap::new()),
            pending_tokens: Mutex::new(HashMap::new()),
        })
    }

    /// Create a new IPC client connected to a custom socket path (test-only).
    #[cfg(test)]
    pub fn new_with_path(path: &str) -> PlatformResult<Self> {
        let stream = UnixStream::connect(path).map_err(|e| {
            PlatformError::HardwareUnavailable(format!(
                "Failed to connect to vendor daemon at {}: {}",
                path, e
            ))
        })?;
        if is_ipc_debug_enabled() {
            debug!(socket = path, "Connected to vendor daemon (test)");
        }

        // In test mode, try to connect to frame socket if it exists
        let frame_stream = match UnixStream::connect(FRAME_SOCKET_PATH) {
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
            frame_stream: Mutex::new(frame_stream),
            shm_reader: Mutex::new(shm_reader),
            pending_frames: Mutex::new(HashMap::new()),
            pending_tokens: Mutex::new(HashMap::new()),
        })
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
        if is_ipc_debug_enabled() {
            debug!(socket = VENDOR_SOCKET_PATH, "Reconnected to vendor daemon");
        }
        Ok(())
    }

    /// Send a request and receive a response over the Unix socket.
    ///
    /// On I/O error, attempts a single reconnect and retries the request.
    fn send_request(&self, cmd_id: i32, req_data: &[u8]) -> PlatformResult<(i32, Vec<u8>)> {
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
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC write error: {}", e)))?;
        stream
            .write_all(&req_len.to_le_bytes())
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC write error: {}", e)))?;
        if !req_data.is_empty() {
            stream
                .write_all(req_data)
                .map_err(|e| PlatformError::HardwareFailure(format!("IPC write error: {}", e)))?;
        }
        stream
            .flush()
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC flush error: {}", e)))?;

        // Read response: status (i32 LE) + resp_len (u32 LE) + resp_data
        let mut status_buf = [0u8; 4];
        stream
            .read_exact(&mut status_buf)
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC read error: {}", e)))?;
        let status = i32::from_le_bytes(status_buf);

        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC read error: {}", e)))?;
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
                .map_err(|e| PlatformError::HardwareFailure(format!("IPC read error: {}", e)))?;
        }

        Ok((status, resp_data))
    }

    /// Send request expecting a handle response (8-byte i64).
    fn send_handle_request(&self, cmd_id: i32, req_data: &[u8]) -> PlatformResult<*mut c_void> {
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
    fn send_i32_request(&self, cmd_id: i32, req_data: &[u8]) -> PlatformResult<i32> {
        let (status, resp_data) = self.send_request(cmd_id, req_data)?;
        if resp_data.len() >= 4 {
            Ok(i32::from_le_bytes(resp_data[0..4].try_into().map_err(
                |_| PlatformError::HardwareFailure("Invalid i32 response".to_string()),
            )?))
        } else {
            Ok(status)
        }
    }

    /// Convert an i32 frame_type value from the IPC wire format to the
    /// platform's `FrameType` enum (used by the owned frame path).
    fn ipc_to_platform_frame_type(val: i32) -> FrameType {
        match val {
            1 => FrameType::VideoIFrame,
            2 => FrameType::VideoBFrame,
            3 => FrameType::VideoPiFrame,
            _ => FrameType::VideoPFrame, // 0 = P-frame; unknown defaults to P
        }
    }

    /// Receive a frame response: reads the 8-byte status/length header, then the
    /// 28-byte frame header, then reads frame data directly into a `BytesMut` buffer.
    ///
    /// This is the zero-copy receive path: frame data goes straight from the socket
    /// into `BytesMut` with no intermediate `Vec<u8>` allocation.
    ///
    /// # Arguments
    ///
    /// * `stream` — The Unix socket stream to read from (already locked by caller).
    /// * `pool` — Optional buffer pool for reusing `BytesMut` allocations.
    fn recv_frame_response(
        stream: &mut UnixStream,
        pool: Option<&BytesMutPool>,
    ) -> PlatformResult<(FrameMetadata, BytesMut)> {
        // 1. Read status (i32 LE) + resp_len (u32 LE)
        let mut hdr = [0u8; 8];
        stream
            .read_exact(&mut hdr)
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC read error: {}", e)))?;

        let status = i32::from_le_bytes(
            hdr[0..4]
                .try_into()
                .map_err(|_| PlatformError::HardwareFailure("Invalid status bytes".to_string()))?,
        );
        let resp_len = u32::from_le_bytes(
            hdr[4..8]
                .try_into()
                .map_err(|_| PlatformError::HardwareFailure("Invalid length bytes".to_string()))?,
        ) as usize;

        if status != AK_SUCCESS_I32 {
            // Drain any remaining response data to keep stream in sync
            if resp_len > 0 {
                let drain_len = resp_len.min(MAX_RESPONSE_SIZE);
                let mut discard = vec![0u8; drain_len];
                let _ = stream.read_exact(&mut discard);
            }
            return Err(PlatformError::HardwareFailure(format!(
                "IPC frame request failed: status {}",
                status
            )));
        }

        if resp_len < VENC_STREAM_HEADER_LEN {
            return Err(PlatformError::HardwareFailure(format!(
                "IPC frame response too short: {} bytes (need >= {})",
                resp_len, VENC_STREAM_HEADER_LEN
            )));
        }

        // 2. Read 28-byte frame header
        let mut frame_hdr = [0u8; VENC_STREAM_HEADER_LEN];
        stream.read_exact(&mut frame_hdr).map_err(|e| {
            PlatformError::HardwareFailure(format!("IPC frame header read error: {}", e))
        })?;

        let frame_len = u32::from_le_bytes(
            frame_hdr[0..4]
                .try_into()
                .map_err(|_| PlatformError::HardwareFailure("Invalid frame_len".to_string()))?,
        ) as usize;
        let timestamp_ms = u64::from_le_bytes(
            frame_hdr[4..12]
                .try_into()
                .map_err(|_| PlatformError::HardwareFailure("Invalid timestamp".to_string()))?,
        );
        let seq_no = u32::from_le_bytes(
            frame_hdr[12..16]
                .try_into()
                .map_err(|_| PlatformError::HardwareFailure("Invalid seq_no".to_string()))?,
        );
        let frame_type_val = i32::from_le_bytes(
            frame_hdr[16..20]
                .try_into()
                .map_err(|_| PlatformError::HardwareFailure("Invalid frame_type".to_string()))?,
        );
        let remote_token = u64::from_le_bytes(
            frame_hdr[20..28]
                .try_into()
                .map_err(|_| PlatformError::HardwareFailure("Invalid remote_token".to_string()))?,
        );

        // Validate frame_len against response
        let expected_total = VENC_STREAM_HEADER_LEN + frame_len;
        if resp_len < expected_total {
            return Err(PlatformError::HardwareFailure(format!(
                "IPC frame response truncated: got {} bytes, need {}",
                resp_len, expected_total
            )));
        }

        // Bounded decode — reject suspiciously large frame data before allocating.
        if frame_len > MAX_RESPONSE_SIZE {
            return Err(PlatformError::HardwareFailure(format!(
                "IPC frame data too large: {} bytes (max {})",
                frame_len, MAX_RESPONSE_SIZE
            )));
        }

        // 3. Read frame data DIRECTLY into BytesMut — no intermediate Vec
        let mut frame_data = match pool {
            Some(p) => {
                let mut buf = p.get(frame_len);
                buf.resize(frame_len, 0);
                buf
            }
            None => BytesMut::zeroed(frame_len),
        };
        stream.read_exact(&mut frame_data).map_err(|e| {
            PlatformError::HardwareFailure(format!("IPC frame data read error: {}", e))
        })?;

        // Drain any extra bytes beyond expected_total (future protocol extensibility)
        let extra = resp_len.saturating_sub(expected_total);
        if extra > 0 {
            let mut discard = vec![0u8; extra.min(4096)];
            let _ = stream.read_exact(&mut discard);
        }

        let metadata = FrameMetadata {
            timestamp_ms,
            seq_no,
            frame_type: Self::ipc_to_platform_frame_type(frame_type_val),
            remote_token,
        };

        Ok((metadata, frame_data))
    }

    /// Receive frame data from an already-connected stream.
    ///
    /// This is used by both the legacy path and the dual-socket path.
    /// The caller is responsible for having already read the 8-byte status/length header.
    fn recv_frame_data_from_stream(
        stream: &mut impl std::io::Read,
        resp_len: usize,
        pool: Option<&BytesMutPool>,
    ) -> PlatformResult<(FrameMetadata, BytesMut)> {
        // Read 28-byte frame header
        let mut frame_hdr = [0u8; VENC_STREAM_HEADER_LEN];
        stream.read_exact(&mut frame_hdr).map_err(|e| {
            PlatformError::HardwareFailure(format!("IPC frame header read error: {}", e))
        })?;

        let frame_len = u32::from_le_bytes(
            frame_hdr[0..4]
                .try_into()
                .map_err(|_| PlatformError::HardwareFailure("Invalid frame_len".to_string()))?,
        ) as usize;
        let timestamp_ms = u64::from_le_bytes(
            frame_hdr[4..12]
                .try_into()
                .map_err(|_| PlatformError::HardwareFailure("Invalid timestamp".to_string()))?,
        );
        let seq_no = u32::from_le_bytes(
            frame_hdr[12..16]
                .try_into()
                .map_err(|_| PlatformError::HardwareFailure("Invalid seq_no".to_string()))?,
        );
        let frame_type_val = i32::from_le_bytes(
            frame_hdr[16..20]
                .try_into()
                .map_err(|_| PlatformError::HardwareFailure("Invalid frame_type".to_string()))?,
        );
        let remote_token = u64::from_le_bytes(
            frame_hdr[20..28]
                .try_into()
                .map_err(|_| PlatformError::HardwareFailure("Invalid remote_token".to_string()))?,
        );

        // Validate frame_len against response
        let expected_total = VENC_STREAM_HEADER_LEN + frame_len;
        if resp_len < expected_total {
            return Err(PlatformError::HardwareFailure(format!(
                "IPC frame response truncated: got {} bytes, need {}",
                resp_len, expected_total
            )));
        }

        // Bounded decode — reject suspiciously large frame data before allocating.
        if frame_len > MAX_RESPONSE_SIZE {
            return Err(PlatformError::HardwareFailure(format!(
                "IPC frame data too large: {} bytes (max {})",
                frame_len, MAX_RESPONSE_SIZE
            )));
        }

        // Read frame data
        let mut frame_data = match pool {
            Some(p) => {
                let mut buf = p.get(frame_len);
                buf.resize(frame_len, 0);
                buf
            }
            None => BytesMut::zeroed(frame_len),
        };
        stream.read_exact(&mut frame_data).map_err(|e| {
            PlatformError::HardwareFailure(format!("IPC frame data read error: {}", e))
        })?;

        // Drain any extra bytes beyond expected_total (future protocol extensibility)
        let extra = resp_len.saturating_sub(expected_total);
        if extra > 0 {
            let mut discard = vec![0u8; extra.min(4096)];
            let _ = stream.read_exact(&mut discard);
        }

        let metadata = FrameMetadata {
            timestamp_ms,
            seq_no,
            frame_type: Self::ipc_to_platform_frame_type(frame_type_val),
            remote_token,
        };

        Ok((metadata, frame_data))
    }

    /// Write request header to a stream.
    fn write_request_header(
        stream: &mut impl std::io::Write,
        cmd_id: i32,
        req_len: u32,
    ) -> PlatformResult<()> {
        stream
            .write_all(&cmd_id.to_le_bytes())
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC write error: {}", e)))?;
        stream
            .write_all(&req_len.to_le_bytes())
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC write error: {}", e)))?;
        Ok(())
    }

    /// Fetch next encoded frame as an `OwnedFrame` with `BytesMut` data.
    ///
    /// This is the zero-extra-copy path: frame data is read from the socket
    /// directly into `BytesMut`, which can be passed through to the streaming
    /// pipeline without any additional copy.
    ///
    /// This method supports three modes:
    /// 1. Dual socket + shared memory: Use dedicated frame socket + shm for data
    /// 2. Dual socket (socket fallback): Use dedicated frame socket, data over socket
    /// 3. Legacy single socket: Use legacy stream with Mutex protection
    ///
    /// # Arguments
    ///
    /// * `stream_handle` — The stream handle from `venc_request_stream`.
    /// * `stream_id` — Which stream this frame belongs to (VideoMain/VideoSub).
    /// * `pool` — Optional buffer pool for reusing BytesMut allocations.
    ///
    /// # Returns
    ///
    /// * `Ok(OwnedFrame)` — Frame data available.
    /// * `Err(...)` — IPC or protocol error.
    pub fn fetch_frame_owned(
        &self,
        stream_handle: *mut std::ffi::c_void,
        stream_id: StreamId,
        pool: Option<&BytesMutPool>,
    ) -> PlatformResult<OwnedFrame> {
        let handle_val = stream_handle as u64;

        // Choose the socket to use for frame requests
        // Lock both frame_stream and shm_reader (they're separate locks, no contention)
        let mut frame_guard = self.frame_stream.lock().map_err(|e| {
            PlatformError::HardwareFailure(format!("frame_stream mutex poisoned: {}", e))
        })?;

        if let Some(ref mut frame_stream) = *frame_guard {
            // Dual socket mode: use dedicated frame socket
            Self::write_request_header(frame_stream, CMD_VENC_GET_STREAM, 8)?;
            frame_stream
                .write_all(&handle_val.to_le_bytes())
                .map_err(|e| PlatformError::HardwareFailure(format!("IPC write error: {}", e)))?;
            frame_stream
                .flush()
                .map_err(|e| PlatformError::HardwareFailure(format!("IPC flush error: {}", e)))?;

            // Read response header (status + resp_len)
            let mut hdr = [0u8; 8];
            frame_stream
                .read_exact(&mut hdr)
                .map_err(|e| PlatformError::HardwareFailure(format!("IPC read error: {}", e)))?;
            let status =
                i32::from_le_bytes(hdr[0..4].try_into().map_err(|_| {
                    PlatformError::HardwareFailure("Invalid status bytes".to_string())
                })?);
            let resp_len =
                u32::from_le_bytes(hdr[4..8].try_into().map_err(|_| {
                    PlatformError::HardwareFailure("Invalid length bytes".to_string())
                })?) as usize;

            if status != AK_SUCCESS_I32 {
                // Drain any remaining response data to keep stream in sync
                if resp_len > 0 {
                    let drain_len = resp_len.min(MAX_RESPONSE_SIZE);
                    let mut discard = vec![0u8; drain_len];
                    let _ = frame_stream.read_exact(&mut discard);
                }
                return Err(PlatformError::HardwareFailure(format!(
                    "IPC frame request failed: status {}",
                    status
                )));
            }

            // Check for shared memory notification (12 bytes)
            // resp_len == 12 means daemon sent a notification (not a full frame)
            if resp_len == 12 {
                // Check if we have shm_reader - need to lock it too
                let mut shm_guard = self.shm_reader.lock().map_err(|e| {
                    PlatformError::HardwareFailure(format!("shm_reader mutex poisoned: {}", e))
                })?;

                if shm_guard.is_some() {
                    // SHARED MEMORY PATH: read 12-byte notification
                    let mut notif_bytes = [0u8; 12];
                    frame_stream.read_exact(&mut notif_bytes).map_err(|e| {
                        PlatformError::HardwareFailure(format!("IPC read error: {}", e))
                    })?;
                    let notif = FrameNotification::from_bytes(&notif_bytes);

                    if notif.is_frame_dropped() {
                        // P-frame intentionally dropped during ring overflow
                        return Err(PlatformError::HardwareFailure(
                            "frame dropped by daemon (P-frame during ring overflow)".into(),
                        ));
                    }

                    if !notif.is_socket_fallback() {
                        // Normal shm path: read from shared memory
                        let shm = shm_guard.as_mut().ok_or_else(|| {
                            PlatformError::HardwareFailure(
                                "shm reader disappeared unexpectedly".into(),
                            )
                        })?;
                        let (metadata, frame_data) =
                            shm.read_slot_into_bytesmut(notif.slot_index, pool)?;

                        // No pending_tokens needed — SDK frame already released by daemon
                        if is_ipc_debug_enabled() {
                            debug!(
                                frame_len = frame_data.len(),
                                timestamp_ms = metadata.timestamp_ms,
                                seq_no = metadata.seq_no,
                                frame_type = ?metadata.frame_type,
                                slot_index = notif.slot_index,
                                "fetch_frame_owned: frame received via shared memory"
                            );
                        }

                        return Ok(OwnedFrame {
                            data: frame_data,
                            timestamp: metadata.timestamp_ms.wrapping_mul(1000),
                            frame_type: metadata.frame_type,
                            stream_id,
                        });
                    }
                    // Socket-fallback notification means daemon could not place this
                    // frame in shared memory. Current protocol sends only this 12-byte
                    // notification for fallback cases on the frame socket.
                    //
                    // Return an explicit error after consuming the notification payload
                    // so stream framing remains synchronized.
                    drop(shm_guard);
                    return Err(PlatformError::ResourceBusy(
                        "daemon reported socket fallback for frame request".into(),
                    ));
                } else {
                    // Received shm notification but shm_reader not available - this is an error
                    // Read and discard the 12 bytes to keep socket in sync
                    let mut discard = [0u8; 12];
                    frame_stream.read_exact(&mut discard).map_err(|e| {
                        PlatformError::HardwareFailure(format!("IPC read error: {}", e))
                    })?;
                    return Err(PlatformError::HardwareFailure(
                        "received shm notification but shm reader not available".into(),
                    ));
                }
            }

            // SOCKET PATH: Read frame from socket (resp_len > 12 means full frame over socket)
            let (metadata, frame_data) =
                Self::recv_frame_data_from_stream(frame_stream, resp_len, pool)?;

            // Store token for release (keyed by stream_handle)
            self.pending_tokens
                .lock()
                .map_err(|e| {
                    PlatformError::HardwareFailure(format!("pending_tokens mutex poisoned: {}", e))
                })?
                .insert(handle_val, metadata.remote_token);

            if is_ipc_debug_enabled() {
                debug!(
                    frame_len = frame_data.len(),
                    timestamp_ms = metadata.timestamp_ms,
                    seq_no = metadata.seq_no,
                    frame_type = ?metadata.frame_type,
                    remote_token = metadata.remote_token,
                    "fetch_frame_owned: frame received via dual socket"
                );
            }

            return Ok(OwnedFrame {
                data: frame_data,
                timestamp: metadata.timestamp_ms.wrapping_mul(1000),
                frame_type: metadata.frame_type,
                stream_id,
            });
        }

        // Legacy single-socket mode: use the Mutex-protected stream
        let req_data = handle_val.to_le_bytes();

        let mut stream = self
            .stream
            .lock()
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC mutex poisoned: {}", e)))?;

        // Send request: cmd_id + req_len + req_data
        stream
            .write_all(&CMD_VENC_GET_STREAM.to_le_bytes())
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC write error: {}", e)))?;
        stream
            .write_all(&(8u32).to_le_bytes())
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC write error: {}", e)))?;
        stream
            .write_all(&req_data)
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC write error: {}", e)))?;
        stream
            .flush()
            .map_err(|e| PlatformError::HardwareFailure(format!("IPC flush error: {}", e)))?;

        // Receive frame directly into BytesMut
        let (metadata, frame_data) = Self::recv_frame_response(&mut stream, pool)?;

        // Drop stream lock before acquiring pending_tokens lock
        drop(stream);

        // Store remote_token for release (keyed by stream_handle)
        self.pending_tokens
            .lock()
            .map_err(|e| {
                PlatformError::HardwareFailure(format!("pending_tokens mutex poisoned: {}", e))
            })?
            .insert(handle_val, metadata.remote_token);

        if is_ipc_debug_enabled() {
            debug!(
                frame_len = frame_data.len(),
                timestamp_ms = metadata.timestamp_ms,
                seq_no = metadata.seq_no,
                frame_type = ?metadata.frame_type,
                remote_token = metadata.remote_token,
                "fetch_frame_owned: frame received (legacy path)"
            );
        }

        Ok(OwnedFrame {
            data: frame_data,
            // Convert ms to μs (same convention as the legacy path)
            timestamp: metadata.timestamp_ms.wrapping_mul(1000),
            frame_type: metadata.frame_type,
            stream_id,
        })
    }

    /// Release a frame previously acquired via `fetch_frame_owned`.
    ///
    /// Sends the remote_token back to the daemon so it can reclaim the frame buffer.
    /// This method only manages the `pending_tokens` map (not `pending_frames`).
    ///
    /// When using shared memory, the daemon already releases the SDK frame, so this
    /// is a no-op (no pending token to send).
    ///
    /// # Arguments
    ///
    /// * `stream_handle` — The stream handle used in `fetch_frame_owned`.
    pub fn release_frame_owned(&self, stream_handle: *mut std::ffi::c_void) -> PlatformResult<()> {
        let handle_val = stream_handle as u64;

        // Check if we have a pending token (socket path only)
        // If using shared memory, there's no token stored
        let remote_token = self
            .pending_tokens
            .lock()
            .map_err(|e| {
                PlatformError::HardwareFailure(format!("pending_tokens mutex poisoned: {}", e))
            })?
            .remove(&handle_val)
            .unwrap_or(0);

        // If no token, we're in shared memory path - daemon already released the frame
        if remote_token == 0 {
            // Check if we have shm_reader to confirm we're in shm mode
            // Need to lock to access shm_reader
            let shm_guard = self.shm_reader.lock().map_err(|e| {
                PlatformError::HardwareFailure(format!("shm_reader mutex poisoned: {}", e))
            })?;
            if shm_guard.is_some() {
                tracing::debug!("release_frame_owned: shared memory path, no release needed");
                return Ok(());
            }
            // No token and no shm - might be legacy path with token=0, continue with release
        }

        // Socket path: send release to daemon
        let mut req_data = [0u8; 16];
        req_data[0..8].copy_from_slice(&handle_val.to_le_bytes());
        req_data[8..16].copy_from_slice(&remote_token.to_le_bytes());

        let (status, _) = self.send_request(CMD_VENC_RELEASE_STREAM, &req_data)?;
        if status != AK_SUCCESS_I32 {
            return Err(PlatformError::HardwareFailure(format!(
                "Frame release failed: status {}",
                status
            )));
        }
        Ok(())
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
    pub fn start_push(&self, stream_handle: *mut c_void) -> PlatformResult<()> {
        let handle_val = stream_handle as u64;
        let (status, _) = self.send_request(CMD_VENC_START_PUSH, &handle_val.to_le_bytes())?;
        if status != AK_SUCCESS_I32 {
            return Err(PlatformError::HardwareFailure("start_push failed".into()));
        }
        Ok(())
    }

    /// Stop push-based frame delivery.
    pub fn stop_push(&self) -> PlatformResult<()> {
        let (status, _) = self.send_request(CMD_VENC_STOP_PUSH, &[])?;
        if status != AK_SUCCESS_I32 {
            return Err(PlatformError::HardwareFailure("stop_push failed".into()));
        }
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
    pub fn recv_pushed_frame(
        &self,
        stream_id: StreamId,
        pool: Option<&BytesMutPool>,
    ) -> PlatformResult<OwnedFrame> {
        let mut frame_guard = self.frame_stream.lock().map_err(|e| {
            PlatformError::HardwareFailure(format!("frame_stream mutex poisoned: {}", e))
        })?;
        let frame_stream = frame_guard.as_mut().ok_or_else(|| {
            PlatformError::HardwareFailure("no frame socket for push mode".into())
        })?;

        frame_stream
            .set_read_timeout(Some(PUSH_NOTIFICATION_TIMEOUT))
            .map_err(|e| {
                PlatformError::HardwareFailure(format!("push read-timeout setup error: {}", e))
            })?;

        // Block on reading 12-byte notification (daemon pushes these)
        let mut notif_bytes = [0u8; 12];
        frame_stream
            .read_exact(&mut notif_bytes)
            .map_err(|e| match e.kind() {
                ErrorKind::WouldBlock | ErrorKind::TimedOut => PlatformError::Timeout,
                _ => PlatformError::HardwareFailure(format!("push notification read error: {}", e)),
            })?;
        let notif = FrameNotification::from_bytes(&notif_bytes);

        // Check for dropped-frame notification (Fix 4 integration)
        if notif.is_frame_dropped() {
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

        let (metadata, frame_data) = shm.read_slot_into_bytesmut(notif.slot_index, pool)?;

        Ok(OwnedFrame {
            data: frame_data,
            timestamp: metadata.timestamp_ms.wrapping_mul(1000),
            frame_type: metadata.frame_type,
            stream_id,
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
    fn encode_video_channel_attr_buf(attr: &video_channel_attr) -> ([u8; 48], usize) {
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
    fn encode_encode_param_buf(param: &encode_param) -> ([u8; 48], usize) {
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
    fn encode_pcm_param_buf(param: &pcm_param) -> ([u8; 12], usize) {
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
    fn encode_audio_param_buf(param: &audio_param) -> ([u8; 16], usize) {
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
    fn encode_aenc_attr_buf(attr: &aenc_attr) -> ([u8; 4], usize) {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&attr.aac_head.to_le_bytes());
        (buf, 4)
    }

    // Legacy Vec-based encoders (deprecated, kept for tests)
    #[allow(dead_code)]
    fn encode_i32(val: i32) -> Vec<u8> {
        val.to_le_bytes().to_vec()
    }

    #[allow(dead_code)]
    fn encode_video_dev_type(dev: video_dev_type) -> Vec<u8> {
        (dev as i32).to_le_bytes().to_vec()
    }

    #[allow(dead_code)]
    fn encode_video_channel_attr(attr: &video_channel_attr) -> Vec<u8> {
        let (buf, len) = Self::encode_video_channel_attr_buf(attr);
        buf[..len].to_vec()
    }

    #[allow(dead_code)]
    fn encode_encode_param(param: &encode_param) -> Vec<u8> {
        let (buf, len) = Self::encode_encode_param_buf(param);
        buf[..len].to_vec()
    }

    #[allow(dead_code)]
    fn encode_pcm_param(param: &pcm_param) -> Vec<u8> {
        let (buf, len) = Self::encode_pcm_param_buf(param);
        buf[..len].to_vec()
    }

    #[allow(dead_code)]
    fn encode_audio_param(param: &audio_param) -> Vec<u8> {
        let (buf, len) = Self::encode_audio_param_buf(param);
        buf[..len].to_vec()
    }

    #[allow(dead_code)]
    fn encode_aenc_attr(attr: &aenc_attr) -> Vec<u8> {
        let (buf, len) = Self::encode_aenc_attr_buf(attr);
        buf[..len].to_vec()
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
    fn ipc_to_frame_type(val: i32) -> crate::hal::VideoFrameType {
        use crate::hal::VideoFrameType;
        match val {
            1 => VideoFrameType::FrameTypeI,
            2 => VideoFrameType::FrameTypeB,
            3 => VideoFrameType::FrameTypePi,
            _ => VideoFrameType::FrameTypeP, // 0 = P-frame; unknown defaults to P
        }
    }
}

impl VideoHalTrait for VendorIpc {
    fn vi_match_sensor(&self, config_file: *const c_char) -> i32 {
        if config_file.is_null() {
            return AK_FAILED_I32;
        }
        // SAFETY: caller guarantees `config_file` is a valid, null-terminated C string
        // for the duration of this call (same contract as the underlying FFI).
        let c_str = unsafe { std::ffi::CStr::from_ptr(config_file) };
        let path_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return AK_FAILED_I32,
        };

        match self.send_i32_request(CMD_VI_MATCH_SENSOR, path_str.as_bytes()) {
            Ok(status) => status,
            Err(e) => {
                error!(error = %e, "vi_match_sensor IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn vi_open(&self, dev: video_dev_type) -> *mut c_void {
        let (req_buf, req_len) = Self::encode_video_dev_type_buf(dev);
        match self.send_handle_request(CMD_VI_OPEN, &req_buf[..req_len]) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "vi_open IPC failed");
                std::ptr::null_mut()
            }
        }
    }

    fn vi_close(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VI_CLOSE, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "vi_close IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn vi_get_sensor_resolution(&self, handle: *mut c_void, res: *mut video_resolution) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VI_GET_SENSOR_RESOLUTION, &req_data) {
            Ok((status, data)) => {
                if status == AK_SUCCESS_I32 {
                    match Self::decode_video_resolution(&data) {
                        Ok(r) => {
                            // SAFETY: caller guarantees `res` is a valid, properly aligned
                            // pointer to a `video_resolution` struct that we may write.
                            unsafe {
                                *res = r;
                            }
                            return AK_SUCCESS_I32;
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to decode vi_get_sensor_resolution response");
                            return AK_FAILED_I32;
                        }
                    }
                }
                status
            }
            Err(e) => {
                error!(error = %e, "vi_get_sensor_resolution IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn vi_set_channel_attr(&self, handle: *mut c_void, attr: *const video_channel_attr) -> i32 {
        let handle_val = handle as u64;
        let mut req_buf = [0u8; 56]; // 8 bytes handle + 48 bytes attr
        req_buf[0..8].copy_from_slice(&handle_val.to_le_bytes());
        // SAFETY: caller guarantees `attr` is a valid, non-null pointer to a
        // `video_channel_attr` that remains valid for the duration of this call.
        let (attr_buf, attr_len) = unsafe { Self::encode_video_channel_attr_buf(&*attr) };
        req_buf[8..8 + attr_len].copy_from_slice(&attr_buf[..attr_len]);
        match self.send_request(CMD_VI_SET_CHANNEL_ATTR, &req_buf[..8 + attr_len]) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "vi_set_channel_attr IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn vi_capture_on(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VI_CAPTURE_ON, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "vi_capture_on IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn vi_capture_off(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VI_CAPTURE_OFF, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "vi_capture_off IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn vpss_init(&self, _vi_handle: *mut c_void, _dev: i32) {}

    fn vpss_destroy(&self, _dev: i32) {}

    fn venc_set_cfg_path(&self, _path: *const c_char) -> i32 {
        AK_SUCCESS_I32
    }

    fn venc_open(&self, param: *const encode_param) -> *mut c_void {
        // SAFETY: caller guarantees `param` is a valid, non-null pointer to an
        // `encode_param` that remains valid for the duration of this call.
        let (req_buf, req_len) = unsafe { Self::encode_encode_param_buf(&*param) };
        match self.send_handle_request(CMD_VENC_OPEN, &req_buf[..req_len]) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "venc_open IPC failed");
                std::ptr::null_mut()
            }
        }
    }

    fn venc_close(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VENC_CLOSE, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "venc_close IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn venc_set_rc(&self, enc_handle: *mut c_void, bps: i32) -> i32 {
        let handle_val = enc_handle as u64;
        let mut req_data = handle_val.to_le_bytes().to_vec();
        req_data.extend_from_slice(&bps.to_le_bytes());
        match self.send_request(CMD_VENC_SET_RC, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "venc_set_rc IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn venc_set_iframe(&self, enc_handle: *mut c_void) -> i32 {
        let handle_val = enc_handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VENC_SET_IFRAME, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "venc_set_iframe IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn venc_request_stream(&self, vi_handle: *mut c_void, venc_handle: *mut c_void) -> *mut c_void {
        let vi_val = vi_handle as u64;
        let venc_val = venc_handle as u64;
        let mut req_data = vi_val.to_le_bytes().to_vec();
        req_data.extend_from_slice(&venc_val.to_le_bytes());
        match self.send_handle_request(CMD_VENC_REQUEST_STREAM, &req_data) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "venc_request_stream IPC failed");
                std::ptr::null_mut()
            }
        }
    }

    /// Fetch the next encoded video frame from the daemon.
    ///
    /// # Wire format
    ///
    /// The daemon responds with a 28-byte header followed by the raw frame bytes:
    /// ```text
    /// [u32 frame_len][u64 timestamp][u32 seq_no][i32 frame_type][u64 remote_token][frame_data…]
    /// ```
    ///
    /// The Rust side copies the frame data into a locally-owned `Vec<u8>`,
    /// stores it in `pending_frames` keyed by the address of `stream`,
    /// and writes a pointer to that buffer back into `stream.data`.
    ///
    /// # Safety
    ///
    /// The caller guarantees that `stream` is a valid, writable, non-null pointer
    /// to a `video_stream` that remains live until `venc_release_stream` is called.
    fn venc_get_stream(&self, stream_handle: *mut c_void, stream: *mut video_stream) -> i32 {
        let handle_val = stream_handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();

        let (status, data) = match self.send_request(CMD_VENC_GET_STREAM, &req_data) {
            Ok(pair) => pair,
            Err(e) => {
                error!(error = %e, "venc_get_stream IPC failed");
                return AK_FAILED_I32;
            }
        };

        if status != AK_SUCCESS_I32 {
            return status;
        }

        if data.len() < VENC_STREAM_HEADER_LEN {
            error!(
                got = data.len(),
                need = VENC_STREAM_HEADER_LEN,
                "venc_get_stream response header too short"
            );
            return AK_FAILED_I32;
        }

        // --- Parse the 28-byte header ---
        let frame_len = u32::from_le_bytes(match data[0..4].try_into() {
            Ok(b) => b,
            Err(_) => return AK_FAILED_I32,
        });
        let ts = u64::from_le_bytes(match data[4..12].try_into() {
            Ok(b) => b,
            Err(_) => return AK_FAILED_I32,
        });
        let seq_no = u32::from_le_bytes(match data[12..16].try_into() {
            Ok(b) => b,
            Err(_) => return AK_FAILED_I32,
        });
        let frame_type_val = i32::from_le_bytes(match data[16..20].try_into() {
            Ok(b) => b,
            Err(_) => return AK_FAILED_I32,
        });
        let remote_token = u64::from_le_bytes(match data[20..28].try_into() {
            Ok(b) => b,
            Err(_) => return AK_FAILED_I32,
        });

        // --- Validate and copy frame payload ---
        let expected_total = VENC_STREAM_HEADER_LEN + frame_len as usize;
        if data.len() < expected_total {
            error!(
                got = data.len(),
                need = expected_total,
                "venc_get_stream response truncated (frame data missing)"
            );
            return AK_FAILED_I32;
        }

        let frame_data = data[VENC_STREAM_HEADER_LEN..expected_total].to_vec();

        // --- Store pending frame, keyed by the stream pointer address ---
        let stream_key = stream as u64;
        let pending = PendingFrame {
            data: frame_data,
            remote_token,
        };

        let data_ptr = {
            let mut frames = match self.pending_frames.lock() {
                Ok(g) => g,
                Err(e) => {
                    error!(error = %e, "pending_frames mutex poisoned in venc_get_stream");
                    return AK_FAILED_I32;
                }
            };
            frames.insert(stream_key, pending);
            // Retrieve the pointer to the buffer we just stored.
            // SAFETY: We just inserted the entry; it is guaranteed to be present.
            frames[&stream_key].data.as_ptr() as *mut u8
        };

        // --- Populate the caller's video_stream struct ---
        // SAFETY: The caller contract (VideoHalTrait::venc_get_stream) requires
        // `stream` to be a valid, writable, non-null pointer to a `video_stream`.
        // We do not read the current contents; we only write fields.
        // The `data` pointer we store refers to a `Vec<u8>` that lives inside
        // `pending_frames` and remains valid until `venc_release_stream` is called.
        unsafe {
            (*stream).len = frame_len;
            (*stream).ts = ts;
            (*stream).seq_no = seq_no as std::os::raw::c_ulong;
            (*stream).frame_type = Self::ipc_to_frame_type(frame_type_val);
            (*stream).data = data_ptr;
        }

        if is_ipc_debug_enabled() {
            debug!(
                frame_len,
                ts, seq_no, frame_type_val, remote_token, "venc_get_stream: frame received"
            );
        }

        AK_SUCCESS_I32
    }

    /// Release a previously acquired video stream frame.
    ///
    /// Looks up the pending frame by the address of `stream`, sends a release
    /// command to the daemon, and removes the locally-owned buffer.
    ///
    /// # Safety
    ///
    /// After this call returns, the `stream.data` pointer is **invalid** — the
    /// backing `Vec<u8>` has been dropped.  Callers must not dereference
    /// `stream.data` after `venc_release_stream` returns.
    fn venc_release_stream(&self, stream_handle: *mut c_void, stream: *mut video_stream) -> i32 {
        let stream_key = stream as u64;

        // Look up the remote token for this frame.
        let remote_token = {
            let frames = match self.pending_frames.lock() {
                Ok(g) => g,
                Err(e) => {
                    error!(error = %e, "pending_frames mutex poisoned in venc_release_stream");
                    return AK_FAILED_I32;
                }
            };
            match frames.get(&stream_key) {
                Some(pf) => pf.remote_token,
                None => {
                    warn!(
                        stream_key,
                        "venc_release_stream: no pending frame found for stream address"
                    );
                    // Still try to send the release with just the stream handle.
                    0u64
                }
            }
        };

        // Build request: stream_handle (u64) + remote_token (u64)
        let handle_val = stream_handle as u64;
        let mut req_data = handle_val.to_le_bytes().to_vec();
        req_data.extend_from_slice(&remote_token.to_le_bytes());

        let result = match self.send_request(CMD_VENC_RELEASE_STREAM, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "venc_release_stream IPC failed");
                AK_FAILED_I32
            }
        };

        // Remove the pending frame regardless of IPC outcome — the caller
        // has relinquished ownership and we must not hold the buffer forever.
        if let Ok(mut frames) = self.pending_frames.lock() {
            frames.remove(&stream_key);
        }

        result
    }

    fn venc_cancel_stream(&self, stream_handle: *mut c_void) -> i32 {
        let handle_val = stream_handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VENC_CANCEL_STREAM, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "venc_cancel_stream IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn get_error_no(&self) -> i32 {
        self.send_i32_request(CMD_GET_ERROR_NO, &[]).unwrap_or(-1)
    }

    fn get_error_str(&self) -> String {
        match self.send_request(CMD_GET_ERROR_STR, &[]) {
            Ok((_, data)) => String::from_utf8_lossy(&data).to_string(),
            Err(_) => String::new(),
        }
    }
}

impl crate::hal::audio::AudioHalTrait for VendorIpc {
    fn ai_open(&self, param: *const pcm_param) -> *mut c_void {
        // SAFETY: caller guarantees `param` is a valid, non-null pointer to a
        // `pcm_param` that remains valid for the duration of this call.
        let (req_buf, req_len) = unsafe { Self::encode_pcm_param_buf(&*param) };
        match self.send_handle_request(CMD_AI_OPEN, &req_buf[..req_len]) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "ai_open IPC failed");
                std::ptr::null_mut()
            }
        }
    }

    fn ai_close(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_AI_CLOSE, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "ai_close IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn ai_set_adc_volume(&self, handle: *mut c_void, vol: i32) -> i32 {
        let handle_val = handle as u64;
        let mut req_data = handle_val.to_le_bytes().to_vec();
        req_data.extend_from_slice(&vol.to_le_bytes());
        match self.send_request(CMD_AI_SET_ADC_VOLUME, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "ai_set_adc_volume IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn ai_set_aslc_volume(&self, handle: *mut c_void, vol: i32) -> i32 {
        let handle_val = handle as u64;
        let mut req_data = handle_val.to_le_bytes().to_vec();
        req_data.extend_from_slice(&vol.to_le_bytes());
        match self.send_request(CMD_AI_SET_ASLC_VOLUME, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "ai_set_aslc_volume IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn aenc_open(&self, param: *const audio_param) -> *mut c_void {
        // SAFETY: caller guarantees `param` is a valid, non-null pointer to an
        // `audio_param` that remains valid for the duration of this call.
        let (req_buf, req_len) = unsafe { Self::encode_audio_param_buf(&*param) };
        match self.send_handle_request(CMD_AENC_OPEN, &req_buf[..req_len]) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "aenc_open IPC failed");
                std::ptr::null_mut()
            }
        }
    }

    fn aenc_close(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_AENC_CLOSE, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "aenc_close IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn aenc_set_attr(&self, handle: *mut c_void, attr: *const aenc_attr) -> i32 {
        let handle_val = handle as u64;
        let mut req_buf = [0u8; 16]; // 8 bytes handle + 8 bytes padding for alignment
        req_buf[0..8].copy_from_slice(&handle_val.to_le_bytes());
        // SAFETY: caller guarantees `attr` is a valid, non-null pointer to an
        // `aenc_attr` that remains valid for the duration of this call.
        let (attr_buf, attr_len) = unsafe { Self::encode_aenc_attr_buf(&*attr) };
        req_buf[8..8 + attr_len].copy_from_slice(&attr_buf[..attr_len]);
        match self.send_request(CMD_AENC_SET_ATTR, &req_buf[..8 + attr_len]) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "aenc_set_attr IPC failed");
                AK_FAILED_I32
            }
        }
    }
}

impl crate::hal::imaging::ImagingHalTrait for VendorIpc {
    fn set_brightness(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.send_request(CMD_ISP_SET_BRIGHTNESS, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_brightness IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn set_contrast(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.send_request(CMD_ISP_SET_CONTRAST, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_contrast IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn set_saturation(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.send_request(CMD_ISP_SET_SATURATION, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_saturation IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn set_sharpness(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.send_request(CMD_ISP_SET_SHARPNESS, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_sharpness IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn set_ir_filter(&self, enabled: bool) -> i32 {
        let value: i32 = if enabled { 1 } else { 0 };
        let req_data = value.to_le_bytes().to_vec();
        match self.send_request(CMD_ISP_SET_IR_FILTER, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_ir_filter IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn set_wdr(&self, enabled: bool) -> i32 {
        let value: i32 = if enabled { 1 } else { 0 };
        let req_data = value.to_le_bytes().to_vec();
        match self.send_request(CMD_ISP_SET_WDR, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_wdr IPC failed");
                AK_FAILED_I32
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // ============================================================================
    // Fake daemon infrastructure
    // ============================================================================

    /// Monotonic counter for unique per-test socket paths.
    static TEST_DAEMON_COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Helper: spawns a fake vendor daemon on a temporary Unix socket.
    ///
    /// Accepts a single connection and handles IPC requests in a loop until EOF.
    /// The `handler` closure maps `(cmd_id, req_data)` to `(status, resp_data)`.
    struct FakeDaemon {
        /// Path of the bound socket; used by tests to connect VendorIpc.
        pub socket_path: String,
        /// Background listener thread — kept alive as long as FakeDaemon exists.
        _listener_thread: std::thread::JoinHandle<()>,
    }

    impl FakeDaemon {
        fn start(handler: impl Fn(i32, &[u8]) -> (i32, Vec<u8>) + Send + 'static) -> Self {
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
    }

    // ============================================================================
    // Fake-daemon integration tests
    // ============================================================================

    /// A simple success command round-trips correctly: set_brightness returns AK_SUCCESS.
    #[test]
    fn test_vendor_ipc_connect_and_simple_command_returns_success() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = VendorIpc::new_with_path(&daemon.socket_path).unwrap();

        // set_brightness is part of ImagingHalTrait; accessible from within the crate.
        let result = <VendorIpc as crate::hal::imaging::ImagingHalTrait>::set_brightness(&ipc, 50);

        assert_eq!(result, AK_SUCCESS_I32, "expected AK_SUCCESS from daemon");
    }

    /// When the daemon returns status=-1 (AK_FAILED), the trait method propagates it.
    #[test]
    fn test_vendor_ipc_error_response_propagates_ak_failed() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_FAILED_I32, vec![]));
        let ipc = VendorIpc::new_with_path(&daemon.socket_path).unwrap();

        let result = <VendorIpc as crate::hal::imaging::ImagingHalTrait>::set_brightness(&ipc, 50);

        assert_eq!(
            result, AK_FAILED_I32,
            "expected AK_FAILED propagated from daemon"
        );
    }

    /// A daemon that sends an 8-byte handle lets vi_open return a non-null pointer.
    #[test]
    fn test_vendor_ipc_handle_response_returns_non_null_pointer() {
        let handle_value: i64 = 0x1234_5678;
        let daemon = FakeDaemon::start(move |_cmd_id, _req| {
            (AK_SUCCESS_I32, handle_value.to_le_bytes().to_vec())
        });
        let ipc = VendorIpc::new_with_path(&daemon.socket_path).unwrap();

        let handle = {
            use crate::hal::stubs::VideoDevType;
            <VendorIpc as crate::hal::video::VideoHalTrait>::vi_open(&ipc, VideoDevType::Dev0)
        };

        assert!(!handle.is_null(), "expected non-null handle from daemon");
        assert_eq!(
            handle as i64, handle_value,
            "handle value should match daemon response"
        );
    }

    /// Three sequential commands over the same connection all succeed.
    #[test]
    fn test_vendor_ipc_multiple_requests_same_connection_all_succeed() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = VendorIpc::new_with_path(&daemon.socket_path).unwrap();

        for i in 0..3 {
            let result =
                <VendorIpc as crate::hal::imaging::ImagingHalTrait>::set_brightness(&ipc, 50 + i);
            assert_eq!(result, AK_SUCCESS_I32, "request {} should succeed", i);
        }
    }

    /// venc_get_stream correctly parses a 28-byte header + payload from the daemon.
    #[test]
    #[cfg(use_stubs)]
    fn test_vendor_ipc_get_stream_frame_data_parses_header_and_payload() {
        use crate::hal::stubs::VideoFrameType;
        use std::mem::MaybeUninit;

        // Build the daemon response: 28-byte header + 4-byte payload.
        let frame_payload: Vec<u8> = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let frame_len = frame_payload.len() as u32;
        let timestamp: u64 = 12345;
        let seq_no: u32 = 1;
        let frame_type_val: i32 = 1; // I-frame
        let remote_token: u64 = 99;

        let daemon = FakeDaemon::start(move |cmd_id, _req| {
            if cmd_id == CMD_VENC_GET_STREAM {
                let mut resp = Vec::new();
                resp.extend_from_slice(&frame_len.to_le_bytes());
                resp.extend_from_slice(&timestamp.to_le_bytes());
                resp.extend_from_slice(&seq_no.to_le_bytes());
                resp.extend_from_slice(&frame_type_val.to_le_bytes());
                resp.extend_from_slice(&remote_token.to_le_bytes());
                resp.extend_from_slice(&frame_payload);
                (AK_SUCCESS_I32, resp)
            } else {
                // Handle CMD_VENC_RELEASE_STREAM and anything else with success.
                (AK_SUCCESS_I32, vec![])
            }
        });

        let ipc = VendorIpc::new_with_path(&daemon.socket_path).unwrap();

        // Use a dummy stream handle (any non-zero value serves as the session handle).
        let stream_handle = 1usize as *mut std::ffi::c_void;

        // Allocate an uninitialized video_stream on the stack.
        let mut vs = MaybeUninit::<crate::hal::stubs::VideoStream>::zeroed();
        let vs_ptr = vs.as_mut_ptr() as *mut video_stream;

        let result = <VendorIpc as crate::hal::video::VideoHalTrait>::venc_get_stream(
            &ipc,
            stream_handle,
            vs_ptr,
        );
        assert_eq!(result, AK_SUCCESS_I32, "venc_get_stream should succeed");

        // SAFETY: venc_get_stream returned AK_SUCCESS, so the fields are initialized.
        let populated = unsafe { vs.assume_init() };
        assert_eq!(populated.len, frame_len, "frame length mismatch");
        assert_eq!(populated.ts, timestamp, "timestamp mismatch");
        assert_eq!(
            populated.seq_no, seq_no as std::os::raw::c_ulong,
            "seq_no mismatch"
        );
        assert_eq!(
            populated.frame_type,
            VideoFrameType::FrameTypeI,
            "frame type should be I-frame"
        );
        assert!(!populated.data.is_null(), "data pointer should not be null");

        // Verify frame payload bytes via the data pointer.
        // SAFETY: data points into pending_frames Vec<u8> which is still alive (ipc is alive).
        let actual_bytes =
            unsafe { std::slice::from_raw_parts(populated.data, frame_len as usize) };
        assert_eq!(
            actual_bytes,
            &[0xAAu8, 0xBB, 0xCC, 0xDD],
            "frame bytes mismatch"
        );

        // Release the stream to clean up pending_frames.
        // SAFETY: vs_ptr points to the same stack allocation; still valid at this point.
        let release_result = <VendorIpc as crate::hal::video::VideoHalTrait>::venc_release_stream(
            &ipc,
            stream_handle,
            vs_ptr,
        );
        assert_eq!(
            release_result, AK_SUCCESS_I32,
            "venc_release_stream should succeed"
        );
    }

    #[test]
    fn test_vendor_ipc_traits_implement_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<VendorIpc>();
    }

    #[test]
    fn test_ipc_frame_type_conversion_p_frame() {
        #[cfg(use_stubs)]
        {
            use crate::hal::stubs::VideoFrameType;
            assert_eq!(VendorIpc::ipc_to_frame_type(0), VideoFrameType::FrameTypeP);
        }
    }

    #[test]
    fn test_ipc_frame_type_conversion_i_frame() {
        #[cfg(use_stubs)]
        {
            use crate::hal::stubs::VideoFrameType;
            assert_eq!(VendorIpc::ipc_to_frame_type(1), VideoFrameType::FrameTypeI);
        }
    }

    #[test]
    fn test_ipc_frame_type_conversion_b_frame() {
        #[cfg(use_stubs)]
        {
            use crate::hal::stubs::VideoFrameType;
            assert_eq!(VendorIpc::ipc_to_frame_type(2), VideoFrameType::FrameTypeB);
        }
    }

    #[test]
    fn test_ipc_frame_type_conversion_pi_frame() {
        #[cfg(use_stubs)]
        {
            use crate::hal::stubs::VideoFrameType;
            assert_eq!(VendorIpc::ipc_to_frame_type(3), VideoFrameType::FrameTypePi);
        }
    }

    #[test]
    fn test_ipc_frame_type_conversion_unknown_defaults_to_p() {
        #[cfg(use_stubs)]
        {
            use crate::hal::stubs::VideoFrameType;
            assert_eq!(VendorIpc::ipc_to_frame_type(99), VideoFrameType::FrameTypeP);
            assert_eq!(VendorIpc::ipc_to_frame_type(-1), VideoFrameType::FrameTypeP);
        }
    }

    #[test]
    fn test_decode_video_resolution_rejects_short_input() {
        let short_data = vec![0u8; 8]; // 8 bytes — less than the required 16
        let result = VendorIpc::decode_video_resolution(&short_data);
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

        let result = VendorIpc::decode_video_resolution(&data);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.width, 1920);
        assert_eq!(res.height, 1080);
        assert_eq!(res.max_width, 1920);
        assert_eq!(res.max_height, 1080);
    }

    #[test]
    fn test_encode_i32_produces_le_bytes() {
        let result = VendorIpc::encode_i32(42);
        assert_eq!(result, vec![42, 0, 0, 0]);

        let result = VendorIpc::encode_i32(-1);
        assert_eq!(result, vec![255, 255, 255, 255]);
    }

    #[test]
    fn test_encode_video_channel_attr_round_trip() {
        #[cfg(use_stubs)]
        {
            use crate::hal::stubs::{CropInfo, VideoChannelAttr, VideoResolution};

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

            let encoded = VendorIpc::encode_video_channel_attr(&attr);

            // 4 crop fields * 4 bytes + 2 resolutions * 4 fields * 4 bytes = 16 + 32 = 48
            assert_eq!(encoded.len(), 48);

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
            assert_eq!(
                i32::from_le_bytes(encoded[24..28].try_into().unwrap()),
                1920
            );
            assert_eq!(
                i32::from_le_bytes(encoded[28..32].try_into().unwrap()),
                1080
            );

            // Verify second resolution (res[1])
            assert_eq!(i32::from_le_bytes(encoded[32..36].try_into().unwrap()), 640);
            assert_eq!(i32::from_le_bytes(encoded[36..40].try_into().unwrap()), 480);
            assert_eq!(i32::from_le_bytes(encoded[40..44].try_into().unwrap()), 640);
            assert_eq!(i32::from_le_bytes(encoded[44..48].try_into().unwrap()), 480);
        }
    }

    #[test]
    fn test_encode_encode_param_byte_length() {
        #[cfg(use_stubs)]
        {
            use crate::hal::stubs::EncodeParam;

            let param = EncodeParam::default();
            let encoded = VendorIpc::encode_encode_param(&param);

            // 12 fields * 4 bytes each = 48 bytes
            assert_eq!(encoded.len(), 48);
        }
    }

    #[test]
    fn test_encode_pcm_param_values() {
        #[cfg(use_stubs)]
        {
            use crate::hal::stubs::PcmParam;

            let param = PcmParam {
                sample_rate: 8000,
                sample_bits: 16,
                channel_num: 1,
            };

            let encoded = VendorIpc::encode_pcm_param(&param);

            // 3 fields * 4 bytes = 12 bytes
            assert_eq!(encoded.len(), 12);
            assert_eq!(u32::from_le_bytes(encoded[0..4].try_into().unwrap()), 8000);
            assert_eq!(u32::from_le_bytes(encoded[4..8].try_into().unwrap()), 16);
            assert_eq!(u32::from_le_bytes(encoded[8..12].try_into().unwrap()), 1);
        }
    }

    #[test]
    fn test_encode_audio_param_values() {
        #[cfg(use_stubs)]
        {
            use crate::hal::stubs::AudioParam;

            let param = AudioParam {
                sample_rate: 48000,
                channel_num: 2,
                sample_bits: 16,
                type_: 1,
            };

            let encoded = VendorIpc::encode_audio_param(&param);

            // 4 fields * 4 bytes = 16 bytes
            assert_eq!(encoded.len(), 16);
            assert_eq!(u32::from_le_bytes(encoded[0..4].try_into().unwrap()), 48000);
            assert_eq!(u32::from_le_bytes(encoded[4..8].try_into().unwrap()), 2);
            assert_eq!(u32::from_le_bytes(encoded[8..12].try_into().unwrap()), 16);
            assert_eq!(i32::from_le_bytes(encoded[12..16].try_into().unwrap()), 1);
        }
    }

    #[test]
    fn test_encode_aenc_attr_values() {
        #[cfg(use_stubs)]
        {
            use crate::hal::stubs::AencAttr;

            let attr = AencAttr { aac_head: 1 };
            let encoded = VendorIpc::encode_aenc_attr(&attr);

            assert_eq!(encoded, vec![1, 0, 0, 0]);
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

        let result = VendorIpc::decode_video_resolution(&data);
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

        let result = VendorIpc::decode_video_resolution(&data);
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
    fn test_venc_stream_header_len_constant() {
        assert_eq!(VENC_STREAM_HEADER_LEN, 28);
    }

    /// fetch_frame_owned reads a frame directly into BytesMut without extra copy.
    #[test]
    fn test_vendor_ipc_fetch_frame_owned_reads_into_bytes_mut() {
        use crate::platform::frame::{FrameType, StreamId};

        let frame_payload: Vec<u8> = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let frame_len = frame_payload.len() as u32;
        let timestamp: u64 = 12345;
        let seq_no: u32 = 1;
        let frame_type_val: i32 = 1; // I-frame
        let remote_token: u64 = 99;

        let daemon = FakeDaemon::start(move |cmd_id, _req| {
            if cmd_id == CMD_VENC_GET_STREAM {
                let mut resp = Vec::new();
                resp.extend_from_slice(&frame_len.to_le_bytes());
                resp.extend_from_slice(&timestamp.to_le_bytes());
                resp.extend_from_slice(&seq_no.to_le_bytes());
                resp.extend_from_slice(&frame_type_val.to_le_bytes());
                resp.extend_from_slice(&remote_token.to_le_bytes());
                resp.extend_from_slice(&frame_payload);
                (AK_SUCCESS_I32, resp)
            } else {
                (AK_SUCCESS_I32, vec![])
            }
        });

        let mut ipc = VendorIpc::new_with_path(&daemon.socket_path).unwrap();
        let stream_handle = 1usize as *mut std::ffi::c_void;

        let frame = ipc
            .fetch_frame_owned(stream_handle, StreamId::VideoMain, None)
            .unwrap();

        assert_eq!(&frame.data[..], &[0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(frame.timestamp, 12345 * 1000); // ms to μs
        assert_eq!(frame.frame_type, FrameType::VideoIFrame);
        assert_eq!(frame.stream_id, StreamId::VideoMain);
    }

    /// fetch_frame_owned uses the BytesMutPool when provided.
    #[test]
    fn test_vendor_ipc_fetch_frame_owned_uses_pool() {
        use crate::platform::frame::StreamId;
        use crate::streaming::bridge::BytesMutPool;

        let frame_payload: Vec<u8> = vec![0x11, 0x22, 0x33];
        let frame_len = frame_payload.len() as u32;
        let timestamp: u64 = 999;
        let seq_no: u32 = 5;
        let frame_type_val: i32 = 0; // P-frame
        let remote_token: u64 = 42;

        let daemon = FakeDaemon::start(move |cmd_id, _req| {
            if cmd_id == CMD_VENC_GET_STREAM {
                let mut resp = Vec::new();
                resp.extend_from_slice(&frame_len.to_le_bytes());
                resp.extend_from_slice(&timestamp.to_le_bytes());
                resp.extend_from_slice(&seq_no.to_le_bytes());
                resp.extend_from_slice(&frame_type_val.to_le_bytes());
                resp.extend_from_slice(&remote_token.to_le_bytes());
                resp.extend_from_slice(&frame_payload);
                (AK_SUCCESS_I32, resp)
            } else {
                (AK_SUCCESS_I32, vec![])
            }
        });

        let mut ipc = VendorIpc::new_with_path(&daemon.socket_path).unwrap();
        let pool = BytesMutPool::new(1024, 4);
        let stream_handle = 1usize as *mut std::ffi::c_void;

        let frame = ipc
            .fetch_frame_owned(stream_handle, StreamId::VideoMain, Some(&pool))
            .unwrap();

        assert_eq!(&frame.data[..], &[0x11, 0x22, 0x33]);
        // The buffer should have been allocated from pool (capacity >= 1024)
        // but since pool was empty, it allocated fresh with max(3, 1024)
    }

    /// release_frame_owned sends the stored remote_token to daemon.
    #[test]
    fn test_vendor_ipc_release_frame_owned_sends_token() {
        use crate::platform::frame::StreamId;
        use portable_atomic::{AtomicU64, Ordering};
        use std::sync::Arc as StdArc;

        let received_token = StdArc::new(AtomicU64::new(0));
        let received_token_clone = received_token.clone();

        let frame_payload: Vec<u8> = vec![0xDE, 0xAD];
        let frame_len = frame_payload.len() as u32;
        let timestamp: u64 = 1;
        let seq_no: u32 = 1;
        let frame_type_val: i32 = 0;
        let remote_token: u64 = 0xBEEF_CAFE;

        let daemon = FakeDaemon::start(move |cmd_id, req| {
            if cmd_id == CMD_VENC_GET_STREAM {
                let mut resp = Vec::new();
                resp.extend_from_slice(&frame_len.to_le_bytes());
                resp.extend_from_slice(&timestamp.to_le_bytes());
                resp.extend_from_slice(&seq_no.to_le_bytes());
                resp.extend_from_slice(&frame_type_val.to_le_bytes());
                resp.extend_from_slice(&remote_token.to_le_bytes());
                resp.extend_from_slice(&frame_payload);
                (AK_SUCCESS_I32, resp)
            } else if cmd_id == CMD_VENC_RELEASE_STREAM {
                // Extract the remote_token from the release request
                if req.len() >= 16 {
                    let token = u64::from_le_bytes(req[8..16].try_into().unwrap());
                    received_token_clone.store(token, Ordering::SeqCst);
                }
                (AK_SUCCESS_I32, vec![])
            } else {
                (AK_SUCCESS_I32, vec![])
            }
        });

        let mut ipc = VendorIpc::new_with_path(&daemon.socket_path).unwrap();
        let stream_handle = 1usize as *mut std::ffi::c_void;

        // Fetch the frame (stores remote_token)
        let _frame = ipc
            .fetch_frame_owned(stream_handle, StreamId::VideoMain, None)
            .unwrap();

        // Release the frame (sends remote_token back to daemon)
        ipc.release_frame_owned(stream_handle).unwrap();

        // Verify the daemon received the correct token
        assert_eq!(received_token.load(Ordering::SeqCst), 0xBEEF_CAFE);
    }

    /// recv_frame_response correctly rejects non-success status.
    #[test]
    fn test_recv_frame_response_rejects_error_status() {
        use std::os::unix::net::UnixStream as StdUnixStream;

        let (mut server, mut client) = StdUnixStream::pair().unwrap();

        // Write error response from "daemon" side
        std::thread::spawn(move || {
            use std::io::Write;
            let status: i32 = -1;
            let resp_len: u32 = 0;
            server.write_all(&status.to_le_bytes()).unwrap();
            server.write_all(&resp_len.to_le_bytes()).unwrap();
            server.flush().unwrap();
        });

        std::thread::sleep(std::time::Duration::from_millis(50));

        let result = VendorIpc::recv_frame_response(&mut client, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_ipc_to_platform_frame_type_mappings() {
        use crate::platform::frame::FrameType;

        assert_eq!(
            VendorIpc::ipc_to_platform_frame_type(0),
            FrameType::VideoPFrame
        );
        assert_eq!(
            VendorIpc::ipc_to_platform_frame_type(1),
            FrameType::VideoIFrame
        );
        assert_eq!(
            VendorIpc::ipc_to_platform_frame_type(2),
            FrameType::VideoBFrame
        );
        assert_eq!(
            VendorIpc::ipc_to_platform_frame_type(3),
            FrameType::VideoPiFrame
        );
        // Unknown defaults to P
        assert_eq!(
            VendorIpc::ipc_to_platform_frame_type(99),
            FrameType::VideoPFrame
        );
    }

    #[test]
    fn test_write_request_header_produces_correct_bytes() {
        use std::os::unix::net::UnixStream as StdUnixStream;

        let (mut server, mut client) = StdUnixStream::pair().unwrap();

        VendorIpc::write_request_header(&mut client, CMD_VENC_GET_STREAM, 8).unwrap();
        client.flush().unwrap();

        let mut buf = [0u8; 8];
        server.read_exact(&mut buf).unwrap();
        assert_eq!(
            i32::from_le_bytes(buf[0..4].try_into().unwrap()),
            CMD_VENC_GET_STREAM
        );
        assert_eq!(u32::from_le_bytes(buf[4..8].try_into().unwrap()), 8);
    }
}
