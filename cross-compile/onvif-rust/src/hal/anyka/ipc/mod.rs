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
//! shared-memory ring and sends 20-byte notifications over
//! `/tmp/vd-frame-main.sock` and `/tmp/vd-frame-sub.sock`.

#![allow(dead_code)]

mod audio;
mod imaging;
mod shm_ring;
mod video;

use crate::platform::PlatformError;
use crate::platform::PlatformResult;
use crate::platform::common::{OwnedFrame, StreamId};
use crate::streaming::bridge::BytesMutPool;
use shm_ring::{FrameNotification, ShmRingReader, VD_NOTIFY_WIRE_SIZE, VD_SHM_SLOT_COUNT};

use std::ffi::c_void;
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot};
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

/// Attach handshake. Returns `[u32 epoch][u32 shm_version]`.
///
/// The only command exempt from the epoch gate — it is how the epoch is learned.
const CMD_HELLO: i32 = 300;

/// Daemon rejected a handle token: dead generation, reused slot, or wrong object
/// kind. Mirrors `VD_STATUS_STALE_EPOCH` in the daemon's `protocol.h`.
///
/// Distinct from the generic error status so this reads as an attachment problem
/// rather than a bad argument. Reaching the client at all means defence in depth
/// caught something the epoch gate should already have refused — a bug or a
/// version skew, worth a loud message either way.
const VD_STATUS_STALE_EPOCH: i32 = -2;

/// Sentinel meaning "not attached to any daemon generation".
///
/// The daemon guarantees a non-zero epoch, so 0 can never collide with a real one.
const EPOCH_DETACHED: u32 = 0;
const PUSH_NOTIFICATION_TIMEOUT: Duration = Duration::from_millis(200);

/// Timeout for blocking reads/writes on the IPC control socket.
///
/// If the vendor daemon stops responding, this bounds how long a single owner-thread
/// send/receive cycle will block before returning an error. The error is surfaced to
/// the caller; the owner thread does not retry.
const IPC_CTRL_TIMEOUT: Duration = Duration::from_secs(10);

/// Timeout for the public async IPC API, set slightly above `IPC_CTRL_TIMEOUT` so the
/// owner thread's socket-level timeout fires first and surfaces a `Timeout` error to the
/// async caller. This bounds how long an executor task awaits a control RPC even if the
/// owner thread is still blocked on the socket.
const IPC_PUBLIC_TIMEOUT: Duration = Duration::from_secs(12);

/// Bounded depth of the control-socket job queue handed to the owner thread.
///
/// Backpressure: if more than this many control RPCs are outstanding, additional
/// async senders await and blocking senders block until the owner drains the queue.
const IPC_JOB_QUEUE_CAP: usize = 32;

/// Result of a control-socket RPC: `(status, response bytes)` or a platform error.
type IpcResponse = PlatformResult<(i32, Vec<u8>)>;

/// Reply channel the owner thread uses to return an [`IpcResponse`] to a caller.
type IpcReplyRx = oneshot::Receiver<IpcResponse>;

/// A single control-socket request handed to the owner thread, paired with the
/// oneshot channel the owner uses to return the response.
struct IpcJob {
    cmd_id: i32,
    req_data: Vec<u8>,
    reply: oneshot::Sender<IpcResponse>,
}

/// A detection site's report that the vendor daemon peer is gone.
///
/// Carries a reason for the log only. Detection sites *report*; they never attach —
/// the daemon's single-owner guards reject a concurrent second attacher rather than
/// serialising it, so re-attaching is the supervisor's job alone.
#[derive(Debug, Clone)]
pub struct PeerLoss {
    /// Which site noticed, and how. Log-only.
    pub reason: String,
}

/// Depth of the peer-loss channel.
///
/// One queued loss is exactly as informative as ten — the supervisor's response is
/// the same either way — and a bounded channel with `try_send` guarantees the owner
/// thread and frame reader never block on a busy supervisor.
const PEER_LOSS_QUEUE_CAP: usize = 1;

/// Everything the owner thread accepts, on one ordered queue.
///
/// Stream installation travels the *same* queue as requests on purpose. The attach
/// path issues `CMD_HELLO` immediately after handing over the stream, and a side
/// channel would let the handshake overtake the installation and be refused on a
/// perfectly healthy daemon.
enum OwnerMsg {
    /// Run a control RPC on the currently installed stream.
    Job(IpcJob),
    /// Install a freshly connected control stream, acknowledging when it is live.
    GiveStream {
        stream: UnixStream,
        ack: oneshot::Sender<()>,
    },
    /// Drop the installed stream. Fire-and-forget: it only makes later requests
    /// fail sooner.
    DropStream,
}

/// Describes how the owner thread connects the control `UnixStream`.
///
/// Connect-only: there is deliberately no reconnect path. A reconnect that resumed
/// with the previous generation's SDK handles is the hazard the epoch gate exists to
/// prevent, so recovery is the supervisor's job and it goes through `attach()`.
#[derive(Clone)]
enum CtrlConnect {
    /// Production: connect to `/tmp/vd-ctrl.sock`, falling back to the legacy path.
    Production,
    /// Test-only: connect to an explicit socket path (e.g. a `FakeDaemon`).
    #[cfg(test)]
    Path(String),
}

impl CtrlConnect {
    /// Establish the initial control-socket connection.
    fn connect(&self) -> PlatformResult<UnixStream> {
        match self {
            CtrlConnect::Production => connect_production_ctrl(),
            #[cfg(test)]
            CtrlConnect::Path(path) => {
                let stream = UnixStream::connect(path).map_err(|e| {
                    PlatformError::HardwareUnavailable(format!(
                        "Failed to connect to vendor daemon at {}: {}",
                        path, e
                    ))
                })?;
                configure_ctrl_timeouts(&stream)?;
                Ok(stream)
            }
        }
    }
}

/// Connect to the production control socket, falling back to the legacy path.
fn connect_production_ctrl() -> PlatformResult<UnixStream> {
    let stream = UnixStream::connect(CTRL_SOCKET_PATH)
        .or_else(|_| UnixStream::connect(VENDOR_SOCKET_PATH))
        .map_err(|e| {
            PlatformError::HardwareUnavailable(format!(
                "Cannot connect to vendor daemon (tried {} and {}): {}",
                CTRL_SOCKET_PATH, VENDOR_SOCKET_PATH, e
            ))
        })?;
    configure_ctrl_timeouts(&stream)?;
    Ok(stream)
}

/// Apply the bounded read/write timeouts to a freshly connected control socket.
fn configure_ctrl_timeouts(stream: &UnixStream) -> PlatformResult<()> {
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
    Ok(())
}

/// IPC client for Anyka vendor daemon communication.
///
/// The control socket is owned exclusively by a dedicated OS thread (see
/// [`AnykaIpc::run_owner`]). Callers never touch the `UnixStream` directly; instead
/// they submit [`IpcJob`]s over a bounded channel and receive responses via oneshot
/// channels. This keeps blocking socket I/O off the tokio worker threads.
pub struct AnykaIpc {
    /// Bounded queue of control-socket jobs handed to the owner thread.
    ///
    /// `None` only during `Drop`, after the sender has been dropped to signal the
    /// owner thread to exit.
    job_tx: Option<mpsc::Sender<OwnerMsg>>,
    /// Join handle for the control-socket owner thread, taken and joined on `Drop`.
    owner_thread: Option<std::thread::JoinHandle<()>>,
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
    /// Daemon generation this client attached to, or [`EPOCH_DETACHED`].
    ///
    /// Set once by `attach`, cleared by `detach`. Every outstanding SDK handle is
    /// implicitly tagged with this value: the handles are raw pointers minted inside
    /// the daemon process, so they are only meaningful for the generation that minted
    /// them.
    attached_epoch: AtomicU32,
    /// Reports peer loss to the supervisor. Never used to attach.
    loss_tx: mpsc::Sender<PeerLoss>,
    /// Receiving half, handed to the supervisor exactly once via `take_loss_rx`.
    loss_rx: Mutex<Option<mpsc::Receiver<PeerLoss>>>,
    /// Latest epoch observed in the ring header, refreshed by the supervisor's poller.
    ///
    /// Kept as an atomic rather than read from the mmap on demand so the request path
    /// never contends with the frame reader for the `shm_reader` mutex.
    /// 0 means the ring is not stamped — while detached, or because the daemon is
    /// re-creating it. Never a usable generation: the gate refuses on 0 like any
    /// other mismatch.
    observed_epoch: AtomicU32,
}

impl Drop for AnykaIpc {
    fn drop(&mut self) {
        // Dropping the only sender closes the job channel, causing the owner thread's
        // `blocking_recv` to return `None` and the owner loop to exit.
        self.job_tx.take();
        if let Some(handle) = self.owner_thread.take() {
            let _ = handle.join();
        }
    }
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
            CMD_HELLO => "HELLO",
            _ => "UNKNOWN",
        }
    }

    /// Create a new IPC client connected to the vendor daemon.
    pub fn new() -> PlatformResult<Self> {
        // Connect to control socket: try new path first, fall back to legacy.
        // The daemon now uses /tmp/vd-ctrl.sock (replacing /tmp/vendor-daemon.sock).
        // Timeouts are applied by `connect`.
        let connect = CtrlConnect::Production;
        let stream = connect.connect()?;
        if is_ipc_debug_enabled() {
            debug!(socket = CTRL_SOCKET_PATH, "Connected to vendor daemon");
        }

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

        let (loss_tx, loss_rx) = mpsc::channel::<PeerLoss>(PEER_LOSS_QUEUE_CAP);
        let (job_tx, owner_thread) = Self::spawn_owner(Some(stream), loss_tx.clone())?;

        Ok(Self {
            job_tx: Some(job_tx),
            owner_thread: Some(owner_thread),
            frame_main_stream: Mutex::new(Some(frame_main_stream)),
            frame_sub_stream: Mutex::new(Some(frame_sub_stream)),
            shm_reader: Mutex::new(Some(shm_reader)),
            prefer_sub_on_tie: AtomicBool::new(false),
            loss_tx,
            loss_rx: Mutex::new(Some(loss_rx)),
            attached_epoch: AtomicU32::new(EPOCH_DETACHED),
            observed_epoch: AtomicU32::new(EPOCH_DETACHED),
        })
    }

    /// Create a new IPC client connected to a custom socket path (test-only).
    #[cfg(test)]
    pub fn new_with_path(path: &str) -> PlatformResult<Self> {
        // Timeouts are applied by `connect` (same values as production).
        let connect = CtrlConnect::Path(path.to_string());
        let stream = connect.connect()?;

        if is_ipc_debug_enabled() {
            debug!(socket = path, "Connected to vendor daemon (test)");
        }

        // In test mode, try to connect to frame sockets if they exist.
        let frame_main_stream = UnixStream::connect(FRAME_MAIN_SOCKET_PATH).ok();
        let frame_sub_stream = UnixStream::connect(FRAME_SUB_SOCKET_PATH).ok();

        // In test mode, try to open shm if available
        let shm_reader = match ShmRingReader::open() {
            Ok(Some(reader)) => Some(reader),
            _ => None,
        };

        let (loss_tx, loss_rx) = mpsc::channel::<PeerLoss>(PEER_LOSS_QUEUE_CAP);
        let (job_tx, owner_thread) = Self::spawn_owner(Some(stream), loss_tx.clone())?;

        Ok(Self {
            job_tx: Some(job_tx),
            owner_thread: Some(owner_thread),
            frame_main_stream: Mutex::new(frame_main_stream),
            frame_sub_stream: Mutex::new(frame_sub_stream),
            shm_reader: Mutex::new(shm_reader),
            prefer_sub_on_tie: AtomicBool::new(false),
            loss_tx,
            loss_rx: Mutex::new(Some(loss_rx)),
            attached_epoch: AtomicU32::new(EPOCH_DETACHED),
            observed_epoch: AtomicU32::new(EPOCH_DETACHED),
        })
    }

    /// Construct an `AnykaIpc` from pre-built parts for unit tests, spawning the owner
    /// thread on the provided control stream.
    #[cfg(test)]
    pub(crate) fn from_parts_for_test(
        ctrl_stream: UnixStream,
        frame_main_stream: Option<UnixStream>,
        frame_sub_stream: Option<UnixStream>,
        shm_reader: Option<ShmRingReader>,
    ) -> Self {
        let _ = configure_ctrl_timeouts(&ctrl_stream);
        let (loss_tx, loss_rx) = mpsc::channel::<PeerLoss>(PEER_LOSS_QUEUE_CAP);
        let (job_tx, owner_thread) =
            Self::spawn_owner(Some(ctrl_stream), loss_tx.clone()).expect("spawn owner thread");
        Self {
            job_tx: Some(job_tx),
            owner_thread: Some(owner_thread),
            frame_main_stream: Mutex::new(frame_main_stream),
            frame_sub_stream: Mutex::new(frame_sub_stream),
            shm_reader: Mutex::new(shm_reader),
            prefer_sub_on_tie: AtomicBool::new(false),
            loss_tx,
            loss_rx: Mutex::new(Some(loss_rx)),
            attached_epoch: AtomicU32::new(EPOCH_DETACHED),
            observed_epoch: AtomicU32::new(EPOCH_DETACHED),
        }
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

        let mut notif_bytes = [0u8; VD_NOTIFY_WIRE_SIZE];
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
                frame_len = notif.frame_len,
                flags = notif.flags,
                stream_id = notif.stream_id,
                seq_no = notif.seq_no,
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

    /// Spawn the dedicated owner thread that exclusively owns the control `UnixStream`.
    ///
    /// Returns the sender half of the bounded job queue and the thread's join handle.
    fn spawn_owner(
        stream: Option<UnixStream>,
        loss_tx: mpsc::Sender<PeerLoss>,
    ) -> PlatformResult<(mpsc::Sender<OwnerMsg>, std::thread::JoinHandle<()>)> {
        let (job_tx, job_rx) = mpsc::channel::<OwnerMsg>(IPC_JOB_QUEUE_CAP);
        let handle = std::thread::Builder::new()
            .name("vd-ctrl-owner".to_string())
            .spawn(move || Self::run_owner(stream, job_rx, loss_tx))
            .map_err(|e| {
                PlatformError::InitializationFailed(format!(
                    "Failed to spawn IPC control owner thread: {}",
                    e
                ))
            })?;
        Ok((job_tx, handle))
    }

    /// Owner-thread main loop.
    ///
    /// Consumes jobs from the bounded queue, performs a blocking send/receive cycle on
    /// the exclusively-owned control stream, and returns the result over the job's
    /// oneshot channel. I/O errors are reported, never repaired here: re-attaching is
    /// the supervisor's job alone.
    /// Exits when the job channel is closed (all senders dropped).
    fn run_owner(
        mut stream: Option<UnixStream>,
        mut job_rx: mpsc::Receiver<OwnerMsg>,
        loss_tx: mpsc::Sender<PeerLoss>,
    ) {
        while let Some(msg) = job_rx.blocking_recv() {
            match msg {
                OwnerMsg::Job(job) => match stream.as_mut() {
                    Some(s) => Self::process_job(s, job, &loss_tx),
                    None => {
                        // Detached: no stream to run on. Reply rather than panic —
                        // the gate normally catches this first, but a request can
                        // race a detach.
                        let _ = job.reply.send(Err(PlatformError::HardwareUnavailable(
                            "IPC not attached: no control stream installed".to_string(),
                        )));
                    }
                },
                OwnerMsg::GiveStream { stream: s, ack } => {
                    stream = Some(s);
                    // Acknowledge only after installation, so the caller's next
                    // request is guaranteed to find the stream in place.
                    let _ = ack.send(());
                }
                OwnerMsg::DropStream => {
                    stream = None;
                }
            }
        }

        if is_ipc_debug_enabled() {
            debug!("IPC control owner thread exiting");
        }
    }

    /// Execute a single job on the owned stream and reply over its oneshot channel.
    ///
    /// On I/O error the error is surfaced to the caller and logged. It is deliberately
    /// not repaired here — see the warning arm below.
    fn process_job(stream: &mut UnixStream, job: IpcJob, loss_tx: &mpsc::Sender<PeerLoss>) {
        let started = Instant::now();
        let cmd_name = Self::cmd_name(job.cmd_id);
        if is_ipc_debug_enabled() {
            debug!(
                cmd_id = job.cmd_id,
                cmd_name,
                req_len = job.req_data.len(),
                "IPC request start"
            );
        }

        let result = Self::exec_on_stream(stream, job.cmd_id, &job.req_data);

        if let Err(ref e) = result {
            warn!(
                cmd_id = job.cmd_id,
                cmd_name,
                elapsed_ms = started.elapsed().as_millis(),
                error = %e,
                "IPC request failed; reporting peer loss to the supervisor"
            );
            // Do NOT reconnect here. A reconnect that resumes with the same
            // handles is exactly the hazard the epoch gate exists to prevent;
            // re-attaching is the supervisor's job and only its job.
            Self::send_peer_loss(loss_tx, format!("control socket error: {e}"));
        } else if is_ipc_debug_enabled() {
            let (status, resp_len) = result
                .as_ref()
                .map(|(s, d)| (*s, d.len()))
                .unwrap_or((0, 0));
            debug!(
                cmd_id = job.cmd_id,
                cmd_name,
                status,
                resp_len,
                elapsed_ms = started.elapsed().as_millis(),
                "IPC request done"
            );
        }

        // The receiver may have gone away (async caller timed out and dropped the
        // oneshot); that is expected and not an error.
        let _ = job.reply.send(result);
    }

    /// Map I/O errors to PlatformError, distinguishing timeouts from hardware failures.
    fn map_io_error(ctx: &str, e: std::io::Error) -> PlatformError {
        if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut {
            PlatformError::Timeout
        } else {
            PlatformError::HardwareFailure(format!("{}: {}", ctx, e))
        }
    }

    /// Perform a single send/receive cycle directly on the owned control stream.
    ///
    /// Runs only on the owner thread, so it takes `&mut UnixStream` directly with no
    /// locking.
    fn exec_on_stream(
        stream: &mut UnixStream,
        cmd_id: i32,
        req_data: &[u8],
    ) -> PlatformResult<(i32, Vec<u8>)> {
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

    /// Build a control-socket job and its reply receiver for the owner thread.
    fn make_job(cmd_id: i32, req_data: &[u8]) -> (OwnerMsg, IpcReplyRx) {
        let (reply_tx, reply_rx) = oneshot::channel();
        let job = OwnerMsg::Job(IpcJob {
            cmd_id,
            req_data: req_data.to_vec(),
            reply: reply_tx,
        });
        (job, reply_rx)
    }

    /// Submit a control-socket request and await the response asynchronously.
    ///
    /// This is the public async IPC API used by executor tasks (imaging/settings and
    /// initialization paths). It never blocks a worker thread: the request is handed to
    /// the owner thread and the caller awaits a oneshot, bounded by [`IPC_PUBLIC_TIMEOUT`].
    pub(crate) async fn request_async(
        &self,
        cmd_id: i32,
        req_data: &[u8],
    ) -> PlatformResult<(i32, Vec<u8>)> {
        self.epoch_gate(cmd_id)?;
        let sender = self.job_tx.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("IPC owner thread stopped".to_string())
        })?;
        let (job, reply_rx) = Self::make_job(cmd_id, req_data);
        sender.send(job).await.map_err(|_| {
            PlatformError::HardwareUnavailable("IPC owner thread stopped".to_string())
        })?;
        match tokio::time::timeout(IPC_PUBLIC_TIMEOUT, reply_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(PlatformError::HardwareFailure(
                "IPC owner dropped reply".to_string(),
            )),
            Err(_) => Err(PlatformError::Timeout),
        }
    }

    /// Refuse a request whose SDK handles belong to a dead daemon generation.
    ///
    /// This is the single chokepoint every handle must pass through: all 39 HAL
    /// trait methods are implemented on `AnykaIpc` and funnel into `request_async` /
    /// `request_blocking`. Checking here makes every outstanding handle inert at
    /// once, which is why handle ownership does not need to move.
    fn epoch_gate(&self, cmd_id: i32) -> PlatformResult<()> {
        if cmd_id == CMD_HELLO {
            return Ok(());
        }
        let attached = self.attached_epoch.load(Ordering::Acquire);
        if attached == EPOCH_DETACHED {
            return Err(PlatformError::HardwareUnavailable(
                "not attached to a vendor daemon".to_string(),
            ));
        }
        let observed = self.observed_epoch.load(Ordering::Acquire);
        // No exemption for observed == 0. Once attached, `attached` is a non-zero
        // v3 epoch (hello() rejects 0) and finish_attach seeds `observed` before
        // `attached`, so there is no legitimate "not yet polled" window. A zero
        // read therefore means the daemon is re-creating the ring right now —
        // vd_ring_create() memsets the header — which is precisely when stale
        // handles must be refused, not waved through.
        if observed != attached {
            return Err(PlatformError::HardwareUnavailable(format!(
                "vendor daemon restarted (attached epoch {attached}, observed {observed}); \
                 handles from the previous generation are stale"
            )));
        }
        Ok(())
    }

    /// Force both epochs, standing in for attach and the poller during tests.
    #[cfg(test)]
    pub(crate) fn set_epochs_for_test(&self, attached: u32, observed: u32) {
        self.attached_epoch.store(attached, Ordering::Release);
        self.observed_epoch.store(observed, Ordering::Release);
    }

    /// Construct an unattached client.
    ///
    /// No daemon needs to exist. Every resource is `None` and the epoch is
    /// [`EPOCH_DETACHED`], so [`Self::epoch_gate`] refuses every request until the
    /// supervisor attaches. This is what lets cold start and recovery share one path.
    pub fn new_detached() -> PlatformResult<Self> {
        let (loss_tx, loss_rx) = mpsc::channel::<PeerLoss>(PEER_LOSS_QUEUE_CAP);
        let (job_tx, owner_thread) = Self::spawn_owner(None, loss_tx.clone())?;
        Ok(Self {
            job_tx: Some(job_tx),
            owner_thread: Some(owner_thread),
            frame_main_stream: Mutex::new(None),
            frame_sub_stream: Mutex::new(None),
            shm_reader: Mutex::new(None),
            prefer_sub_on_tie: AtomicBool::new(false),
            loss_tx,
            loss_rx: Mutex::new(Some(loss_rx)),
            attached_epoch: AtomicU32::new(EPOCH_DETACHED),
            observed_epoch: AtomicU32::new(EPOCH_DETACHED),
        })
    }

    /// Install a freshly connected control stream on the owner thread, and wait for
    /// the owner to confirm it is live.
    ///
    /// Acknowledged rather than fire-and-forget on purpose: `finish_attach` issues
    /// `CMD_HELLO` on this same queue immediately afterwards. If installation were
    /// only queued, the handshake could reach the owner while its stream is still
    /// `None` and attach would fail spuriously against a healthy daemon.
    pub(crate) async fn give_ctrl_stream(&self, stream: UnixStream) -> PlatformResult<()> {
        let sender = self.job_tx.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("IPC owner thread stopped".to_string())
        })?;
        let (ack_tx, ack_rx) = oneshot::channel();
        sender
            .send(OwnerMsg::GiveStream {
                stream,
                ack: ack_tx,
            })
            .await
            .map_err(|_| {
                PlatformError::HardwareUnavailable("IPC owner thread stopped".to_string())
            })?;
        ack_rx.await.map_err(|_| {
            PlatformError::HardwareFailure("IPC owner dropped the stream ack".to_string())
        })
    }

    /// Clear the owner thread's control stream.
    ///
    /// Fire-and-forget: unlike installation this has no ordering requirement, because
    /// it can only ever make a subsequent request fail sooner.
    pub(crate) fn drop_ctrl_stream(&self) {
        if let Some(sender) = self.job_tx.as_ref() {
            let _ = sender.try_send(OwnerMsg::DropStream);
        }
    }

    /// Establish frame sockets, ring mapping and epoch against a live daemon.
    ///
    /// Connects in *reverse* creation order. The daemon creates the ring, then the
    /// control socket, then frame-main, then frame-sub, so a successful frame-sub
    /// connect proves the rest already exists. Attaching in creation order would
    /// race a still-initialising daemon and waste the retry budget.
    ///
    /// On any failure the partial attachment is rolled back via [`Self::detach`],
    /// so a failed attempt never leaves half a connection behind.
    pub(crate) async fn attach(&self) -> PlatformResult<u32> {
        let result = self.try_attach().await;
        if result.is_err() {
            self.detach();
        }
        result
    }

    async fn try_attach(&self) -> PlatformResult<u32> {
        // Readiness barrier: frame-sub is the last thing the daemon creates.
        let frame_sub = UnixStream::connect(FRAME_SUB_SOCKET_PATH).map_err(|e| {
            PlatformError::HardwareUnavailable(format!(
                "sub frame socket {FRAME_SUB_SOCKET_PATH} not ready: {e}"
            ))
        })?;
        let frame_main = UnixStream::connect(FRAME_MAIN_SOCKET_PATH).map_err(|e| {
            PlatformError::HardwareUnavailable(format!(
                "main frame socket {FRAME_MAIN_SOCKET_PATH} not ready: {e}"
            ))
        })?;
        *self
            .frame_sub_stream
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(frame_sub);
        *self
            .frame_main_stream
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(frame_main);

        // Control last of the three, per the documented order. `finish_attach`
        // issues CMD_HELLO through the owner thread, which replies
        // HardwareUnavailable while its stream is `None`, so the owner must be
        // holding a live stream before the handshake runs.
        let ctrl = connect_production_ctrl()?;
        // Must complete before finish_attach: `hello()` goes through the same
        // owner thread, and if installation were merely queued the handshake
        // could reach the owner first and be refused as HardwareUnavailable.
        self.give_ctrl_stream(ctrl).await?;

        let reader = ShmRingReader::open()?.ok_or_else(|| {
            PlatformError::HardwareUnavailable("shared memory ring not present".to_string())
        })?;
        self.finish_attach(reader).await
    }

    /// Handshake and epoch agreement, given an already-opened ring reader.
    ///
    /// Split out from `try_attach` so tests can supply an anonymous ring.
    async fn finish_attach(&self, reader: ShmRingReader) -> PlatformResult<u32> {
        let (epoch, version) = self.hello().await?;

        let ring_epoch = reader.epoch();
        if ring_epoch != epoch {
            return Err(PlatformError::HardwareUnavailable(format!(
                "daemon generation changed mid-attach (HELLO {epoch}, ring {ring_epoch}); retrying"
            )));
        }

        *self.shm_reader.lock().unwrap_or_else(|e| e.into_inner()) = Some(reader);
        // observed before attached: the gate reads attached first, so this ordering
        // can never leave a window where attached is set but observed is stale.
        self.observed_epoch.store(epoch, Ordering::Release);
        self.attached_epoch.store(epoch, Ordering::Release);

        tracing::info!(
            event = "ipc_attached",
            epoch,
            shm_version = version,
            "IPC attached to vendor daemon"
        );
        Ok(epoch)
    }

    #[cfg(test)]
    pub(crate) async fn finish_attach_for_test(
        &self,
        reader: ShmRingReader,
    ) -> PlatformResult<u32> {
        let result = self.finish_attach(reader).await;
        if result.is_err() {
            self.detach();
        }
        result
    }

    /// Report peer loss to the supervisor. Never blocks, never attaches.
    ///
    /// `try_send` on a depth-1 channel: if a loss is already queued the supervisor
    /// has not yet acted on it, and a second report would tell it nothing new.
    /// Dropping is correct here — blocking the owner thread or the frame reader on a
    /// busy supervisor would turn a recoverable outage into a stall.
    fn send_peer_loss(tx: &mpsc::Sender<PeerLoss>, reason: impl Into<String>) {
        let _ = tx.try_send(PeerLoss {
            reason: reason.into(),
        });
    }

    /// Report peer loss from a detection site that has `&self`.
    pub(crate) fn report_peer_loss(&self, reason: impl Into<String>) {
        Self::send_peer_loss(&self.loss_tx, reason);
    }

    /// Take the peer-loss receiver. Returns `Some` exactly once.
    ///
    /// Handing it out twice would split the loss stream between two consumers, so
    /// only the supervisor ever calls this.
    pub(crate) fn take_loss_rx(&self) -> Option<mpsc::Receiver<PeerLoss>> {
        self.loss_rx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    /// Refresh `observed_epoch` from the ring and report whether it still matches.
    ///
    /// Uses `try_lock`: the frame reader holds `shm_reader` during a read, and the
    /// poller must never block behind it. Contention means "no new information this
    /// tick", which is correct — the next tick is 1 s away.
    ///
    /// A live epoch of 0 reports *healthy*. That is not a hole in the gate: 0 only
    /// appears in the brief window where `vd_ring_create()` has memset the header but
    /// not yet stamped the new generation, and [`Self::epoch_gate`] already refuses
    /// every request while `observed != attached`. Tearing the pipeline down on that
    /// sample would trade a guaranteed-transient blip for a real restart; the next
    /// tick sees the new generation and reports loss properly.
    pub(crate) fn refresh_observed_epoch(&self) -> bool {
        let Ok(guard) = self.shm_reader.try_lock() else {
            return true; // no information; do not report loss on lock contention
        };
        let Some(reader) = guard.as_ref() else {
            return false; // detached
        };
        let live = reader.epoch();
        self.observed_epoch.store(live, Ordering::Release);
        let attached = self.attached_epoch.load(Ordering::Acquire);
        live == EPOCH_DETACHED || live == attached
    }

    /// Attach to a freshly created anonymous ring stamped with `epoch`.
    ///
    /// Test seam for the supervisor, which cannot reach `shm_ring`'s module-private
    /// test helpers.
    #[cfg(test)]
    pub(crate) fn attach_anon_ring_for_test(&self, epoch: u32) {
        let reader = shm_ring::tests::create_test_anon_reader();
        // SAFETY: offset 48 is inside the validated 64-byte header.
        unsafe {
            reader
                .base_ptr_for_test()
                .add(48)
                .cast::<u32>()
                .write_volatile(epoch);
        }
        *self.shm_reader.lock().unwrap_or_else(|e| e.into_inner()) = Some(reader);
        self.observed_epoch.store(epoch, Ordering::Release);
        self.attached_epoch.store(epoch, Ordering::Release);
    }

    /// Stamp a new generation into the mapped ring, standing in for a daemon restart.
    #[cfg(test)]
    pub(crate) fn stamp_ring_epoch_for_test(&self, epoch: u32) {
        let guard = self.shm_reader.lock().unwrap_or_else(|e| e.into_inner());
        let reader = guard.as_ref().expect("no ring mapped");
        // SAFETY: offset 48 is inside the validated 64-byte header.
        unsafe {
            reader
                .base_ptr_for_test()
                .add(48)
                .cast::<u32>()
                .write_volatile(epoch);
        }
    }

    /// Tear down the current attachment.
    ///
    /// Clears the epoch first: from this instant every in-flight request is refused
    /// by [`Self::epoch_gate`], so no stale handle can race the teardown. Then drops
    /// the frame sockets and unmaps the ring.
    ///
    /// Idempotent — the supervisor calls it on every failed attach attempt as well as
    /// on peer loss. Uses the poisoned-lock recovery path deliberately: a panicked
    /// frame reader must not make the connection permanently un-teardownable.
    pub(crate) fn detach(&self) {
        self.attached_epoch.store(EPOCH_DETACHED, Ordering::Release);
        self.observed_epoch.store(EPOCH_DETACHED, Ordering::Release);

        let mut main = self
            .frame_main_stream
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *main = None;
        drop(main);

        let mut sub = self
            .frame_sub_stream
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *sub = None;
        drop(sub);

        let mut shm = self.shm_reader.lock().unwrap_or_else(|e| e.into_inner());
        *shm = None;
        drop(shm);

        self.drop_ctrl_stream();

        tracing::info!(event = "ipc_detached", "IPC detached from vendor daemon");
    }

    #[cfg(test)]
    pub(crate) fn attached_epoch_for_test(&self) -> u32 {
        self.attached_epoch.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(crate) fn observed_epoch_for_test(&self) -> u32 {
        self.observed_epoch.load(Ordering::Acquire)
    }

    /// Perform the attach handshake and learn this daemon generation's epoch.
    ///
    /// Returns `(epoch, shm_version)`. Exempt from the epoch gate by construction:
    /// `epoch_gate` short-circuits on `CMD_HELLO`.
    pub(crate) async fn hello(&self) -> PlatformResult<(u32, u32)> {
        let (status, resp) = self.request_async(CMD_HELLO, &[]).await?;
        if status != AK_SUCCESS_I32 {
            return Err(PlatformError::HardwareUnavailable(format!(
                "vendor daemon rejected CMD_HELLO with status {status}"
            )));
        }
        if resp.len() < 8 {
            return Err(PlatformError::HardwareFailure(format!(
                "CMD_HELLO response too short: {} bytes (want 8)",
                resp.len()
            )));
        }
        let epoch = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
        let version = u32::from_le_bytes([resp[4], resp[5], resp[6], resp[7]]);
        if epoch == EPOCH_DETACHED {
            return Err(PlatformError::HardwareUnavailable(
                "vendor daemon reported epoch 0; it is pre-v3 or misbehaving".to_string(),
            ));
        }
        Ok((epoch, version))
    }

    /// Submit a control-socket request and block the current OS thread for the response.
    ///
    /// This is the sync-by-need variant for true OS threads (the `venc-read` frame thread
    /// issuing IDR/venc commands and shutdown workers). It MUST NOT be called directly from
    /// a tokio executor task — [`Self::send_request`] routes async contexts to
    /// [`Self::request_async`] instead.
    pub(crate) fn request_blocking(
        &self,
        cmd_id: i32,
        req_data: &[u8],
    ) -> PlatformResult<(i32, Vec<u8>)> {
        self.epoch_gate(cmd_id)?;
        let sender = self.job_tx.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("IPC owner thread stopped".to_string())
        })?;
        let (job, reply_rx) = Self::make_job(cmd_id, req_data);
        sender.blocking_send(job).map_err(|_| {
            PlatformError::HardwareUnavailable("IPC owner thread stopped".to_string())
        })?;
        reply_rx
            .blocking_recv()
            .map_err(|_| PlatformError::HardwareFailure("IPC owner dropped reply".to_string()))?
    }

    /// Context-adaptive synchronous request for the remaining **sync-only** callers.
    ///
    /// This is intended for two cases and *only* these two cases:
    /// - **Plain OS threads** with no tokio runtime context (e.g. the `venc-read` frame
    ///   thread issuing IDR/venc commands, shutdown workers) → routes to
    ///   [`Self::request_blocking`].
    /// - The **one-time init `block_on` driver** thread (the multi-thread runtime's
    ///   `block_on` entry, which is a runtime context but not a task being polled) →
    ///   drives [`Self::request_async`] under `block_in_place`.
    ///
    /// # WARNING: must not be called from an async task / handler
    /// `block_in_place` does **not** free the calling worker — it only allows tokio to
    /// spawn an *additional* worker. The calling worker stays parked for the full RPC
    /// (up to [`IPC_PUBLIC_TIMEOUT`]). Async handlers (imaging/settings and any other
    /// ONVIF control RPC running on a tokio worker) MUST `.await` [`Self::request_async`]
    /// directly instead of calling this method. The imaging HAL path was migrated to
    /// `request_async` in Phase 2 for exactly this reason. Do not reintroduce
    /// `send_request` on an async-handler path.
    ///
    /// # Panics
    /// `block_in_place` panics on a `current_thread` runtime. Production uses a
    /// multi-thread runtime; unit tests that exercise this from an async context use the
    /// multi-thread flavor.
    pub(crate) fn send_request(
        &self,
        cmd_id: i32,
        req_data: &[u8],
    ) -> PlatformResult<(i32, Vec<u8>)> {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(move || {
                handle.block_on(self.request_async(cmd_id, req_data))
            }),
            Err(_) => self.request_blocking(cmd_id, req_data),
        }
    }

    /// Send request expecting a handle response (8-byte i64).
    pub(crate) fn send_handle_request(
        &self,
        cmd_id: i32,
        req_data: &[u8],
    ) -> PlatformResult<*mut c_void> {
        let (status, resp_data) = self.send_request(cmd_id, req_data)?;
        if status == VD_STATUS_STALE_EPOCH {
            return Err(PlatformError::HardwareUnavailable(format!(
                "vendor daemon rejected {} as a stale handle; the attachment is from a \
                 dead generation",
                Self::cmd_name(cmd_id)
            )));
        }
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
    /// writes frames to the ring buffer, and pushes unsolicited 20-byte
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

    /// Whether the vendor-daemon has signalled shutdown via the shared ring header.
    ///
    /// Returns `false` when the shm reader is unavailable or the lock is contended —
    /// callers treat that as "not shut down" and fall back to socket-level detection.
    pub fn shm_is_shutdown(&self) -> bool {
        match self.shm_reader.try_lock() {
            Ok(guard) => guard.as_ref().is_some_and(|shm| shm.is_shutdown()),
            Err(_) => false,
        }
    }

    /// Map a "no frame available" condition onto [`PlatformError::Shutdown`] when the
    /// daemon has flagged shutdown, otherwise return `fallback` unchanged.
    fn shutdown_or(&self, fallback: PlatformError) -> PlatformError {
        if self.shm_is_shutdown() {
            warn!(
                event = "push_daemon_shutdown_flagged",
                diag_monotonic_ms = monotonic_millis(),
                suppressed = %fallback,
                "vendor-daemon set VD_FLAG_SHUTDOWN; ending push frame delivery"
            );
            PlatformError::Shutdown("vendor-daemon set VD_FLAG_SHUTDOWN".into())
        } else {
            fallback
        }
    }

    /// Current `(write_seq, read_seq)` from the shared ring header, or `None` when the ring
    /// is unavailable.
    ///
    /// A poisoned lock or absent reader must not report `(0, 0)`: that reads as an empty
    /// ring, which is the opposite conclusion from the wedged ring this telemetry exists to
    /// catch.
    ///
    /// `try_lock`, matching [`AnykaIpc::shm_is_shutdown`]: the caller reads this while holding
    /// both frame-socket mutexes, so blocking on a contended ring would stall frame delivery
    /// for a diagnostic. Contention reports the ring as unavailable, which it effectively is.
    fn shm_ring_sequences(&self) -> Option<(u32, u32)> {
        let guard = self.shm_reader.try_lock().ok()?;
        guard.as_ref().map(|shm| (shm.write_seq(), shm.read_seq()))
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
    /// In push mode, the daemon sends 20-byte notifications proactively.
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
            // A daemon that shuts down cleanly sets VD_FLAG_SHUTDOWN but need not
            // close the notification socket, so poll just keeps timing out. Without
            // this check the caller would retry forever on a producer that is gone.
            return Err(self.shutdown_or(PlatformError::Timeout));
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
            // A hang-up after the daemon flagged shutdown is orderly, not a failure.
            let err = self.shutdown_or(PlatformError::HardwareFailure(format!(
                "push notification socket disconnected (main_hup_err={}, sub_hup_err={})",
                main_hup_err, sub_hup_err
            )));
            // Report only an unexpected hang-up. A shutdown the daemon announced is
            // not peer loss, and waking the supervisor for it would have it attach
            // into a daemon that is on its way out.
            if !self.shm_is_shutdown() {
                self.report_peer_loss(format!(
                    "frame socket EOF (main_hup_err={main_hup_err}, sub_hup_err={sub_hup_err})"
                ));
            }
            return Err(err);
        } else {
            return Err(self.shutdown_or(PlatformError::Timeout));
        };

        // Check for dropped-frame notification (Fix 4 integration)
        if notif.is_frame_dropped() {
            // Ring occupancy is the only thing that distinguishes a transient burst from a
            // permanently wedged ring (`write_seq - read_seq` stuck at or above the slot
            // count), and the periodic delivery telemetry stops firing once drops are total.
            match self.shm_ring_sequences() {
                Some((write_seq, read_seq)) => warn!(
                    event = "push_notification_frame_dropped",
                    diag_monotonic_ms = monotonic_millis(),
                    channel = channel_name,
                    slot_index = notif.slot_index,
                    flags = notif.flags,
                    write_seq,
                    read_seq,
                    in_flight = write_seq.wrapping_sub(read_seq),
                    slot_count = VD_SHM_SLOT_COUNT,
                    "daemon reported dropped frame notification"
                ),
                // Occupancy fields omitted rather than zeroed: unavailable is not "empty".
                None => warn!(
                    event = "push_notification_frame_dropped",
                    diag_monotonic_ms = monotonic_millis(),
                    channel = channel_name,
                    slot_index = notif.slot_index,
                    flags = notif.flags,
                    ring_state = "unavailable",
                    slot_count = VD_SHM_SLOT_COUNT,
                    "daemon reported dropped frame notification"
                ),
            }
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
            .read_notified_slot_into_bytesmut(&notif, pool)
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
                    while let Some((cmd_id, req_data)) = Self::read_request(&mut stream) {
                        let (status, resp_data) = handler(cmd_id, &req_data);
                        Self::write_response(&mut stream, status, &resp_data);
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

        /// Read one request frame: cmd_id (i32 LE) + req_len (u32 LE) + req_data.
        ///
        /// Returns `None` on EOF or any read error, signalling the serve loop to stop.
        fn read_request(stream: &mut UnixStream) -> Option<(i32, Vec<u8>)> {
            let mut cmd_buf = [0u8; 4];
            stream.read_exact(&mut cmd_buf).ok()?;
            let cmd_id = i32::from_le_bytes(cmd_buf);

            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf).ok()?;
            let req_len = u32::from_le_bytes(len_buf) as usize;

            let mut req_data = vec![0u8; req_len];
            if req_len > 0 {
                stream.read_exact(&mut req_data).ok()?;
            }
            Some((cmd_id, req_data))
        }

        /// Write one response frame: status (i32 LE) + resp_len (u32 LE) + resp_data.
        fn write_response(stream: &mut UnixStream, status: i32, resp_data: &[u8]) {
            let _ = stream.write_all(&status.to_le_bytes());
            let _ = stream.write_all(&(resp_data.len() as u32).to_le_bytes());
            if !resp_data.is_empty() {
                let _ = stream.write_all(resp_data);
            }
            let _ = stream.flush();
        }

        /// Spawns a daemon that accepts one connection then immediately closes it,
        /// simulating a daemon that died mid-request.
        pub fn start_then_hangup() -> Self {
            let counter = TEST_DAEMON_COUNTER.fetch_add(1, Ordering::SeqCst);
            let socket_path = format!(
                "/tmp/test-vendor-daemon-hangup-{}-{}.sock",
                std::process::id(),
                counter
            );
            let _ = std::fs::remove_file(&socket_path);
            let listener = UnixListener::bind(&socket_path).unwrap();
            let path_clone = socket_path.clone();
            let handle = std::thread::spawn(move || {
                if let Ok((stream, _)) = listener.accept() {
                    drop(stream);
                }
                let _ = std::fs::remove_file(&path_clone);
            });
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
        // The owner thread idles on the control stream (these tests issue no control
        // RPCs), so the peer end can be dropped without affecting the test.
        let (ctrl_a, _ctrl_b) = UnixStream::pair().unwrap();
        let (main_a, _main_b) = UnixStream::pair().unwrap();
        let (sub_a, _sub_b) = UnixStream::pair().unwrap();
        AnykaIpc::from_parts_for_test(ctrl_a, Some(main_a), Some(sub_a), None)
    }
}

// ============================================================================
// Core protocol and encoding tests (stay in mod.rs)
// ============================================================================

#[cfg(test)]
mod tests {
    /// `(cmd_id, request_bytes)` captured by the fake daemon, shared with the test thread.
    type CapturedCommands = std::sync::Arc<std::sync::Mutex<Vec<(i32, Vec<u8>)>>>;

    use super::test_helpers::*;
    use super::*;
    use crate::hal::common::AK_FAILED_I32;
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    /// A simple success command round-trips correctly: set_brightness returns AK_SUCCESS.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ipc_connect_and_simple_command_returns_success() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);

        // set_brightness is part of ImagingHalTrait; accessible from within the crate.
        let result =
            <AnykaIpc as crate::hal::common::imaging::ImagingHalTrait>::set_brightness(&ipc, 50)
                .await;

        assert_eq!(result, AK_SUCCESS_I32, "expected AK_SUCCESS from daemon");
    }

    /// When the daemon returns status=-1 (AK_FAILED), the trait method propagates it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ipc_error_response_propagates_ak_failed() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_FAILED_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        let result =
            <AnykaIpc as crate::hal::common::imaging::ImagingHalTrait>::set_brightness(&ipc, 50)
                .await;

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
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);

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
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ipc_multiple_requests_same_connection_all_succeed() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);

        for i in 0..3 {
            let result =
                <AnykaIpc as crate::hal::common::imaging::ImagingHalTrait>::set_brightness(
                    &ipc,
                    50 + i,
                )
                .await;
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
        let stream_handle = std::ptr::dangling_mut::<std::ffi::c_void>();
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
    fn test_read_push_notification_reads_20_byte_notification() {
        let (mut writer, mut reader) = UnixStream::pair().unwrap();
        let raw = [
            0x02, 0x00, 0x00, 0x00, // slot_index = 2
            0x34, 0x12, 0x00, 0x00, // frame_len = 0x1234
            0x01, 0x00, 0x00, 0x00, // flags = VD_NOTIFY_LAST_FRAGMENT
            0x01, 0x00, 0x00, 0x00, // stream_id = sub
            0x66, 0x00, 0x00, 0x00, // seq_no = 102
        ];
        writer.write_all(&raw).unwrap();
        writer.flush().unwrap();

        let notif = AnykaIpc::read_push_notification(&mut reader, "main").unwrap();
        assert_eq!(notif.slot_index, 2);
        assert_eq!(notif.frame_len, 0x1234);
        assert_eq!(notif.flags, 1);
        assert_eq!(notif.stream_id, 1);
        assert_eq!(notif.seq_no, 102);
    }

    #[test]
    fn test_recv_pushed_frame_reports_shutdown_when_daemon_flags_it() {
        use super::shm_ring::tests::create_test_anon_reader;

        // Frame socket stays open but silent, so poll() times out with no data —
        // exactly what a cleanly-stopped daemon looks like from the reader's side.
        let (frame_reader, _frame_writer) = UnixStream::pair().unwrap();
        let (ctrl_a, _ctrl_b) = UnixStream::pair().unwrap();

        let shm = create_test_anon_reader();
        shm.set_shutdown_for_test();

        let ipc = AnykaIpc::from_parts_for_test(ctrl_a, Some(frame_reader), None, Some(shm));

        match ipc.recv_pushed_frame(None) {
            Err(PlatformError::Shutdown(reason)) => {
                assert!(
                    reason.contains("VD_FLAG_SHUTDOWN"),
                    "reason should name the flag, got: {reason}"
                );
            }
            Err(other) => panic!("expected Shutdown once the daemon flags it, got: {other:?}"),
            Ok(_) => panic!("expected Shutdown once the daemon flags it, got a frame"),
        }
    }

    #[test]
    fn test_recv_pushed_frame_still_times_out_when_shutdown_not_flagged() {
        use super::shm_ring::tests::create_test_anon_reader;

        // Same silent-socket setup, but the daemon has *not* flagged shutdown, so the
        // caller must still see a retryable Timeout rather than a terminal error.
        let (frame_reader, _frame_writer) = UnixStream::pair().unwrap();
        let (ctrl_a, _ctrl_b) = UnixStream::pair().unwrap();

        let ipc = AnykaIpc::from_parts_for_test(
            ctrl_a,
            Some(frame_reader),
            None,
            Some(create_test_anon_reader()),
        );

        assert!(
            matches!(ipc.recv_pushed_frame(None), Err(PlatformError::Timeout)),
            "a live-but-idle daemon must stay retryable"
        );
    }

    #[test]
    fn test_recv_pushed_frame_returns_resource_busy_on_frame_drop_notification() {
        // Create a Unix socket pair - one end goes to AnykaIpc, other we write to
        let (reader, mut writer) = UnixStream::pair().unwrap();

        // A throwaway control stream for the owner thread; this test only exercises the
        // frame-drop path and issues no control RPCs.
        let (ctrl_a, _ctrl_b) = UnixStream::pair().unwrap();

        // Construct AnykaIpc with only frame_main_stream set (no shm reader needed for drop path)
        let ipc = AnykaIpc::from_parts_for_test(ctrl_a, Some(reader), None, None);

        // Write a 20-byte drop notification: slot_index=0, frame_len=0, flags=VD_NOTIFY_FRAME_DROPPED(4)
        let drop_notification = [
            0x00, 0x00, 0x00, 0x00, // slot_index = 0
            0x00, 0x00, 0x00, 0x00, // frame_len = 0
            0x04, 0x00, 0x00, 0x00, // flags = VD_NOTIFY_FRAME_DROPPED (1 << 2)
            0x00, 0x00, 0x00, 0x00, // stream_id = main
            0x09, 0x00, 0x00, 0x00, // seq_no = 9
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

    /// An idle ring reads `(0, 0)`. That must come back as `Some((0, 0))`, not `None`: the
    /// whole point of the `Option` is that "empty" and "unavailable" are different answers.
    #[test]
    fn test_shm_ring_sequences_reports_an_idle_ring_as_zero_not_unavailable() {
        use super::shm_ring::tests::create_test_anon_reader;

        let (ctrl_a, _ctrl_b) = UnixStream::pair().unwrap();
        let ipc =
            AnykaIpc::from_parts_for_test(ctrl_a, None, None, Some(create_test_anon_reader()));

        assert_eq!(
            ipc.shm_ring_sequences(),
            Some((0, 0)),
            "an available but idle ring must report occupancy, not unavailability"
        );
    }

    /// No reader at all is genuinely unavailable, and the dropped-frame warning must omit the
    /// occupancy fields rather than publish a zero that reads as an empty ring.
    #[test]
    fn test_shm_ring_sequences_reports_none_when_reader_absent() {
        let (ctrl_a, _ctrl_b) = UnixStream::pair().unwrap();
        let ipc = AnykaIpc::from_parts_for_test(ctrl_a, None, None, None);

        assert_eq!(ipc.shm_ring_sequences(), None);
    }

    /// The caller reads ring occupancy while holding both frame-socket mutexes, so a contended
    /// ring must report unavailable instead of blocking frame delivery for a diagnostic.
    ///
    /// The holder releases on a timeout rather than waiting for this thread, so a regression to
    /// a blocking `lock()` fails the assertion instead of hanging the suite.
    #[test]
    fn test_shm_ring_sequences_reports_none_while_the_ring_lock_is_held() {
        use super::shm_ring::tests::create_test_anon_reader;
        use std::sync::mpsc;

        let (ctrl_a, _ctrl_b) = UnixStream::pair().unwrap();
        let ipc =
            AnykaIpc::from_parts_for_test(ctrl_a, None, None, Some(create_test_anon_reader()));

        let (held_tx, held_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();

        let holder = &ipc;
        std::thread::scope(|scope| {
            // `move` so the non-Sync Receiver is owned by the holder rather than borrowed.
            scope.spawn(move || {
                let _guard = holder.shm_reader.lock().expect("hold the ring lock");
                held_tx.send(()).expect("signal that the lock is held");
                let _ = release_rx.recv_timeout(Duration::from_secs(2));
            });

            held_rx.recv().expect("ring lock should be held");
            assert_eq!(
                ipc.shm_ring_sequences(),
                None,
                "a contended ring must report unavailable rather than wait for the lock"
            );
            let _ = release_tx.send(());
        });
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

        let captured: CapturedCommands = Arc::new(Mutex::new(Vec::new()));
        let captured_closure = Arc::clone(&captured);
        let daemon = FakeDaemon::start(move |cmd_id, req| {
            captured_closure
                .lock()
                .unwrap()
                .push((cmd_id, req.to_vec()));
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);
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

        let captured: CapturedCommands = Arc::new(Mutex::new(Vec::new()));
        let captured_closure = Arc::clone(&captured);
        let daemon = FakeDaemon::start(move |cmd_id, req| {
            captured_closure
                .lock()
                .unwrap()
                .push((cmd_id, req.to_vec()));
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);

        ipc.stop_push(Some(StreamId::VideoMain)).unwrap();

        let captured = captured.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].0, CMD_VENC_STOP_PUSH);
        assert_eq!(captured[0].1, 0u32.to_le_bytes().to_vec());
    }

    #[test]
    fn test_stop_push_without_stream_id_sends_empty_payload() {
        use std::sync::{Arc, Mutex};

        let captured: CapturedCommands = Arc::new(Mutex::new(Vec::new()));
        let captured_closure = Arc::clone(&captured);
        let daemon = FakeDaemon::start(move |cmd_id, req| {
            captured_closure
                .lock()
                .unwrap()
                .push((cmd_id, req.to_vec()));
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);

        ipc.stop_push(None).unwrap();

        let captured = captured.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].0, CMD_VENC_STOP_PUSH);
        assert!(captured[0].1.is_empty());
    }

    /// Test that IPC control socket times out when the daemon hangs (stops responding).
    /// This verifies that the IPC_CTRL_TIMEOUT (10 seconds) is properly enforced and
    /// returns PlatformError::Timeout rather than hanging forever or returning HardwareFailure.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ipc_ctrl_socket_times_out_on_hung_daemon() {
        use std::time::{Duration, Instant};

        // Create a fake daemon that delays response longer than the timeout (10 seconds)
        // We use 15 seconds delay to ensure timeout fires
        let delay = Duration::from_secs(15);
        let daemon = FakeDaemon::start_with_delay(delay, |_cmd_id, _req| (AK_SUCCESS_I32, vec![]));

        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);

        // Time the request - it should timeout
        let start = Instant::now();
        let result =
            <AnykaIpc as crate::hal::common::imaging::ImagingHalTrait>::set_brightness(&ipc, 50)
                .await;
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
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ipc_ctrl_socket_timeout_fires_within_15_seconds() {
        // Create a fake daemon that delays response for 20 seconds (longer than timeout + margin)
        let delay = Duration::from_secs(20);
        let daemon = FakeDaemon::start_with_delay(delay, |_cmd_id, _req| (AK_SUCCESS_I32, vec![]));

        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        let start = Instant::now();
        let _result =
            <AnykaIpc as crate::hal::common::imaging::ImagingHalTrait>::set_brightness(&ipc, 50)
                .await;
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

    // ------------------------------------------------------------------------
    // Phase 2: owner-thread async / blocking / adaptive request paths
    // ------------------------------------------------------------------------

    /// The async public API round-trips a request through the owner thread.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_request_async_success() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);

        let (status, resp) = ipc
            .request_async(CMD_ISP_SET_BRIGHTNESS, &50i32.to_le_bytes())
            .await
            .expect("request_async should succeed");
        assert_eq!(status, AK_SUCCESS_I32);
        assert!(resp.is_empty());
    }

    /// The async public API returns an error (rather than hanging) when the daemon is
    /// hung, bounded by the socket timeout and [`IPC_PUBLIC_TIMEOUT`].
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_request_async_errors_on_hung_daemon() {
        use std::time::Instant;
        let daemon = FakeDaemon::start_with_delay(Duration::from_secs(15), |_c, _r| {
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);

        let start = Instant::now();
        let result = ipc
            .request_async(CMD_ISP_SET_BRIGHTNESS, &50i32.to_le_bytes())
            .await;
        let elapsed = start.elapsed();

        assert!(result.is_err(), "hung daemon should yield an error");
        assert!(
            elapsed >= Duration::from_secs(10),
            "should wait for the socket timeout, took {:?}",
            elapsed
        );
        assert!(
            elapsed < Duration::from_secs(13),
            "should be bounded by the public timeout, took {:?}",
            elapsed
        );
    }

    /// While one task awaits a hung control RPC, the executor keeps scheduling other
    /// tasks/timers promptly — the awaiting task must not occupy a worker thread.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_request_async_does_not_stall_executor() {
        use std::time::Instant;
        let daemon = FakeDaemon::start_with_delay(Duration::from_secs(15), |_c, _r| {
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        let hung = tokio::spawn(async move {
            ipc.request_async(CMD_ISP_SET_BRIGHTNESS, &50i32.to_le_bytes())
                .await
        });

        // Unrelated timers must keep firing while the control RPC is stuck.
        let start = Instant::now();
        for _ in 0..5 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "executor stalled: unrelated timers took {:?}",
            start.elapsed()
        );

        // Drain the hung task so teardown is orderly (it errors once the owner's socket
        // timeout fires).
        let _ = hung.await;
    }

    /// The context-adaptive `send_request` works from the multi-thread runtime's
    /// `block_on` driver thread (the init path) without panicking in `block_in_place`.
    #[test]
    fn test_send_request_within_multi_thread_block_on_succeeds() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();

        let result =
            rt.block_on(async { ipc.send_request(CMD_ISP_SET_BRIGHTNESS, &50i32.to_le_bytes()) });
        let (status, _resp) = result.expect("send_request from block_on driver should succeed");
        assert_eq!(status, AK_SUCCESS_I32);
    }

    /// `request_blocking` works from a plain OS thread (the venc-read / shutdown path).
    #[test]
    fn test_request_blocking_from_os_thread_succeeds() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);

        let (status, _resp) = ipc
            .request_blocking(CMD_ISP_SET_BRIGHTNESS, &50i32.to_le_bytes())
            .expect("request_blocking should succeed");
        assert_eq!(status, AK_SUCCESS_I32);
    }

    // ========================================================================
    // Attach handshake (CMD_HELLO)
    // ========================================================================

    #[tokio::test]
    async fn hello_parses_epoch_and_version_from_the_daemon() {
        let daemon = test_helpers::FakeDaemon::start(|cmd_id, _req| {
            if cmd_id == CMD_HELLO {
                let mut resp = Vec::with_capacity(8);
                resp.extend_from_slice(&0x1234_5678u32.to_le_bytes());
                resp.extend_from_slice(&3u32.to_le_bytes());
                (AK_SUCCESS_I32, resp)
            } else {
                (AK_FAILED_I32, vec![])
            }
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        let (epoch, version) = ipc.hello().await.unwrap();

        assert_eq!(epoch, 0x1234_5678);
        assert_eq!(version, 3);
    }

    #[test]
    fn stale_handle_status_is_reported_as_unavailable_not_a_generic_failure() {
        // The daemon returns VD_STATUS_STALE_EPOCH when a token names a dead
        // generation, a reused slot, or the wrong object kind. That is an
        // attachment problem, not a bad argument, and must read as such.
        let daemon =
            test_helpers::FakeDaemon::start(|_c, _r| (VD_STATUS_STALE_EPOCH, vec![0u8; 8]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        let err = ipc.send_handle_request(CMD_VI_OPEN, &[0u8; 4]).unwrap_err();

        match err {
            PlatformError::HardwareUnavailable(msg) => {
                assert!(msg.contains("stale"), "message should say stale: {msg}");
            }
            other => panic!("expected HardwareUnavailable, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ctrl_error_reports_peer_loss_without_attaching() {
        let daemon = test_helpers::FakeDaemon::start_then_hangup();
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(4, 4);
        let mut loss_rx = ipc.take_loss_rx().expect("receiver available once");

        let _ = ipc.request_async(CMD_VI_CLOSE, &[0u8; 8]).await;

        let loss = loss_rx.recv().await.expect("ctrl error must report loss");
        assert!(
            loss.reason.contains("control"),
            "reason should name the site, got {:?}",
            loss.reason
        );
        // The detection site reports; it must never attach or detach on its own.
        assert_eq!(ipc.attached_epoch_for_test(), 4, "must not self-heal");
    }

    #[tokio::test]
    async fn repeated_losses_do_not_block_the_reporter() {
        // The channel is bounded at 1: one queued loss is as informative as ten,
        // and the owner thread must never block on a supervisor that is busy.
        let daemon = test_helpers::FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        let _rx = ipc.take_loss_rx();

        for _ in 0..100 {
            ipc.report_peer_loss("synthetic");
        }
        // Reaching here without hanging is the assertion.
    }

    #[test]
    fn loss_receiver_is_handed_out_once() {
        let ipc = AnykaIpc::new_detached().unwrap();
        assert!(ipc.take_loss_rx().is_some());
        assert!(
            ipc.take_loss_rx().is_none(),
            "a second owner would split the loss stream"
        );
    }

    #[tokio::test]
    async fn ctrl_io_error_does_not_silently_reconnect() {
        // A daemon that accepts, then hangs up. The old code reconnected to the
        // production socket and retried; that must no longer happen.
        let daemon = test_helpers::FakeDaemon::start_then_hangup();
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        let err = ipc
            .request_async(CMD_VI_CLOSE, &[0u8; 8])
            .await
            .unwrap_err();

        assert!(
            matches!(
                err,
                PlatformError::HardwareFailure(_) | PlatformError::Timeout
            ),
            "expected the I/O error to surface, got {err:?}"
        );
    }

    // ========================================================================
    // Attach / detach lifecycle
    // ========================================================================

    #[tokio::test]
    async fn attach_rejects_a_ring_epoch_that_disagrees_with_hello() {
        // Daemon restarted between HELLO and the ring being mapped: the two epochs
        // disagree, so the attachment is already stale and must not be pinned.
        let daemon = test_helpers::FakeDaemon::start(|_c, _r| {
            let mut resp = Vec::new();
            resp.extend_from_slice(&11u32.to_le_bytes());
            resp.extend_from_slice(&3u32.to_le_bytes());
            (AK_SUCCESS_I32, resp)
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        let reader = shm_ring::tests::create_test_anon_reader();
        // SAFETY: offset 48 is inside the validated header.
        unsafe {
            reader
                .base_ptr_for_test()
                .add(48)
                .cast::<u32>()
                .write_volatile(12);
        }

        let err = ipc.finish_attach_for_test(reader).await.unwrap_err();

        assert!(matches!(err, PlatformError::HardwareUnavailable(_)));
        assert_eq!(ipc.attached_epoch_for_test(), EPOCH_DETACHED);
    }

    #[tokio::test]
    async fn attach_pins_the_epoch_when_hello_and_ring_agree() {
        let daemon = test_helpers::FakeDaemon::start(|_c, _r| {
            let mut resp = Vec::new();
            resp.extend_from_slice(&11u32.to_le_bytes());
            resp.extend_from_slice(&3u32.to_le_bytes());
            (AK_SUCCESS_I32, resp)
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        let reader = shm_ring::tests::create_test_anon_reader();
        unsafe {
            reader
                .base_ptr_for_test()
                .add(48)
                .cast::<u32>()
                .write_volatile(11);
        }

        ipc.finish_attach_for_test(reader).await.unwrap();

        assert_eq!(ipc.attached_epoch_for_test(), 11);
        assert_eq!(ipc.observed_epoch_for_test(), 11);
    }

    #[test]
    fn new_detached_succeeds_without_a_daemon() {
        // R5: cold start and recovery share one path, so construction must not
        // require a live daemon. Attaching is the supervisor's job.
        let ipc = AnykaIpc::new_detached().expect("construction must not need a daemon");

        assert_eq!(ipc.attached_epoch_for_test(), EPOCH_DETACHED);
        assert!(ipc.frame_main_stream.lock().unwrap().is_none());
        assert!(ipc.shm_reader.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn detached_owner_refuses_requests_until_a_stream_is_installed() {
        // The owner thread holds no stream after new_detached(). A request that
        // slips past the gate must still get a clean error, not a panic.
        let ipc = AnykaIpc::new_detached().unwrap();
        ipc.set_epochs_for_test(1, 1); // bypass the gate to reach the owner

        let err = ipc
            .request_async(CMD_VI_CLOSE, &[0u8; 8])
            .await
            .unwrap_err();

        assert!(
            matches!(err, PlatformError::HardwareUnavailable(_)),
            "expected HardwareUnavailable, got {err:?}"
        );
    }

    #[tokio::test]
    async fn give_ctrl_stream_is_ordered_ahead_of_the_next_request() {
        // give_ctrl_stream must be acknowledged, not fire-and-forget: hello() goes
        // through the same owner thread immediately afterwards and would otherwise
        // race the installation and be refused on a healthy daemon.
        let daemon = test_helpers::FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_detached().unwrap();
        ipc.set_epochs_for_test(1, 1);

        let stream = UnixStream::connect(&daemon.socket_path).unwrap();
        ipc.give_ctrl_stream(stream).await.unwrap();

        let (status, _) = ipc.request_async(CMD_VI_CLOSE, &[0u8; 8]).await.unwrap();
        assert_eq!(status, AK_SUCCESS_I32);
    }

    #[test]
    fn detach_clears_every_resource_and_the_epoch() {
        let daemon = test_helpers::FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(5, 5);

        ipc.detach();

        assert_eq!(ipc.attached_epoch_for_test(), EPOCH_DETACHED);
        assert_eq!(ipc.observed_epoch_for_test(), EPOCH_DETACHED);
        assert!(ipc.frame_main_stream.lock().unwrap().is_none());
        assert!(ipc.frame_sub_stream.lock().unwrap().is_none());
        assert!(ipc.shm_reader.lock().unwrap().is_none());
    }

    #[test]
    fn detach_is_idempotent() {
        let daemon = test_helpers::FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(5, 5);

        ipc.detach();
        ipc.detach(); // must not panic or poison a mutex

        assert_eq!(ipc.attached_epoch_for_test(), EPOCH_DETACHED);
    }

    // ========================================================================
    // Epoch gate
    // ========================================================================

    #[tokio::test]
    async fn request_is_refused_without_writing_when_the_epoch_moved() {
        use std::sync::Arc as StdArc;
        use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

        let seen = StdArc::new(AtomicUsize::new(0));
        let seen_in_daemon = StdArc::clone(&seen);
        let daemon = test_helpers::FakeDaemon::start(move |_cmd_id, _req| {
            seen_in_daemon.fetch_add(1, AtomicOrdering::SeqCst);
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        // Attached to generation 7, but the ring now reports generation 8.
        ipc.set_epochs_for_test(7, 8);

        let err = ipc
            .request_async(CMD_VI_CLOSE, &[0u8; 8])
            .await
            .unwrap_err();

        assert!(matches!(err, PlatformError::HardwareUnavailable(_)));
        assert_eq!(
            seen.load(AtomicOrdering::SeqCst),
            0,
            "a stale handle must never reach the daemon"
        );
    }

    #[tokio::test]
    async fn request_is_refused_when_detached() {
        let daemon = test_helpers::FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(EPOCH_DETACHED, 0);

        let err = ipc
            .request_async(CMD_VI_CLOSE, &[0u8; 8])
            .await
            .unwrap_err();

        assert!(matches!(err, PlatformError::HardwareUnavailable(_)));
    }

    #[tokio::test]
    async fn request_passes_when_epochs_agree() {
        let daemon = test_helpers::FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(7, 7);

        let (status, _) = ipc.request_async(CMD_VI_CLOSE, &[0u8; 8]).await.unwrap();

        assert_eq!(status, AK_SUCCESS_I32);
    }

    #[test]
    fn hello_is_exempt_from_the_gate() {
        let daemon = test_helpers::FakeDaemon::start(|_c, _r| {
            let mut resp = Vec::new();
            resp.extend_from_slice(&9u32.to_le_bytes());
            resp.extend_from_slice(&3u32.to_le_bytes());
            (AK_SUCCESS_I32, resp)
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(EPOCH_DETACHED, 0);

        // Must succeed while detached, or attach could never happen.
        let (status, _) = ipc.request_blocking(CMD_HELLO, &[]).unwrap();

        assert_eq!(status, AK_SUCCESS_I32);
    }

    #[tokio::test]
    async fn hello_rejects_a_zero_epoch() {
        // A daemon that reports epoch 0 is either pre-v3 or broken. Either way we
        // must not pin 0, because 0 is our own "detached" sentinel.
        let daemon = test_helpers::FakeDaemon::start(|_cmd_id, _req| {
            let mut resp = Vec::with_capacity(8);
            resp.extend_from_slice(&0u32.to_le_bytes());
            resp.extend_from_slice(&3u32.to_le_bytes());
            (AK_SUCCESS_I32, resp)
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        let err = ipc.hello().await.unwrap_err();

        assert!(
            matches!(err, PlatformError::HardwareUnavailable(_)),
            "expected HardwareUnavailable, got {err:?}"
        );
    }
}
