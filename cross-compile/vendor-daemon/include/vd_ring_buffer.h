/**
 * @file vd_ring_buffer.h
 * @brief Shared memory ring buffer for zero-copy frame transfer
 *        between vendor-daemon (C) and onvif-rust (Rust)
 *
 * This header is the single source of truth for both C and Rust implementations.
 * Ring buffer layout: [header (64B)][slot 0][slot 1]...[slot N]
 * Each slot: [slot_header (64B)][data (128KB - 64B)]
 */

#ifndef VD_RING_BUFFER_H
#define VD_RING_BUFFER_H

#include <stdint.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <time.h>

#ifdef __cplusplus
extern "C" {
#endif

/*============================================================================
 * Constants and Magic Numbers
 *============================================================================*/

/** Magic identifier for shared memory validation */
#define VD_SHM_MAGIC       0x56444653  /* "VDFS" */

/** Protocol version - bump on incompatible layout changes
 *  v1: Initial layout (8 header fields + 32 bytes padding)
 *  v2: Added overflow diagnostic counters in header, wall-clock timing in slot header
 *  v3: Added the `epoch` field used to detect daemon restarts
 *  v4: VD_SHM_SLOT_SIZE 128 KB -> 256 KB. Both sides index slots by this
 *      stride, so a v3 reader against a v4 ring resolves every slot to the
 *      wrong offset. The bump makes that mismatch fail validation instead of
 *      silently returning garbage frames.
 */
#define VD_SHM_VERSION     4

/** Default total shared memory size: 1 MB */
#define VD_SHM_DEFAULT_SIZE (1024 * 1024)

/** Number of slots in the ring buffer */
#define VD_SHM_SLOT_COUNT  8

/**
 * Size of each slot (including header): 256 KB.
 *
 * Must stay in lockstep with VD_SHM_SLOT_SIZE in
 * onvif-rust/src/hal/anyka/ipc/shm_ring.rs -- both sides mmap
 * VD_SHM_TOTAL_SIZE and index slots by this stride, so a mismatch silently
 * desyncs every offset.
 *
 * Sized from measurement, not guesswork: 720p keyframes on the main stream run
 * ~184 KB and creep upward with scene detail, so at the previous 128 KB every
 * one of them was rejected by the size guard in push.c and the stream carried
 * P-frames only. See also METHOD_ISIZE_CTRL in handlers_venc.c, which caps the
 * encoder side so this ceiling is not approached again.
 */
#define VD_SHM_SLOT_SIZE   (256 * 1024)

/** Ring buffer header size */
#define VD_SHM_HEADER_SIZE 64

/** Per-slot header size */
#define VD_SHM_SLOT_HDR_SIZE 64

/** Usable data size per slot (slot_size - slot_header) */
#define VD_SHM_SLOT_DATA_SIZE (VD_SHM_SLOT_SIZE - VD_SHM_SLOT_HDR_SIZE)

/** Total shared memory size needed */
#define VD_SHM_TOTAL_SIZE (VD_SHM_HEADER_SIZE + VD_SHM_SLOT_COUNT * VD_SHM_SLOT_SIZE)

/** Path to shared memory file */
#define VD_SHM_PATH        "/tmp/vendor-frame-ring.shm"

/* Slot states */
#define VD_SLOT_EMPTY      0
#define VD_SLOT_WRITING    1
#define VD_SLOT_READY      2
#define VD_SLOT_READING    3  /* Consumer has checked out this slot (lease-based) */

/* Ring buffer flags */
#define VD_FLAG_SHUTDOWN   (1 << 0)
#define VD_FLAG_OVERFLOW   (1 << 1)

/* Frame notification flags */
#define VD_NOTIFY_LAST_FRAGMENT     (1 << 0)
#define VD_NOTIFY_SOCKET_FALLBACK   (1 << 1)
#define VD_NOTIFY_FRAME_DROPPED     (1 << 2)

/* Frame types (matching vendor SDK enum) */
#define VD_FRAME_TYPE_P   0
#define VD_FRAME_TYPE_I   1
#define VD_FRAME_TYPE_B   2
#define VD_FRAME_TYPE_PI  3

/* Stream IDs */
#define VD_STREAM_MAIN    0
#define VD_STREAM_SUB     1
#define VD_STREAM_AUDIO   2

/*============================================================================
 * Struct Definitions
 *============================================================================*/

/**
 * @brief Ring buffer header (64 bytes, at offset 0x0000)
 *
 * Must be exactly 64 bytes. Uses __attribute__((packed)) to ensure
 * identical layout between C and Rust.
 */
struct vd_ring_header {
    uint32_t magic;           /* VD_SHM_MAGIC */
    uint32_t version;         /* VD_SHM_VERSION */
    uint32_t total_size;      /* Total mmap'd region size */
    uint32_t slot_count;      /* Number of slots */
    uint32_t slot_data_size;  /* Usable data bytes per slot */
    uint32_t write_seq;       /* Monotonically increasing write sequence (daemon) */
    uint32_t read_seq;        /* Monotonically increasing read sequence (Rust) */
    uint32_t flags;           /* Shutdown, overflow indicators */
    /* Diagnostic counters (16 bytes, version >= 2) */
    uint32_t overflow_count;       /* Total ring-full events */
    uint32_t eviction_count;       /* P-frame evictions for I-frame priority */
    uint32_t socket_fallback_count;/* Frames sent via socket fallback */
    uint32_t dropped_count;        /* P-frames dropped during overflow */
    /*
     * Daemon generation counter (version >= 3).  Re-randomised on every
     * vd_ring_create(), i.e. on every daemon start.  Never 0 -- 0 is reserved
     * by the client to mean "not attached".
     *
     * This exists because the ring file is REUSED across restarts (O_CREAT
     * without O_TRUNC, same inode) and the magic/version are rewritten
     * identically, so the mapping itself carries no evidence of a restart.
     */
    uint32_t epoch;
    uint8_t  _padding[12];         /* pad to 64 bytes */
} __attribute__((packed));

/**
 * @brief Slot header (64 bytes, at offset header_size + slot_index * slot_size)
 *
 * Each slot has a header followed by data. Must be exactly 64 bytes.
 */
struct vd_slot_header {
    uint32_t state;           /* VD_SLOT_EMPTY | WRITING | READY | READING */
    uint32_t frame_len;       /* Actual frame data length */
    uint32_t timestamp_ms;   /* Timestamp in milliseconds (SDK ts directly) */
    uint32_t _ts_pad;        /* Maintain 64-byte struct alignment */
    uint32_t seq_no;          /* Frame sequence number */
    uint32_t frame_type;      /* 0=P, 1=I, 2=B, 3=Pi */
    uint32_t stream_id;       /* 0=main, 1=sub, 2=audio */
    uint32_t checksum;        /* CRC32 of frame data (0 = not computed) */
    /* Timing diagnostics (version >= 2, 16 bytes) */
    uint64_t wall_clock_us;   /* CLOCK_MONOTONIC at ring write time */
    uint32_t _reserved2;      /* Reserved (was inter_frame_us, now unused) */
    uint32_t _reserved;
    uint8_t  _padding[16];   /* Reduced from 32: pad to 64 bytes */
} __attribute__((packed));

/**
 * @brief Frame notification (20 bytes, sent on Unix socket)
 *
 * Sent from daemon to Rust when a new frame is ready.
 * Includes enough metadata for Rust to reject stale notifications when a slot
 * has already been reused before the socket notification is consumed.
 * Must match the Rust equivalent exactly.
 */
struct vd_frame_notify {
    uint32_t slot_index;      /* Which ring buffer slot */
    uint32_t frame_len;       /* Frame data length */
    uint32_t flags;          /* bit 0: is_last_fragment, bit 1: socket_fallback */
    uint32_t stream_id;      /* Stream encoded into the slot */
    uint32_t seq_no;         /* Frame sequence number encoded into the slot */
} __attribute__((packed));

/* Compile-time assertion for struct sizes */
_Static_assert(sizeof(struct vd_ring_header) == 64, "vd_ring_header must be 64 bytes");
_Static_assert(sizeof(struct vd_slot_header) == 64, "vd_slot_header must be 64 bytes");
_Static_assert(sizeof(struct vd_frame_notify) == 20, "vd_frame_notify must be 20 bytes");

/*============================================================================
 * Helper Macros
 *============================================================================*/

/**
 * @brief Calculate offset of slot N's header within the mmap'd region
 */
#define VD_SLOT_HDR_OFFSET(n) \
    (VD_SHM_HEADER_SIZE + (n) * (VD_SHM_SLOT_HDR_SIZE + VD_SHM_SLOT_DATA_SIZE))

/**
 * @brief Calculate offset of slot N's data within the mmap'd region
 */
#define VD_SLOT_DATA_OFFSET(n) \
    (VD_SLOT_HDR_OFFSET(n) + VD_SHM_SLOT_HDR_SIZE)

/*============================================================================
 * ARM Memory Barrier Macros
 *============================================================================*/

/**
 * @brief Data memory barrier for ARM926EJ-S
 * Required after memcpy of frame data, before setting slot state to READY.
 * This ensures all writes are visible before the state change.
 */
#define VD_DATA_MEMORY_BARRIER() __sync_synchronize()

/*============================================================================
 * Inline Helper Functions
 *============================================================================*/

/**
 * @brief Get pointer to ring buffer header
 */
static inline struct vd_ring_header *vd_ring_get_header(void *base)
{
    return (struct vd_ring_header *)base;
}

/**
 * @brief Get pointer to slot header by index
 */
static inline struct vd_slot_header *vd_ring_get_slot_hdr(void *base, uint32_t idx)
{
    return (struct vd_slot_header *)((uint8_t *)base + VD_SLOT_HDR_OFFSET(idx));
}

/**
 * @brief Get pointer to slot data by index
 */
static inline void *vd_ring_get_slot_data(void *base, uint32_t idx)
{
    return (uint8_t *)base + VD_SLOT_DATA_OFFSET(idx);
}

/**
 * @brief Generate a fresh, non-zero epoch for this daemon generation.
 *
 * Drawn from /dev/urandom.  PID and CLOCK_MONOTONIC are a poor source on this
 * target: a reboot restarts the monotonic clock near zero and hands out the
 * same PIDs in the same boot order, so two generations at the same point in
 * two boots can land very close together.  The epoch's only job is to differ
 * from the previous generation, so it is drawn from real entropy instead.
 *
 * Retries on a zero draw -- 0 is reserved for "detached" on the client side.
 * Clamping to 1 instead would make 0 twice as likely as any other value, which
 * matters here only in that it is free to avoid.
 */
static inline uint32_t vd_ring_new_epoch(void)
{
    uint32_t epoch = 0;
    int fd;

    fd = open("/dev/urandom", O_RDONLY | O_CLOEXEC);
    if (fd >= 0) {
        while (epoch == 0) {
            if (read(fd, &epoch, sizeof(epoch)) != (ssize_t)sizeof(epoch)) {
                epoch = 0;
                break;
            }
        }
        close(fd);
    }

    if (epoch == 0) {
        /* Degraded: no entropy source.  Fall back to the PID/clock mix rather
         * than failing ring creation and taking the camera down for a missing
         * /dev/urandom.  Restart detection still works; only the collision
         * margin is worse.  No logging here on purpose -- this header is
         * compiled standalone by tests/test_ring_epoch.c and must not pull in
         * the daemon's log.h.  vd_ring_create() is the place to log it. */
        struct timespec ts;

        if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) {
            ts.tv_sec = 0;
            ts.tv_nsec = 0;
        }
        epoch = (uint32_t)getpid() * 2654435761u;
        epoch ^= (uint32_t)ts.tv_nsec;
        epoch ^= (uint32_t)ts.tv_sec << 16;
        if (epoch == 0) {
            epoch = 1u;
        }
    }

    return epoch;
}

/**
 * @brief Create and initialize the ring buffer (daemon side)
 *
 * Creates the shared memory file, truncates to VD_SHM_TOTAL_SIZE,
 * maps it, and initializes the header.
 *
 * @return Pointer to mapped region on success, NULL on failure
 */
static inline void *vd_ring_create(void)
{
    int fd;
    void *base;
    struct vd_ring_header *hdr;

    /* Open/create shared memory file using regular file (works without /dev/shm) */
    fd = open(VD_SHM_PATH, O_CREAT | O_RDWR, 0600);
    if (fd >= 0) {
        /* Set close-on-exec to prevent fd leaking to child processes */
        int flags = fcntl(fd, F_GETFD);
        if (flags >= 0) {
            fcntl(fd, F_SETFD, flags | FD_CLOEXEC);
        }
    }
    if (fd < 0) {
        return NULL;
    }

    /* Set size */
    if (ftruncate(fd, VD_SHM_TOTAL_SIZE) < 0) {
        close(fd);
        unlink(VD_SHM_PATH);
        return NULL;
    }

    /* Map the region */
    base = mmap(NULL, VD_SHM_TOTAL_SIZE, PROT_READ | PROT_WRITE,
                MAP_SHARED, fd, 0);
    close(fd);

    if (base == MAP_FAILED) {
        unlink(VD_SHM_PATH);
        return NULL;
    }

    /* Initialize header */
    hdr = vd_ring_get_header(base);
    memset(hdr, 0, sizeof(*hdr));
    hdr->magic = VD_SHM_MAGIC;
    hdr->version = VD_SHM_VERSION;
    hdr->total_size = VD_SHM_TOTAL_SIZE;
    hdr->slot_count = VD_SHM_SLOT_COUNT;
    hdr->slot_data_size = VD_SHM_SLOT_DATA_SIZE;
    hdr->write_seq = 0;
    hdr->read_seq = 0;
    hdr->flags = 0;
    hdr->epoch = vd_ring_new_epoch();

    /* Initialize all slots to EMPTY state */
    for (uint32_t i = 0; i < VD_SHM_SLOT_COUNT; i++) {
        struct vd_slot_header *slot = vd_ring_get_slot_hdr(base, i);
        memset(slot, 0, sizeof(*slot));
        slot->state = VD_SLOT_EMPTY;
    }

    /* Ensure all writes are visible */
    VD_DATA_MEMORY_BARRIER();

    return base;
}

/**
 * @brief Write a frame to the next available slot (daemon side)
 *
 * Uses atomic operations for sequence numbers. Performs overflow check
 * before writing. Sets slot state to READY after copying data.
 *
 * @param base       Ring buffer base pointer
 * @param frame_data Pointer to frame data to copy
 * @param frame_len  Length of frame data
 * @param timestamp_ms Frame timestamp in milliseconds
 * @param seq_no     Frame sequence number
 * @param frame_type Frame type (0=P, 1=I, 2=B, 3=Pi)
 * @param stream_id  Stream identifier (0=main, 1=sub, 2=audio)
 *
 * @return Slot index on success, -1 on overflow, -2 on error
 */
static inline int vd_ring_write(void *base, const void *frame_data, uint32_t frame_len,
                                uint32_t timestamp_ms, uint32_t seq_no,
                                uint32_t frame_type, uint32_t stream_id)
{
    struct vd_ring_header *hdr;
    struct vd_slot_header *slot;
    uint32_t slot_idx;
    void *slot_data;

    if (base == NULL || frame_data == NULL || frame_len > VD_SHM_SLOT_DATA_SIZE) {
        return -2;
    }

    hdr = vd_ring_get_header(base);

    /* Check for shutdown (atomic load) */
    if (__atomic_load_n(&hdr->flags, __ATOMIC_ACQUIRE) & VD_FLAG_SHUTDOWN) {
        return -2;
    }

    /* Atomically load current sequence numbers */
    uint32_t write_seq = __atomic_load_n(&hdr->write_seq, __ATOMIC_ACQUIRE);
    uint32_t read_seq = __atomic_load_n(&hdr->read_seq, __ATOMIC_ACQUIRE);

    /* Overflow detection: if write_seq - read_seq >= slot_count, buffer is full */
    if (write_seq - read_seq >= VD_SHM_SLOT_COUNT) {
        /* Set overflow flag */
        __atomic_or_fetch(&hdr->flags, VD_FLAG_OVERFLOW, __ATOMIC_RELEASE);
        return -1;
    }

    /* Calculate slot index using circular buffer */
    slot_idx = write_seq % VD_SHM_SLOT_COUNT;

    /* Get slot pointer */
    slot = vd_ring_get_slot_hdr(base, slot_idx);

    /* Ensure slot is in a writable state (should be EMPTY or was READ) */
    /* Mark as WRITING */
    __atomic_store_n(&slot->state, VD_SLOT_WRITING, __ATOMIC_RELEASE);

    /* Copy frame data */
    slot_data = vd_ring_get_slot_data(base, slot_idx);
    memcpy(slot_data, frame_data, frame_len);

    /* Data memory barrier - ensure memcpy completes before updating header */
    VD_DATA_MEMORY_BARRIER();

    /* Fill in slot header */
    slot->frame_len = frame_len;
    slot->timestamp_ms = timestamp_ms;
    slot->seq_no = seq_no;
    slot->frame_type = frame_type;
    slot->stream_id = stream_id;
    slot->checksum = 0;  /* CRC32 not computed by default */

    /* Mark slot as READY - this signals to consumer that data is valid */
    __atomic_store_n(&slot->state, VD_SLOT_READY, __ATOMIC_RELEASE);

    /* Increment write sequence */
    __atomic_add_fetch(&hdr->write_seq, 1, __ATOMIC_RELEASE);

    return (int)slot_idx;
}

/**
 * @brief Evict the oldest P/Pi-frame from the ring buffer to make room for an I-frame.
 *
 * Scans from read_seq to write_seq looking for the first P-frame or Pi-frame
 * slot in READY state. Uses CAS to atomically transition it to EMPTY.
 * This allows I-frames to always be placed in shared memory, avoiding
 * the expensive socket fallback path for large I-frames (15-80KB on ARM).
 *
 * @param base  Ring buffer base pointer
 *
 * @return Evicted slot index on success, -1 if no P/Pi-frame could be evicted
 */
static inline int vd_ring_evict_oldest_pframe(void *base)
{
    struct vd_ring_header *hdr = vd_ring_get_header(base);
    uint32_t read_seq = __atomic_load_n(&hdr->read_seq, __ATOMIC_ACQUIRE);
    uint32_t write_seq = __atomic_load_n(&hdr->write_seq, __ATOMIC_ACQUIRE);

    for (uint32_t i = read_seq; i < write_seq; i++) {
        uint32_t idx = i % VD_SHM_SLOT_COUNT;
        struct vd_slot_header *slot = vd_ring_get_slot_hdr(base, idx);
        uint32_t expected = VD_SLOT_READY;
        /* Evict P-frames and Pi-frames (not I or B) */
        if (slot->frame_type == VD_FRAME_TYPE_P || slot->frame_type == VD_FRAME_TYPE_PI) {
            if (__atomic_compare_exchange_n(&slot->state, &expected, VD_SLOT_EMPTY,
                                             0, __ATOMIC_RELEASE, __ATOMIC_ACQUIRE)) {
                __atomic_add_fetch(&hdr->read_seq, 1, __ATOMIC_RELEASE);
                return (int)idx;
            }
        }
    }
    return -1;
}

/**
 * @brief Reset ring buffer state for a new push session
 *
 * Resets sequences, flags, diagnostic counters, and all slot states to
 * their initial values.  Must be called before the first push thread
 * starts writing to avoid stale overflow from a previous session.
 *
 * Only the mutable header fields are cleared — magic, version, sizes,
 * and slot_count are preserved so the Rust consumer can still validate
 * the header after re-opening.
 *
 * @param base  Ring buffer base pointer (must not be NULL)
 */
static inline void vd_ring_reset(void *base)
{
    struct vd_ring_header *hdr = vd_ring_get_header(base);

    __atomic_store_n(&hdr->write_seq, 0, __ATOMIC_RELEASE);
    __atomic_store_n(&hdr->read_seq, 0, __ATOMIC_RELEASE);
    __atomic_store_n(&hdr->flags, 0, __ATOMIC_RELEASE);
    hdr->overflow_count = 0;
    hdr->eviction_count = 0;
    hdr->socket_fallback_count = 0;
    hdr->dropped_count = 0;

    for (uint32_t i = 0; i < VD_SHM_SLOT_COUNT; i++) {
        struct vd_slot_header *slot = vd_ring_get_slot_hdr(base, i);
        __atomic_store_n(&slot->state, VD_SLOT_EMPTY, __ATOMIC_RELEASE);
    }

    VD_DATA_MEMORY_BARRIER();
}

/**
 * @brief Signal shutdown to consumer
 */
static inline void vd_ring_shutdown(void *base)
{
    struct vd_ring_header *hdr;
    if (base == NULL) return;
    hdr = vd_ring_get_header(base);
    __atomic_or_fetch(&hdr->flags, VD_FLAG_SHUTDOWN, __ATOMIC_RELEASE);
}

/**
 * @brief Clean up ring buffer
 *
 * @param base       Ring buffer base pointer
 * @param is_creator If 1, also unlink the shared memory file
 */
static inline void vd_ring_destroy(void *base, int is_creator)
{
    if (base == NULL) return;

    munmap(base, VD_SHM_TOTAL_SIZE);

    if (is_creator) {
        unlink(VD_SHM_PATH);
    }
}

#ifdef __cplusplus
}
#endif

#endif /* VD_RING_BUFFER_H */
