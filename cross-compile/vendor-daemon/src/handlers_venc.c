#include <stdint.h>
#include <string.h>

#include "handlers_venc.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "globals.h"
#include "ak_venc.h"

/* CMD_VENC_SET_CFG_PATH (no-op). */
int handle_venc_set_cfg_path(int fd, const uint8_t *req, uint32_t req_len)
{
    (void)req;
    (void)req_len;
    log_debug("[venc] set_cfg_path: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/* CMD_VENC_OPEN — resp [u64 token]. Wire: encode_param 48 bytes LE. */
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

    /*
     * Scene check defaults to ON (ak_venc.h: "the default switch flag is open")
     * and overrides goplen when it decides the scene is motionless: the encoder
     * then emits one IDR at open and never another. A recorder cuts segments on
     * keyframes, so that camera records ~15% of wall clock -- one segment per
     * client reconnect. Deterministic GOPs are worth more here than the bitrate
     * saving. Must be called after open and before the stream is requested.
     */
    if (ak_venc_set_check_scene(handle, 0) != 0)
        log_warn("[venc] set_check_scene(0) failed; goplen may be ignored on a static scene");

    /*
     * The goplen passed to ak_venc_open is not honoured on the main channel:
     * that stream emits one IDR at open and never another, so a recorder can
     * never cut a segment. Read back what the encoder actually took, then set
     * it explicitly through the runtime setter and read it back again. The log
     * line is the evidence for which of the two paths the SDK respects.
     */
    /*
     * Cap keyframe size at the encoder. A 720p keyframe measured ~184 KB and
     * grows with scene detail; anything over VD_SHM_SLOT_DATA_SIZE is rejected
     * outright by the ring write in push.c, which drops every keyframe and
     * leaves the stream undecodable for a late joiner. The slot was enlarged to
     * match reality, and this keeps the encoder from walking back up to it.
     */
    if (ak_venc_set_method(handle, METHOD_ISIZE_CTRL) != 0)
        log_warn("[venc] set_method(ISIZE_CTRL) failed; keyframes are not size-capped");

    int gop_at_open = ak_venc_get_gop_len(handle);
    int gop_set_rc  = ak_venc_set_gop_len(handle, (int)param.goplen);
    int gop_now     = ak_venc_get_gop_len(handle);
    log_warn("[venc] chn=%d goplen requested=%ld at_open=%d set_rc=%d now=%d",
             (int)param.use_chn, param.goplen, gop_at_open, gop_set_rc, gop_now);
    int slot = vd_obj_register(VD_OBJ_KIND_VENC, handle);
    if (slot < 0) {
        log_error("[venc] object table full; refusing open");
        ak_venc_close(handle);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_token_response(fd, vd_obj_token(slot));
}

/* CMD_VENC_CLOSE. Wire format: [u64 handle] = 8 bytes. */
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

/* CMD_VENC_SET_RC. Wire format: [u64 handle][i32 bps] = 12 bytes. */
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

/* CMD_VENC_SET_IFRAME. Wire format: [u64 handle] = 8 bytes. */
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

/* CMD_VENC_REQUEST_STREAM. Wire format: [u64 vi_handle][u64 venc_handle] = 16 bytes. */
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
        log_error("[venc] object table full; refusing request_stream");
        /* Transfer ownership before cancel so malloc/pthread/timeout failure
         * still leaves a reclaim path: a newer orphan displaces this one and
         * cancels it (see vd_stream_orphan_set). */
        vd_stream_orphan_set(stream_handle);
        if (vd_cancel_stream_bounded(stream_handle, NULL) == 0)
            vd_stream_orphan_clear(stream_handle);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_token_response(fd, vd_obj_token(slot));
}

/* CMD_VENC_CANCEL_STREAM. Wire format: [u64 stream_handle] = 8 bytes. */
int handle_venc_cancel_stream(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);

    void *stream_handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_STREAM, &stream_handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    log_debug("[venc] cancel_stream handle=%p", stream_handle);

    /* Retire the token before the cancel runs, not after: the cancel can time
     * out and leave the detached thread still working on the object, and a
     * client retry must not be able to resolve it again in the meantime. */
    vd_obj_unregister(VD_OBJ_KIND_STREAM, stream_handle);

    int sdk_ret = -1;
    int rc = vd_cancel_stream_bounded(stream_handle, &sdk_ret);
    if (rc == -1) {
        /* Worker never started: restore registry reachability so a retry or
         * close_all can still find the live SDK stream. */
        if (vd_obj_register(VD_OBJ_KIND_STREAM, stream_handle) < 0) {
            log_error("[venc] cancel_stream: pthread/malloc failed and re-register "
                      "failed; parking %p as orphan", stream_handle);
            vd_stream_orphan_set(stream_handle);
        }
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (rc == -2) {
        /* Detached worker still owns the cancel call; park for close_all in
         * case the worker never finishes. */
        vd_stream_orphan_set(stream_handle);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    return send_response(fd, sdk_ret, NULL, 0);
}
