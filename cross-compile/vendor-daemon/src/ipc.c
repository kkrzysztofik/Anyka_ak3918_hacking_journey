#include <errno.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <time.h>

#include "ipc.h"
#include "globals.h"
#include "log.h"
#include "vd_ring_buffer.h"

/**
 * read_exact - Read exactly n bytes from a file descriptor.
 *
 * Loops on EINTR and partial reads until n bytes are consumed or an
 * unrecoverable error (EOF or real I/O error) is encountered.
 *
 * @param fd   File descriptor to read from.
 * @param buf  Destination buffer; must be at least n bytes.
 * @param n    Number of bytes to read.
 * @return     0 on success, -1 on EOF or I/O error.
 */
int read_exact(int fd, void *buf, size_t n)
{
    size_t done = 0;
    while (done < n) {
        ssize_t r = read(fd, (char *)buf + done, n - done);
        if (r <= 0) {
            if (r < 0 && errno == EINTR)
                continue;
            return -1;
        }
        done += (size_t)r;
    }
    return 0;
}

/**
 * write_exact - Write exactly n bytes to a file descriptor.
 *
 * Loops on EINTR and partial writes until n bytes are written or an
 * unrecoverable error is encountered.
 *
 * @param fd   File descriptor to write to.
 * @param buf  Source buffer; must be at least n bytes.
 * @param n    Number of bytes to write.
 * @return     0 on success, -1 on I/O error.
 */
int write_exact(int fd, const void *buf, size_t n)
{
    size_t done = 0;
    while (done < n) {
        ssize_t w = write(fd, (const char *)buf + done, n - done);
        if (w <= 0) {
            if (w < 0 && errno == EINTR)
                continue;
            return -1;
        }
        done += (size_t)w;
    }
    return 0;
}

/**
 * send_response - Serialise and send an IPC response frame.
 *
 * Wire format: [i32 status : 4 bytes][u32 len : 4 bytes][data : len bytes]
 * All fields are little-endian on the wire (native on this ARM target).
 *
 * @param fd     Client socket file descriptor.
 * @param status Status code (STATUS_OK or STATUS_ERROR).
 * @param data   Optional response payload; may be NULL when len is 0.
 * @param len    Length of @p data in bytes.
 * @return       0 on success, -1 on I/O error.
 */
int send_response(int fd, int32_t status, const void *data, uint32_t len)
{
    if (write_exact(fd, &status, sizeof(status)) != 0) return -1;
    if (write_exact(fd, &len,    sizeof(len))    != 0) return -1;
    if (len > 0 && data != NULL)
        if (write_exact(fd, data, len) != 0)           return -1;
    return 0;
}

/**
 * frame_client_lock_for_stream - Return the mutex guarding the frame client fd for a stream.
 *
 * Maps stream_id to the appropriate global mutex so callers can lock
 * before reading or modifying the corresponding client fd.
 *
 * @param stream_id  Stream identifier (VD_STREAM_MAIN or VD_STREAM_SUB).
 * @return           Pointer to the mutex for the given stream.
 */
pthread_mutex_t *frame_client_lock_for_stream(uint32_t stream_id)
{
    if (stream_id == VD_STREAM_SUB) {
        return &g_frame_sub_client_lock;
    }
    return &g_frame_main_client_lock;
}

/**
 * frame_client_fd_for_stream - Return a pointer to the frame client fd storage for a stream.
 *
 * Maps stream_id to the appropriate global integer holding the connected
 * frame-notification client file descriptor (-1 when no client is connected).
 * Callers must hold the corresponding lock from frame_client_lock_for_stream()
 * before dereferencing the returned pointer.
 *
 * @param stream_id  Stream identifier (VD_STREAM_MAIN or VD_STREAM_SUB).
 * @return           Pointer to the client fd integer for the given stream.
 */
int *frame_client_fd_for_stream(uint32_t stream_id)
{
    if (stream_id == VD_STREAM_SUB) {
        return &g_frame_sub_client_fd;
    }
    return &g_frame_main_client_fd;
}

/**
 * diag_monotonic_ms - Return the current CLOCK_MONOTONIC time in milliseconds.
 *
 * Used for diagnostic log fields and inter-frame timing calculations.
 * The value is not wall time; it is only meaningful for duration arithmetic.
 *
 * @return Current monotonic clock value in milliseconds as a uint64_t.
 */
uint64_t diag_monotonic_ms(void)
{
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return ((uint64_t)now.tv_sec * 1000ULL) + ((uint64_t)now.tv_nsec / 1000000ULL);
}

static void log_push_notify_send_debug(
    uint32_t stream_id,
    int client_fd,
    const struct vd_frame_notify *notif,
    const char *status
)
{
    log_debug(
        "event=push_notify_send stream=%u fd=%d slot=%u frame_len=%u flags=0x%x notif_stream=%u notif_seq_no=%u status=%s diag_monotonic_ms=%llu",
        stream_id,
        client_fd,
        notif->slot_index,
        notif->frame_len,
        notif->flags,
        notif->stream_id,
        notif->seq_no,
        status,
        (unsigned long long)diag_monotonic_ms()
    );
}

/**
 * send_frame_notification - Send a 20-byte frame notification to the frame client.
 *
 * Locks the per-stream mutex, checks whether a frame client is connected,
 * and writes the vd_frame_notify struct on the dedicated frame socket.
 * If no client is connected the notification is silently discarded.
 *
 * @param stream_id  Stream identifier (VD_STREAM_MAIN or VD_STREAM_SUB).
 * @param notif      Pointer to the notification to send.
 * @return           0 on success or when no client is connected, -1 on write error.
 */
int send_frame_notification(uint32_t stream_id, const struct vd_frame_notify *notif)
{
    pthread_mutex_t *lock = frame_client_lock_for_stream(stream_id);
    int *client_fd_ptr = frame_client_fd_for_stream(stream_id);
    int ret = 0;
    pthread_mutex_lock(lock);
    int client_fd = *client_fd_ptr;
    if (client_fd >= 0) {
        if (write_exact(client_fd, notif, sizeof(*notif)) != 0) {
            ret = -1;
            log_warn(
                "event=push_notify_send stream=%u fd=%d slot=%u frame_len=%u flags=0x%x notif_stream=%u notif_seq_no=%u status=error errno=%d diag_monotonic_ms=%llu",
                stream_id,
                client_fd,
                notif->slot_index,
                notif->frame_len,
                notif->flags,
                notif->stream_id,
                notif->seq_no,
                errno,
                (unsigned long long)diag_monotonic_ms()
            );
        } else {
            log_push_notify_send_debug(stream_id, client_fd, notif, "ok");
        }
    } else {
        log_push_notify_send_debug(stream_id, client_fd, notif, "no_client");
    }
    pthread_mutex_unlock(lock);
    return ret;
}

/**
 * create_unix_socket - Create, bind, and listen on a Unix domain socket.
 *
 * Removes any stale socket file at @p path before binding so the bind
 * always succeeds even after an unclean shutdown.
 *
 * @param path  Filesystem path for the Unix domain socket.
 * @return      Listening socket file descriptor on success, -1 on error.
 */
int create_unix_socket(const char *path)
{
    int fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (fd < 0) {
        log_error("[daemon] socket: %s", strerror(errno));
        return -1;
    }

    unlink(path);

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, path, sizeof(addr.sun_path) - 1);

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        log_error("[daemon] bind %s: %s", path, strerror(errno));
        close(fd);
        return -1;
    }

    if (listen(fd, 4) < 0) {
        log_error("[daemon] listen %s: %s", path, strerror(errno));
        close(fd);
        unlink(path);
        return -1;
    }

    return fd;
}
