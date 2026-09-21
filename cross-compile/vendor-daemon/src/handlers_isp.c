#include <string.h>
#include <stdint.h>

#include "handlers_isp.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "globals.h"
#include "ak_vpss.h"
#include "ak_vi.h"
#include "ak_isp_sdk.h"

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

/* ISP effect cmds — wire: [i32 value]; effect_type selects VPSS_EFFECT_*. */
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

/* CMD_ISP_SET_IR_FILTER. Wire format: [i32 mode] = 4 bytes. */
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

/* Return current_calc_avg_lumi for the sole VI. */
/* CMD_ISP_SET_BLC. Wire format: [i32 level] = 4 bytes, the ONVIF effect
 * offset [-50, 50] (0 = "use the ISP profile's own setting").
 *
 * BLC has no ak_vpss effect, so it goes through the low-level ISP SDK:
 * a black-level offset lift applied in manual BLC mode. libakispsdk is
 * already linked and initialised in-process by libplat_vi; never call
 * AK_ISP_sdk_init here.
 */
#define BLC_MODE_MANUAL 0
#define BLC_MODE_LINKAGE 1
/*
 * The compiled driver (component/ispdrv_lib/ak39_isp2_3a.c) applies m_blc
 * when blc_mode == MODE_MANUAL, and its enum work_mode has MODE_MANUAL=0 /
 * MODE_LINKAGE=1 — the comments in ak_isp_drv.h and the isptool headers
 * state the opposite. The driver code is what is actually compiled into the
 * shipped libs, so we follow it; the hardware gate confirms the direction.
 */
/*
 * bl_*_offset register range is [-2048, 2047] (ak_isp_drv.h). Scaling the
 * incoming [-50, 50] offset to a quarter of that keeps the full ONVIF
 * slider range visible in the image without crushing the black point
 * entirely. Bump it up if the hardware gate shows the top of the slider
 * is not visibly different.
 */
#define BLC_OFFSET_FULL_SCALE 512

/* Pristine profile BLC attr, cached on first touch so a neutral level
 * can restore the profile's own settings. */
static AK_ISP_BLC_ATTR g_blc_profile;
static int g_blc_manual;

int handle_isp_set_blc(int fd, const uint8_t *req, uint32_t req_len)
{
    AK_ISP_BLC_ATTR attr;
    int32_t level;
    int offset;

    if (req_len < 4) {
        log_warn("[isp] set_blc: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    level = req_read_i32(req, 0);

    /* Read-modify-write: a fresh struct would zero the tuning fields we do
     * not model, which is worse than leaving BLC alone. */
    if (AK_ISP_get_blc_attr(&attr)) {
        log_warn("[isp] set_blc: read failed; not writing");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (!g_blc_manual) {
        g_blc_profile = attr;
    }

    if (level == 0) {
        if (!g_blc_manual) {
            /* Profile settings already in force; nothing to write. */
            return send_response(fd, STATUS_OK, NULL, 0);
        }
        attr = g_blc_profile; /* restore the profile's own BLC */
        g_blc_manual = 0;
    } else {
        offset = level * BLC_OFFSET_FULL_SCALE / 50;
        attr.blc_mode = BLC_MODE_MANUAL;
        attr.m_blc.black_level_enable = 1;
        attr.m_blc.bl_r_offset = offset;
        attr.m_blc.bl_gr_offset = offset;
        attr.m_blc.bl_gb_offset = offset;
        attr.m_blc.bl_b_offset = offset;
        g_blc_manual = 1;
    }

    log_debug("[isp] set_blc level=%d mode=%u", (int)level, (unsigned)attr.blc_mode);
    return send_response(fd, AK_ISP_set_blc_attr(&attr), NULL, 0);
}

/* CMD_ISP_GET_BLC. Returns the raw AK_ISP_BLC_ATTR bytes; the daemon does
 * not interpret them. */
int handle_isp_get_blc(int fd, const uint8_t *req, uint32_t req_len)
{
    AK_ISP_BLC_ATTR attr;

    (void)req;
    if (req_len != 0) {
        log_warn("[isp] get_blc: expected empty request (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (AK_ISP_get_blc_attr(&attr)) {
        log_warn("[isp] get_blc: read failed");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_response(fd, STATUS_OK, (const uint8_t *)&attr, sizeof(attr));
}

/* CMD_ISP_SET_WB_TYPE. Wire format: [i32 wb_type] = 4 bytes, passed through
 * to the SDK uninterpreted (WB_OPS_TYPE_MANU=0, WB_OPS_TYPE_AUTO=1). The
 * SDK layer needs no VI handle: libplat_vi owns the SDK handle in-process,
 * the same path the effect and BLC handlers use. */
int handle_isp_set_wb_type(int fd, const uint8_t *req, uint32_t req_len)
{
    AK_ISP_WB_TYPE_ATTR attr;
    int32_t wb_type;

    if (req_len < 4) {
        log_warn("[isp] set_wb_type: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    wb_type = req_read_i32(req, 0);
    attr.wb_type = (T_U16)wb_type;
    log_debug("[isp] set_wb_type type=%d", (int)wb_type);
    return send_response(fd, AK_ISP_set_wb_type(&attr), NULL, 0);
}

/* CMD_ISP_SET_MWB_ATTR. Wire format: [u16 r_gain][u16 b_gain] = 4 bytes.
 * Read-modify-write: a fresh struct would zero g_gain and the three offsets,
 * so only the two gains we model are replaced. */
int handle_isp_set_mwb_attr(int fd, const uint8_t *req, uint32_t req_len)
{
    AK_ISP_MWB_ATTR attr;
    uint32_t gains;
    uint16_t r_gain;
    uint16_t b_gain;

    if (req_len < 4) {
        log_warn("[isp] set_mwb_attr: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    gains = req_read_u32(req, 0);
    r_gain = (uint16_t)(gains & 0xffff);
    b_gain = (uint16_t)(gains >> 16);

    if (AK_ISP_get_mwb_attr(&attr)) {
        log_warn("[isp] set_mwb_attr: read failed; not writing");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    attr.r_gain = r_gain;
    attr.b_gain = b_gain;

    log_debug("[isp] set_mwb_attr r=%u b=%u (g=%u preserved)",
              (unsigned)r_gain, (unsigned)b_gain, (unsigned)attr.g_gain);
    return send_response(fd, AK_ISP_set_mwb_attr(&attr), NULL, 0);
}

/* CMD_ISP_GET_MWB_ATTR. Returns the raw AK_ISP_MWB_ATTR bytes; the daemon
 * does not interpret them. */
int handle_isp_get_mwb_attr(int fd, const uint8_t *req, uint32_t req_len)
{
    AK_ISP_MWB_ATTR attr;

    (void)req;
    if (req_len != 0) {
        log_warn("[isp] get_mwb_attr: expected empty request (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (AK_ISP_get_mwb_attr(&attr)) {
        log_warn("[isp] get_mwb_attr: read failed");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_response(fd, STATUS_OK, (const uint8_t *)&attr, sizeof(attr));
}

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

/* Return the vendor's day/night luminance ratio. Empty request. Response payload: [i32 lum_facto... */
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

/* Return the ISP's live AWB colour-bin statistics. Empty request. Response payload: [i32 total_c... */
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
