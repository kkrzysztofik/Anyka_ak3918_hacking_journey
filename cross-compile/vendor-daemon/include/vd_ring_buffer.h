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

#ifdef __cplusplus
extern "C" {
#endif

/*============================================================================
 * Constants and Magic Numbers
 *============================================================================*/

/** Magic identifier for shared memory validation */
#define VD_SHM_MAGIC       0x56444653  /* "VDFS" */

/** Protocol version - bump on incompatible layout changes */
#define VD_SHM_VERSION     1

/** Default total shared memory size: 1 MB */
#define VD_SHM_DEFAULT_SIZE (1024 * 1024)

/** Number of slots in the ring buffer */
#define VD_SHM_SLOT_COUNT  8

/** Size of each slot (including header): 128 KB */
#define VD_SHM_SLOT_SIZE   (128 * 1024)

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
    uint8_t  _padding[32];    /* Pad to 64 bytes (cache line alignment) */
} __attribute__((packed));

/**
 * @brief Slot header (64 bytes, at offset header_size + slot_index * slot_size)
 *
 * Each slot has a header followed by data. Must be exactly 64 bytes.
 */
struct vd_slot_header {
    uint32_t state;           /* VD_SLOT_EMPTY | WRITING | READY | READING */
    uint32_t frame_len;       /* Actual frame data length */
    uint64_t timestamp_us;   /* Timestamp in microseconds */
    uint32_t seq_no;          /* Frame sequence number */
    uint32_t frame_type;      /* 0=P, 1=I, 2=B, 3=Pi */
    uint32_t stream_id;       /* 0=main, 1=sub, 2=audio */
    uint32_t checksum;        /* CRC32 of frame data (0 = not computed) */
    uint8_t  _padding[32];   /* Pad to 64 bytes */
} __attribute__((packed));

/**
 * @brief Frame notification (12 bytes, sent on Unix socket)
 *
 * Sent from daemon to Rust when a new frame is ready.
 * Must match the Rust equivalent exactly.
 */
struct vd_frame_notify {
    uint32_t slot_index;      /* Which ring buffer slot */
    uint32_t frame_len;       /* Frame data length */
    uint32_t flags;          /* bit 0: is_last_fragment, bit 1: socket_fallback */
} __attribute__((packed));

/* Compile-time assertion for struct sizes */
_Static_assert(sizeof(struct vd_ring_header) == 64, "vd_ring_header must be 64 bytes");
_Static_assert(sizeof(struct vd_slot_header) == 64, "vd_slot_header must be 64 bytes");
_Static_assert(sizeof(struct vd_frame_notify) == 12, "vd_frame_notify must be 12 bytes");

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
#if defined(__arm__)
#define VD_DATA_MEMORY_BARRIER() \
    __asm__ __volatile__("mcr p15, 0, %0, c7, c10, 4" :: "r"(0) : "memory")
#else
#define VD_DATA_MEMORY_BARRIER() \
    __asm__ __volatile__("" ::: "memory")
#endif

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
 * @brief Open existing ring buffer (client side - Rust)
 *
 * Opens the shared memory file and maps it. The Rust side uses this
 * to access frames written by the daemon.
 *
 * @return Pointer to mapped region on success, NULL on failure
 */
static inline void *vd_ring_open(void)
{
    int fd;
    void *base;
    struct vd_ring_header *hdr;

    /* Open existing shared memory file using regular file */
    fd = open(VD_SHM_PATH, O_RDWR, 0600);
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

    /* Map the region */
    base = mmap(NULL, VD_SHM_TOTAL_SIZE, PROT_READ | PROT_WRITE,
                MAP_SHARED, fd, 0);
    close(fd);

    if (base == MAP_FAILED) {
        return NULL;
    }

    /* Validate header */
    hdr = vd_ring_get_header(base);
    if (hdr->magic != VD_SHM_MAGIC || hdr->version != VD_SHM_VERSION) {
        munmap(base, VD_SHM_TOTAL_SIZE);
        return NULL;
    }

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
 * @param timestamp_us Frame timestamp in microseconds
 * @param seq_no     Frame sequence number
 * @param frame_type Frame type (0=P, 1=I, 2=B, 3=Pi)
 * @param stream_id  Stream identifier (0=main, 1=sub, 2=audio)
 *
 * @return Slot index on success, -1 on overflow, -2 on error
 */
static inline int vd_ring_write(void *base, const void *frame_data, uint32_t frame_len,
                                uint64_t timestamp_us, uint32_t seq_no,
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
    slot->timestamp_us = timestamp_us;
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
 * @brief Read a frame from a slot (Rust consumer side)
 *
 * Called when receiving notification of a ready slot.
 * Uses CAS to atomically check and claim the slot (lease-based access).
 * Does NOT mark slot as EMPTY - caller must call vd_ring_release() after processing.
 *
 * @param base       Ring buffer base pointer
 * @param slot_idx   Slot index to read from
 * @param out_data   Buffer to copy frame data into (can be NULL if only checking)
 * @param out_cap    Capacity of out_data buffer (must be >= frame_len)
 * @param out_len    Pointer to store frame length (can be NULL)
 *
 * @return 0 on success, -1 if slot not ready or already claimed, -2 on error or buffer too small
 */
static inline int vd_ring_read(void *base, uint32_t slot_idx, 
                                void *out_data, uint32_t out_cap, uint32_t *out_len)
{
    struct vd_slot_header *slot;
    uint32_t expected = VD_SLOT_READY;

    if (base == NULL || slot_idx >= VD_SHM_SLOT_COUNT) {
        return -2;
    }

    slot = vd_ring_get_slot_hdr(base, slot_idx);

    /* Atomically CAS from READY to READING (lease-based checkout) */
    if (!__atomic_compare_exchange_n(&slot->state, &expected, VD_SLOT_READING,
                                     0, __ATOMIC_ACQUIRE, __ATOMIC_ACQUIRE)) {
        return -1;  /* Slot not ready or already claimed */
    }

    /* Check buffer capacity before copying */
    if (out_data != NULL && slot->frame_len > out_cap) {
        /* Release slot back to EMPTY */
        __atomic_store_n(&slot->state, VD_SLOT_EMPTY, __ATOMIC_RELEASE);
        return -2;  /* Buffer too small */
    }

    /* Copy frame data if buffers provided */
    if (out_data != NULL && out_len != NULL) {
        *out_len = slot->frame_len;
        memcpy(out_data, vd_ring_get_slot_data(base, slot_idx), slot->frame_len);
    } else if (out_len != NULL) {
        /* Only fetch length if requested */
        *out_len = slot->frame_len;
    }

    return 0;
}

/**
 * @brief Release a slot back to the pool (Rust consumer)
 *
 * Called after processing a frame to release the slot back to the pool.
 * Uses CAS to atomically transition from READING to EMPTY.
 *
 * @param base     Ring buffer base pointer
 * @param slot_idx Slot index to release
 *
 * @return 0 on success, -1 if slot not in READING state, -2 on error
 */
static inline int vd_ring_release(void *base, uint32_t slot_idx)
{
    struct vd_ring_header *hdr;
    struct vd_slot_header *slot;
    uint32_t expected = VD_SLOT_READING;

    if (base == NULL || slot_idx >= VD_SHM_SLOT_COUNT) {
        return -2;
    }

    slot = vd_ring_get_slot_hdr(base, slot_idx);

    /* CAS from READING to EMPTY */
    if (!__atomic_compare_exchange_n(&slot->state, &expected, VD_SLOT_EMPTY,
                                     0, __ATOMIC_RELEASE, __ATOMIC_ACQUIRE)) {
        return -1;  /* Slot not in READING state */
    }

    /* Increment read sequence */
    hdr = vd_ring_get_header(base);
    __atomic_add_fetch(&hdr->read_seq, 1, __ATOMIC_RELEASE);

    return 0;
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
 * @brief Check if shutdown was requested
 */
static inline int vd_ring_is_shutdown(void *base)
{
    struct vd_ring_header *hdr;
    if (base == NULL) return 1;
    hdr = vd_ring_get_header(base);
    return (hdr->flags & VD_FLAG_SHUTDOWN) != 0;
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
