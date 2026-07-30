#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#include "dispatcher.h"
#include "globals.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "ak_error.h"
#include "ak_vpss.h"
#include "handlers_vi.h"
#include "handlers_vpss.h"
#include "handlers_venc.h"
#include "handlers_audio.h"
#include "handlers_isp.h"
#include "push.h"
#include "vd_ring_buffer.h"

/* ---- Internal helpers (file-static) ------------------------------------- */

/**
 * is_lifecycle_cmd - Check whether cmd_id is a hardware lifecycle command.
 *
 * Lifecycle commands alter hardware state (open/close VI, VPSS, VENC, audio)
 * and may only be issued by the control client.
 *
 * @param cmd  Command identifier to test.
 * @return     1 if cmd is a lifecycle command, 0 otherwise.
 */
static int is_lifecycle_cmd(int32_t cmd)
{
    switch ((enum cmd_id)cmd) {
    case CMD_VI_MATCH_SENSOR:
    case CMD_VI_OPEN:
    case CMD_VI_CLOSE:
    case CMD_VI_SET_CHANNEL_ATTR:
    case CMD_VI_CAPTURE_ON:
    case CMD_VI_CAPTURE_OFF:
    case CMD_VPSS_INIT:
    case CMD_VPSS_DESTROY:
    case CMD_VENC_SET_CFG_PATH:
    case CMD_VENC_OPEN:
    case CMD_VENC_CLOSE:
    case CMD_VENC_REQUEST_STREAM:
    case CMD_VENC_CANCEL_STREAM:
    case CMD_AI_OPEN:
    case CMD_AI_CLOSE:
    case CMD_AENC_OPEN:
    case CMD_AENC_CLOSE:
    case CMD_AENC_SET_ATTR:
    case CMD_SHUTDOWN:
        return 1;
    default:
        return 0;
    }
}

/**
 * acquire_control - Try to acquire control-client status for fd.
 *
 * If no control client is currently assigned, promotes fd to that role and
 * logs the promotion.  Idempotent if fd is already the control client.
 *
 * @param fd  Client file descriptor requesting control.
 * @return    1 if fd is (or just became) the control client, 0 otherwise.
 */
static int acquire_control(int fd)
{
    if (g_control_fd == fd)
        return 1;
    if (g_control_fd == -1) {
        g_control_fd = fd;
        log_info("[daemon] fd=%d promoted to control client", fd);
        return 1;
    }
    return 0;
}

/*
 * handle_hello - IPC handler for CMD_HELLO.
 *
 * Reports this daemon generation's epoch and the shm protocol version so the
 * client can pin them for the lifetime of its attachment.
 *
 * Deliberately requires no control-client status: a second client must be able
 * to say hello in order to discover that it lost the control race.
 *
 * Response: [u32 epoch][u32 shm_version]
 */
static int handle_hello(int fd)
{
    uint8_t resp[8];
    uint32_t epoch = 0;
    uint32_t version = VD_SHM_VERSION;

    if (g_ring_buffer != NULL) {
        epoch = vd_ring_get_header(g_ring_buffer)->epoch;
    }

    memcpy(&resp[0], &epoch, 4);
    memcpy(&resp[4], &version, 4);
    return send_response(fd, STATUS_OK, resp, sizeof(resp));
}

/**
 * handle_get_error_no - IPC handler for CMD_GET_ERROR_NO.
 *
 * Retrieves the current Anyka SDK error number via ak_get_error_no() and
 * returns it as an i32 payload.
 *
 * @param fd  Client socket file descriptor, used to send the response.
 * @return    0 on success, -1 on I/O error.
 */
static int handle_get_error_no(int fd)
{
    int32_t err = (int32_t)ak_get_error_no();
    return send_response(fd, STATUS_OK, &err, sizeof(err));
}

/**
 * handle_get_error_str - IPC handler for CMD_GET_ERROR_STR.
 *
 * Retrieves the current Anyka SDK error string via ak_get_error_str() and
 * returns it as a NUL-terminated string payload.
 *
 * @param fd  Client socket file descriptor, used to send the response.
 * @return    0 on success, -1 on I/O error.
 */
static int handle_get_error_str(int fd)
{
    int err_no = ak_get_error_no();
    char *msg = ak_get_error_str(err_no);
    if (msg == NULL)
        msg = "(null)";
    uint32_t slen = (uint32_t)(strlen(msg) + 1);
    return send_response(fd, STATUS_OK, msg, slen);
}

/* ---- Public interface ---------------------------------------------------- */

/**
 * release_control - Clear the control-client slot if fd holds it.
 *
 * Called on client disconnect to free the control role so the next
 * lifecycle command from any client can claim it.
 *
 * @param fd  File descriptor of the disconnecting client.
 */
void release_control(int fd)
{
    if (g_control_fd == fd) {
        log_info("[daemon] control client fd=%d released", fd);
        g_control_fd = -1;
    }
}

/**
 * process_request - Read one IPC frame, enforce access control, and dispatch.
 *
 * Reads the 8-byte header (i32 cmd_id + u32 req_len), then the payload,
 * enforces the control-client rule for lifecycle commands, and calls the
 * appropriate handler.  The payload is always consumed even on rejection to
 * keep the stream in sync.
 *
 * @param fd  Client socket file descriptor.
 * @return    0 on success, -1 on I/O error or disconnect, -2 on CMD_SHUTDOWN.
 */
int process_request(int fd)
{
    int32_t  cmd_id;
    uint32_t req_len;

    if (read_exact(fd, &cmd_id,  sizeof(cmd_id))  != 0) return -1;
    if (read_exact(fd, &req_len, sizeof(req_len)) != 0) return -1;

    if (req_len > MAX_REQUEST_SIZE) {
        log_error("[daemon] req_len %u exceeds max %u, disconnecting",
                req_len, (unsigned)MAX_REQUEST_SIZE);
        return -1;
    }

    log_debug("[daemon] fd=%d dispatch cmd=%d len=%u", fd, cmd_id, req_len);

    /* Read request payload into a heap buffer (may be up to 1 MB).
     * We must consume the payload even if we reject the command,
     * otherwise the stream gets out of sync. */
    uint8_t *req_buf = NULL;
    if (req_len > 0) {
        req_buf = (uint8_t *)malloc(req_len);
        if (req_buf == NULL) {
            log_error("[daemon] malloc(%u) failed", req_len);
            return -1;
        }
        if (read_exact(fd, req_buf, req_len) != 0) {
            free(req_buf);
            return -1;
        }
    }

    /* Guard: lifecycle commands require control-client status */
    if (is_lifecycle_cmd(cmd_id)) {
        if (!acquire_control(fd)) {
            log_warn("[daemon] fd=%d rejected lifecycle cmd=%d (control held by fd=%d)",
                     fd, cmd_id, g_control_fd);
            free(req_buf);
            return send_response(fd, STATUS_ERROR, NULL, 0);
        }
    }

    int ret = 0;

    switch ((enum cmd_id)cmd_id) {
    /* --- Video Input --- */
    case CMD_VI_MATCH_SENSOR:
        ret = handle_vi_match_sensor(fd, req_buf, req_len);
        break;
    case CMD_VI_OPEN:
        ret = handle_vi_open(fd, req_buf, req_len);
        break;
    case CMD_VI_CLOSE:
        ret = handle_vi_close(fd, req_buf, req_len);
        break;
    case CMD_VI_GET_SENSOR_RESOLUTION:
        ret = handle_vi_get_sensor_resolution(fd, req_buf, req_len);
        break;
    case CMD_VI_SET_CHANNEL_ATTR:
        ret = handle_vi_set_channel_attr(fd, req_buf, req_len);
        break;
    case CMD_VI_CAPTURE_ON:
        ret = handle_vi_capture_on(fd, req_buf, req_len);
        break;
    case CMD_VI_CAPTURE_OFF:
        ret = handle_vi_capture_off(fd, req_buf, req_len);
        break;

    /* --- VPSS --- */
    case CMD_VPSS_INIT:
        ret = handle_vpss_init(fd, req_buf, req_len);
        break;
    case CMD_VPSS_DESTROY:
        ret = handle_vpss_destroy(fd, req_buf, req_len);
        break;

    /* --- Video Encoder --- */
    case CMD_VENC_SET_CFG_PATH:
        ret = handle_venc_set_cfg_path(fd, req_buf, req_len);
        break;
    case CMD_VENC_OPEN:
        ret = handle_venc_open(fd, req_buf, req_len);
        break;
    case CMD_VENC_CLOSE:
        ret = handle_venc_close(fd, req_buf, req_len);
        break;
    case CMD_VENC_SET_RC:
        ret = handle_venc_set_rc(fd, req_buf, req_len);
        break;
    case CMD_VENC_SET_IFRAME:
        ret = handle_venc_set_iframe(fd, req_buf, req_len);
        break;
    case CMD_VENC_REQUEST_STREAM:
        ret = handle_venc_request_stream(fd, req_buf, req_len);
        break;
    case CMD_VENC_GET_STREAM:
    case CMD_VENC_RELEASE_STREAM:
        ret = send_response(fd, STATUS_ERROR, NULL, 0);
        break;
    case CMD_VENC_CANCEL_STREAM:
        ret = handle_venc_cancel_stream(fd, req_buf, req_len);
        break;
    case CMD_VENC_START_PUSH:
        ret = handle_venc_start_push(fd, req_buf, req_len);
        break;
    case CMD_VENC_STOP_PUSH:
        ret = handle_venc_stop_push(fd, req_buf, req_len);
        break;

    /* --- Audio Input --- */
    case CMD_AI_OPEN:
        ret = handle_ai_open(fd, req_buf, req_len);
        break;
    case CMD_AI_CLOSE:
        ret = handle_ai_close(fd, req_buf, req_len);
        break;
    case CMD_AI_SET_ADC_VOLUME:
        ret = handle_ai_set_adc_volume(fd, req_buf, req_len);
        break;
    case CMD_AI_SET_ASLC_VOLUME:
        ret = handle_ai_set_aslc_volume(fd, req_buf, req_len);
        break;

    /* --- Audio Encoder --- */
    case CMD_AENC_OPEN:
        ret = handle_aenc_open(fd, req_buf, req_len);
        break;
    case CMD_AENC_CLOSE:
        ret = handle_aenc_close(fd, req_buf, req_len);
        break;
    case CMD_AENC_SET_ATTR:
        ret = handle_aenc_set_attr(fd, req_buf, req_len);
        break;

    /* --- ISP / Imaging --- */
    case CMD_ISP_SET_BRIGHTNESS:
        ret = handle_isp_effect(fd, req_buf, req_len,
                                VPSS_EFFECT_BRIGHTNESS, "set_brightness");
        break;
    case CMD_ISP_SET_CONTRAST:
        ret = handle_isp_effect(fd, req_buf, req_len,
                                VPSS_EFFECT_CONTRAST, "set_contrast");
        break;
    case CMD_ISP_SET_SATURATION:
        ret = handle_isp_effect(fd, req_buf, req_len,
                                VPSS_EFFECT_SATURATION, "set_saturation");
        break;
    case CMD_ISP_SET_SHARPNESS:
        ret = handle_isp_effect(fd, req_buf, req_len,
                                VPSS_EFFECT_SHARP, "set_sharpness");
        break;
    case CMD_ISP_SET_IR_FILTER:
        ret = handle_isp_set_ir_filter(fd, req_buf, req_len);
        break;
    case CMD_ISP_SET_WDR:
        ret = handle_isp_set_wdr(fd, req_buf, req_len);
        break;

    /* --- Utility --- */
    case CMD_GET_ERROR_NO:
        ret = handle_get_error_no(fd);
        break;
    case CMD_GET_ERROR_STR:
        ret = handle_get_error_str(fd);
        break;

    /* --- Session --- */
    case CMD_HELLO:
        ret = handle_hello(fd);
        break;

    case CMD_SHUTDOWN:
        log_info("[daemon] CMD_SHUTDOWN received");
        send_response(fd, STATUS_OK, NULL, 0);
        ret = -2;  /* signal caller to shut everything down */
        break;

    default:
        log_warn("[daemon] unknown cmd_id=%d, sending error", cmd_id);
        ret = send_response(fd, STATUS_ERROR, NULL, 0);
        break;
    }

    free(req_buf);
    log_debug("[daemon] fd=%d cmd=%d completed ret=%d", fd, cmd_id, ret);
    return ret;
}
