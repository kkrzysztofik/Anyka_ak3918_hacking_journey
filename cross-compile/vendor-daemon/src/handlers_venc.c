#include <string.h>
#include <stdint.h>
#include <stdlib.h>
#include <time.h>
#include <pthread.h>

#include "handlers_venc.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "globals.h"
#include "ak_venc.h"

/* Timeout in seconds for ak_venc_cancel_stream() before giving up.
 * If the SDK's internal capture thread is stuck, this prevents the
 * event loop from blocking forever. */
#define CANCEL_STREAM_TIMEOUT_SEC 3

/**
 * handle_venc_set_cfg_path - IPC handler for CMD_VENC_SET_CFG_PATH (no-op).
 *
 * ak_venc_set_cfg_path() is not exported by the libre_anyka_app SDK variant; this
 * function always responds STATUS_OK without calling any SDK function.
 * The V2 encoder in this SDK variant uses the default path (/etc/jffs2/venc.cfg)
 * and does not support overriding it at runtime.
 *
 * @param fd      Client socket file descriptor.
 * @param req     Ignored.
 * @param req_len Ignored.
 * @return        0 on success, -1 on write error.
 */
int handle_venc_set_cfg_path(int fd, const uint8_t *req, uint32_t req_len)
{
    (void)req;
    (void)req_len;
    log_debug("[venc] set_cfg_path: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_venc_open - IPC handler for CMD_VENC_OPEN.
 *
 * Opens a video encoder instance and returns a 64-bit opaque handle.
 *
 * Wire format for encode_param (48 bytes, all i32/u32 LE):
 *   width(u32) height(u32) minqp(i32) maxqp(i32) fps(i32) goplen(i32)
 *   bps(i32) profile(i32) use_chn(i32) enc_grp(i32) br_mode(i32) enc_out_type(i32)
 * Response payload: [i64 handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_venc_open(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < 48) {
        log_warn("[venc] open: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    struct encode_param param;
    memset(&param, 0, sizeof(param));

    uint32_t u32tmp;
    int32_t  i32tmp;

    memcpy(&u32tmp, req +  0, 4); param.width        = (unsigned long)u32tmp;
    memcpy(&u32tmp, req +  4, 4); param.height       = (unsigned long)u32tmp;
    memcpy(&i32tmp, req +  8, 4); param.minqp        = (signed long)i32tmp;
    memcpy(&i32tmp, req + 12, 4); param.maxqp        = (signed long)i32tmp;
    memcpy(&i32tmp, req + 16, 4); param.fps          = (signed long)i32tmp;
    memcpy(&i32tmp, req + 20, 4); param.goplen       = (signed long)i32tmp;
    memcpy(&i32tmp, req + 24, 4); param.bps          = (signed long)i32tmp;
    memcpy(&i32tmp, req + 28, 4); param.profile      = (enum profile_mode)i32tmp;
    memcpy(&i32tmp, req + 32, 4); param.use_chn      = (enum encode_use_chn)i32tmp;
    memcpy(&i32tmp, req + 36, 4); param.enc_grp      = (enum encode_group_type)i32tmp;
    memcpy(&i32tmp, req + 40, 4); param.br_mode      = (enum bitrate_ctrl_mode)i32tmp;
    memcpy(&i32tmp, req + 44, 4); param.enc_out_type = (enum encode_output_type)i32tmp;

    log_debug("[venc] open %lux%lu fps=%ld bps=%ld type=%d",
            param.width, param.height, param.fps, param.bps, param.enc_out_type);

    void *handle = ak_venc_open(&param);
    if (handle == NULL) {
        log_error("[venc] open failed (NULL handle)");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    int slot = vd_obj_register(VD_OBJ_KIND_VENC, handle);
    if (slot < 0) {
        log_error("[venc] object table full; refusing open");
        ak_venc_close(handle);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_token_response(fd, vd_obj_token(slot));
}

/**
 * handle_venc_close - IPC handler for CMD_VENC_CLOSE.
 *
 * Closes the video encoder identified by the 64-bit handle in the request.
 *
 * Wire format: [u64 handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_venc_close(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VENC, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    log_debug("[venc] close handle=%p", handle);
    int ret = ak_venc_close(handle);
    if (ret == 0)
        vd_obj_unregister(VD_OBJ_KIND_VENC, handle);
    else
        log_warn("[venc] close failed ret=%d; keeping object tracked for reclaim", ret);
    return send_response(fd, ret, NULL, 0);
}

/**
 * handle_venc_set_rc - IPC handler for CMD_VENC_SET_RC.
 *
 * Sets the bitrate (rate control) parameter on a running encoder.
 *
 * Wire format: [u64 handle][i32 bps] = 12 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_venc_set_rc(int fd, const uint8_t *req, uint32_t req_len)
{
    /* u64 handle + i32 bps = 12 bytes */
    if (req_len < 8 + 4)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VENC, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    int32_t bps  = req_read_i32(req, 8);
    int ret = ak_venc_set_rc(handle, (int)bps);
    return send_response(fd, ret, NULL, 0);
}

/**
 * handle_venc_set_iframe - IPC handler for CMD_VENC_SET_IFRAME.
 *
 * Forces the next encoded frame on the given encoder to be an intra (I) frame.
 *
 * Wire format: [u64 handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_venc_set_iframe(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VENC, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    int ret = ak_venc_set_iframe(handle);
    return send_response(fd, ret, NULL, 0);
}

/**
 * handle_venc_request_stream - IPC handler for CMD_VENC_REQUEST_STREAM.
 *
 * Binds a VI input to a VENC encoder and opens a stream handle used for
 * subsequent push/cancel calls.
 *
 * Wire format: [u64 vi_handle][u64 venc_handle] = 16 bytes.
 * Response payload: [i64 stream_handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
static void cancel_untracked_stream(void *handle);

int handle_venc_request_stream(int fd, const uint8_t *req, uint32_t req_len)
{
    /* u64 vi_handle + u64 venc_handle = 16 bytes */
    if (req_len < 16)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *vi_handle;
    void *venc_handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &vi_handle) != 0 ||
        vd_obj_resolve(req_read_u64(req, 8), VD_OBJ_KIND_VENC, &venc_handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);

    log_debug("[venc] request_stream vi=%p venc=%p", vi_handle, venc_handle);
    void *stream_handle = ak_venc_request_stream(vi_handle, venc_handle);
    if (stream_handle == NULL) {
        log_error("[venc] request_stream failed (NULL)");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    int slot = vd_obj_register(VD_OBJ_KIND_STREAM, stream_handle);
    if (slot < 0) {
        log_error("[venc] object table full; refusing open");
        cancel_untracked_stream(stream_handle);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_token_response(fd, vd_obj_token(slot));
}

/* ---- Timeout-guarded cancel_stream helpers ----------------------------- */

struct cancel_arg {
    void *handle;
    int   result;
    volatile int done;   /* set to 1 by worker thread when cancel returns */
};

/**
 * cancel_thread_fn - Worker thread that calls ak_venc_cancel_stream().
 *
 * Runs detached.  Sets ca->done=1 on completion.  Does NOT free ca —
 * ownership rules:
 *   - If the caller sees done=1 within the timeout: caller frees.
 *   - If the caller times out: 16-byte leak (acceptable for rare error path).
 *
 * We cannot have the thread free ca because there is an unavoidable race
 * between the thread's free() and the caller's read of ca->result.
 */
static void *cancel_thread_fn(void *arg)
{
    struct cancel_arg *ca = (struct cancel_arg *)arg;
    ca->result = ak_venc_cancel_stream(ca->handle);
    __atomic_store_n(&ca->done, 1, __ATOMIC_RELEASE);
    return NULL;
}

/**
 * handle_venc_cancel_stream - IPC handler for CMD_VENC_CANCEL_STREAM.
 *
 * Cancels an active stream binding created by CMD_VENC_REQUEST_STREAM.
 * Uses a detached helper thread with a timeout to prevent blocking the
 * event loop if the SDK's cancel call hangs.
 *
 * Wire format: [u64 stream_handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
/*
 * cancel_untracked_stream - Bounded cancel for a stream that was never registered.
 *
 * Used when ak_venc_request_stream succeeded but vd_obj_register failed: the
 * SDK capture_thread must still be stopped. No vd_obj_unregister — the pointer
 * never entered the table.
 */
static void cancel_untracked_stream(void *handle)
{
    struct cancel_arg *ca = (struct cancel_arg *)malloc(sizeof(*ca));
    if (ca == NULL) {
        log_error("[venc] cancel_untracked: malloc failed ptr=%p", handle);
        return;
    }
    ca->handle = handle;
    ca->result = -1;
    ca->done = 0;

    pthread_t tid;
    if (pthread_create(&tid, NULL, cancel_thread_fn, ca) != 0) {
        log_error("[venc] cancel_untracked: pthread_create failed ptr=%p", handle);
        free(ca);
        return;
    }
    pthread_detach(tid);

    struct timespec deadline;
    clock_gettime(CLOCK_MONOTONIC, &deadline);
    deadline.tv_sec += CANCEL_STREAM_TIMEOUT_SEC;

    while (!__atomic_load_n(&ca->done, __ATOMIC_ACQUIRE)) {
        struct timespec now;
        clock_gettime(CLOCK_MONOTONIC, &now);
        if (now.tv_sec > deadline.tv_sec ||
            (now.tv_sec == deadline.tv_sec && now.tv_nsec >= deadline.tv_nsec)) {
            log_error("[venc] cancel_untracked timed out after %ds ptr=%p (leaking arg)",
                      CANCEL_STREAM_TIMEOUT_SEC, handle);
            return;
        }
        struct timespec ts = { .tv_sec = 0, .tv_nsec = 10 * 1000000L };
        nanosleep(&ts, NULL);
    }

    log_info("[venc] cancelled untracked stream ptr=%p ret=%d", handle, ca->result);
    free(ca);
}

int handle_venc_cancel_stream(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);

    void *stream_handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_STREAM, &stream_handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    log_debug("[venc] cancel_stream handle=%p", stream_handle);

    struct cancel_arg *ca = (struct cancel_arg *)malloc(sizeof(*ca));
    if (!ca) {
        log_error("[venc] cancel_stream: malloc failed");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    ca->handle = stream_handle;
    ca->result = -1;
    ca->done = 0;

    /* Retire the token before the cancel runs, not after: the cancel can time
     * out and leave the detached thread still working on the object, and a
     * client retry must not be able to resolve it again in the meantime. */
    vd_obj_unregister(VD_OBJ_KIND_STREAM, stream_handle);

    pthread_t tid;
    if (pthread_create(&tid, NULL, cancel_thread_fn, ca) != 0) {
        log_error("[venc] failed to spawn cancel thread");
        free(ca);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    pthread_detach(tid);

    struct timespec deadline;
    clock_gettime(CLOCK_MONOTONIC, &deadline);
    deadline.tv_sec += CANCEL_STREAM_TIMEOUT_SEC;

    while (!__atomic_load_n(&ca->done, __ATOMIC_ACQUIRE)) {
        struct timespec now;
        clock_gettime(CLOCK_MONOTONIC, &now);
        if (now.tv_sec > deadline.tv_sec ||
            (now.tv_sec == deadline.tv_sec && now.tv_nsec >= deadline.tv_nsec)) {
            log_error("[venc] cancel_stream timed out after %ds, handle=%p",
                      CANCEL_STREAM_TIMEOUT_SEC, stream_handle);
            /* Intentional leak of ca (16 bytes).  The detached thread may
             * still be writing to it, so we cannot free here.  It will be
             * reclaimed when the daemon exits or restarts. */
            return send_response(fd, STATUS_ERROR, NULL, 0);
        }
        struct timespec ts = { .tv_sec = 0, .tv_nsec = 10 * 1000000L }; /* 10ms */
        nanosleep(&ts, NULL);
    }

    int ret = ca->result;
    free(ca);
    return send_response(fd, ret, NULL, 0);
}
