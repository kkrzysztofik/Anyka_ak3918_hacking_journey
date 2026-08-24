#include <string.h>
#include <stdint.h>

#include "handlers_vi.h"
#include "handlers_osd.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "globals.h"
#include "ak_vi.h"

/**
 * handle_vi_match_sensor - IPC handler for CMD_VI_MATCH_SENSOR.
 *
 * Calls ak_vi_match_sensor() with the NUL-terminated sensor library path
 * provided in the request payload.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes: NUL-terminated sensor library path.
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_vi_match_sensor(int fd, const uint8_t *req, uint32_t req_len)
{
    char path[512];
    uint32_t copy = req_len < sizeof(path) - 1 ? req_len : sizeof(path) - 1;
    memcpy(path, req, copy);
    path[copy] = '\0';

    log_debug("[vi] match_sensor path=%s", path);
    int ret = ak_vi_match_sensor(path);
    if (ret != 0)
        log_error("[vi] match_sensor failed: %d", ret);
    return send_response(fd, ret, NULL, 0);
}

/**
 * handle_vi_open - IPC handler for CMD_VI_OPEN.
 *
 * Opens a VI device and returns a 64-bit opaque handle.
 *
 * Wire format: [i32 video_dev_type] = 4 bytes.
 * Response payload: [i64 handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_vi_open(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(int32_t)) {
        log_warn("[vi] open: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    int32_t dev_type_i32;
    memcpy(&dev_type_i32, req, sizeof(dev_type_i32));
    enum video_dev_type dev = (enum video_dev_type)dev_type_i32;

    log_debug("[vi] open dev=%d", dev_type_i32);
    void *handle = ak_vi_open(dev);
    if (handle == NULL) {
        log_error("[vi] open failed (NULL handle)");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    int slot = vd_obj_register(VD_OBJ_KIND_VI, handle);
    if (slot < 0) {
        log_error("[vi] object table full; refusing open");
        ak_vi_close(handle);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_token_response(fd, vd_obj_token(slot));
}

/**
 * handle_vi_close - IPC handler for CMD_VI_CLOSE.
 *
 * Closes the VI device identified by the 64-bit handle in the request.
 *
 * Wire format: [u64 handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_vi_close(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t)) {
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);

    log_debug("[vi] close handle=%p", handle);
    /* OSD buffers are bound to the VI handle — destroy before close. */
    osd_shutdown();
    int ret = ak_vi_close(handle);
    if (ret == 0)
        vd_obj_unregister(VD_OBJ_KIND_VI, handle);
    else
        log_warn("[vi] close failed ret=%d; keeping object tracked for reclaim", ret);
    return send_response(fd, ret, NULL, 0);
}

/**
 * handle_vi_get_sensor_resolution - IPC handler for CMD_VI_GET_SENSOR_RESOLUTION.
 *
 * Queries the sensor's current and maximum resolution and returns them as
 * four i32 values.
 *
 * Wire format: [u64 handle] = 8 bytes.
 * Response payload: [i32 width][i32 height][i32 max_width][i32 max_height] = 16 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_vi_get_sensor_resolution(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t)) {
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);

    struct video_resolution res;
    memset(&res, 0, sizeof(res));
    int ret = ak_vi_get_sensor_resolution(handle, &res);
    if (ret != 0) {
        log_error("[vi] get_sensor_resolution failed: %d", ret);
        return send_response(fd, ret, NULL, 0);
    }

    /* Response: 4 x i32 = width, height, max_width, max_height */
    int32_t resp[4];
    resp[0] = (int32_t)res.width;
    resp[1] = (int32_t)res.height;
    resp[2] = (int32_t)res.max_width;
    resp[3] = (int32_t)res.max_height;
    return send_response(fd, STATUS_OK, resp, sizeof(resp));
}

/**
 * handle_vi_set_channel_attr - IPC handler for CMD_VI_SET_CHANNEL_ATTR.
 *
 * Sets the crop region and per-channel output resolutions on the VI device.
 *
 * Wire format for video_channel_attr (packed, LE):
 *   [u64 handle]
 *   crop:  left(i32) top(i32) width(i32) height(i32)
 *   res[0]: width(i32) height(i32) max_width(i32) max_height(i32)
 *   res[1]: width(i32) height(i32) max_width(i32) max_height(i32)
 *   Total = 8 + 12 i32s = 56 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_vi_set_channel_attr(int fd, const uint8_t *req, uint32_t req_len)
{
    /* u64 handle + 12 i32s = 8 + 48 = 56 bytes */
    if (req_len < 8 + 48) {
        log_warn("[vi] set_channel_attr: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);

    struct video_channel_attr attr;
    memset(&attr, 0, sizeof(attr));

    attr.crop.left         = (int)req_read_i32(req,  8);
    attr.crop.top          = (int)req_read_i32(req, 12);
    attr.crop.width        = (int)req_read_i32(req, 16);
    attr.crop.height       = (int)req_read_i32(req, 20);
    attr.res[0].width      = (int)req_read_i32(req, 24);
    attr.res[0].height     = (int)req_read_i32(req, 28);
    attr.res[0].max_width  = (int)req_read_i32(req, 32);
    attr.res[0].max_height = (int)req_read_i32(req, 36);
    attr.res[1].width      = (int)req_read_i32(req, 40);
    attr.res[1].height     = (int)req_read_i32(req, 44);
    attr.res[1].max_width  = (int)req_read_i32(req, 48);
    attr.res[1].max_height = (int)req_read_i32(req, 52);

    log_debug(
        "[vi] set_channel_attr decoded: crop=%dx%d main=%dx%d main_max=%dx%d sub=%dx%d sub_max=%dx%d",
        attr.crop.width,
        attr.crop.height,
        attr.res[VIDEO_CHN_MAIN].width,
        attr.res[VIDEO_CHN_MAIN].height,
        attr.res[VIDEO_CHN_MAIN].max_width,
        attr.res[VIDEO_CHN_MAIN].max_height,
        attr.res[VIDEO_CHN_SUB].width,
        attr.res[VIDEO_CHN_SUB].height,
        attr.res[VIDEO_CHN_SUB].max_width,
        attr.res[VIDEO_CHN_SUB].max_height
    );

    int ret = ak_vi_set_channel_attr(handle, &attr);
    if (ret != 0) {
        log_error(
            "[vi] set_channel_attr failed: ret=%d main=%dx%d main_max=%dx%d sub=%dx%d sub_max=%dx%d",
            ret,
            attr.res[VIDEO_CHN_MAIN].width,
            attr.res[VIDEO_CHN_MAIN].height,
            attr.res[VIDEO_CHN_MAIN].max_width,
            attr.res[VIDEO_CHN_MAIN].max_height,
            attr.res[VIDEO_CHN_SUB].width,
            attr.res[VIDEO_CHN_SUB].height,
            attr.res[VIDEO_CHN_SUB].max_width,
            attr.res[VIDEO_CHN_SUB].max_height
        );
    }
    return send_response(fd, ret, NULL, 0);
}

/**
 * handle_vi_capture_on - IPC handler for CMD_VI_CAPTURE_ON.
 *
 * Starts frame capture on the VI device identified by the handle.
 *
 * Wire format: [u64 handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_vi_capture_on(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    int ret = ak_vi_capture_on(handle);
    return send_response(fd, ret, NULL, 0);
}

/**
 * handle_vi_capture_off - IPC handler for CMD_VI_CAPTURE_OFF.
 *
 * Stops frame capture on the VI device identified by the handle.
 *
 * Wire format: [u64 handle] = 8 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_vi_capture_off(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);
    int ret = ak_vi_capture_off(handle);
    return send_response(fd, ret, NULL, 0);
}

/**
 * handle_vi_set_flip_mirror - IPC handler for CMD_VI_SET_FLIP_MIRROR.
 *
 * Calls ak_vi_set_flip_mirror() on the VI device identified by the handle.
 *
 * Wire format: [u64 handle][u8 flip][u8 mirror] = 10 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_vi_set_flip_mirror(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t) + 2) {
        log_warn("[vi] set_flip_mirror: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);

    int flip = req[8];
    int mirror = req[9];

    log_debug("[vi] set_flip_mirror handle=%p flip=%d mirror=%d", handle, flip, mirror);
    int ret = ak_vi_set_flip_mirror(handle, flip, mirror);
    if (ret != 0)
        log_error("[vi] set_flip_mirror failed: %d", ret);
    return send_response(fd, ret, NULL, 0);
}
