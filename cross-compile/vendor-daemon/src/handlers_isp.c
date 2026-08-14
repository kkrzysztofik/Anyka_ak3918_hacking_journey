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

/*
 * isp_get_cur_lum_factor - the ISP's current luminance factor.
 *
 * Declared here rather than included: no vendor header we ship declares it, but
 * libplat_vi.so exports it (`T isp_get_cur_lum_factor`), and the copy on the
 * cameras is byte-identical to cross-compile/vendor-daemon/lib/libplat_vi.so.
 * Prototype from anyka_reference/platform/libplat/src/include/isp_basic.h:300.
 */
extern int isp_get_cur_lum_factor(void);

/**
 * handle_isp_get_lum_factor - Return the vendor's day/night luminance ratio.
 *
 * Reproduces calc_cur_lumi() from the stock firmware
 * (anyka_reference/platform/libplat/src/vpss/ak_vpss_isp.c:205):
 *
 *     lum_factor = isp_get_cur_lum_factor() * 40 / avg_lumi
 *
 * avg_lumi is the AE loop's *regulated* output, pinned near its setpoint until
 * the AE runs out of gain, which is why it is useless alone as a light meter.
 * Dividing the ISP's luminance factor by it yields exposure effort per unit of
 * achieved brightness, so HIGHER MEANS DARKER. The stock thresholds are already
 * on every camera in /etc/jffs2/anyka_cfg.ini [autoir]: day_to_night_lum = 6400,
 * night_to_day_lum = 2048, and the gap between them is the hysteresis band.
 *
 * See docs/reference/vendor-day-night-implementation.md.
 *
 * Empty request. Response payload: [i32 lum_factor] = 4 bytes.
 */
int handle_isp_get_lum_factor(int fd, const uint8_t *req, uint32_t req_len)
{
    void *vi;
    struct vpss_isp_ae_run_info info;
    int avg_lumi;
    int raw_factor;
    int32_t resp[1];

    (void)req;
    (void)req_len;

    if (isp_first_vi(&vi) != 0) {
        log_warn("[isp] get_lum_factor: no VI registered");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    /* isp_get_cur_lum_factor() returns -1 on three separate failure paths
     * (uninitialised fps config, bad high_fps_exp_time, AE stat read failure --
     * isp_basic.c:2403). It MUST NOT be forwarded: a negative factor is below
     * every day threshold, so the caller would read "bright" and switch night
     * vision off the moment the ISP hiccups. Report unavailable and let the
     * caller hold its current mode. The vendor tolerates -1 here only because
     * its AWB gate is a second opinion; we have no such gate. */
    raw_factor = isp_get_cur_lum_factor();
    if (raw_factor <= 0) {
        log_warn("[isp] get_lum_factor: isp_get_cur_lum_factor failed (%d)", raw_factor);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    memset(&info, 0, sizeof(info));
    if (ak_vpss_isp_get_ae_run_info(vi, &info) != 0) {
        log_warn("[isp] get_lum_factor: ak_vpss_isp_get_ae_run_info failed");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    /* The vendor's own guard (ak_vpss_isp.c:216): a zero reading would divide by
     * zero, and 40 is the AE setpoint it substitutes. */
    avg_lumi = (int)info.current_calc_avg_lumi;
    if (avg_lumi == 0)
        avg_lumi = 40;

    resp[0] = (int32_t)(raw_factor * 40 / avg_lumi);
    log_debug("[isp] get_lum_factor vi=%p raw=%d avg_lumi=%d factor=%d",
              vi, raw_factor, avg_lumi, (int)resp[0]);
    return send_response(fd, STATUS_OK, resp, sizeof(resp));
}

/**
 * handle_isp_get_ae_attr - Return the ISP's live AE attributes verbatim.
 *
 * The profile loaded by a day/night switch sets the exposure and gain ceilings
 * (`a_gain_max` is 24 by day and 10 at night on the gc1084), and this is the
 * only way to see what the ISP actually holds rather than what we asked for.
 * Deliberately uninterpreted: the caller decodes. See
 * docs/reference/vendor-day-night-implementation.md.
 *
 * Empty request. Response payload: struct vpss_isp_ae_attr = 204 bytes.
 */
_Static_assert(sizeof(struct vpss_isp_ae_attr) == 204, "AE attr wire size changed");

int handle_isp_get_ae_attr(int fd, const uint8_t *req, uint32_t req_len)
{
    void *vi;
    struct vpss_isp_ae_attr attr;

    (void)req;
    (void)req_len;

    if (isp_first_vi(&vi) != 0) {
        log_warn("[isp] get_ae_attr: no VI registered");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    memset(&attr, 0, sizeof(attr));
    if (ak_vpss_isp_get_ae_attr(vi, &attr) != 0) {
        log_warn("[isp] get_ae_attr: ak_vpss_isp_get_ae_attr failed");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    /* A 0 return means the ioctl succeeded, not that a profile is loaded. An
     * all-zero read is indistinguishable from a real one to the caller, and
     * the planned AE override read-modify-writes this struct -- writing a
     * zeroed one back would wipe hist_weight, envi_gain_range and
     * target_lumiance. a_gain_max is never legitimately 0 on a loaded
     * profile (24 by day, 10 at night on the gc1084). */
    if (attr.a_gain_max == 0) {
        log_warn("[isp] get_ae_attr: AE attrs unpopulated (a_gain_max=0)");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    log_debug("[isp] get_ae_attr vi=%p a_gain_max=%lu exp_time_max=%lu target_lum=%lu",
              vi, attr.a_gain_max, attr.exp_time_max, attr.target_lumiance);
    return send_response(fd, STATUS_OK, &attr, sizeof(attr));
}

/*
 * isp_get_statinfo - read ISP statistic module data (AWB, AE, AF, 3D-NR...).
 *
 * Declared here rather than included: no vendor header we ship declares it, but
 * libplat_vi.so exports it (`T isp_get_statinfo`), and the copy on the cameras
 * is byte-identical to cross-compile/vendor-daemon/lib/libplat_vi.so.
 * Prototype from anyka_reference/platform/libplat/src/include/isp_basic.h:100.
 */
extern int isp_get_statinfo(int module_id, void *buf, unsigned int *size);

/*
 * ISP_AWBSTAT = 27: counted from ISP_BB = 0 in the isp_module_id enum at
 * anyka_reference/platform/libplat/src/include/isp_basic.h:11-45 (28 entries,
 * ISP_AWBSTAT is the 28th, i.e. index 27). Defined locally rather than
 * vendoring the whole enum.
 */
#define ISP_AWBSTAT 27

_Static_assert(sizeof(struct vpss_isp_awb_stat_info) == 196, "AWB stat wire size changed");

/**
 * handle_isp_get_awb_stat - Return the ISP's live AWB colour-bin statistics.
 *
 * The vendor's own day/night switch never trusts luminance alone: it gates on
 * these AWB colour-temperature bin counts (total_cnt[10]) because chroma
 * collapses under IR illumination even when the AE loop reads "bright". This
 * handler copies that subset out uninterpreted; the caller decodes. See
 * docs/reference/vendor-day-night-implementation.md.
 *
 * Empty request. Response payload: [i32 total_cnt[10]] = 40 bytes.
 */
int handle_isp_get_awb_stat(int fd, const uint8_t *req, uint32_t req_len)
{
    void *vi;
    struct vpss_isp_awb_stat_info info;
    unsigned int size = 0;
    int32_t resp[10];
    unsigned long total;
    int i;

    (void)req;
    (void)req_len;

    if (isp_first_vi(&vi) != 0) {
        log_warn("[isp] get_awb_stat: no VI registered");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    /* Unlike ak_vpss_isp_get_ae_attr (a55baf97), the vendor's ISP_AWBSTAT path
     * (isp_basic.c ISP_AWBSTAT case -> Ak_ISP_get_awb_stat_info) never inspects
     * total_cnt to decide success, so this return-code check is a weak signal:
     * it can report success without a fresh AWB pass having run. We do NOT
     * compensate with a total_cnt==0 guard -- zero counts are a legitimate
     * per-frame reading (e.g. AWB going quiet under IR illumination is exactly
     * what the next task measures), so mapping them to STATUS_ERROR would make
     * "AWB is idle" indistinguishable from "the read failed". */
    memset(&info, 0, sizeof(info));
    if (isp_get_statinfo(ISP_AWBSTAT, &info, &size) != 0) {
        log_warn("[isp] get_awb_stat: isp_get_statinfo failed");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    total = 0;
    for (i = 0; i < 10; i++)
        total += info.total_cnt[i];

    for (i = 0; i < 10; i++)
        resp[i] = (int32_t)info.total_cnt[i];

    log_debug("[isp] get_awb_stat vi=%p total=%lu", vi, total);
    return send_response(fd, STATUS_OK, resp, sizeof(resp));
}
