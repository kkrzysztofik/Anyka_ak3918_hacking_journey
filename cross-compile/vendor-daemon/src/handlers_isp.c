#include <string.h>
#include <stdint.h>

#include "handlers_isp.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "globals.h"
#include "ak_vpss.h"
#include "ak_vi.h"

/* ponytail: sole-VI, pass token only if multi-VI appears. */
static int isp_first_vi(void **out)
{
    int i;
    for (i = 0; i < VD_OBJ_SLOTS; i++) {
        if (g_obj_slots[i].live && g_obj_slots[i].kind == VD_OBJ_KIND_VI) {
            *out = g_obj_slots[i].ptr;
            return 0;
        }
    }
    return -1;
}

/**
 * handle_isp_effect - Generic IPC handler for ISP image-effect commands.
 *
 * All ISP effect commands share the same wire format and dispatch to
 * ak_vpss_effect_set() with the appropriate effect_type constant.
 * CMD_ISP_SET_IR_FILTER and CMD_ISP_SET_WDR are handled by dedicated functions.
 *
 * Wire format: [i32 value] = 4 bytes.
 *
 * @param fd          Client socket file descriptor, used to send the response.
 * @param req         Request payload bytes (little-endian, layout described above).
 * @param req_len     Length of @p req in bytes.
 * @param effect_type VPSS_EFFECT_* enum constant selecting the ISP parameter.
 * @param name        Human-readable effect name used in log messages.
 * @return            0 on success, -1 on I/O error.
 */
int handle_isp_effect(int fd, const uint8_t *req, uint32_t req_len,
                      int effect_type, const char *name)
{
    void *vi_handle;
    int32_t value;
    int ret;

    if (req_len < 4) {
        log_warn("[isp] %s: req too short (%u)", name, req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (isp_first_vi(&vi_handle) != 0) {
        log_warn("[isp] %s: no VI registered", name);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    value = req_read_i32(req, 0);
    log_debug("[isp] %s vi=%p value=%d", name, vi_handle, (int)value);
    ret = ak_vpss_effect_set(vi_handle, (enum vpss_effect_type)effect_type, (int)value);
    return send_response(fd, ret, NULL, 0);
}

/**
 * handle_isp_set_wdr - IPC handler for CMD_ISP_SET_WDR (no-op).
 *
 * ak_vpss_open_wdr() and ak_vpss_close_wdr() are not exported by the
 * libre_anyka_app SDK variant; this function always responds STATUS_OK
 * without calling any SDK function.
 * WDR control is not available in this SDK variant.
 *
 * @param fd      Client socket file descriptor.
 * @param req     Ignored.
 * @param req_len Ignored.
 * @return        0 on success, -1 on write error.
 */
int handle_isp_set_wdr(int fd, const uint8_t *req, uint32_t req_len)
{
    (void)req;
    (void)req_len;
    log_debug("[isp] set_wdr: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_isp_set_ir_filter - IPC handler for CMD_ISP_SET_IR_FILTER.
 *
 * Uses the VI day/night switch (ak_vi_switch_mode) rather than a VPSS effect
 * to control the IR cut filter.
 *
 * Wire format: [i32 mode] = 4 bytes.
 * mode: 0 = day (IR filter in), 1 = night (IR filter out).
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_isp_set_ir_filter(int fd, const uint8_t *req, uint32_t req_len)
{
    void *vi_handle;
    int32_t mode;
    enum video_daynight_mode dn;
    int ret;

    if (req_len < 4)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    if (isp_first_vi(&vi_handle) != 0) {
        log_warn("[isp] set_ir_filter: no VI registered");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    mode = req_read_i32(req, 0);
    dn = (mode != 0) ? VI_MODE_NIGHT : VI_MODE_DAY;
    log_debug("[isp] set_ir_filter vi=%p mode=%d", vi_handle, (int)dn);
    ret = ak_vi_switch_mode(vi_handle, dn);
    return send_response(fd, ret, NULL, 0);
}

/**
 * handle_isp_get_ae_luma - Return current_calc_avg_lumi for the sole VI.
 *
 * Empty request. Uses the first live VD_OBJ_KIND_VI slot (single-camera boards).
 * Response payload: 1 byte luma on STATUS_OK.
 */
int handle_isp_get_ae_luma(int fd, const uint8_t *req, uint32_t req_len)
{
    void *vi;
    struct vpss_isp_ae_run_info info;
    uint8_t luma;

    (void)req;
    (void)req_len;

    if (isp_first_vi(&vi) != 0) {
        log_warn("[isp] get_ae_luma: no VI registered");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    memset(&info, 0, sizeof(info));
    if (ak_vpss_isp_get_ae_run_info(vi, &info) != 0) {
        log_warn("[isp] get_ae_luma: ak_vpss_isp_get_ae_run_info failed");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    luma = info.current_calc_avg_lumi;
    log_debug("[isp] get_ae_luma vi=%p luma=%u", vi, (unsigned)luma);
    return send_response(fd, STATUS_OK, &luma, 1);
}
