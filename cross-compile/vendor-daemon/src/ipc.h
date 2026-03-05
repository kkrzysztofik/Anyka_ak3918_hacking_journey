#ifndef VENDOR_DAEMON_IPC_H
#define VENDOR_DAEMON_IPC_H

#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include <pthread.h>

#include "protocol.h"

/* Forward declaration — full definition is in vd_ring_buffer.h */
struct vd_frame_notify;

/* ---- I/O helpers --------------------------------------------------------- */

/**
 * read_exact - read exactly n bytes, retrying on partial reads.
 * Returns 0 on success, -1 on EOF or error.
 */
int read_exact(int fd, void *buf, size_t n);

/**
 * write_exact - write exactly n bytes, retrying on partial writes.
 * Returns 0 on success, -1 on error.
 */
int write_exact(int fd, const void *buf, size_t n);

/**
 * send_response - send [i32 status][u32 len][data] to client.
 * Returns 0 on success, -1 on write error.
 */
int send_response(int fd, int32_t status, const void *data, uint32_t len);

/* ---- Frame client helpers ----------------------------------------------- */

/** Map stream id to its dedicated frame client lock. */
pthread_mutex_t *frame_client_lock_for_stream(uint32_t stream_id);

/** Map stream id to its dedicated frame client fd storage. */
int *frame_client_fd_for_stream(uint32_t stream_id);

/**
 * send_frame_notification - send one 12-byte frame notification on the
 * dedicated channel for the given stream.
 */
int send_frame_notification(uint32_t stream_id, const struct vd_frame_notify *notif);

/* ---- Socket creation helper --------------------------------------------- */

/**
 * create_unix_socket - create, bind, and listen on a Unix domain socket.
 * Returns socket fd on success, -1 on error.
 */
int create_unix_socket(const char *path);

/* ---- Timing helper ------------------------------------------------------ */

/** Returns current CLOCK_MONOTONIC time in milliseconds. */
uint64_t diag_monotonic_ms(void);

/* ---- Request-parsing and response helpers (inline) ---- */

/**
 * req_read_u32 - Read a little-endian u32 from a request buffer.
 *
 * @param req    Request payload buffer.
 * @param offset Byte offset into @p req.
 * @return       The u32 value at @p offset.
 */
static inline uint32_t req_read_u32(const uint8_t *req, uint32_t offset)
{
    uint32_t v;
    memcpy(&v, req + offset, sizeof(v));
    return v;
}

/**
 * req_read_i32 - Read a little-endian i32 from a request buffer.
 *
 * @param req    Request payload buffer.
 * @param offset Byte offset into @p req.
 * @return       The i32 value at @p offset.
 */
static inline int32_t req_read_i32(const uint8_t *req, uint32_t offset)
{
    int32_t v;
    memcpy(&v, req + offset, sizeof(v));
    return v;
}

/**
 * req_read_handle - Decode a 64-bit opaque SDK handle from a request buffer.
 *
 * Reads a u64 at @p offset and converts it to a void pointer suitable for
 * passing to Anyka SDK functions.
 *
 * @param req    Request payload buffer.
 * @param offset Byte offset into @p req where the u64 handle is stored.
 * @return       The SDK handle as a void pointer.
 */
static inline void *req_read_handle(const uint8_t *req, uint32_t offset)
{
    uint64_t h;
    memcpy(&h, req + offset, sizeof(h));
    return (void *)(uintptr_t)h;
}

/**
 * send_handle_response - Send a STATUS_OK response carrying a single SDK handle.
 *
 * Converts @p handle to a signed i64 and sends it as the response payload,
 * which is the standard way handles are returned to the Rust caller.
 *
 * @param fd     Client socket file descriptor.
 * @param handle SDK handle returned by an open/request call.
 * @return       0 on success, -1 on I/O error.
 */
static inline int send_handle_response(int fd, void *handle)
{
    int64_t h64 = (int64_t)(uintptr_t)handle;
    return send_response(fd, STATUS_OK, &h64, sizeof(h64));
}

#endif /* VENDOR_DAEMON_IPC_H */
