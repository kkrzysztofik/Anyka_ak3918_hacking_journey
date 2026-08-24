#include <string.h>
#include <stdint.h>

#include "handlers_audio.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "globals.h"
#include "ak_ai.h"
#include "ak_aenc.h"

/* CMD_AI_OPEN — resp [u64 token]. Wire: rate/bits/ch u32x3. */
int handle_ai_open(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < 12) {
        log_warn("[ai] open: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    struct pcm_param param;
    param.sample_rate = (unsigned int)req_read_u32(req, 0);
    param.sample_bits = (unsigned int)req_read_u32(req, 4);
    param.channel_num = (unsigned int)req_read_u32(req, 8);

    log_debug("[ai] open rate=%u bits=%u ch=%u",
            param.sample_rate, param.sample_bits, param.channel_num);
    void *handle = ak_ai_open(&param);
    if (handle == NULL) {
        log_error("[ai] open failed (NULL handle)");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    int slot = vd_obj_register(VD_OBJ_KIND_AI, handle);
    if (slot < 0) {
        log_error("[ai] object table full; refusing open");
        ak_ai_close(handle);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_token_response(fd, vd_obj_token(slot));
}

/* CMD_AI_CLOSE — wire: [u64 handle]. */
int handle_ai_close(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_AI, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    log_debug("[ai] close handle=%p", handle);
    int ret = ak_ai_close(handle);
    if (ret == 0)
        vd_obj_unregister(VD_OBJ_KIND_AI, handle);
    else
        log_warn("[ai] close failed ret=%d; keeping object tracked for reclaim", ret);
    return send_response(fd, ret, NULL, 0);
}

/* CMD_AI_SET_ADC_VOLUME — no-op (SDK has no export). */
int handle_ai_set_adc_volume(int fd, const uint8_t *req, uint32_t req_len)
{
    (void)req;
    (void)req_len;
    log_debug("[ai] set_adc_volume: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/* CMD_AI_SET_ASLC_VOLUME — no-op (SDK has no export). */
int handle_ai_set_aslc_volume(int fd, const uint8_t *req, uint32_t req_len)
{
    (void)req;
    (void)req_len;
    log_debug("[ai] set_aslc_volume: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/*
 * CMD_AENC_OPEN — resp [u64 token].
 * Wire: rate/ch/bits u32, type i32 (not SDK struct order).
 */
int handle_aenc_open(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < 16) {
        log_warn("[aenc] open: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    struct audio_param param;
    param.sample_rate = (unsigned int)req_read_u32(req,  0);
    param.channel_num = (unsigned int)req_read_u32(req,  4);
    param.sample_bits = (unsigned int)req_read_u32(req,  8);
    param.type        = (enum ak_audio_type)req_read_i32(req, 12);

    log_debug("[aenc] open type=%d rate=%u bits=%u ch=%u",
            (int)param.type, param.sample_rate, param.sample_bits, param.channel_num);
    void *handle = ak_aenc_open(&param);
    if (handle == NULL) {
        log_error("[aenc] open failed (NULL handle)");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    int slot = vd_obj_register(VD_OBJ_KIND_AENC, handle);
    if (slot < 0) {
        log_error("[aenc] object table full; refusing open");
        ak_aenc_close(handle);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_token_response(fd, vd_obj_token(slot));
}

/* CMD_AENC_CLOSE — wire: [u64 handle]. */
int handle_aenc_close(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_AENC, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    log_debug("[aenc] close handle=%p", handle);
    int ret = ak_aenc_close(handle);
    if (ret == 0)
        vd_obj_unregister(VD_OBJ_KIND_AENC, handle);
    else
        log_warn("[aenc] close failed ret=%d; keeping object tracked for reclaim", ret);
    return send_response(fd, ret, NULL, 0);
}

/* CMD_AENC_SET_ATTR — wire: [u64 handle][i32 aac_head]. */
int handle_aenc_set_attr(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < 8 + 4)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_AENC, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    int32_t aac_head_i32 = req_read_i32(req, 8);

    struct aenc_attr attr;
    attr.aac_head = (enum aenc_aac_attr)aac_head_i32;
    int ret = ak_aenc_set_attr(handle, &attr);
    return send_response(fd, ret, NULL, 0);
}
