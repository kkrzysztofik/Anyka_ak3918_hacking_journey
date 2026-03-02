#include <string.h>
#include <stdint.h>

#include "handlers_venc.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "ak_venc.h"

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
    return send_handle_response(fd, handle);
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
    void *handle = req_read_handle(req, 0);
    log_debug("[venc] close handle=%p", handle);
    int ret = ak_venc_close(handle);
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
    void *handle = req_read_handle(req, 0);
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
    void *handle = req_read_handle(req, 0);
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
int handle_venc_request_stream(int fd, const uint8_t *req, uint32_t req_len)
{
    /* u64 vi_handle + u64 venc_handle = 16 bytes */
    if (req_len < 16)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *vi_handle   = req_read_handle(req, 0);
    void *venc_handle = req_read_handle(req, 8);

    log_debug("[venc] request_stream vi=%p venc=%p", vi_handle, venc_handle);
    void *stream_handle = ak_venc_request_stream(vi_handle, venc_handle);
    if (stream_handle == NULL) {
        log_error("[venc] request_stream failed (NULL)");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_handle_response(fd, stream_handle);
}

/**
 * handle_venc_cancel_stream - IPC handler for CMD_VENC_CANCEL_STREAM.
 *
 * Cancels an active stream binding created by CMD_VENC_REQUEST_STREAM.
 *
 * Wire format: [u64 stream_handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_venc_cancel_stream(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *stream_handle = req_read_handle(req, 0);
    log_debug("[venc] cancel_stream handle=%p", stream_handle);
    int ret = ak_venc_cancel_stream(stream_handle);
    return send_response(fd, ret, NULL, 0);
}
