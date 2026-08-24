/*
 * OSD handlers — thin wrappers over libmpi_osd.so.
 *
 * Deliberately dumb: no timers, no strftime, no timezone handling.  Rust owns
 * all of that because it already has the config, the timezone and chrono, and
 * because policy there is host-testable.  These handlers only draw what they
 * are told, where they are told.
 *
 * ak_osd_init calls osd_sys_ipc_register internally; ak_cmd_register_module /
 * ak_cmd_unregister_module are satisfied by no-op stubs in osd_ipcsrv_stubs.c.
 * Do not remove those stubs. ISP mem/context attrs go through osd_vpss_wrap.c.
 */
#include <string.h>
#include <stdlib.h>

#include "handlers_osd.h"
#include "globals.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "ak_osd.h"
#include "ak_vi.h"
#include "ak_error.h"

#define OSD_FONT_PATH   "/usr/local/ak_font_16.bin"
#define OSD_FONT_SIZE   16
#define OSD_MAX_CHANNEL 1      /* channels are 0 (main) and 1 (sub) */
#define OSD_MAX_RECT    2      /* rects are 0..2 */
#define OSD_MAX_GLYPHS  128    /* bounds CMD_OSD_DRAW_STR; a rect cannot show more */

/* Set once by handle_osd_init so osd_shutdown() knows whether to destroy. */
static int g_osd_ready = 0;

/**
 * osd_args_valid - Bounds-check a channel/rect pair from an untrusted request.
 *
 * The vendor library does not validate these and indexes arrays with them, so
 * an out-of-range value is a memory-safety problem, not just a wrong picture.
 */
static int osd_args_valid(int32_t channel, int32_t rect)
{
    return channel >= 0 && channel <= OSD_MAX_CHANNEL &&
           rect >= 0 && rect <= OSD_MAX_RECT;
}

/**
 * handle_osd_init - IPC handler for CMD_OSD_INIT.
 *
 * Request: [u64 vi_token]
 * Response: [i32 main_w][i32 main_h][i32 sub_w][i32 sub_h] — the per-channel
 * max rect, which Rust needs for its layout math.
 *
 * Font file must be set BEFORE ak_osd_init; that ordering is what
 * platform/libmpi/demo/osd_demo does and it is load-bearing.
 */
int handle_osd_init(int fd, const uint8_t *req, uint32_t req_len)
{
    void *handle = NULL;
    int32_t dims[4] = { 0, 0, 0, 0 };
    int channel;

    if (req_len < 8) {
        log_error("[osd] init: short request (%u bytes)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &handle) != 0) {
        log_error("[osd] init: bad VI token");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    if (ak_osd_set_font_file(OSD_FONT_SIZE, OSD_FONT_PATH) < 0) {
        log_error("[osd] init: set_font_file(%s) failed", OSD_FONT_PATH);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (ak_osd_init(handle) < 0) {
        log_error("[osd] init: ak_osd_init failed");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    for (channel = 0; channel <= OSD_MAX_CHANNEL; channel++) {
        int w = 0, h = 0;
        if (ak_osd_get_max_rect(channel, &w, &h) < 0) {
            log_error("[osd] init: get_max_rect(chn=%d) failed", channel);
            ak_osd_destroy();
            return send_response(fd, STATUS_ERROR, NULL, 0);
        }
        if (w <= 0 || h <= 0 || w > 4096 || h > 4096) {
            log_error("[osd] init: chn=%d max_rect=%dx%d looks bogus",
                      channel, w, h);
            if (channel == 0) {
                ak_osd_destroy();
                return send_response(fd, STATUS_ERROR, NULL, 0);
            }
            /* Sub only: leave zeros so Rust skips that channel. */
            continue;
        }
        dims[channel * 2]     = (int32_t)w;
        dims[channel * 2 + 1] = (int32_t)h;
        log_info("[osd] init: chn=%d max_rect=%dx%d", channel, w, h);
    }

    g_osd_ready = 1;
    return send_response(fd, STATUS_OK, dims, sizeof(dims));
}

/**
 * handle_osd_set_rect - IPC handler for CMD_OSD_SET_RECT.
 *
 * Request: [u64 vi_token][i32 channel][i32 rect][i32 x][i32 y][i32 w][i32 h]
 */
int handle_osd_set_rect(int fd, const uint8_t *req, uint32_t req_len)
{
    void *handle = NULL;
    int32_t channel, rect, x, y, w, h;

    if (req_len < 8 + 6 * 4) {
        log_error("[osd] set_rect: short request (%u bytes)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &handle) != 0)
        return send_response(fd, STATUS_ERROR, NULL, 0);

    channel = req_read_i32(req, 8);
    rect    = req_read_i32(req, 12);
    x       = req_read_i32(req, 16);
    y       = req_read_i32(req, 20);
    w       = req_read_i32(req, 24);
    h       = req_read_i32(req, 28);

    if (!osd_args_valid(channel, rect)) {
        log_error("[osd] set_rect: bad chn=%d rect=%d", channel, rect);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    if (ak_osd_set_rect(handle, channel, rect, x, y, w, h) < 0) {
        log_error("[osd] set_rect: chn=%d rect=%d %dx%d@%d,%d failed",
                  channel, rect, w, h, x, y);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_osd_draw_str - IPC handler for CMD_OSD_DRAW_STR.
 *
 * Request: [i32 channel][i32 rect][i32 x][i32 y][u16 glyph_count][u16 glyphs...]
 *
 * Glyphs are already vendor-encoded by Rust (ASCII: u16 == byte).  Rust also
 * space-pads a shrinking string to its previous length, which is why there is
 * no CMD_OSD_CLEAN_STR — the vendor's own osd_disp_stat does exactly this.
 */
int handle_osd_draw_str(int fd, const uint8_t *req, uint32_t req_len)
{
    int32_t channel, rect, x, y;
    uint32_t count;
    unsigned short *glyphs;
    uint32_t i;
    int rc;

    if (req_len < 18) {
        log_error("[osd] draw_str: short request (%u bytes)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    channel = req_read_i32(req, 0);
    rect    = req_read_i32(req, 4);
    x       = req_read_i32(req, 8);
    y       = req_read_i32(req, 12);
    count   = (uint32_t)req[16] | ((uint32_t)req[17] << 8);

    if (!osd_args_valid(channel, rect)) {
        log_error("[osd] draw_str: bad chn=%d rect=%d", channel, rect);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (count == 0 || count > OSD_MAX_GLYPHS || req_len < 18 + count * 2) {
        log_error("[osd] draw_str: bad glyph count %u (req_len=%u)", count, req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    glyphs = malloc(count * sizeof(unsigned short));
    if (!glyphs)
        return send_response(fd, STATUS_ERROR, NULL, 0);

    /* Decode little-endian u16 explicitly rather than casting the request
     * buffer: it may not be 2-byte aligned, and armv5te faults on that. */
    for (i = 0; i < count; i++)
        glyphs[i] = (unsigned short)(req[18 + i * 2] |
                                     ((unsigned short)req[19 + i * 2] << 8));

    rc = ak_osd_draw_str(channel, rect, x, y, glyphs, (int)count);
    free(glyphs);

    if (rc < 0) {
        log_error("[osd] draw_str: chn=%d rect=%d failed", channel, rect);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_osd_set_enable - IPC handler for CMD_OSD_SET_ENABLE.
 *
 * Request: [i32 channel][i32 rect][i32 enable]
 */
int handle_osd_set_enable(int fd, const uint8_t *req, uint32_t req_len)
{
    int32_t channel, rect, enable;

    if (req_len < 12)
        return send_response(fd, STATUS_ERROR, NULL, 0);

    channel = req_read_i32(req, 0);
    rect    = req_read_i32(req, 4);
    enable  = req_read_i32(req, 8);

    if (!osd_args_valid(channel, rect))
        return send_response(fd, STATUS_ERROR, NULL, 0);

    if (ak_osd_set_rect_enable(channel, rect, enable ? 1 : 0) < 0) {
        log_error("[osd] set_enable: chn=%d rect=%d en=%d failed",
                  channel, rect, enable);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_osd_set_style - IPC handler for CMD_OSD_SET_STYLE.
 *
 * Request: [i32 front_color][i32 bg_color][i32 edge_color][i32 alpha]
 *
 * All four are DEVICE-GLOBAL in the vendor API — no channel, no rect.  The
 * ONVIF layer advertises this honestly rather than faking per-OSD colour.
 */
int handle_osd_set_style(int fd, const uint8_t *req, uint32_t req_len)
{
    int32_t front, bg, edge, alpha;

    if (req_len < 16)
        return send_response(fd, STATUS_ERROR, NULL, 0);

    front = req_read_i32(req, 0);
    bg    = req_read_i32(req, 4);
    edge  = req_read_i32(req, 8);
    alpha = req_read_i32(req, 12);

    if (front < 0 || front > 15 || bg < 0 || bg > 15 ||
        edge < 0 || edge > 15 || alpha < 1 || alpha > 100) {
        log_error("[osd] set_style: out of range front=%d bg=%d edge=%d alpha=%d",
                  front, bg, edge, alpha);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    if (ak_osd_set_color(front, bg) < 0 ||
        ak_osd_set_edge_color(edge) < 0 ||
        ak_osd_set_alpha(alpha) < 0) {
        log_error("[osd] set_style: vendor call failed");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_response(fd, STATUS_OK, NULL, 0);
}

void osd_shutdown(void)
{
    if (g_osd_ready) {
        ak_osd_destroy();
        g_osd_ready = 0;
        log_info("[osd] destroyed");
    }
}

