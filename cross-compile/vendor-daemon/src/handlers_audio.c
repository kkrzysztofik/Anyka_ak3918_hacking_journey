#include <string.h>
#include <stdint.h>

#include "handlers_audio.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "globals.h"
#include "ak_ai.h"
#include "ak_aenc.h"

/**
 * handle_ai_open - IPC handler for CMD_AI_OPEN.
 *
 * Opens an audio input device and returns a 64-bit opaque handle.
 *
 * Wire format for pcm_param (12 bytes):
 *   sample_rate(u32) sample_bits(u32) channel_num(u32)
 * Response payload: [i64 handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
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

/**
 * handle_ai_close - IPC handler for CMD_AI_CLOSE.
 *
 * Closes the audio input device identified by the 64-bit handle.
 *
 * Wire format: [u64 handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_ai_close(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_AI, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    int ret = ak_ai_close(handle);
    vd_obj_unregister(VD_OBJ_KIND_AI, handle);
    return send_response(fd, ret, NULL, 0);
}

/**
 * handle_ai_set_adc_volume - IPC handler for CMD_AI_SET_ADC_VOLUME (no-op).
 *
 * ak_ai_set_adc_volume() is not exported by the libre_anyka_app SDK variant; this
 * function always responds STATUS_OK without calling any SDK function.
 * Audio ADC volume control is not available in this SDK variant.
 *
 * @param fd      Client socket file descriptor.
 * @param req     Ignored.
 * @param req_len Ignored.
 * @return        0 on success, -1 on write error.
 */
int handle_ai_set_adc_volume(int fd, const uint8_t *req, uint32_t req_len)
{
    (void)req;
    (void)req_len;
    log_debug("[ai] set_adc_volume: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_ai_set_aslc_volume - IPC handler for CMD_AI_SET_ASLC_VOLUME (no-op).
 *
 * ak_ai_set_aslc_volume() is not exported by the libre_anyka_app SDK variant; this
 * function always responds STATUS_OK without calling any SDK function.
 * Audio ASLC volume control is not available in this SDK variant.
 *
 * @param fd      Client socket file descriptor.
 * @param req     Ignored.
 * @param req_len Ignored.
 * @return        0 on success, -1 on write error.
 */
int handle_ai_set_aslc_volume(int fd, const uint8_t *req, uint32_t req_len)
{
    (void)req;
    (void)req_len;
    log_debug("[ai] set_aslc_volume: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_aenc_open - IPC handler for CMD_AENC_OPEN.
 *
 * Opens an audio encoder instance and returns a 64-bit opaque handle.
 *
 * Wire format for audio_param (16 bytes), as specified in the IPC protocol:
 *   sample_rate(u32) channel_num(u32) sample_bits(u32) type(i32)
 *
 * Note: the SDK struct audio_param has fields in a different order
 * (type, sample_rate, sample_bits, channel_num), so we deserialise
 * explicitly from the wire layout rather than casting the buffer directly.
 * Response payload: [i64 handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
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

/**
 * handle_aenc_close - IPC handler for CMD_AENC_CLOSE.
 *
 * Closes the audio encoder identified by the 64-bit handle.
 *
 * Wire format: [u64 handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_aenc_close(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_AENC, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    int ret = ak_aenc_close(handle);
    vd_obj_unregister(VD_OBJ_KIND_AENC, handle);
    return send_response(fd, ret, NULL, 0);
}

/**
 * handle_aenc_set_attr - IPC handler for CMD_AENC_SET_ATTR.
 *
 * Sets the AAC header mode attribute on an open audio encoder.
 *
 * Wire format: [u64 handle][i32 aac_head] = 12 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_aenc_set_attr(int fd, const uint8_t *req, uint32_t req_len)
{
    /* u64 handle + i32 aac_head = 12 bytes */
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
