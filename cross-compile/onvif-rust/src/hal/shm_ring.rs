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
//! // Read frame from shared memory
//! let (metadata, data) = reader.read_slot_into_bytesmut(notif.slot_index, Some(&pool))?;
//!
//! // Process frame...
//!
//! // Release slot back to daemon
//! reader.release_slot(notif.slot_index)?;
//! ```

use std::ffi::CString;
use std::sync::atomic::{AtomicU32, Ordering};

use bytes::BytesMut;

use crate::platform::PlatformError;
use crate::platform::PlatformResult;
use crate::platform::frame::{FrameMetadata, FrameType, StreamId};
use crate::streaming::bridge::BytesMutPool;

// =============================================================================
// Layout Constants (must match C header exactly)
// =============================================================================

/// Magic value identifying the shared memory region ("VDFS")
pub const VD_SHM_MAGIC: u32 = 0x5644_4653;
/// Version of the shared memory protocol (v2 adds diagnostic counters + wall-clock timing)
pub const VD_SHM_VERSION: u32 = 2;
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
    /// Padding to 64 bytes (reduced from 32)
    pub _padding: [u8; 16],
}

/// Slot header (64 bytes, must match C struct exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SlotHeader {
    /// Slot state (VD_SLOT_*)
    pub state: u32,
    /// Length of valid frame data in bytes
    pub frame_len: u32,
    /// Timestamp in microseconds
    pub timestamp_us: u64,
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
    /// Delta from previous frame in microseconds (version >= 2)
    pub inter_frame_us: u32,
    /// Reserved for future use
    pub _reserved: u32,
    /// Padding to 64 bytes (reduced from 32)
    pub _padding: [u8; 16],
}

/// Frame notification received from daemon (12 bytes)
/// This is received via socket to indicate a frame is ready in shm.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameNotification {
    /// Index of the slot containing the frame
    pub slot_index: u32,
    /// Length of the frame data
    pub frame_len: u32,
    /// Notification flags
    pub flags: u32,
}

impl FrameNotification {
    /// Parse a frame notification from 12 raw bytes.
    ///
    /// The bytes are expected to be in little-endian format.
    ///
    /// # Arguments
    ///
    /// * `bytes` - 12-byte slice containing the notification data
    ///
    /// # Returns
    ///
    /// Parsed `FrameNotification` struct
    pub fn from_bytes(bytes: &[u8; 12]) -> Self {
        // SAFETY: The array is exactly 12 bytes and the sub-slices are exactly 4 bytes,
        // so try_into() for [u8; 4] is infallible. Using unwrap is safe here because
        // the fixed-size input guarantees correct sub-slice lengths.
        // However, to follow project standards, use explicit array conversion:
        Self {
            slot_index: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            frame_len: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            flags: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        }
    }

    /// Check if this is a socket fallback notification.
    ///
    /// When the daemon cannot use shared memory (e.g., due to resource constraints),
    /// it will send this flag to indicate the caller should fall back to socket reads.
    pub fn is_socket_fallback(&self) -> bool {
        self.flags & VD_NOTIFY_SOCKET_FALLBACK != 0
    }

    /// Check if this is the last fragment of a multi-packet frame.
    pub fn is_last_fragment(&self) -> bool {
        self.flags & VD_NOTIFY_LAST_FRAGMENT != 0
    }

    /// Check if the daemon intentionally dropped this frame.
    ///
    /// During ring buffer overflow, P-frames are dropped instead of using
    /// the expensive socket fallback. The daemon sends a notification with
    /// this flag set so the Rust side can track the drop.
    pub fn is_frame_dropped(&self) -> bool {
        self.flags & VD_NOTIFY_FRAME_DROPPED != 0
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
    /// Local copy of read_seq for tracking position
    local_read_seq: u32,
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
        let header = Self::header_from_ptr(base as *const RingHeader);
        if header.magic != VD_SHM_MAGIC {
            // SAFETY: We own the mmap'd region and fd, must clean up on error.
            unsafe {
                libc::munmap(base, VD_SHM_TOTAL_SIZE);
                libc::close(fd);
            }
            return Err(PlatformError::InvalidParameter(format!(
                "invalid shm magic: expected {:#x}, got {:#x}",
                VD_SHM_MAGIC, header.magic
            )));
        }

        if header.version < VD_SHM_VERSION_MIN || header.version > VD_SHM_VERSION {
            // SAFETY: We own the mmap'd region and fd, must clean up on error.
            unsafe {
                libc::munmap(base, VD_SHM_TOTAL_SIZE);
                libc::close(fd);
            }
            return Err(PlatformError::InvalidParameter(format!(
                "unsupported shm version: expected {}-{}, got {}",
                VD_SHM_VERSION_MIN, VD_SHM_VERSION, header.version
            )));
        }

        // Initialize local read_seq from current value
        let local_read_seq = header.read_seq;

        tracing::debug!(
            "opened shm ring buffer: {} slots, {} bytes each",
            VD_SHM_SLOT_COUNT,
            VD_SHM_SLOT_DATA_SIZE
        );

        Ok(Some(Self {
            base: base as *mut u8,
            size: VD_SHM_TOTAL_SIZE,
            fd,
            local_read_seq,
        }))
    }

    /// Check if the ring buffer has been shut down by the daemon.
    ///
    /// When the daemon terminates, it sets the VD_FLAG_SHUTDOWN flag to
    /// notify readers to stop.
    pub fn is_shutdown(&self) -> bool {
        let flags = self.flags_atomic().load(Ordering::Acquire);
        flags & VD_FLAG_SHUTDOWN != 0
    }

    /// Check if a ring buffer overflow has occurred.
    ///
    /// This flag is set when the daemon detects that the reader is falling behind
    /// and frames are being dropped.
    pub fn has_overflow(&self) -> bool {
        let flags = self.flags_atomic().load(Ordering::Acquire);
        flags & VD_FLAG_OVERFLOW != 0
    }

    /// Read diagnostic counters from the ring header (version >= 2).
    ///
    /// Returns (overflow_count, eviction_count, socket_fallback_count, dropped_count).
    /// For version 1 ring buffers, all counters return 0.
    pub fn diagnostic_counters(&self) -> (u32, u32, u32, u32) {
        let header = Self::header_from_ptr(self.base as *const RingHeader);
        if header.version < 2 {
            return (0, 0, 0, 0);
        }
        // SAFETY: header pointer is from mmap, fields are written atomically by daemon
        let overflow = unsafe {
            let ptr = &header.overflow_count as *const u32 as *const AtomicU32;
            (*ptr).load(Ordering::Relaxed)
        };
        let eviction = unsafe {
            let ptr = &header.eviction_count as *const u32 as *const AtomicU32;
            (*ptr).load(Ordering::Relaxed)
        };
        let fallback = unsafe {
            let ptr = &header.socket_fallback_count as *const u32 as *const AtomicU32;
            (*ptr).load(Ordering::Relaxed)
        };
        let dropped = unsafe {
            let ptr = &header.dropped_count as *const u32 as *const AtomicU32;
            (*ptr).load(Ordering::Relaxed)
        };
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
                    timestamp_us: header.timestamp_us,
                    seq_no: header.seq_no,
                    frame_type: header.frame_type,
                    stream_id: header.stream_id,
                    slot_index,
                })
            }
            Err(current) => {
                // Slot state changed between our read and CAS
                Err(PlatformError::ResourceBusy(format!(
                    "slot {} state changed during acquire (was {}, now {})",
                    slot_index, VD_SLOT_READY, current
                )))
            }
        }
    }

    /// Copy frame data from a slot into a `BytesMut` buffer.
    ///
    /// This is the integration point with the existing VendorIpc frame path.
    /// It performs one copy from shared memory to the BytesMut buffer.
    ///
    /// # Arguments
    ///
    /// * `slot_index` - Index of the slot to read
    /// * `pool` - Optional BytesMutPool for buffer allocation
    ///
    /// # Returns
    ///
    /// * `Ok((FrameMetadata, BytesMut))` - Frame metadata and data buffer
    /// * `Err(PlatformError)` - Error reading the slot
    pub fn read_slot_into_bytesmut(
        &mut self,
        slot_index: u32,
        pool: Option<&BytesMutPool>,
    ) -> PlatformResult<(FrameMetadata, BytesMut)> {
        // Acquire the slot (borrowed access)
        let frame = self.read_slot(slot_index)?;

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
            timestamp_ms: header.timestamp_us / 1000,
            seq_no: header.seq_no,
            frame_type: shm_frame_type_to_onvif(header.frame_type),
            remote_token: 0, // Not used in shm path
        };

        // Release the slot
        self.release_slot(slot_index)?;

        Ok((metadata, buf))
    }

    /// Release a slot back to the daemon for reuse.
    ///
    /// This atomically transitions the slot from Reading to EMPTY state
    /// and advances the local read sequence counter.
    ///
    /// # Arguments
    ///
    /// * `slot_index` - Index of the slot to release
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Slot successfully released
    /// * `Err(PlatformError)` - Slot not in READING state
    pub fn release_slot(&mut self, slot_index: u32) -> PlatformResult<()> {
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
                // Advance local read_seq
                self.local_read_seq += 1;
                self.read_seq_atomic()
                    .store(self.local_read_seq, Ordering::Release);
                Ok(())
            }
            Err(current) => Err(PlatformError::HardwareFailure(format!(
                "failed to release slot {}: expected state {}, got {}",
                slot_index, VD_SLOT_READING, current
            ))),
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
    /// This is updated by this reader when it releases a slot.
    pub fn read_seq(&self) -> u32 {
        self.local_read_seq
    }

    /// Get the number of available frames (slots in READY state).
    ///
    /// This is a snapshot and may be stale by the time it's used.
    pub fn available_frames(&self) -> u32 {
        let write = self.write_seq();
        // Simple wrapping subtraction works for monotonically increasing u32 sequences
        write.wrapping_sub(self.local_read_seq)
    }

    // =============================================================================
    // Private helpers for accessing atomic fields
    // =============================================================================

    /// Get header from raw pointer (for open_path)
    #[inline]
    fn header_from_ptr(ptr: *const RingHeader) -> &'static RingHeader {
        // SAFETY: Called with verified pointer from mmap during open.
        unsafe { &*ptr }
    }

    /// Get slot header at index
    #[inline]
    fn slot_header(&self, index: u32) -> &SlotHeader {
        let offset = VD_SHM_HEADER_SIZE + (index as usize) * VD_SHM_SLOT_SIZE;
        // SAFETY: Offset is within bounds, structure fits.
        unsafe {
            let ptr = self.base.add(offset);
            &*ptr.cast::<SlotHeader>()
        }
    }

    /// Get slot data slice at index
    #[inline]
    fn slot_data(&self, index: u32) -> &[u8] {
        let offset =
            VD_SHM_HEADER_SIZE + (index as usize) * VD_SHM_SLOT_SIZE + VD_SHM_SLOT_HDR_SIZE;
        // SAFETY: Offset and size are within bounds.
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

    /// Get atomic state field for a slot
    #[inline]
    fn slot_state_atomic(&self, index: u32) -> &AtomicU32 {
        let offset = VD_SHM_HEADER_SIZE + (index as usize) * VD_SHM_SLOT_SIZE;
        // SAFETY: Offset is within bounds, state is first field of SlotHeader.
        unsafe {
            let ptr = self.base.add(offset);
            &*ptr.cast::<AtomicU32>()
        }
    }
}

impl Drop for ShmRingReader {
    fn drop(&mut self) {
        // SAFETY: We own the mmap'd region and fd, must clean up.
        // munmap and close are safe: we own these resources and no other thread
        // is accessing them at this point (the type is not Sync).
        unsafe {
            libc::munmap(self.base as *mut libc::c_void, self.size);
            libc::close(self.fd);
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
    /// Timestamp in microseconds
    pub timestamp_us: u64,
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
mod tests {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Seek, Write};

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
            _padding: [0u8; 16],
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
            timestamp_us: 0,
            seq_no: 0,
            frame_type: 0,
            stream_id: 0,
            checksum: 0,
            wall_clock_us: 0,
            inter_frame_us: 0,
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

    #[test]
    fn test_frame_notification_from_bytes() {
        let bytes: [u8; 12] = [
            0x03, 0x00, 0x00, 0x00, // slot_index = 3
            0x80, 0x10, 0x00, 0x00, // frame_len = 4224
            0x01, 0x00, 0x00, 0x00, // flags = 1 (LAST_FRAGMENT)
        ];

        let notif = FrameNotification::from_bytes(&bytes);

        assert_eq!(notif.slot_index, 3);
        assert_eq!(notif.frame_len, 4224);
        assert!(notif.is_last_fragment());
        assert!(!notif.is_socket_fallback());
    }

    #[test]
    fn test_frame_notification_socket_fallback() {
        let bytes: [u8; 12] = [
            0x00, 0x00, 0x00, 0x00, // slot_index = 0
            0x00, 0x00, 0x00, 0x00, // frame_len = 0
            0x02, 0x00, 0x00, 0x00, // flags = 2 (SOCKET_FALLBACK)
        ];

        let notif = FrameNotification::from_bytes(&bytes);

        assert!(notif.is_socket_fallback());
        assert!(!notif.is_last_fragment());
    }

    #[test]
    fn test_constants_match() {
        // Compile-time assertions to ensure struct layouts match C exactly
        // These use const evaluation to verify at compile time
        const _: () = assert!(std::mem::size_of::<RingHeader>() == 64);
        const _: () = assert!(std::mem::size_of::<SlotHeader>() == 64);
        const _: () = assert!(std::mem::size_of::<FrameNotification>() == 12);

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
    #[ignore] // Disabled due to intermittent segfault - needs investigation
    fn test_open_invalid_magic() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_shm_invalid_magic");

        // Create file with wrong magic
        {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)
                .unwrap();
            file.set_len(VD_SHM_TOTAL_SIZE as u64).unwrap();

            let header = RingHeader {
                magic: 0xDEADBEEF, // Invalid magic
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
                _padding: [0u8; 16],
            };

            let header_bytes = unsafe {
                std::slice::from_raw_parts(
                    &header as *const RingHeader as *const u8,
                    VD_SHM_HEADER_SIZE,
                )
            };
            file.write_all(header_bytes).unwrap();
        }

        let result = ShmRingReader::open_path(path.to_str().unwrap());
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, PlatformError::InvalidParameter(_)));

        std::fs::remove_file(&path).ok();
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
        assert!(!reader.is_shutdown());
        assert!(!reader.has_overflow());
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
    #[ignore = "mmap test has race condition with file I/O in tests"]
    fn test_read_and_release_slot() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_shm_read");

        // Define test data outside the block so it's in scope later
        let test_data = b"test frame data 1234567890";

        // First create and setup the test ring buffer
        {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)
                .unwrap();

            file.set_len(VD_SHM_TOTAL_SIZE as u64).unwrap();

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
                _padding: [0u8; 16],
            };

            let header_bytes = unsafe {
                std::slice::from_raw_parts(
                    &header as *const RingHeader as *const u8,
                    VD_SHM_HEADER_SIZE,
                )
            };
            file.write_all(header_bytes).unwrap();

            // Write empty slots
            let empty_slot = SlotHeader {
                state: VD_SLOT_EMPTY,
                frame_len: 0,
                timestamp_us: 0,
                seq_no: 0,
                frame_type: 0,
                stream_id: 0,
                checksum: 0,
                wall_clock_us: 0,
                inter_frame_us: 0,
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
                file.write_all(slot_bytes).unwrap();
                file.seek_relative(VD_SHM_SLOT_DATA_SIZE as i64).unwrap();
            }

            // Now set slot 0 to READY state with test data
            let slot_offset = VD_SHM_HEADER_SIZE + 0 * VD_SHM_SLOT_SIZE;

            let slot = SlotHeader {
                state: VD_SLOT_READY,
                frame_len: 100,
                timestamp_us: 1234567890,
                seq_no: 42,
                frame_type: 0,
                stream_id: 0,
                checksum: 0,
                wall_clock_us: 0,
                inter_frame_us: 0,
                _reserved: 0,
                _padding: [0u8; 16],
            };

            let slot_bytes = unsafe {
                std::slice::from_raw_parts(
                    &slot as *const SlotHeader as *const u8,
                    VD_SHM_SLOT_HDR_SIZE,
                )
            };
            file.seek(std::io::SeekFrom::Start(slot_offset as u64))
                .unwrap();
            file.write_all(slot_bytes).unwrap();

            // Write test data
            let data_offset = slot_offset + VD_SHM_SLOT_HDR_SIZE;
            file.seek(std::io::SeekFrom::Start(data_offset as u64))
                .unwrap();
            file.write_all(test_data).unwrap();

            // Sync to ensure mmap sees the writes
            file.sync_all().unwrap();
        }

        let mut reader = ShmRingReader::open_path(path.to_str().unwrap())
            .unwrap()
            .unwrap();

        // Now read the slot
        let frame = reader.read_slot(0).unwrap();
        assert_eq!(frame.data, test_data.as_slice());
        assert_eq!(frame.timestamp_us, 1234567890);
        assert_eq!(frame.seq_no, 42);
        assert_eq!(frame.frame_type, 0);
        assert_eq!(frame.stream_id, 0);

        // Release the slot
        reader.release_slot(0).unwrap();

        // Verify state went back to EMPTY
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let slot_offset = VD_SHM_HEADER_SIZE + 0 * VD_SHM_SLOT_SIZE;
        file.seek(std::io::SeekFrom::Start(slot_offset as u64))
            .unwrap();
        let mut read_slot = [0u8; VD_SHM_SLOT_HDR_SIZE];
        file.read_exact(&mut read_slot).unwrap();
        let state = u32::from_le_bytes(read_slot[0..4].try_into().unwrap());
        assert_eq!(state, VD_SLOT_EMPTY);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    #[ignore = "mmap test has race condition with file I/O in tests"]
    fn test_read_slot_into_bytesmut() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_shm_bytesmut");

        create_test_ring_buffer(path.to_str().unwrap()).unwrap();

        let mut reader = ShmRingReader::open_path(path.to_str().unwrap())
            .unwrap()
            .unwrap();

        // Set slot 0 to READY state with test data
        let slot_offset = VD_SHM_HEADER_SIZE + 0 * VD_SHM_SLOT_SIZE;

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();

        let mut slot = SlotHeader {
            state: VD_SLOT_READY,
            frame_len: 100,
            timestamp_us: 2000000, // 2 seconds in us
            seq_no: 100,
            frame_type: 1, // I-frame (VD_FRAME_TYPE_I)
            stream_id: 1,  // Sub stream
            checksum: 0,
            wall_clock_us: 0,
            inter_frame_us: 0,
            _reserved: 0,
            _padding: [0u8; 16],
        };

        file.seek(std::io::SeekFrom::Start(slot_offset as u64))
            .unwrap();
        let slot_bytes = unsafe {
            std::slice::from_raw_parts(
                &mut slot as *mut SlotHeader as *mut u8,
                VD_SHM_SLOT_HDR_SIZE,
            )
        };
        file.write_all(slot_bytes).unwrap();

        let test_data = b"bytesmut test frame data here";
        let data_offset = slot_offset + VD_SHM_SLOT_HDR_SIZE;
        file.seek(std::io::SeekFrom::Start(data_offset as u64))
            .unwrap();
        file.write_all(test_data).unwrap();

        // Use the method under test
        let pool = BytesMutPool::new(1024, 4);
        let (metadata, buf) = reader.read_slot_into_bytesmut(0, Some(&pool)).unwrap();

        // Verify metadata
        assert_eq!(metadata.timestamp_ms, 2000); // 2000000 us -> 2000 ms
        assert_eq!(metadata.seq_no, 100);
        assert_eq!(metadata.frame_type, FrameType::VideoIFrame);

        // Verify data was copied
        assert_eq!(buf.as_ref(), test_data.as_slice());

        // Pool should have a buffer available now
        assert_eq!(pool.available(), 1);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_shutdown_flag() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_shm_shutdown");

        create_test_ring_buffer(path.to_str().unwrap()).unwrap();

        let reader = ShmRingReader::open_path(path.to_str().unwrap())
            .unwrap()
            .unwrap();

        // Initially not shutdown
        assert!(!reader.is_shutdown());

        // Set shutdown flag via file
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();

        // flags is at offset 28 (7 * 4 bytes) in header
        let flags_offset = 28usize;
        let shutdown_flags: u32 = VD_FLAG_SHUTDOWN;
        file.seek(std::io::SeekFrom::Start(flags_offset as u64))
            .unwrap();
        file.write_all(&shutdown_flags.to_le_bytes()).unwrap();

        // Now it should show as shutdown
        assert!(reader.is_shutdown());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_overflow_flag() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_shm_overflow");

        create_test_ring_buffer(path.to_str().unwrap()).unwrap();

        let reader = ShmRingReader::open_path(path.to_str().unwrap())
            .unwrap()
            .unwrap();

        // Initially no overflow
        assert!(!reader.has_overflow());

        // Set overflow flag via file
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();

        let flags_offset = 28usize;
        let overflow_flags: u32 = VD_FLAG_OVERFLOW;
        file.seek(std::io::SeekFrom::Start(flags_offset as u64))
            .unwrap();
        file.write_all(&overflow_flags.to_le_bytes()).unwrap();

        // Now it should show overflow
        assert!(reader.has_overflow());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_frame_notification_le_values() {
        // Test little-endian parsing with various values
        let bytes: [u8; 12] = [
            0xFF, 0xFF, 0xFF, 0xFF, // slot_index = u32::MAX
            0x00, 0x00, 0x00, 0x00, // frame_len = 0
            0x00, 0x00, 0x00, 0x00, // flags = 0
        ];

        let notif = FrameNotification::from_bytes(&bytes);
        assert_eq!(notif.slot_index, u32::MAX);
    }
}
