//! Shared memory ring buffer reader for zero-copy frame delivery.
//!
//! This module implements the Rust (reader) side of a POSIX shared memory ring buffer
//! for receiving encoded video frames from the C vendor-daemon without socket copies.
//!
//! # Layout
//!
//! ```text
//! +------------------+
//! |  Ring Header     | 64 bytes
//! +------------------+
//! |  Slot 0 Header  | 64 bytes
//! +------------------+
//! |  Slot 0 Data    | 128KB - 64B
//! +------------------+
//! |  Slot 1 Header  | 64 bytes
//! +------------------+
//! |  Slot 1 Data    | 128KB - 64B
//! +------------------+
//!       ...               ...
//! +------------------+
//! |  Slot 7 Header  | 64 bytes
//! +------------------+
//! |  Slot 7 Data    | 128KB - 64B
//! +------------------+
//! ```
//!
//! # Usage
//!
//! ```ignore
//! // Open the shared memory region
//! let reader = ShmRingReader::open()?.expect("vendor-daemon not using shm mode");
//!
//! // Receive notification from daemon (via socket or other IPC)
//! let notif = FrameNotification::from_bytes(&notification_bytes);
//!
//! if notif.is_socket_fallback() {
//!     // Fall back to socket read
//!     return Err(...);
//! }
//!
//! // Read frame from shared memory; the slot is released back to the daemon
//! // as part of this call.
//! let (metadata, data) = reader.read_notified_slot_into_bytesmut(&notif, Some(&pool))?;
//!
//! // Process frame...
//! ```

use std::ffi::CString;
use std::sync::atomic::{AtomicU32, Ordering};

use bytes::BytesMut;

// Shared with the parent IPC module so both log against the same epoch.
use super::monotonic_millis;

use crate::platform::PlatformError;
use crate::platform::PlatformResult;
use crate::platform::common::{FrameMetadata, FrameType, StreamId};
use crate::streaming::bridge::BytesMutPool;

// =============================================================================
// Layout Constants (must match C header exactly)
// =============================================================================

/// Magic value identifying the shared memory region ("VDFS")
pub const VD_SHM_MAGIC: u32 = 0x5644_4653;
/// Version of the shared memory protocol (v3 adds the daemon `epoch`)
pub const VD_SHM_VERSION: u32 = 3;
/// Minimum supported version (v1 layout still accepted for backward compat)
pub const VD_SHM_VERSION_MIN: u32 = 1;
/// Number of slots in the ring buffer
pub const VD_SHM_SLOT_COUNT: u32 = 8;
/// Size of each slot (header + data)
pub const VD_SHM_SLOT_SIZE: usize = 128 * 1024; // 128 KB per slot
/// Size of the ring header structure
pub const VD_SHM_HEADER_SIZE: usize = 64;
/// Size of each slot header
pub const VD_SHM_SLOT_HDR_SIZE: usize = 64;
/// Size of data portion of each slot
pub const VD_SHM_SLOT_DATA_SIZE: usize = VD_SHM_SLOT_SIZE - VD_SHM_SLOT_HDR_SIZE;
/// Total size of the shared memory region
pub const VD_SHM_TOTAL_SIZE: usize =
    VD_SHM_HEADER_SIZE + (VD_SHM_SLOT_COUNT as usize) * VD_SHM_SLOT_SIZE;
/// Default path to the shared memory file
pub const VD_SHM_PATH: &str = "/tmp/vendor-frame-ring.shm";

// Slot states
/// Slot is empty and available for writing
pub const VD_SLOT_EMPTY: u32 = 0;
/// Daemon is currently writing to the slot
pub const VD_SLOT_WRITING: u32 = 1;
/// Slot contains a complete frame, ready for reading
pub const VD_SLOT_READY: u32 = 2;
/// Rust reader is currently reading from the slot
pub const VD_SLOT_READING: u32 = 3;

// Ring buffer flags
/// Daemon has shut down the ring buffer
pub const VD_FLAG_SHUTDOWN: u32 = 1 << 0;
/// Ring buffer overflow occurred (frames were dropped)
pub const VD_FLAG_OVERFLOW: u32 = 1 << 1;

// Notification flags
/// This is the last fragment of a multi-packet frame
pub const VD_NOTIFY_LAST_FRAGMENT: u32 = 1 << 0;
/// Socket fallback notification (daemon couldn't use shm)
pub const VD_NOTIFY_SOCKET_FALLBACK: u32 = 1 << 1;
/// Frame was intentionally dropped by daemon (P-frame during ring overflow)
pub const VD_NOTIFY_FRAME_DROPPED: u32 = 1 << 2;
/// Notification payload size on the Unix socket.
pub const VD_NOTIFY_WIRE_SIZE: usize = 20;

#[inline]
fn slot_state_name(state: u32) -> &'static str {
    match state {
        VD_SLOT_EMPTY => "EMPTY",
        VD_SLOT_WRITING => "WRITING",
        VD_SLOT_READY => "READY",
        VD_SLOT_READING => "READING",
        _ => "UNKNOWN",
    }
}

// =============================================================================
// C-compatible Structures
// =============================================================================

/// Ring buffer header (64 bytes, must match C struct exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RingHeader {
    /// Magic value (VD_SHM_MAGIC)
    pub magic: u32,
    /// Protocol version
    pub version: u32,
    /// Total shared memory size
    pub total_size: u32,
    /// Number of slots
    pub slot_count: u32,
    /// Size of data portion per slot
    pub slot_data_size: u32,
    /// Sequence number of last written slot (updated atomically by daemon)
    pub write_seq: u32,
    /// Sequence number of last read slot (updated atomically by reader)
    pub read_seq: u32,
    /// Ring buffer flags
    pub flags: u32,
    /// Diagnostic: total ring-full events (version >= 2)
    pub overflow_count: u32,
    /// Diagnostic: P-frame evictions for I-frame priority (version >= 2)
    pub eviction_count: u32,
    /// Diagnostic: frames sent via socket fallback (version >= 2)
    pub socket_fallback_count: u32,
    /// Diagnostic: P-frames dropped during overflow (version >= 2)
    pub dropped_count: u32,
    /// Daemon generation counter (version >= 3); 0 means "no epoch reported".
    pub epoch: u32,
    /// Padding to 64 bytes
    pub _padding: [u8; 12],
}

/// Slot header (64 bytes, must match C struct exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SlotHeader {
    /// Slot state (VD_SLOT_*)
    pub state: u32,
    /// Length of valid frame data in bytes
    pub frame_len: u32,
    /// Timestamp in milliseconds (SDK ts directly, no μs inflation)
    pub timestamp_ms: u32,
    /// Padding (was part of u64 timestamp_us)
    pub _ts_pad: u32,
    /// Frame sequence number
    pub seq_no: u32,
    /// Frame type (video/audio, encoding type)
    pub frame_type: u32,
    /// Stream identifier
    pub stream_id: u32,
    /// Checksum of frame data
    pub checksum: u32,
    /// CLOCK_MONOTONIC at ring write time (version >= 2)
    pub wall_clock_us: u64,
    /// Reserved for future use (was inter_frame_us, now unused)
    pub _reserved2: u32,
    /// Reserved for future use
    pub _reserved: u32,
    /// Padding to 64 bytes (reduced from 32)
    pub _padding: [u8; 16],
}

/// Frame notification received from the vendor daemon (**20 bytes**, Unix socket / SCM payload).
///
/// Layout must match C `struct vd_frame_notify` in `vendor-daemon/include/vd_ring_buffer.h` and
/// [`VD_NOTIFY_WIRE_SIZE`]. Bump the shared ring header `version` (or deploy daemon + consumer
/// together) whenever this wire format changes so both sides stay aligned on `slot_index`,
/// `frame_len`, `flags`, `stream_id`, and `seq_no`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameNotification {
    /// Index of the slot containing the frame
    pub slot_index: u32,
    /// Length of the frame data
    pub frame_len: u32,
    /// Notification flags
    pub flags: u32,
    /// Stream identifier expected in the slot
    pub stream_id: u32,
    /// Sequence number expected in the slot
    pub seq_no: u32,
}

impl FrameNotification {
    /// Parse a frame notification from 20 raw bytes.
    ///
    /// The bytes are expected to be in little-endian format.
    ///
    /// # Arguments
    ///
    /// * `bytes` - 20-byte slice containing the notification data
    ///
    /// # Returns
    ///
    /// Parsed `FrameNotification` struct
    pub fn from_bytes(bytes: &[u8; VD_NOTIFY_WIRE_SIZE]) -> Self {
        // Input is exactly [`VD_NOTIFY_WIRE_SIZE`] (20) bytes; each field uses four contiguous
        // bytes, so explicit `from_le_bytes` on fixed arrays is infallible (no `unsafe`).
        Self {
            slot_index: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            frame_len: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            flags: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            stream_id: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            seq_no: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
        }
    }

    /// Check if this is a socket fallback notification.
    ///
    /// When the daemon cannot use shared memory (e.g., due to resource constraints),
    /// it will send this flag to indicate the caller should fall back to socket reads.
    pub fn is_socket_fallback(&self) -> bool {
        self.flags & VD_NOTIFY_SOCKET_FALLBACK != 0
    }

    /// Check if the daemon intentionally dropped this frame.
    ///
    /// During ring buffer overflow, P-frames are dropped instead of using
    /// the expensive socket fallback. The daemon sends a notification with
    /// this flag set so the Rust side can track the drop.
    pub fn is_frame_dropped(&self) -> bool {
        self.flags & VD_NOTIFY_FRAME_DROPPED != 0
    }

    fn matches_slot(&self, frame_len: u32, seq_no: u32, stream_id: u32) -> bool {
        self.frame_len == frame_len && self.seq_no == seq_no && self.stream_id == stream_id
    }
}

// =============================================================================
// ShmRingReader
// =============================================================================

/// Reader for the shared memory ring buffer.
///
/// This struct provides zero-copy access to video frames produced by the vendor-daemon.
/// The reader uses atomic operations for synchronization with the daemon.
///
/// # Safety
///
/// This type is `Send` because all shared state is accessed via atomic operations
/// with appropriate memory ordering. The mmap'd region is only accessed through
/// properly ordered atomic loads/stores.
pub struct ShmRingReader {
    /// Pointer to the mmap'd region
    base: *mut u8,
    /// Total mmap'd size
    size: usize,
    /// File descriptor (for cleanup)
    fd: i32,
}

// SAFETY: ShmRingReader uses atomic operations for all shared state access.
// The mmap'd region is only accessed through properly ordered atomic loads/stores.
unsafe impl Send for ShmRingReader {}

// SAFETY: ShmRingReader does not expose any mutable state to other threads.
// All reads are atomic and coordinated via the state machine.
unsafe impl Sync for ShmRingReader {}

impl ShmRingReader {
    /// Open the shared memory ring buffer created by vendor-daemon.
    ///
    /// Returns `Ok(None)` if the shm file doesn't exist (daemon not using shm mode).
    /// Returns `Ok(Some(reader))` on success.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` - Shared memory file doesn't exist
    /// * `Ok(Some(reader))` - Successfully opened the ring buffer
    /// * `Err(PlatformError)` - Error opening or validating the ring buffer
    pub fn open() -> PlatformResult<Option<Self>> {
        Self::open_path(VD_SHM_PATH)
    }

    /// Open the shared memory ring buffer at a custom path.
    ///
    /// This is primarily useful for testing with a temporary file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the shared memory file
    ///
    /// # Returns
    ///
    /// * `Ok(None)` - Shared memory file doesn't exist
    /// * `Ok(Some(reader))` - Successfully opened the ring buffer
    /// * `Err(PlatformError)` - Error opening or validating the ring buffer
    pub fn open_path(path: &str) -> PlatformResult<Option<Self>> {
        // Convert path to CString (required for null-terminated C strings)
        let c_path = CString::new(path)
            .map_err(|_| PlatformError::InvalidParameter("path contains null byte".into()))?;

        // Open the file read-write for mmap
        // SAFETY: We own the fd and will close it if this function returns an error.
        // libc::open is thread-safe and the path is validated as a CString.
        let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDWR) };

        if fd < 0 {
            // File doesn't exist - daemon not using shm mode
            return Ok(None);
        }

        // Get file size
        // SAFETY: We own fd, fstat is safe to call with a valid fd and valid stat buffer.
        let stat = unsafe {
            let mut stat_buf: libc::stat = std::mem::zeroed();
            let ret = libc::fstat(fd, &mut stat_buf);
            if ret < 0 {
                let err = std::io::Error::last_os_error();
                // SAFETY: We own fd, must close on error path.
                libc::close(fd);
                return Err(PlatformError::HardwareUnavailable(format!(
                    "fstat failed: {}",
                    err
                )));
            }
            stat_buf.st_size as usize
        };

        // Verify file size
        if stat != VD_SHM_TOTAL_SIZE {
            // SAFETY: We own fd, must close on error.
            unsafe {
                libc::close(fd);
            }
            return Err(PlatformError::InvalidParameter(format!(
                "invalid shm size: expected {}, got {}",
                VD_SHM_TOTAL_SIZE, stat
            )));
        }

        // Memory map the file
        // SAFETY: We verified the file size matches VD_SHM_TOTAL_SIZE.
        // The pointer is only used internally with proper atomic operations.
        // mmap is safe: we request correct size, protection, flags, and fd is valid.
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                VD_SHM_TOTAL_SIZE,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if base == libc::MAP_FAILED {
            let err = std::io::Error::last_os_error();
            // SAFETY: We own fd, must close on error.
            unsafe {
                libc::close(fd);
            }
            return Err(PlatformError::HardwareUnavailable(format!(
                "mmap failed: {}",
                err
            )));
        }

        // Validate magic and version
        if let Err(e) = Self::validate_header(base as *const u8) {
            // SAFETY: We own the mmap'd region and fd, must clean up on error.
            unsafe {
                libc::munmap(base, VD_SHM_TOTAL_SIZE);
                libc::close(fd);
            }
            return Err(e);
        }

        tracing::debug!(
            "opened shm ring buffer: {} slots, {} bytes each",
            VD_SHM_SLOT_COUNT,
            VD_SHM_SLOT_DATA_SIZE
        );

        Ok(Some(Self {
            base: base as *mut u8,
            size: VD_SHM_TOTAL_SIZE,
            fd,
        }))
    }

    /// Read diagnostic counters from the ring header (version >= 2).
    ///
    /// Set the shutdown flag, standing in for the daemon during tests.
    #[cfg(test)]
    pub(in crate::hal::anyka::ipc) fn set_shutdown_for_test(&self) {
        self.flags_atomic()
            .fetch_or(VD_FLAG_SHUTDOWN, Ordering::Release);
    }

    /// Check whether the daemon has shut the ring buffer down.
    ///
    /// The vendor-daemon sets `VD_FLAG_SHUTDOWN` before it stops producing frames.
    /// A clean shutdown does not necessarily close the notification socket, so this
    /// flag is the only signal distinguishing "daemon is done" from "no frames yet";
    /// the push receive loop consults it whenever a notification poll times out.
    pub fn is_shutdown(&self) -> bool {
        self.flags_atomic().load(Ordering::Acquire) & VD_FLAG_SHUTDOWN != 0
    }

    /// Read the daemon generation counter from the ring header (version >= 3).
    ///
    /// Returns 0 for a v1/v2 ring, or for a ring the daemon has not stamped yet —
    /// including the window in which `vd_ring_create()` has memset the header
    /// during a restart. 0 is never a valid generation, and the epoch gate treats
    /// it as a mismatch rather than as "no information".
    ///
    /// Uses `read_volatile`: the daemon rewrites this field concurrently on
    /// restart, and creating a `&u32` to it would violate strict aliasing.
    pub fn epoch(&self) -> u32 {
        // SAFETY: offset 48 is within the validated VD_SHM_HEADER_SIZE (64) region.
        unsafe { self.base.add(48).cast::<u32>().read_volatile() }
    }

    /// Raw base pointer, for tests that stand in for the daemon.
    #[cfg(test)]
    pub(in crate::hal::anyka::ipc) fn base_ptr_for_test(&self) -> *mut u8 {
        self.base
    }

    /// Returns (overflow_count, eviction_count, socket_fallback_count, dropped_count).
    /// For version 1 ring buffers, all counters return 0.
    ///
    /// Uses `read_volatile` to access fields that are concurrently written by the
    /// C vendor-daemon, avoiding the creation of `&RingHeader` references that would
    /// violate strict aliasing guarantees.
    pub fn diagnostic_counters(&self) -> (u32, u32, u32, u32) {
        // Read version using volatile read (daemon may update concurrently)
        // RingHeader field offsets: magic(0) version(4) total_size(8) slot_count(12)
        //   slot_data_size(16) write_seq(20) read_seq(24) flags(28)
        //   overflow_count(32) eviction_count(36) socket_fallback_count(40) dropped_count(44)
        let version = unsafe { self.base.add(4).cast::<u32>().read_volatile() };
        if version < 2 {
            return (0, 0, 0, 0);
        }
        // SAFETY: Offsets are within the validated mmap region (VD_SHM_HEADER_SIZE = 64).
        // Using read_volatile avoids creating &u32 references to concurrently-written memory.
        let overflow = unsafe { self.base.add(32).cast::<u32>().read_volatile() };
        let eviction = unsafe { self.base.add(36).cast::<u32>().read_volatile() };
        let fallback = unsafe { self.base.add(40).cast::<u32>().read_volatile() };
        let dropped = unsafe { self.base.add(44).cast::<u32>().read_volatile() };
        (overflow, eviction, fallback, dropped)
    }

    /// Read a frame from the specified slot.
    ///
    /// This method performs a lease-based read: it atomically transitions the slot
    /// from READY to READING state, preventing the daemon from overwriting it.
    ///
    /// IMPORTANT: The returned `ShmFrame` borrows from the shared memory region.
    /// The caller MUST call `release_slot` when done, or the slot will be leaked.
    ///
    /// # Arguments
    ///
    /// * `slot_index` - Index of the slot to read (must be < VD_SHM_SLOT_COUNT)
    ///
    /// # Returns
    ///
    /// * `Ok(ShmFrame)` - Frame data with metadata
    /// * `Err(PlatformError)` - Slot not ready, invalid index, or state error
    pub fn read_slot(&self, slot_index: u32) -> PlatformResult<ShmFrame<'_>> {
        // Validate slot index
        if slot_index >= VD_SHM_SLOT_COUNT {
            return Err(PlatformError::InvalidParameter(format!(
                "slot index {} out of range (max {})",
                slot_index,
                VD_SHM_SLOT_COUNT - 1
            )));
        }

        // Load slot state with Acquire ordering
        let state_atomic = self.slot_state_atomic(slot_index);
        let state = state_atomic.load(Ordering::Acquire);

        // Verify slot is ready
        if state != VD_SLOT_READY {
            let write_seq = self.write_seq();
            let available_frames = write_seq.wrapping_sub(self.read_seq());
            tracing::warn!(
                event = "shm_slot_not_ready",
                diag_monotonic_ms = monotonic_millis(),
                slot_index,
                state,
                state_name = slot_state_name(state),
                expected_state = VD_SLOT_READY,
                expected_state_name = slot_state_name(VD_SLOT_READY),
                write_seq,
                read_seq = self.read_seq(),
                available_frames,
                "shared memory slot not ready during read"
            );
            return Err(PlatformError::InvalidParameter(format!(
                "slot {} not ready for reading (state: {})",
                slot_index, state
            )));
        }

        // CAS state from READY to READING (lease-based access)
        let prev = state_atomic.compare_exchange(
            VD_SLOT_READY,
            VD_SLOT_READING,
            Ordering::AcqRel,
            Ordering::Acquire,
        );

        match prev {
            Ok(_) => {
                // Successfully acquired the slot
                let header = self.slot_header(slot_index);
                let frame_len = header.frame_len as usize;

                // Validate frame_len doesn't exceed slot capacity
                if frame_len > VD_SHM_SLOT_DATA_SIZE {
                    // Release the slot since we can't use it
                    self.slot_state_atomic(slot_index)
                        .store(VD_SLOT_EMPTY, Ordering::Release);
                    return Err(PlatformError::InvalidParameter(format!(
                        "slot {} frame_len {} exceeds slot capacity {}",
                        slot_index, frame_len, VD_SHM_SLOT_DATA_SIZE
                    )));
                }

                // Only return the valid frame data, not the entire slot
                let all_data = self.slot_data(slot_index);
                let data = &all_data[..frame_len];

                Ok(ShmFrame {
                    data,
                    timestamp_ms: header.timestamp_ms,
                    seq_no: header.seq_no,
                    frame_type: header.frame_type,
                    stream_id: header.stream_id,
                    slot_index,
                })
            }
            Err(current) => {
                let write_seq = self.write_seq();
                let available_frames = write_seq.wrapping_sub(self.read_seq());
                tracing::warn!(
                    event = "shm_slot_acquire_race",
                    diag_monotonic_ms = monotonic_millis(),
                    slot_index,
                    expected_state = VD_SLOT_READY,
                    expected_state_name = slot_state_name(VD_SLOT_READY),
                    current_state = current,
                    current_state_name = slot_state_name(current),
                    write_seq,
                    read_seq = self.read_seq(),
                    available_frames,
                    "shared memory slot changed while acquiring read lease"
                );
                // Slot state changed between our read and CAS
                Err(PlatformError::ResourceBusy(format!(
                    "slot {} state changed during acquire (was {}, now {})",
                    slot_index, VD_SLOT_READY, current
                )))
            }
        }
    }

    /// Copy frame data from a socket-notified slot into a `BytesMut` buffer.
    ///
    /// Acquires the slot, copies payload bytes into a pooled or freshly allocated
    /// [`BytesMut`], and returns [`FrameMetadata`]. When `notification` is supplied,
    /// the implementation first verifies that the notification's `(frame_len, seq_no,
    /// stream_id)` still matches the slot header after the read lease is taken. If the
    /// notification is **stale** (the producer advanced while the reader was
    /// scheduling work), the slot is cleared without advancing the reader's
    /// sequence counter and this method returns [`PlatformError::ResourceBusy`]
    /// with a message containing `"stale notification"` so callers can retry.
    ///
    /// # Arguments
    ///
    /// * `notification` - The [`FrameNotification`] received over the socket;
    ///   its `slot_index` selects the slot; `frame_len`, `seq_no`, and
    ///   `stream_id` must match the slot for the read to proceed.
    /// * `pool` - Optional [`BytesMutPool`] for buffer reuse.
    ///
    /// # Returns
    ///
    /// * `Ok((FrameMetadata, BytesMut))` - Metadata and frame bytes when the
    ///   notification matches the slot contents.
    ///
    /// # Errors
    ///
    /// * [`PlatformError::ResourceBusy`] - Stale notification (message includes
    ///   `"stale notification"` and the slot index) or other races while
    ///   acquiring/releasing the slot.
    /// * Other [`PlatformError`] variants - Invalid slot, hardware/shutdown
    ///   issues, allocation failures.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // After receiving FrameNotification from the vendor socket:
    /// let pool = BytesMutPool::new(65536, 4);
    /// let (_meta, buf) = reader.read_notified_slot_into_bytesmut(&notification, Some(&pool))?;
    /// ```
    pub fn read_notified_slot_into_bytesmut(
        &mut self,
        notification: &FrameNotification,
        pool: Option<&BytesMutPool>,
    ) -> PlatformResult<(FrameMetadata, BytesMut)> {
        self.read_slot_into_bytesmut_inner(notification.slot_index, Some(notification), pool)
    }

    fn read_slot_into_bytesmut_inner(
        &mut self,
        slot_index: u32,
        notification: Option<&FrameNotification>,
        pool: Option<&BytesMutPool>,
    ) -> PlatformResult<(FrameMetadata, BytesMut)> {
        // Acquire the slot (borrowed access)
        let frame = self.read_slot(slot_index)?;
        let slot_frame_len = frame.data.len() as u32;
        let slot_seq_no = frame.seq_no;
        let slot_stream_id = frame.stream_id;

        if let Some(notif) = notification
            && !notif.matches_slot(slot_frame_len, slot_seq_no, slot_stream_id)
        {
            tracing::warn!(
                event = "shm_notification_stale",
                diag_monotonic_ms = monotonic_millis(),
                slot_index,
                notif_stream_id = notif.stream_id,
                notif_seq_no = notif.seq_no,
                notif_frame_len = notif.frame_len,
                slot_stream_id,
                slot_seq_no,
                slot_frame_len,
                "shared memory notification no longer matches slot contents"
            );
            if let Err(release_error) = self.release_slot_without_advancing_read_seq(slot_index) {
                tracing::warn!(
                    event = "shm_notification_stale_release_error",
                    diag_monotonic_ms = monotonic_millis(),
                    slot_index,
                    error = %release_error,
                    "stale notification cleanup hit a release race"
                );
            }
            return Err(PlatformError::ResourceBusy(format!(
                "stale notification for slot {} (expected stream {}, seq {}, len {}; got stream {}, seq {}, len {})",
                slot_index,
                notif.stream_id,
                notif.seq_no,
                notif.frame_len,
                slot_stream_id,
                slot_seq_no,
                slot_frame_len
            )));
        }

        // Get buffer from pool or allocate new
        let mut buf = if let Some(p) = pool {
            p.get(frame.data.len())
        } else {
            BytesMut::with_capacity(frame.data.len().max(1024))
        };

        // Copy frame data into BytesMut
        buf.extend_from_slice(frame.data);

        // Parse metadata from slot header
        let header = self.slot_header(slot_index);
        let metadata = FrameMetadata {
            timestamp_ms: header.timestamp_ms,
            seq_no: header.seq_no,
            frame_type: shm_frame_type_to_onvif(header.frame_type),
            remote_token: 0, // Not used in shm path
            stream_id: shm_stream_id_to_onvif(header.stream_id),
        };

        // Release the slot
        self.release_slot_with_expectation(slot_index, notification)?;

        Ok((metadata, buf))
    }

    /// Clear a slot from `READING` to `EMPTY` without advancing `read_seq`.
    ///
    /// Used when a notification proves stale after the frame was read: the slot
    /// must be returned to the daemon, but the reader must not consume a frame
    /// from the logical read sequence (no successful delivery occurred).
    fn release_slot_without_advancing_read_seq(&mut self, slot_index: u32) -> PlatformResult<()> {
        if slot_index >= VD_SHM_SLOT_COUNT {
            return Err(PlatformError::InvalidParameter(format!(
                "slot index {} out of range",
                slot_index
            )));
        }

        let state_atomic = self.slot_state_atomic(slot_index);
        match state_atomic.compare_exchange(
            VD_SLOT_READING,
            VD_SLOT_EMPTY,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => Ok(()),
            Err(current) => Err(PlatformError::HardwareFailure(format!(
                "failed to clear stale slot {}: expected state {}, got {}",
                slot_index, VD_SLOT_READING, current
            ))),
        }
    }

    fn release_slot_with_expectation(
        &mut self,
        slot_index: u32,
        notification: Option<&FrameNotification>,
    ) -> PlatformResult<()> {
        if slot_index >= VD_SHM_SLOT_COUNT {
            return Err(PlatformError::InvalidParameter(format!(
                "slot index {} out of range",
                slot_index
            )));
        }

        let state_atomic = self.slot_state_atomic(slot_index);

        // CAS state from READING to EMPTY
        let prev = state_atomic.compare_exchange(
            VD_SLOT_READING,
            VD_SLOT_EMPTY,
            Ordering::AcqRel,
            Ordering::Acquire,
        );

        match prev {
            Ok(_) => {
                // Advance `read_seq` with a relative increment, matching the daemon's
                // `vd_ring_release()` contract. An absolute store would rewind the
                // increments the daemon makes itself (I-frame eviction) and would
                // resurrect a stale position after `vd_ring_reset()`, either of which
                // permanently inflates `write_seq - read_seq` until the ring reports
                // itself full forever.
                self.read_seq_atomic().fetch_add(1, Ordering::AcqRel);
                Ok(())
            }
            Err(current) => {
                let header = self.slot_header(slot_index);
                let write_seq = self.write_seq();
                let available_frames = write_seq.wrapping_sub(self.read_seq());
                let notif_stream_id = notification.map(|notif| notif.stream_id);
                let notif_seq_no = notification.map(|notif| notif.seq_no);
                let notif_frame_len = notification.map(|notif| notif.frame_len);
                let notification_matches_header = notification
                    .map(|notif| {
                        notif.matches_slot(header.frame_len, header.seq_no, header.stream_id)
                    })
                    .unwrap_or(false);
                tracing::warn!(
                    event = "shm_slot_release_mismatch",
                    diag_monotonic_ms = monotonic_millis(),
                    slot_index,
                    expected_state = VD_SLOT_READING,
                    expected_state_name = slot_state_name(VD_SLOT_READING),
                    current_state = current,
                    current_state_name = slot_state_name(current),
                    frame_seq_no = header.seq_no,
                    frame_stream_id = header.stream_id,
                    frame_len = header.frame_len,
                    notif_stream_id,
                    notif_seq_no,
                    notif_frame_len,
                    notification_matches_header,
                    frame_timestamp_ms = header.timestamp_ms,
                    write_seq,
                    read_seq = self.read_seq(),
                    available_frames,
                    "shared memory slot release observed unexpected state"
                );
                // CAS failed: another writer/reader changed the slot state. If we had a live
                // notification and its stream/seq/len no longer match the slot header, treat it
                // as a producer/consumer race and surface [`PlatformError::ResourceBusy`] so the
                // caller can retry; otherwise the mismatch is unexpected hardware/state — return
                // [`PlatformError::HardwareFailure`] below.
                if notification.is_some() && !notification_matches_header {
                    return Err(PlatformError::ResourceBusy(format!(
                        "stale notification release race on slot {}: expected stream/seq/len {:?}/{:?}/{:?}, got {}/{}/{}",
                        slot_index,
                        notif_stream_id,
                        notif_seq_no,
                        notif_frame_len,
                        header.stream_id,
                        header.seq_no,
                        header.frame_len
                    )));
                }
                Err(PlatformError::HardwareFailure(format!(
                    "failed to release slot {}: expected state {}, got {}",
                    slot_index, VD_SLOT_READING, current
                )))
            }
        }
    }

    /// Get the current write sequence number.
    ///
    /// This is updated by the daemon when it writes a new frame.
    pub fn write_seq(&self) -> u32 {
        self.write_seq_atomic().load(Ordering::Acquire)
    }

    /// Get the current read sequence number.
    ///
    /// Read straight from the shared header: this reader advances it on release, but
    /// the daemon also advances it when it evicts a P-frame, and resets it between
    /// push sessions. A cached copy would drift from both.
    pub fn read_seq(&self) -> u32 {
        self.read_seq_atomic().load(Ordering::Acquire)
    }

    // =============================================================================
    // Private helpers for accessing atomic fields
    // =============================================================================

    /// Get header from raw pointer (for open_path)
    #[inline]
    /// Validate the ring buffer header at the given base pointer.
    ///
    /// Checks magic number and version range. This is extracted from `open_path`
    /// so it can be tested independently with anonymous mmap.
    fn validate_header(base: *const u8) -> PlatformResult<()> {
        // SAFETY: Called during open() — header is stable (daemon not yet interacting).
        let header = unsafe { Self::header_from_ptr(base as *const RingHeader) };
        if header.magic != VD_SHM_MAGIC {
            return Err(PlatformError::InvalidParameter(format!(
                "invalid shm magic: expected {:#x}, got {:#x}",
                VD_SHM_MAGIC, header.magic
            )));
        }
        if header.version < VD_SHM_VERSION_MIN || header.version > VD_SHM_VERSION {
            return Err(PlatformError::InvalidParameter(format!(
                "unsupported shm version: expected {}-{}, got {}",
                VD_SHM_VERSION_MIN, VD_SHM_VERSION, header.version
            )));
        }
        Ok(())
    }

    /// Create a `ShmRingReader` from a raw mmap'd pointer (test-only).
    ///
    /// Used with anonymous mmap (`MAP_ANONYMOUS | MAP_SHARED`) to avoid
    /// file I/O race conditions in unit tests.
    ///
    /// # Safety
    ///
    /// - `base` must point to a valid mmap'd region of at least `VD_SHM_TOTAL_SIZE` bytes
    /// - The region must be properly initialized with a valid ring buffer layout
    /// - When `fd` is -1 (anonymous mmap), Drop will skip `close(fd)`
    #[cfg(test)]
    unsafe fn open_from_raw(base: *mut u8) -> Self {
        Self {
            base,
            size: VD_SHM_TOTAL_SIZE,
            fd: -1,
        }
    }

    /// Read the ring header from a raw pointer.
    ///
    /// # Safety
    ///
    /// Only safe to call when the daemon is NOT concurrently writing header fields
    /// being accessed through the returned reference (e.g., during initialization
    /// before the daemon starts writing, or for immutable fields like magic/version).
    /// For fields written concurrently by the daemon (write_seq, flags, counters),
    /// use the `*_atomic()` methods or `read_volatile` instead.
    unsafe fn header_from_ptr<'a>(ptr: *const RingHeader) -> &'a RingHeader {
        unsafe { &*ptr }
    }

    /// Get slot header at index.
    ///
    /// # Panics
    ///
    /// Panics if `index >= VD_SHM_SLOT_COUNT` or the computed offset exceeds the
    /// mmap'd region. This is a defense-in-depth check — callers like `read_slot`
    /// already validate the index, but corrupted data from the daemon socket could
    /// bypass the public API.
    #[inline]
    fn slot_header(&self, index: u32) -> &SlotHeader {
        assert!(
            (index as usize) < VD_SHM_SLOT_COUNT as usize,
            "slot_header: index {} out of bounds (max {})",
            index,
            VD_SHM_SLOT_COUNT - 1
        );
        let offset = VD_SHM_HEADER_SIZE + (index as usize) * VD_SHM_SLOT_SIZE;
        debug_assert!(offset + VD_SHM_SLOT_HDR_SIZE <= self.size);
        // SAFETY: Offset validated by assert above, structure fits within slot.
        unsafe {
            let ptr = self.base.add(offset);
            &*ptr.cast::<SlotHeader>()
        }
    }

    /// Get slot data slice at index.
    ///
    /// # Panics
    ///
    /// Panics if `index >= VD_SHM_SLOT_COUNT`.
    #[inline]
    fn slot_data(&self, index: u32) -> &[u8] {
        assert!(
            (index as usize) < VD_SHM_SLOT_COUNT as usize,
            "slot_data: index {} out of bounds (max {})",
            index,
            VD_SHM_SLOT_COUNT - 1
        );
        let offset =
            VD_SHM_HEADER_SIZE + (index as usize) * VD_SHM_SLOT_SIZE + VD_SHM_SLOT_HDR_SIZE;
        debug_assert!(offset + VD_SHM_SLOT_DATA_SIZE <= self.size);
        // SAFETY: Offset validated by assert above, data portion fits within slot.
        unsafe {
            let ptr = self.base.add(offset);
            std::slice::from_raw_parts(ptr, VD_SHM_SLOT_DATA_SIZE)
        }
    }

    /// Get atomic write_seq field
    #[inline]
    fn write_seq_atomic(&self) -> &AtomicU32 {
        // Offset to write_seq in RingHeader: 5 * 4 = 20 bytes
        let offset = 20_usize;
        // SAFETY: Offset is within bounds, field is u32.
        unsafe {
            let ptr = self.base.add(offset);
            &*ptr.cast::<AtomicU32>()
        }
    }

    /// Get atomic read_seq field
    #[inline]
    fn read_seq_atomic(&self) -> &AtomicU32 {
        // Offset to read_seq in RingHeader: 6 * 4 = 24 bytes
        let offset = 24_usize;
        // SAFETY: Offset is within bounds, field is u32.
        unsafe {
            let ptr = self.base.add(offset);
            &*ptr.cast::<AtomicU32>()
        }
    }

    /// Get atomic flags field
    #[inline]
    fn flags_atomic(&self) -> &AtomicU32 {
        // Offset to flags in RingHeader: 7 * 4 = 28 bytes
        let offset = 28_usize;
        // SAFETY: Offset is within bounds, field is u32.
        unsafe {
            let ptr = self.base.add(offset);
            &*ptr.cast::<AtomicU32>()
        }
    }

    /// Get atomic state field for a slot.
    ///
    /// # Panics
    ///
    /// Panics if `index >= VD_SHM_SLOT_COUNT`.
    #[inline]
    fn slot_state_atomic(&self, index: u32) -> &AtomicU32 {
        assert!(
            (index as usize) < VD_SHM_SLOT_COUNT as usize,
            "slot_state_atomic: index {} out of bounds (max {})",
            index,
            VD_SHM_SLOT_COUNT - 1
        );
        let offset = VD_SHM_HEADER_SIZE + (index as usize) * VD_SHM_SLOT_SIZE;
        // SAFETY: Offset validated by assert above, state is first field of SlotHeader.
        unsafe {
            let ptr = self.base.add(offset);
            &*ptr.cast::<AtomicU32>()
        }
    }
}

impl Drop for ShmRingReader {
    fn drop(&mut self) {
        // SAFETY: We own the mmap'd region, must clean up.
        // munmap is safe: we own this resource and no other thread
        // is accessing it at this point.
        unsafe {
            libc::munmap(self.base as *mut libc::c_void, self.size);
            // fd == -1 for anonymous mmap (test-only), skip close.
            if self.fd >= 0 {
                libc::close(self.fd);
            }
        }
        tracing::debug!("closed shm ring buffer");
    }
}

// =============================================================================
// ShmFrame - Borrowed frame data
// =============================================================================

/// Borrowed frame data from shared memory.
///
/// This struct holds references to frame data in the shared memory region.
/// It is only valid until `release_slot` is called on the parent reader.
///
/// # Safety
///
/// The caller must ensure that `release_slot` is called before the parent
/// `ShmRingReader` is dropped or the slot is reused.
pub struct ShmFrame<'a> {
    /// Frame data (borrowed from shared memory)
    pub data: &'a [u8],
    /// Timestamp in milliseconds
    pub timestamp_ms: u32,
    /// Frame sequence number
    pub seq_no: u32,
    /// Frame type (raw value from daemon)
    pub frame_type: u32,
    /// Stream identifier
    pub stream_id: u32,
    /// Slot index (for release tracking/debugging)
    #[allow(dead_code)]
    slot_index: u32,
}

impl<'a> ShmFrame<'a> {
    /// Convert raw frame_type to ONVIF FrameType enum.
    pub fn frame_type_onvif(&self) -> FrameType {
        shm_frame_type_to_onvif(self.frame_type)
    }

    /// Convert raw stream_id to ONVIF StreamId enum.
    pub fn stream_id_onvif(&self) -> StreamId {
        shm_stream_id_to_onvif(self.stream_id)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Convert raw frame type from daemon to ONVIF FrameType.
///
/// The daemon uses:
/// - 0 = P-frame (VD_FRAME_TYPE_P)
/// - 1 = I-frame (VD_FRAME_TYPE_I)
/// - 2 = B-frame (VD_FRAME_TYPE_B)
/// - 3 = Pi-frame (VD_FRAME_TYPE_PI) — partial refresh, treated as P for streaming
fn shm_frame_type_to_onvif(raw: u32) -> FrameType {
    match raw {
        0 => FrameType::VideoPFrame,  // VD_FRAME_TYPE_P
        1 => FrameType::VideoIFrame,  // VD_FRAME_TYPE_I
        2 => FrameType::VideoBFrame,  // VD_FRAME_TYPE_B
        3 => FrameType::VideoPiFrame, // VD_FRAME_TYPE_PI
        _ => FrameType::VideoPFrame,  // Unknown defaults to P
    }
}

/// Convert raw stream ID from daemon to ONVIF StreamId.
///
/// The daemon uses: 0=main, 1=sub, 2+=audio
fn shm_stream_id_to_onvif(raw: u32) -> StreamId {
    match raw {
        0 => StreamId::VideoMain,
        1 => StreamId::VideoSub,
        _ => StreamId::Audio,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
pub(in crate::hal::anyka::ipc) mod tests {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::io::{Seek, Write};

    /// Create a test ring buffer file with valid header
    fn create_test_ring_buffer(path: &str) -> std::io::Result<File> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        file.set_len(VD_SHM_TOTAL_SIZE as u64)?;

        // Write header
        let header = RingHeader {
            magic: VD_SHM_MAGIC,
            version: VD_SHM_VERSION,
            total_size: VD_SHM_TOTAL_SIZE as u32,
            slot_count: VD_SHM_SLOT_COUNT,
            slot_data_size: VD_SHM_SLOT_DATA_SIZE as u32,
            write_seq: 0,
            read_seq: 0,
            flags: 0,
            overflow_count: 0,
            eviction_count: 0,
            socket_fallback_count: 0,
            dropped_count: 0,
            epoch: 0,
            _padding: [0u8; 12],
        };

        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &header as *const RingHeader as *const u8,
                VD_SHM_HEADER_SIZE,
            )
        };
        file.write_all(header_bytes)?;

        // Initialize all slots to EMPTY
        let empty_slot = SlotHeader {
            state: VD_SLOT_EMPTY,
            frame_len: 0,
            timestamp_ms: 0,
            _ts_pad: 0,
            seq_no: 0,
            frame_type: 0,
            stream_id: 0,
            checksum: 0,
            wall_clock_us: 0,
            _reserved2: 0,
            _reserved: 0,
            _padding: [0u8; 16],
        };
        let slot_bytes = unsafe {
            std::slice::from_raw_parts(
                &empty_slot as *const SlotHeader as *const u8,
                VD_SHM_SLOT_HDR_SIZE,
            )
        };

        for _ in 0..VD_SHM_SLOT_COUNT {
            file.write_all(slot_bytes)?;
            // Skip data portion
            file.seek_relative(VD_SHM_SLOT_DATA_SIZE as i64)?;
        }

        Ok(file)
    }

    /// Create an anonymous mmap region initialized as a valid ring buffer.
    ///
    /// Returns a `ShmRingReader` backed by anonymous memory (no file I/O),
    /// eliminating the race condition between file writes and mmap visibility.
    pub(in crate::hal::anyka::ipc) fn create_test_anon_reader() -> ShmRingReader {
        // SAFETY: MAP_ANONYMOUS | MAP_SHARED gives us a zeroed memory region
        // with no file backing, avoiding all file I/O race conditions.
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                VD_SHM_TOTAL_SIZE,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_SHARED,
                -1,
                0,
            )
        };
        assert_ne!(base, libc::MAP_FAILED, "anonymous mmap failed");
        let base = base as *mut u8;

        // Initialize ring header directly in mmap'd memory.
        // SAFETY: base points to a freshly allocated, zeroed mmap region of
        // VD_SHM_TOTAL_SIZE bytes. Writing a RingHeader at offset 0 is safe.
        unsafe {
            let header = base as *mut RingHeader;
            (*header).magic = VD_SHM_MAGIC;
            (*header).version = VD_SHM_VERSION;
            (*header).total_size = VD_SHM_TOTAL_SIZE as u32;
            (*header).slot_count = VD_SHM_SLOT_COUNT;
            (*header).slot_data_size = VD_SHM_SLOT_DATA_SIZE as u32;
            // All other fields are zero from mmap, which means:
            // write_seq=0, read_seq=0, flags=0, all slots state=VD_SLOT_EMPTY
        }

        // SAFETY: base is a valid mmap'd region with valid ring header.
        unsafe { ShmRingReader::open_from_raw(base) }
    }

    /// Write a test frame into a slot of an anonymous mmap-backed reader.
    ///
    /// This writes directly into the mmap'd region, no file I/O needed.
    #[allow(clippy::too_many_arguments)] // mirrors the C slot header field-for-field
    unsafe fn write_test_slot(
        reader: &ShmRingReader,
        slot_index: u32,
        state: u32,
        frame_len: u32,
        timestamp_ms: u32,
        seq_no: u32,
        frame_type: u32,
        stream_id: u32,
        data: &[u8],
    ) {
        let slot_offset = VD_SHM_HEADER_SIZE + (slot_index as usize) * VD_SHM_SLOT_SIZE;
        // SAFETY: slot_offset is within the mmap'd region, and SlotHeader fits.
        unsafe {
            let slot_ptr = reader.base.add(slot_offset) as *mut SlotHeader;
            (*slot_ptr).state = state;
            (*slot_ptr).frame_len = frame_len;
            (*slot_ptr).timestamp_ms = timestamp_ms;
            (*slot_ptr).seq_no = seq_no;
            (*slot_ptr).frame_type = frame_type;
            (*slot_ptr).stream_id = stream_id;

            if !data.is_empty() {
                let data_ptr = reader.base.add(slot_offset + VD_SHM_SLOT_HDR_SIZE);
                std::ptr::copy_nonoverlapping(data.as_ptr(), data_ptr, data.len());
            }
        }
    }

    #[test]
    fn test_frame_notification_from_bytes() {
        let bytes: [u8; VD_NOTIFY_WIRE_SIZE] = [
            0x03, 0x00, 0x00, 0x00, // slot_index = 3
            0x80, 0x10, 0x00, 0x00, // frame_len = 4224
            0x01, 0x00, 0x00, 0x00, // flags = 1 (LAST_FRAGMENT)
            0x01, 0x00, 0x00, 0x00, // stream_id = 1 (sub)
            0x2A, 0x00, 0x00, 0x00, // seq_no = 42
        ];

        let notif = FrameNotification::from_bytes(&bytes);

        assert_eq!(notif.slot_index, 3);
        assert_eq!(notif.frame_len, 4224);
        assert_eq!(notif.stream_id, 1);
        assert_eq!(notif.seq_no, 42);
        assert!(!notif.is_socket_fallback());
    }

    #[test]
    fn test_frame_notification_socket_fallback() {
        let bytes: [u8; VD_NOTIFY_WIRE_SIZE] = [
            0x00, 0x00, 0x00, 0x00, // slot_index = 0
            0x00, 0x00, 0x00, 0x00, // frame_len = 0
            0x02, 0x00, 0x00, 0x00, // flags = 2 (SOCKET_FALLBACK)
            0x00, 0x00, 0x00, 0x00, // stream_id = 0
            0x00, 0x00, 0x00, 0x00, // seq_no = 0
        ];

        let notif = FrameNotification::from_bytes(&bytes);

        assert!(notif.is_socket_fallback());
    }

    #[test]
    fn test_constants_match() {
        // Compile-time assertions to ensure struct layouts match C exactly
        // These use const evaluation to verify at compile time
        const _: () = assert!(std::mem::size_of::<RingHeader>() == 64);
        const _: () = assert!(std::mem::size_of::<SlotHeader>() == 64);
        const _: () = assert!(std::mem::size_of::<FrameNotification>() == VD_NOTIFY_WIRE_SIZE);

        // Print actual sizes for debugging
        eprintln!("RingHeader size: {}", std::mem::size_of::<RingHeader>());
        eprintln!("SlotHeader size: {}", std::mem::size_of::<SlotHeader>());

        // Verify our constants align with the expected layout
        assert_eq!(std::mem::size_of::<RingHeader>(), VD_SHM_HEADER_SIZE);
        assert_eq!(std::mem::size_of::<SlotHeader>(), VD_SHM_SLOT_HDR_SIZE);
        assert_eq!(
            VD_SHM_SLOT_DATA_SIZE,
            VD_SHM_SLOT_SIZE - VD_SHM_SLOT_HDR_SIZE
        );
        assert_eq!(
            VD_SHM_TOTAL_SIZE,
            VD_SHM_HEADER_SIZE + (VD_SHM_SLOT_COUNT as usize) * VD_SHM_SLOT_SIZE
        );
    }

    #[test]
    fn test_open_invalid_magic() {
        // Use anonymous mmap to avoid file I/O race conditions that caused
        // intermittent segfaults with file-backed mmap.
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                VD_SHM_TOTAL_SIZE,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_SHARED,
                -1,
                0,
            )
        };
        assert_ne!(base, libc::MAP_FAILED);
        let base_ptr = base as *mut u8;

        // Write invalid magic directly into the mmap'd region
        unsafe {
            let header = base_ptr as *mut RingHeader;
            (*header).magic = 0xDEADBEEF; // Invalid magic
            (*header).version = VD_SHM_VERSION;
            (*header).total_size = VD_SHM_TOTAL_SIZE as u32;
            (*header).slot_count = VD_SHM_SLOT_COUNT;
            (*header).slot_data_size = VD_SHM_SLOT_DATA_SIZE as u32;
        }

        let result = ShmRingReader::validate_header(base_ptr);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, PlatformError::InvalidParameter(_)));

        unsafe {
            libc::munmap(base, VD_SHM_TOTAL_SIZE);
        }
    }

    #[test]
    fn test_open_nonexistent() {
        let result = ShmRingReader::open_path("/nonexistent/path/shm");
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_open_valid_ring_buffer() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_shm_valid");

        create_test_ring_buffer(path.to_str().unwrap()).unwrap();

        let result = ShmRingReader::open_path(path.to_str().unwrap());
        assert!(result.is_ok());
        let reader = result.unwrap();
        assert!(reader.is_some());

        // Check initial state
        let reader = reader.unwrap();
        assert_eq!(reader.write_seq(), 0);
        assert_eq!(reader.read_seq(), 0);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_read_slot_empty_slot() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_shm_empty");

        create_test_ring_buffer(path.to_str().unwrap()).unwrap();

        let reader = ShmRingReader::open_path(path.to_str().unwrap())
            .unwrap()
            .unwrap();

        // Try to read slot 0 which is in EMPTY state
        let result = reader.read_slot(0);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, PlatformError::InvalidParameter(_)));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_read_slot_out_of_range() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_shm_range");

        create_test_ring_buffer(path.to_str().unwrap()).unwrap();

        let reader = ShmRingReader::open_path(path.to_str().unwrap())
            .unwrap()
            .unwrap();

        // Try invalid slot index
        let result = reader.read_slot(VD_SHM_SLOT_COUNT);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, PlatformError::InvalidParameter(_)));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_read_and_release_slot() {
        let test_data = b"test frame data 1234567890";
        let mut reader = create_test_anon_reader();

        // Write a READY frame into slot 0 directly in mmap'd memory.
        // frame_len must match test_data.len() so read_slot returns the right slice.
        unsafe {
            write_test_slot(
                &reader,
                0,
                VD_SLOT_READY,
                test_data.len() as u32,
                1234567,
                42,
                0,
                0,
                test_data,
            );
        }

        // Read the slot
        let frame = reader.read_slot(0).unwrap();
        assert_eq!(frame.data, test_data.as_slice());
        assert_eq!(frame.timestamp_ms, 1234567);
        assert_eq!(frame.seq_no, 42);
        assert_eq!(frame.frame_type, 0);
        assert_eq!(frame.stream_id, 0);

        // Release the slot
        reader.release_slot_with_expectation(0, None).unwrap();

        // Verify state went back to EMPTY via the atomic accessor
        let state = reader.slot_state_atomic(0).load(Ordering::Relaxed);
        assert_eq!(state, VD_SLOT_EMPTY);
    }

    /// The daemon advances `read_seq` itself when it evicts a P-frame to make room for an
    /// I-frame. Releasing a slot must add to that, not overwrite it: an absolute store
    /// rewinds the eviction and permanently costs the ring one slot of capacity, until
    /// `write_seq - read_seq` pins at the slot count and every frame is dropped.
    #[test]
    fn test_release_preserves_daemon_read_seq_advance() {
        let test_data = b"eviction accounting";
        let mut reader = create_test_anon_reader();

        // Daemon evicted one P-frame: read_seq is now 1 without the reader consuming it.
        reader.read_seq_atomic().store(1, Ordering::Release);

        unsafe {
            write_test_slot(
                &reader,
                0,
                VD_SLOT_READY,
                test_data.len() as u32,
                1234567,
                7,
                0,
                0,
                test_data,
            );
        }

        reader.read_slot(0).unwrap();
        reader.release_slot_with_expectation(0, None).unwrap();

        assert_eq!(reader.read_seq(), 2, "release must not rewind the eviction");
    }

    #[test]
    fn test_read_slot_into_bytesmut() {
        let test_data = b"bytesmut test frame data here";
        let mut reader = create_test_anon_reader();

        // Write a READY I-frame into slot 0 (frame_type=1, stream_id=1).
        // frame_len must match test_data.len() for correct slice return.
        unsafe {
            write_test_slot(
                &reader,
                0,
                VD_SLOT_READY,
                test_data.len() as u32,
                2000,
                100,
                1,
                1,
                test_data,
            );
        }

        // Use the method under test
        let pool = BytesMutPool::new(1024, 4);
        let (metadata, buf) = reader
            .read_slot_into_bytesmut_inner(0, None, Some(&pool))
            .unwrap();

        // Verify metadata
        assert_eq!(metadata.timestamp_ms, 2000);
        assert_eq!(metadata.seq_no, 100);
        assert_eq!(metadata.frame_type, FrameType::VideoIFrame);

        // Verify data was copied
        assert_eq!(buf.as_ref(), test_data.as_slice());

        // Pool starts empty; get() allocated a fresh buffer, so pool is still empty.
        // Returning the buffer to the pool would make it available.
        assert_eq!(pool.available(), 0);
        pool.put(buf);
        assert_eq!(pool.available(), 1);
    }

    #[test]
    fn test_read_notified_slot_into_bytesmut_success_drains_slot() {
        let test_data = b"notified read ok";
        let mut reader = create_test_anon_reader();
        unsafe {
            write_test_slot(
                &reader,
                0,
                VD_SLOT_READY,
                test_data.len() as u32,
                3000,
                200,
                1,
                0,
                test_data,
            );
        }
        let notification = FrameNotification {
            slot_index: 0,
            frame_len: test_data.len() as u32,
            flags: VD_NOTIFY_LAST_FRAGMENT,
            stream_id: 0,
            seq_no: 200,
        };
        let pool = BytesMutPool::new(1024, 2);
        let (metadata, buf) = reader
            .read_notified_slot_into_bytesmut(&notification, Some(&pool))
            .expect("matching notification should read slot");
        assert_eq!(metadata.seq_no, 200);
        assert_eq!(buf.as_ref(), test_data.as_slice());
        assert_eq!(
            reader.slot_state_atomic(0).load(Ordering::Relaxed),
            VD_SLOT_EMPTY
        );
    }

    #[test]
    fn test_read_notified_slot_into_bytesmut_rejects_stale_notification_as_resource_busy() {
        let test_data = b"newest frame";
        let mut reader = create_test_anon_reader();

        unsafe {
            write_test_slot(
                &reader,
                0,
                VD_SLOT_READY,
                test_data.len() as u32,
                3456,
                77,
                1,
                0,
                test_data,
            );
        }

        let pool = BytesMutPool::new(1024, 2);
        let stale_notification = FrameNotification {
            slot_index: 0,
            frame_len: test_data.len() as u32,
            flags: VD_NOTIFY_LAST_FRAGMENT,
            stream_id: 1,
            seq_no: 88,
        };

        let err = reader
            .read_notified_slot_into_bytesmut(&stale_notification, Some(&pool))
            .expect_err("stale notification should be recoverable");

        match err {
            PlatformError::ResourceBusy(message) => {
                assert!(message.contains("stale notification"));
                assert!(message.contains("slot 0"));
            }
            other => panic!("expected ResourceBusy, got {:?}", other),
        }

        let state = reader.slot_state_atomic(0).load(Ordering::Relaxed);
        assert_eq!(state, VD_SLOT_EMPTY);
    }

    #[test]
    fn test_read_notified_slot_into_bytesmut_rejects_stale_seq_only_as_resource_busy() {
        let test_data = b"seq mismatch only";
        let mut reader = create_test_anon_reader();

        unsafe {
            write_test_slot(
                &reader,
                0,
                VD_SLOT_READY,
                test_data.len() as u32,
                1000,
                42,
                1,
                2,
                test_data,
            );
        }

        let pool = BytesMutPool::new(1024, 2);
        let stale_notification = FrameNotification {
            slot_index: 0,
            frame_len: test_data.len() as u32,
            flags: VD_NOTIFY_LAST_FRAGMENT,
            stream_id: 2,
            seq_no: 99,
        };

        let err = reader
            .read_notified_slot_into_bytesmut(&stale_notification, Some(&pool))
            .expect_err("stale seq_no should be recoverable");

        match err {
            PlatformError::ResourceBusy(message) => {
                assert!(message.contains("stale notification"), "{}", message);
                assert!(message.contains("slot 0"), "{}", message);
            }
            other => panic!("expected ResourceBusy, got {:?}", other),
        }

        assert_eq!(
            reader.slot_state_atomic(0).load(Ordering::Relaxed),
            VD_SLOT_EMPTY
        );
    }

    #[test]
    fn test_read_notified_slot_into_bytesmut_rejects_stale_stream_only_as_resource_busy() {
        let test_data = b"stream mismatch only";
        let mut reader = create_test_anon_reader();

        unsafe {
            write_test_slot(
                &reader,
                0,
                VD_SLOT_READY,
                test_data.len() as u32,
                2000,
                50,
                1,
                0,
                test_data,
            );
        }

        let pool = BytesMutPool::new(1024, 2);
        let stale_notification = FrameNotification {
            slot_index: 0,
            frame_len: test_data.len() as u32,
            flags: VD_NOTIFY_LAST_FRAGMENT,
            stream_id: 7,
            seq_no: 50,
        };

        let err = reader
            .read_notified_slot_into_bytesmut(&stale_notification, Some(&pool))
            .expect_err("stale stream_id should be recoverable");

        match err {
            PlatformError::ResourceBusy(message) => {
                assert!(message.contains("stale notification"), "{}", message);
                assert!(message.contains("slot 0"), "{}", message);
            }
            other => panic!("expected ResourceBusy, got {:?}", other),
        }

        assert_eq!(
            reader.slot_state_atomic(0).load(Ordering::Relaxed),
            VD_SLOT_EMPTY
        );
    }

    #[test]
    fn test_read_notified_slot_into_bytesmut_rejects_stale_frame_len_only_as_resource_busy() {
        let test_data = b"frame len mismatch only";
        let mut reader = create_test_anon_reader();

        unsafe {
            write_test_slot(
                &reader,
                0,
                VD_SLOT_READY,
                test_data.len() as u32,
                4000,
                33,
                1,
                5,
                test_data,
            );
        }

        let pool = BytesMutPool::new(1024, 2);
        let stale_notification = FrameNotification {
            slot_index: 0,
            frame_len: (test_data.len() as u32).saturating_add(100),
            flags: VD_NOTIFY_LAST_FRAGMENT,
            stream_id: 5,
            seq_no: 33,
        };

        let err = reader
            .read_notified_slot_into_bytesmut(&stale_notification, Some(&pool))
            .expect_err("stale frame_len only should be recoverable");

        match err {
            PlatformError::ResourceBusy(message) => {
                assert!(message.contains("stale notification"), "{}", message);
                assert!(message.contains("slot 0"), "{}", message);
            }
            other => panic!("expected ResourceBusy, got {:?}", other),
        }

        assert_eq!(
            reader.slot_state_atomic(0).load(Ordering::Relaxed),
            VD_SLOT_EMPTY
        );
    }

    #[test]
    fn test_is_shutdown_reflects_ring_header_flag() {
        let reader = create_test_anon_reader();
        assert!(
            !reader.is_shutdown(),
            "freshly mapped ring must not report shutdown"
        );

        // Daemon sets VD_FLAG_SHUTDOWN in the ring header when it stops producing.
        reader
            .flags_atomic()
            .store(VD_FLAG_SHUTDOWN, Ordering::Release);
        assert!(reader.is_shutdown());
    }

    #[test]
    fn test_is_shutdown_ignores_unrelated_flags() {
        let reader = create_test_anon_reader();
        reader
            .flags_atomic()
            .store(VD_FLAG_OVERFLOW, Ordering::Release);
        assert!(
            !reader.is_shutdown(),
            "overflow alone must not be read as shutdown"
        );

        reader
            .flags_atomic()
            .store(VD_FLAG_OVERFLOW | VD_FLAG_SHUTDOWN, Ordering::Release);
        assert!(
            reader.is_shutdown(),
            "shutdown must be detected alongside other flags"
        );
    }

    #[test]
    fn test_frame_notification_le_values() {
        // Test little-endian parsing with various values
        let bytes: [u8; VD_NOTIFY_WIRE_SIZE] = [
            0xFF, 0xFF, 0xFF, 0xFF, // slot_index = u32::MAX
            0x00, 0x00, 0x00, 0x00, // frame_len = 0
            0x00, 0x00, 0x00, 0x00, // flags = 0
            0xFF, 0xFF, 0xFF, 0xFF, // stream_id = u32::MAX
            0xFE, 0xFF, 0xFF, 0xFF, // seq_no = u32::MAX - 1
        ];

        let notif = FrameNotification::from_bytes(&bytes);
        assert_eq!(notif.slot_index, u32::MAX);
        assert_eq!(notif.stream_id, u32::MAX);
        assert_eq!(notif.seq_no, u32::MAX - 1);
    }

    #[test]
    fn epoch_reads_back_the_value_the_daemon_stamped() {
        let reader = create_test_anon_reader();

        // create_test_anon_reader leaves epoch at 0 (freshly zeroed mmap), which is
        // exactly what "no epoch / v2 daemon" looks like.
        assert_eq!(reader.epoch(), 0, "zeroed ring must report epoch 0");

        // Stand in for the daemon stamping a generation.
        // SAFETY: offset 48 is inside the validated 64-byte header.
        unsafe {
            reader
                .base_ptr_for_test()
                .add(48)
                .cast::<u32>()
                .write_volatile(0xDEAD_BEEF);
        }
        assert_eq!(reader.epoch(), 0xDEAD_BEEF);
    }
}
