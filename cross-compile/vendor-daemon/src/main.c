/**
 * vendor-daemon - Anyka SDK IPC bridge
 *
 * Receives binary IPC commands from the Rust onvif-rust server over a Unix
 * domain socket (/tmp/vendor-daemon.sock) and dispatches them to the real
 * Anyka SDK C API calls.
 *
 * Protocol (little-endian):
 *   Request:  [i32 cmd_id][u32 req_len][req_data bytes]
 *   Response: [i32 status][u32 resp_len][resp_data bytes]
 *
 * Dual Socket Mode (Approach B Phase 3):
 *   - Control socket: /tmp/vd-ctrl.sock (formerly /tmp/vendor-daemon.sock)
 *     Handles all IPC commands including lifecycle operations
 *   - Frame socket: /tmp/vd-frame.sock
 *     Dedicated socket for frame streaming with shared memory support
 *
 * Shared Memory Ring Buffer (Approach A):
 *   - Zero-copy frame delivery via shared memory
 *   - 12-byte notification protocol on frame socket
 *   - Falls back to socket-based delivery on ring buffer overflow
 *
 * Connection model:
 *   poll()-based multiplexing of up to MAX_CLIENTS concurrent connections.
 *   The first client to issue a lifecycle command (vi_open, venc_open, etc.)
 *   becomes the "control client" — only it may call lifecycle commands.
 *   Additional clients are restricted to streaming ops (get/release_stream,
 *   set_iframe) and queries (get_error_*, isp_*).  When the control client
 *   disconnects, the slot opens for the next lifecycle command.
 *
 * Build (old Anyka toolchain):
 *   arm-anykav200-linux-uclibcgnueabi-gcc -std=gnu99 -march=armv5te \
 *     -mfloat-abi=soft -o vendor-daemon main.c \
 *     -I<sdk-include-dir> -L<sdk-lib-dir> \
 *     -lplat_vi -lplat_vpss -lplat_venc_cb -lmpi_venc ...
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <errno.h>
#include <signal.h>
#include <unistd.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <fcntl.h>
#include <poll.h>
#include "log.h"

#include "ak_vi.h"
#include "ak_vpss.h"
#include "ak_venc.h"
#include "ak_ai.h"
#include "ak_aenc.h"
#include "ak_common.h"
#include "ak_global.h"
#include "ak_error.h"
#include "vd_ring_buffer.h"

/* ---- constants ----------------------------------------------------------- */

/* Dual socket paths (Approach B Phase 3) */
#define CTRL_SOCKET_PATH   "/tmp/vd-ctrl.sock"   /* Formerly /tmp/vendor-daemon.sock */
#define FRAME_SOCKET_PATH  "/tmp/vd-frame.sock"  /* Dedicated frame delivery */
#define MAX_REQUEST_SIZE (1 * 1024 * 1024)  /* 1 MB – for frame data */
#define LOG_FILE_PATH_DEFAULT "/mnt/logs/vendor_daemon.log"
#define LOG_FILE_PATH_MAX    512
#define STATUS_OK        0
#define STATUS_ERROR     (-1)

/* Maximum number of video_stream structs held pending release */
#define MAX_PENDING_STREAMS 16

/* Maximum simultaneous clients (control + streaming + snapshot + spare) */
#define MAX_CLIENTS 4
#define SDK_ERROR_NO_DATA 23

/* ---- command IDs --------------------------------------------------------- */

enum cmd_id {
    /* Video Input */
    CMD_VI_MATCH_SENSOR           = 1,
    CMD_VI_OPEN                   = 2,
    CMD_VI_CLOSE                  = 3,
    CMD_VI_GET_SENSOR_RESOLUTION  = 4,
    CMD_VI_SET_CHANNEL_ATTR       = 5,
    CMD_VI_CAPTURE_ON             = 6,
    CMD_VI_CAPTURE_OFF            = 7,
    /* VPSS */
    CMD_VPSS_INIT                 = 8,
    CMD_VPSS_DESTROY              = 9,
    /* Video Encoder */
    CMD_VENC_SET_CFG_PATH         = 10,
    CMD_VENC_OPEN                 = 11,
    CMD_VENC_CLOSE                = 12,
    CMD_VENC_SET_RC               = 13,
    CMD_VENC_SET_IFRAME           = 14,
    CMD_VENC_REQUEST_STREAM       = 15,
    CMD_VENC_GET_STREAM           = 16,
    CMD_VENC_RELEASE_STREAM       = 17,
    CMD_VENC_CANCEL_STREAM        = 18,
    /* Audio Input */
    CMD_AI_OPEN                   = 50,
    CMD_AI_CLOSE                  = 51,
    CMD_AI_SET_ADC_VOLUME         = 52,
    CMD_AI_SET_ASLC_VOLUME        = 53,
    /* Audio Encoder */
    CMD_AENC_OPEN                 = 54,
    CMD_AENC_CLOSE                = 55,
    CMD_AENC_SET_ATTR             = 56,
    /* Imaging / ISP */
    CMD_ISP_SET_BRIGHTNESS        = 100,
    CMD_ISP_SET_CONTRAST          = 101,
    CMD_ISP_SET_SATURATION        = 102,
    CMD_ISP_SET_SHARPNESS         = 103,
    CMD_ISP_SET_IR_FILTER         = 104,
    CMD_ISP_SET_WDR               = 105,
    /* Utility */
    CMD_GET_ERROR_NO              = 200,
    CMD_GET_ERROR_STR             = 201,
    CMD_SHUTDOWN                  = 255
};

/* ---- pending-release table ----------------------------------------------- */
/*
 * After ak_venc_get_stream() succeeds we store the video_stream struct here
 * and hand the Rust client the slot index as a "remote_token". The client
 * must pass the token back with CMD_VENC_RELEASE_STREAM so we can call
 * ak_venc_release_stream() with the correct struct.
 */
static struct {
    struct video_stream stream;
    void *stream_handle;  /* SDK stream handle for release */
    int in_use;
    int client_fd;        /* owning client fd for cleanup on disconnect */
} g_pending[MAX_PENDING_STREAMS];

/* ---- shutdown flag ------------------------------------------------------- */
static volatile int g_shutdown = 0;

/* ---- control client guard ------------------------------------------------ */
/*
 * The "control client" is the first connection that owns the hardware
 * lifecycle.  Only this fd may issue lifecycle commands (vi_open, vi_close,
 * venc_open, venc_close, venc_request_stream, venc_cancel_stream, etc.).
 * Additional clients are restricted to streaming ops (get_stream,
 * release_stream, set_iframe, set_rc) and queries (get_error_*, isp_*).
 *
 * When the control client disconnects, the slot opens and the next
 * lifecycle command from any client promotes that client to control.
 */
static int g_control_fd = -1;
static unsigned long long g_venc_no_data_count = 0;
static unsigned long long g_venc_stream_error_count = 0;

/* ---- Shared memory ring buffer (Approach A) ------------------------------ */
/*
 * Shared memory ring buffer for zero-copy frame delivery.
 * When initialized, frame requests on the frame socket will:
 *   1. Write frame to ring buffer
 *   2. Send 12-byte notification instead of full frame data
 *   3. Release SDK frame immediately (data now in shm)
 */
static void *g_ring_buffer = NULL;

/* ---- Dual socket support (Approach B Phase 3) -------------------------- */
/*
 * Control socket: handles all IPC commands (lifecycle + streaming)
 * Frame socket: dedicated for frame delivery with shared memory support
 */
static int g_ctrl_server_fd = -1;   /* Control socket server */
static int g_frame_server_fd = -1;  /* Frame socket server */
static int g_frame_client_fd = -1;  /* At most 1 frame client */

/* Saved file descriptors for stdout/stderr restoration on shutdown */
static int g_saved_stdout = -1;
static int g_saved_stderr = -1;
static FILE *g_log_fp = NULL;

static void signal_handler(int sig)
{
    (void)sig;
    g_shutdown = 1;
}

/* ---- I/O helpers --------------------------------------------------------- */

/**
 * read_exact - read exactly n bytes, retrying on partial reads.
 * Returns 0 on success, -1 on EOF or error.
 */
static int read_exact(int fd, void *buf, size_t n)
{
    size_t done = 0;
    while (done < n) {
        ssize_t r = read(fd, (char *)buf + done, n - done);
        if (r <= 0) {
            if (r < 0 && errno == EINTR)
                continue;
            return -1;
        }
        done += (size_t)r;
    }
    return 0;
}

/**
 * write_exact - write exactly n bytes, retrying on partial writes.
 * Returns 0 on success, -1 on error.
 */
static int write_exact(int fd, const void *buf, size_t n)
{
    size_t done = 0;
    while (done < n) {
        ssize_t w = write(fd, (const char *)buf + done, n - done);
        if (w <= 0) {
            if (w < 0 && errno == EINTR)
                continue;
            return -1;
        }
        done += (size_t)w;
    }
    return 0;
}

/**
 * send_response - send [i32 status][u32 len][data] to client.
 * Returns 0 on success, -1 on write error.
 */
static int send_response(int fd, int32_t status, const void *data, uint32_t len)
{
    if (write_exact(fd, &status, sizeof(status)) != 0) return -1;
    if (write_exact(fd, &len,    sizeof(len))    != 0) return -1;
    if (len > 0 && data != NULL)
        if (write_exact(fd, data, len) != 0)           return -1;
    return 0;
}

/* ---- pending-release helpers --------------------------------------------- */

static int pending_alloc(int client_fd)
{
    int i;
    for (i = 0; i < MAX_PENDING_STREAMS; i++) {
        if (!g_pending[i].in_use) {
            g_pending[i].in_use = 1;
            g_pending[i].client_fd = client_fd;
            return i;
        }
    }
    return -1;
}

/**
 * Release all pending video_stream slots owned by a disconnecting client.
 * Prevents SDK ring buffer leaks when a client exits without releasing.
 */
static void cleanup_pending_for_fd(int client_fd)
{
    int i;
    int cleaned = 0;
    for (i = 0; i < MAX_PENDING_STREAMS; i++) {
        if (g_pending[i].in_use && g_pending[i].client_fd == client_fd) {
            ak_venc_release_stream(g_pending[i].stream_handle,
                                   &g_pending[i].stream);
            g_pending[i].in_use = 0;
            g_pending[i].client_fd = -1;
            cleaned++;
        }
    }
    if (cleaned > 0) {
        log_info("[daemon] cleaned up %d pending frames for fd=%d",
                 cleaned, client_fd);
    }
}

static void pending_free(int idx)
{
    if (idx >= 0 && idx < MAX_PENDING_STREAMS) {
        g_pending[idx].in_use = 0;
        g_pending[idx].client_fd = -1;
    }
}

/* ---- Socket creation helper --------------------------------------------- */

/**
 * create_unix_socket - create, bind, and listen on a Unix domain socket
 * Returns socket fd on success, -1 on error
 */
static int create_unix_socket(const char *path)
{
    int fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (fd < 0) {
        log_error("[daemon] socket: %s", strerror(errno));
        return -1;
    }

    unlink(path);

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, path, sizeof(addr.sun_path) - 1);

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        log_error("[daemon] bind %s: %s", path, strerror(errno));
        close(fd);
        return -1;
    }

    if (listen(fd, 4) < 0) {
        log_error("[daemon] listen %s: %s", path, strerror(errno));
        close(fd);
        unlink(path);
        return -1;
    }

    return fd;
}

/* ---- VI handlers --------------------------------------------------------- */

static int handle_vi_match_sensor(int fd, const uint8_t *req, uint32_t req_len)
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

static int handle_vi_open(int fd, const uint8_t *req, uint32_t req_len)
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
    int64_t h64 = (int64_t)(uintptr_t)handle;
    return send_response(fd, STATUS_OK, &h64, sizeof(h64));
}

static int handle_vi_close(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t)) {
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    uint64_t h64;
    memcpy(&h64, req, sizeof(h64));
    void *handle = (void *)(uintptr_t)h64;

    log_debug("[vi] close handle=%p", handle);
    int ret = ak_vi_close(handle);
    return send_response(fd, ret, NULL, 0);
}

static int handle_vi_get_sensor_resolution(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t)) {
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    uint64_t h64;
    memcpy(&h64, req, sizeof(h64));
    void *handle = (void *)(uintptr_t)h64;

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

/*
 * Wire format for video_channel_attr (packed, LE):
 *   crop:  left(i32) top(i32) width(i32) height(i32)
 *   res[0]: width(i32) height(i32) max_width(i32) max_height(i32)
 *   res[1]: width(i32) height(i32) max_width(i32) max_height(i32)
 * Total = 12 i32s = 48 bytes (after u64 handle = 56 bytes)
 */
static int handle_vi_set_channel_attr(int fd, const uint8_t *req, uint32_t req_len)
{
    /* u64 handle + 12 i32s = 8 + 48 = 56 bytes */
    if (req_len < 8 + 48) {
        log_warn("[vi] set_channel_attr: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    uint64_t h64;
    memcpy(&h64, req, sizeof(h64));
    void *handle = (void *)(uintptr_t)h64;

    const uint8_t *p = req + 8;
    struct video_channel_attr attr;
    memset(&attr, 0, sizeof(attr));

    int32_t tmp;
    memcpy(&tmp, p +  0, 4); attr.crop.left      = (int)tmp;
    memcpy(&tmp, p +  4, 4); attr.crop.top       = (int)tmp;
    memcpy(&tmp, p +  8, 4); attr.crop.width     = (int)tmp;
    memcpy(&tmp, p + 12, 4); attr.crop.height    = (int)tmp;

    memcpy(&tmp, p + 16, 4); attr.res[0].width      = (int)tmp;
    memcpy(&tmp, p + 20, 4); attr.res[0].height     = (int)tmp;
    memcpy(&tmp, p + 24, 4); attr.res[0].max_width  = (int)tmp;
    memcpy(&tmp, p + 28, 4); attr.res[0].max_height = (int)tmp;

    memcpy(&tmp, p + 32, 4); attr.res[1].width      = (int)tmp;
    memcpy(&tmp, p + 36, 4); attr.res[1].height     = (int)tmp;
    memcpy(&tmp, p + 40, 4); attr.res[1].max_width  = (int)tmp;
    memcpy(&tmp, p + 44, 4); attr.res[1].max_height = (int)tmp;

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

static int handle_vi_capture_on(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t h64;
    memcpy(&h64, req, sizeof(h64));
    void *handle = (void *)(uintptr_t)h64;
    int ret = ak_vi_capture_on(handle);
    return send_response(fd, ret, NULL, 0);
}

static int handle_vi_capture_off(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t h64;
    memcpy(&h64, req, sizeof(h64));
    void *handle = (void *)(uintptr_t)h64;
    int ret = ak_vi_capture_off(handle);
    return send_response(fd, ret, NULL, 0);
}

/* ---- VPSS handlers ------------------------------------------------------- */

static int handle_vpss_init(int fd, const uint8_t *req, uint32_t req_len)
{
    /*
     * libre_anyka_app SDK: ak_vpss_init() is not exported by libplat_vpss.so.
     * VPSS lifecycle is managed internally by the VI subsystem in this SDK
     * variant — no explicit init call is needed or possible.
     */
    (void)req;
    (void)req_len;
    log_debug("[vpss] init: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

static int handle_vpss_destroy(int fd, const uint8_t *req, uint32_t req_len)
{
    /*
     * libre_anyka_app SDK: ak_vpss_destroy() is not exported by libplat_vpss.so.
     * VPSS teardown is managed internally by the VI subsystem in this SDK variant.
     */
    (void)req;
    (void)req_len;
    log_debug("[vpss] destroy: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/* ---- VENC handlers ------------------------------------------------------- */

static int handle_venc_set_cfg_path(int fd, const uint8_t *req, uint32_t req_len)
{
    /*
     * libre_anyka_app SDK: ak_venc_set_cfg_path() is not exported by libmpi_venc.so.
     * The V2 encoder in this SDK variant uses the default path (/etc/jffs2/venc.cfg)
     * and does not support overriding it at runtime.
     */
    (void)req;
    (void)req_len;
    log_debug("[venc] set_cfg_path: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/*
 * Wire format for encode_param (48 bytes, all i32/u32 LE):
 *   width(u32) height(u32) minqp(i32) maxqp(i32) fps(i32) goplen(i32)
 *   bps(i32) profile(i32) use_chn(i32) enc_grp(i32) br_mode(i32) enc_out_type(i32)
 */
static int handle_venc_open(int fd, const uint8_t *req, uint32_t req_len)
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
    int64_t h64 = (int64_t)(uintptr_t)handle;
    return send_response(fd, STATUS_OK, &h64, sizeof(h64));
}

static int handle_venc_close(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t h64;
    memcpy(&h64, req, sizeof(h64));
    void *handle = (void *)(uintptr_t)h64;
    log_debug("[venc] close handle=%p", handle);
    int ret = ak_venc_close(handle);
    return send_response(fd, ret, NULL, 0);
}

static int handle_venc_set_rc(int fd, const uint8_t *req, uint32_t req_len)
{
    /* u64 handle + i32 bps = 12 bytes */
    if (req_len < 8 + 4)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t h64;
    int32_t bps;
    memcpy(&h64, req,     sizeof(h64));
    memcpy(&bps, req + 8, sizeof(bps));
    void *handle = (void *)(uintptr_t)h64;
    int ret = ak_venc_set_rc(handle, (int)bps);
    return send_response(fd, ret, NULL, 0);
}

static int handle_venc_set_iframe(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t h64;
    memcpy(&h64, req, sizeof(h64));
    void *handle = (void *)(uintptr_t)h64;
    int ret = ak_venc_set_iframe(handle);
    return send_response(fd, ret, NULL, 0);
}

static int handle_venc_request_stream(int fd, const uint8_t *req, uint32_t req_len)
{
    /* u64 vi_handle + u64 venc_handle = 16 bytes */
    if (req_len < 16)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t vi_h64, venc_h64;
    memcpy(&vi_h64,   req,     sizeof(vi_h64));
    memcpy(&venc_h64, req + 8, sizeof(venc_h64));
    void *vi_handle   = (void *)(uintptr_t)vi_h64;
    void *venc_handle = (void *)(uintptr_t)venc_h64;

    log_debug("[venc] request_stream vi=%p venc=%p", vi_handle, venc_handle);
    void *stream_handle = ak_venc_request_stream(vi_handle, venc_handle);
    if (stream_handle == NULL) {
        log_error("[venc] request_stream failed (NULL)");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    int64_t h64 = (int64_t)(uintptr_t)stream_handle;
    return send_response(fd, STATUS_OK, &h64, sizeof(h64));
}

/*
 * Convert SDK frame type to ring buffer frame type
 */
static uint32_t convert_frame_type(enum video_frame_type sdk_type)
{
    switch (sdk_type) {
    case FRAME_TYPE_I:
        return VD_FRAME_TYPE_I;
    case FRAME_TYPE_B:
        return VD_FRAME_TYPE_B;
    case FRAME_TYPE_P:
    default:
        return VD_FRAME_TYPE_P;
    }
}

/*
 * handle_venc_get_stream_shm - Frame delivery with shared memory support
 * 
 * On the frame socket with ring buffer initialized:
 *   1. Get frame from SDK
 *   2. Write to ring buffer via vd_ring_write()
 *   3. Send 12-byte notification instead of full frame
 *   4. Release SDK frame immediately
 * 
 * Falls back to socket-based delivery on ring buffer overflow.
 */
static int handle_venc_get_stream_shm(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t sh64;
    memcpy(&sh64, req, sizeof(sh64));
    void *stream_handle = (void *)(uintptr_t)sh64;

    /* Allocate a slot in the pending table */
    int slot = pending_alloc(fd);
    if (slot < 0) {
        log_error("[venc] get_stream: pending table full");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    struct video_stream *vs = &g_pending[slot].stream;
    memset(vs, 0, sizeof(*vs));
    g_pending[slot].stream_handle = stream_handle;

    int ret = ak_venc_get_stream(stream_handle, vs);
    if (ret != 0) {
        int err_no = ak_get_error_no();
        if (err_no == SDK_ERROR_NO_DATA) {
            g_venc_no_data_count++;
            if ((g_venc_no_data_count % 1000ULL) == 0ULL) {
                log_debug("[venc] get_stream no-data events=%llu", g_venc_no_data_count);
            }
        } else {
            g_venc_stream_error_count++;
            log_warn(
                "[venc] get_stream failed ret=%d err_no=%d total_errors=%llu",
                ret,
                err_no,
                g_venc_stream_error_count
            );
        }
        pending_free(slot);
        return send_response(fd, ret, NULL, 0);
    }

    uint32_t frame_len = vs->len;
    uint64_t timestamp = (uint64_t)vs->ts;
    uint32_t seq_no = (uint32_t)vs->seq_no;
    int32_t frame_type = (int32_t)vs->frame_type;

    /*
     * Try shared memory path first (if ring buffer initialized and frame fits)
     */
    if (g_ring_buffer != NULL && frame_len <= VD_SHM_SLOT_DATA_SIZE) {
        /* Convert timestamp from ms to us */
        uint64_t timestamp_us = timestamp * 1000ULL;
        uint32_t ring_frame_type = convert_frame_type(vs->frame_type);
        
        int ring_slot = vd_ring_write(g_ring_buffer, vs->data, frame_len,
                                       timestamp_us, seq_no,
                                       ring_frame_type, VD_STREAM_MAIN);
        
        if (ring_slot >= 0) {
            /* Success: send 12-byte notification instead of full frame */
            struct vd_frame_notify notif;
            notif.slot_index = (uint32_t)ring_slot;
            notif.frame_len = frame_len;
            notif.flags = VD_NOTIFY_LAST_FRAGMENT;
            
            /* Send notification: [status][resp_len=12][notification] */
            int32_t status = STATUS_OK;
            uint32_t resp_len = sizeof(notif);  /* 12 bytes */
            
            if (write_exact(fd, &status, sizeof(status)) != 0) {
                /* Socket error, but frame is in ring buffer - still release it */
                ak_venc_release_stream(stream_handle, vs);
                pending_free(slot);
                return -1;
            }
            if (write_exact(fd, &resp_len, sizeof(resp_len)) != 0) {
                ak_venc_release_stream(stream_handle, vs);
                pending_free(slot);
                return -1;
            }
            if (write_exact(fd, &notif, sizeof(notif)) != 0) {
                ak_venc_release_stream(stream_handle, vs);
                pending_free(slot);
                return -1;
            }
            
            /* Release SDK frame immediately (data is now in shm) */
            ak_venc_release_stream(stream_handle, vs);
            pending_free(slot);
            return 0;
        }
        
        /* Ring buffer full or error - fall through to socket fallback */
        log_debug("[venc] ring buffer full, using socket fallback");
    }

    /*
     * Socket fallback: existing frame delivery over socket
     * This path is used when:
     *   - Ring buffer not initialized
     *   - Frame too large for ring buffer
     *   - Ring buffer write failed
     */
    uint64_t token = (uint64_t)slot;

    /* Header: 28 bytes + frame data */
    uint32_t resp_payload_len = 28u + frame_len;

    uint8_t hdr[28];
    memcpy(hdr +  0, &frame_len,  4);
    memcpy(hdr +  4, &timestamp,  8);
    memcpy(hdr + 12, &seq_no,     4);
    memcpy(hdr + 16, &frame_type, 4);
    memcpy(hdr + 20, &token,      8);

    /* Send header parts manually to avoid a large allocation */
    int32_t status = STATUS_OK;
    if (write_exact(fd, &status,          sizeof(status))          != 0) goto fail;
    if (write_exact(fd, &resp_payload_len, sizeof(resp_payload_len)) != 0) goto fail;
    if (write_exact(fd, hdr,              sizeof(hdr))              != 0) goto fail;
    if (frame_len > 0 && vs->data != NULL)
        if (write_exact(fd, vs->data, frame_len) != 0)               goto fail;

    /* Do NOT release yet; Rust must call CMD_VENC_RELEASE_STREAM with token */
    return 0;

fail:
    pending_free(slot);
    return -1;
}

/*
 * CMD_VENC_GET_STREAM response layout (after status + resp_len header):
 *   [u32 frame_len][u64 timestamp][u32 seq_no][i32 frame_type]
 *   [u64 remote_token][frame_data_bytes]
 * Total header = 4+8+4+4+8 = 28 bytes + frame data
 */
static int handle_venc_get_stream(int fd, const uint8_t *req, uint32_t req_len)
{
    /*
     * If this request is on the frame socket AND ring buffer is available,
     * use the shared memory path
     */
    if (fd == g_frame_client_fd && g_ring_buffer != NULL) {
        return handle_venc_get_stream_shm(fd, req, req_len);
    }

    /*
     * Legacy socket path: always send full frame data over socket
     */
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t sh64;
    memcpy(&sh64, req, sizeof(sh64));
    void *stream_handle = (void *)(uintptr_t)sh64;

    /* Allocate a slot in the pending table */
    int slot = pending_alloc(fd);
    if (slot < 0) {
        log_error("[venc] get_stream: pending table full");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    struct video_stream *vs = &g_pending[slot].stream;
    memset(vs, 0, sizeof(*vs));
    g_pending[slot].stream_handle = stream_handle;

    int ret = ak_venc_get_stream(stream_handle, vs);
    if (ret != 0) {
        int err_no = ak_get_error_no();
        if (err_no == SDK_ERROR_NO_DATA) {
            g_venc_no_data_count++;
            if ((g_venc_no_data_count % 1000ULL) == 0ULL) {
                log_debug("[venc] get_stream no-data events=%llu", g_venc_no_data_count);
            }
        } else {
            g_venc_stream_error_count++;
            log_warn(
                "[venc] get_stream failed ret=%d err_no=%d total_errors=%llu",
                ret,
                err_no,
                g_venc_stream_error_count
            );
        }
        pending_free(slot);
        return send_response(fd, ret, NULL, 0);
    }

    /* Build response payload – we copy frame data so Rust can own it */
    uint32_t frame_len   = vs->len;
    uint64_t timestamp   = (uint64_t)vs->ts;
    uint32_t seq_no      = (uint32_t)vs->seq_no;
    int32_t  frame_type  = (int32_t)vs->frame_type;
    uint64_t token       = (uint64_t)slot;

    /* Header: 28 bytes + frame data */
    uint32_t resp_payload_len = 28u + frame_len;

    /* Allocate response buffer on stack for header; frame_data may be large */
    uint8_t hdr[28];
    memcpy(hdr +  0, &frame_len,  4);
    memcpy(hdr +  4, &timestamp,  8);
    memcpy(hdr + 12, &seq_no,     4);
    memcpy(hdr + 16, &frame_type, 4);
    memcpy(hdr + 20, &token,      8);

    /* Send header parts manually to avoid a large allocation */
    int32_t  status = STATUS_OK;
    if (write_exact(fd, &status,          sizeof(status))          != 0) goto fail;
    if (write_exact(fd, &resp_payload_len, sizeof(resp_payload_len)) != 0) goto fail;
    if (write_exact(fd, hdr,              sizeof(hdr))              != 0) goto fail;
    if (frame_len > 0 && vs->data != NULL)
        if (write_exact(fd, vs->data, frame_len) != 0)               goto fail;

    /* Do NOT release yet; Rust must call CMD_VENC_RELEASE_STREAM with token */
    return 0;

fail:
    pending_free(slot);
    return -1;
}

static int handle_venc_release_stream(int fd, const uint8_t *req, uint32_t req_len)
{
    /* u64 stream_handle + u64 remote_token = 16 bytes */
    if (req_len < 16)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t sh64, token;
    memcpy(&sh64,  req,     sizeof(sh64));
    memcpy(&token, req + 8, sizeof(token));
    void *stream_handle = (void *)(uintptr_t)sh64;

    int slot = (int)token;
    if (slot < 0 || slot >= MAX_PENDING_STREAMS || !g_pending[slot].in_use) {
        log_error("[venc] release_stream: bad token %llu",
                (unsigned long long)token);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    int ret = ak_venc_release_stream(stream_handle, &g_pending[slot].stream);
    pending_free(slot);
    return send_response(fd, ret, NULL, 0);
}

static int handle_venc_cancel_stream(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t sh64;
    memcpy(&sh64, req, sizeof(sh64));
    void *stream_handle = (void *)(uintptr_t)sh64;
    log_debug("[venc] cancel_stream handle=%p", stream_handle);
    int ret = ak_venc_cancel_stream(stream_handle);
    return send_response(fd, ret, NULL, 0);
}

/* ---- Audio Input handlers ------------------------------------------------ */

/*
 * Wire format for pcm_param (12 bytes):
 *   sample_rate(u32) sample_bits(u32) channel_num(u32)
 */
static int handle_ai_open(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < 12) {
        log_warn("[ai] open: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    struct pcm_param param;
    uint32_t tmp;
    memcpy(&tmp, req + 0, 4); param.sample_rate  = (unsigned int)tmp;
    memcpy(&tmp, req + 4, 4); param.sample_bits  = (unsigned int)tmp;
    memcpy(&tmp, req + 8, 4); param.channel_num  = (unsigned int)tmp;

    log_debug("[ai] open rate=%u bits=%u ch=%u",
            param.sample_rate, param.sample_bits, param.channel_num);
    void *handle = ak_ai_open(&param);
    if (handle == NULL) {
        log_error("[ai] open failed (NULL handle)");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    int64_t h64 = (int64_t)(uintptr_t)handle;
    return send_response(fd, STATUS_OK, &h64, sizeof(h64));
}

static int handle_ai_close(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t h64;
    memcpy(&h64, req, sizeof(h64));
    void *handle = (void *)(uintptr_t)h64;
    int ret = ak_ai_close(handle);
    return send_response(fd, ret, NULL, 0);
}

static int handle_ai_set_adc_volume(int fd, const uint8_t *req, uint32_t req_len)
{
    /*
     * libre_anyka_app SDK: ak_ai_set_adc_volume() is not exported by libplat_ai.so.
     * Audio ADC volume control is not available in this SDK variant.
     */
    (void)req;
    (void)req_len;
    log_debug("[ai] set_adc_volume: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

static int handle_ai_set_aslc_volume(int fd, const uint8_t *req, uint32_t req_len)
{
    /*
     * libre_anyka_app SDK: ak_ai_set_aslc_volume() is not exported by libplat_ai.so.
     * Audio ASLC volume control is not available in this SDK variant.
     */
    (void)req;
    (void)req_len;
    log_debug("[ai] set_aslc_volume: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/* ---- Audio Encoder handlers ---------------------------------------------- */

/*
 * Wire format for audio_param (16 bytes), as specified in the IPC protocol:
 *   sample_rate(u32) channel_num(u32) sample_bits(u32) type(i32)
 *
 * Note: the SDK struct audio_param has fields in a different order
 * (type, sample_rate, sample_bits, channel_num), so we deserialise
 * explicitly from the wire layout rather than casting the buffer directly.
 */
static int handle_aenc_open(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < 16) {
        log_warn("[aenc] open: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    struct audio_param param;
    int32_t  itype;
    uint32_t tmp;
    memcpy(&tmp,   req +  0, 4); param.sample_rate  = (unsigned int)tmp;
    memcpy(&tmp,   req +  4, 4); param.channel_num  = (unsigned int)tmp;
    memcpy(&tmp,   req +  8, 4); param.sample_bits  = (unsigned int)tmp;
    memcpy(&itype, req + 12, 4); param.type         = (enum ak_audio_type)itype;

    log_debug("[aenc] open type=%d rate=%u bits=%u ch=%u",
            (int)param.type, param.sample_rate, param.sample_bits, param.channel_num);
    void *handle = ak_aenc_open(&param);
    if (handle == NULL) {
        log_error("[aenc] open failed (NULL handle)");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    int64_t h64 = (int64_t)(uintptr_t)handle;
    return send_response(fd, STATUS_OK, &h64, sizeof(h64));
}

static int handle_aenc_close(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t))
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t h64;
    memcpy(&h64, req, sizeof(h64));
    void *handle = (void *)(uintptr_t)h64;
    int ret = ak_aenc_close(handle);
    return send_response(fd, ret, NULL, 0);
}

static int handle_aenc_set_attr(int fd, const uint8_t *req, uint32_t req_len)
{
    /* u64 handle + i32 aac_head = 12 bytes */
    if (req_len < 8 + 4)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t h64;
    int32_t aac_head_i32;
    memcpy(&h64,          req,     sizeof(h64));
    memcpy(&aac_head_i32, req + 8, sizeof(aac_head_i32));
    void *handle = (void *)(uintptr_t)h64;

    struct aenc_attr attr;
    attr.aac_head = (enum aenc_aac_attr)aac_head_i32;
    int ret = ak_aenc_set_attr(handle, &attr);
    return send_response(fd, ret, NULL, 0);
}

/* ---- ISP / Imaging handlers ---------------------------------------------- */

/*
 * All ISP effect commands share the same wire format:
 *   u64 vi_handle + i32 value = 12 bytes
 *
 * They map to ak_vpss_effect_set(vi_handle, <effect_type>, value).
 * CMD_ISP_SET_IR_FILTER and CMD_ISP_SET_WDR are handled specially.
 */
static int handle_isp_effect(int fd, const uint8_t *req, uint32_t req_len,
                              enum vpss_effect_type effect_type, const char *name)
{
    if (req_len < 8 + 4) {
        log_warn("[isp] %s: req too short (%u)", name, req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    uint64_t h64;
    int32_t value;
    memcpy(&h64,   req,     sizeof(h64));
    memcpy(&value, req + 8, sizeof(value));
    void *vi_handle = (void *)(uintptr_t)h64;

    log_debug("[isp] %s vi=%p value=%d", name, vi_handle, (int)value);
    int ret = ak_vpss_effect_set(vi_handle, effect_type, (int)value);
    return send_response(fd, ret, NULL, 0);
}

static int handle_isp_set_wdr(int fd, const uint8_t *req, uint32_t req_len)
{
    /*
     * libre_anyka_app SDK: ak_vpss_open_wdr() and ak_vpss_close_wdr() are not
     * exported by libplat_vpss.so. WDR control is not available in this SDK variant.
     */
    (void)req;
    (void)req_len;
    log_debug("[isp] set_wdr: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/*
 * IR filter: use the vi day/night switch rather than vpss effect.
 * req: u64 vi_handle + i32 mode  (0 = day, 1 = night)
 */
static int handle_isp_set_ir_filter(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < 8 + 4)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    uint64_t h64;
    int32_t mode;
    memcpy(&h64,  req,     sizeof(h64));
    memcpy(&mode, req + 8, sizeof(mode));
    void *vi_handle = (void *)(uintptr_t)h64;

    enum video_daynight_mode dn = (mode != 0) ? VI_MODE_NIGHT : VI_MODE_DAY;
    log_debug("[isp] set_ir_filter vi=%p mode=%d", vi_handle, (int)dn);
    int ret = ak_vi_switch_mode(vi_handle, dn);
    return send_response(fd, ret, NULL, 0);
}

/* ---- Utility handlers ---------------------------------------------------- */

static int handle_get_error_no(int fd)
{
    int32_t err = (int32_t)ak_get_error_no();
    return send_response(fd, STATUS_OK, &err, sizeof(err));
}

static int handle_get_error_str(int fd)
{
    int err_no = ak_get_error_no();
    char *msg = ak_get_error_str(err_no);
    if (msg == NULL)
        msg = "(null)";
    uint32_t slen = (uint32_t)(strlen(msg) + 1);
    return send_response(fd, STATUS_OK, msg, slen);
}

/* ---- Control client helpers ---------------------------------------------- */

/**
 * Check whether cmd_id is a hardware lifecycle command that only the
 * control client is allowed to issue.
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
 * Try to acquire control-client status for fd.
 * Returns 1 if fd is (or just became) the control client, 0 otherwise.
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

/**
 * Release control-client status when the control client disconnects.
 */
static void release_control(int fd)
{
    if (g_control_fd == fd) {
        log_info("[daemon] control client fd=%d released", fd);
        g_control_fd = -1;
    }
}

/* ---- Main request dispatcher --------------------------------------------- */

/**
 * process_request - read one IPC request and dispatch to handler.
 *
 * Returns:
 *   0  – success, continue loop
 *  -1  – I/O or protocol error; caller should close client fd
 *  -2  – CMD_SHUTDOWN; caller should close client fd AND stop accept loop
 */
static int process_request(int fd)
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
        ret = handle_venc_get_stream(fd, req_buf, req_len);
        break;
    case CMD_VENC_RELEASE_STREAM:
        ret = handle_venc_release_stream(fd, req_buf, req_len);
        break;
    case CMD_VENC_CANCEL_STREAM:
        ret = handle_venc_cancel_stream(fd, req_buf, req_len);
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

/* ---- main ---------------------------------------------------------------- */

int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;

    /* ================================================================
     * LOG FILE PATH INITIALIZATION
     * ================================================================ */

    /* Determine log file path from environment variable or use default.
     * Env var: VENDOR_DAEMON_LOG_FILE
     * Default: /mnt/logs/vendor_daemon.log
     */
    char log_file_path[LOG_FILE_PATH_MAX];
    const char *env_log_path = getenv("VENDOR_DAEMON_LOG_FILE");
    if (env_log_path && env_log_path[0] != '\0') {
        strncpy(log_file_path, env_log_path, LOG_FILE_PATH_MAX - 1);
        log_file_path[LOG_FILE_PATH_MAX - 1] = '\0';
    } else {
        strncpy(log_file_path, LOG_FILE_PATH_DEFAULT, LOG_FILE_PATH_MAX - 1);
        log_file_path[LOG_FILE_PATH_MAX - 1] = '\0';
    }

    /* ================================================================
     * LOGGING INITIALIZATION
     * ================================================================ */

    /* Open log file */
    g_log_fp = fopen(log_file_path, "a");
    if (!g_log_fp) {
        fprintf(stderr, "Failed to open log file %s: %s\n",
                log_file_path, strerror(errno));
        g_log_fp = NULL;
    }

    /* Save original stdout/stderr for restoration on shutdown */
    g_saved_stdout = dup(STDOUT_FILENO);
    g_saved_stderr = dup(STDERR_FILENO);

    /* Redirect stdout/stderr to log file to capture Anyka SDK ak_print() */
    if (g_log_fp) {
        dup2(fileno(g_log_fp), STDOUT_FILENO);
        dup2(fileno(g_log_fp), STDERR_FILENO);
    }

    /* Initialize log.c - quiet mode suppresses stderr (we redirected it) */
    log_set_level(LOG_DEBUG);
    if (g_log_fp) {
        log_add_fp(g_log_fp, LOG_DEBUG);
        log_set_quiet(true);  /* Only write via file callback, not stderr */
    }

    log_info("========================================");
    log_info("vendor-daemon starting");
    log_info("log file: %s", log_file_path);

    /* Configure Anyka SDK logging:
     * - Set print level to DEBUG so all SDK messages go to stdout
     *   (which we redirected to our log file)
     * - Disable syslog output to avoid duplicate messages
     */
    ak_print_set_level(LOG_LEVEL_DEBUG);
    ak_print_set_syslog_level(0);
    log_info("SDK logging: level=DEBUG, syslog=disabled");

    /* ================================================================
     * SIGNAL HANDLING
     * ================================================================ */

    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = signal_handler;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGINT,  &sa, NULL);
    sigaction(SIGTERM, &sa, NULL);

    /* Ignore SIGPIPE so write errors return EPIPE instead of killing us */
    signal(SIGPIPE, SIG_IGN);

    /* Initialise pending-release table */
    memset(g_pending, 0, sizeof(g_pending));

    /* ================================================================
     * SHARED MEMORY RING BUFFER INITIALIZATION (Approach A)
     * ================================================================ */

    g_ring_buffer = vd_ring_create();
    if (g_ring_buffer) {
        log_info("[daemon] shared memory ring buffer initialized (%d slots, %d bytes/slot)",
                 VD_SHM_SLOT_COUNT, VD_SHM_SLOT_DATA_SIZE);
    } else {
        log_warn("[daemon] shared memory init failed, using socket-only mode");
    }

    /* ================================================================
     * DUAL SOCKET SETUP (Approach B Phase 3)
     * ================================================================ */

    /* Control socket: handles all IPC commands */
    g_ctrl_server_fd = create_unix_socket(CTRL_SOCKET_PATH);
    if (g_ctrl_server_fd < 0) {
        log_fatal("[daemon] control socket creation failed");
        goto shutdown;
    }
    log_info("[daemon] control socket listening on %s", CTRL_SOCKET_PATH);

    /* Frame socket: dedicated for frame delivery (optional) */
    g_frame_server_fd = create_unix_socket(FRAME_SOCKET_PATH);
    if (g_frame_server_fd < 0) {
        log_warn("[daemon] frame socket creation failed, using legacy single-socket mode");
        g_frame_server_fd = -1;
    } else {
        log_info("[daemon] frame socket listening on %s", FRAME_SOCKET_PATH);
    }

    /* ================================================================
     * MAIN EVENT LOOP (poll-based multiplexing)
     * ================================================================
     *
     * fds[0]         = ctrl_server_fd (control socket)
     * fds[1]         = frame_server_fd (frame socket, if available)
     * fds[2..nfds-1] = connected client fds
     *
     * This replaces the old blocking accept()-then-inner-loop model so
     * multiple clients can be serviced without queuing in the kernel
     * backlog.  Each client with pending data is serviced round-robin.
     *
     * Frame socket accepts at most 1 client (dedicated frame reader).
     */

    /* pollfd array: [ctrl_server, frame_server (optional), clients...] */
    struct pollfd fds[MAX_CLIENTS + 2];  /* +2 for dual sockets */
    int nfds = 0;

    memset(fds, 0, sizeof(fds));

    /* Add control server socket */
    fds[nfds].fd = g_ctrl_server_fd;
    fds[nfds].events = POLLIN;
    nfds++;

    /* Add frame server socket (if available) */
    if (g_frame_server_fd >= 0) {
        fds[nfds].fd = g_frame_server_fd;
        fds[nfds].events = POLLIN;
        nfds++;
    }

    while (!g_shutdown) {
        int ready = poll(fds, nfds, 1000); /* 1s timeout for shutdown check */
        if (ready < 0) {
            if (errno == EINTR)
                continue;
            log_error("poll: %s", strerror(errno));
            break;
        }
        if (ready == 0)
            continue; /* timeout – re-check g_shutdown */

        /* ── Accept new connections on control socket ─────────────────── */
        if (fds[0].revents & POLLIN) {
            int client_fd = accept(g_ctrl_server_fd, NULL, NULL);
            if (client_fd >= 0) {
                if (nfds < MAX_CLIENTS + 2) {
                    fds[nfds].fd = client_fd;
                    fds[nfds].events = POLLIN;
                    fds[nfds].revents = 0;
                    nfds++;
                    log_info("[daemon] control client connected fd=%d (total=%d)",
                             client_fd, nfds - 1);
                } else {
                    log_error("[daemon] max clients (%d) reached, rejecting fd=%d",
                              MAX_CLIENTS, client_fd);
                    close(client_fd);
                }
            } else if (errno != EINTR) {
                log_error("accept (control): %s", strerror(errno));
            }
        }

        /* ── Accept new connection on frame socket ───────────────────── */
        if (g_frame_server_fd >= 0 && fds[1].revents & POLLIN) {
            if (g_frame_client_fd >= 0) {
                /* Already have a frame client - reject new connection */
                int reject_fd = accept(g_frame_server_fd, NULL, NULL);
                if (reject_fd >= 0) {
                    log_warn("[daemon] frame client already connected, rejecting new connection");
                    close(reject_fd);
                }
            } else {
                int client_fd = accept(g_frame_server_fd, NULL, NULL);
                if (client_fd >= 0) {
                    g_frame_client_fd = client_fd;
                    /* Add to poll array after control clients */
                    fds[nfds].fd = client_fd;
                    fds[nfds].events = POLLIN;
                    fds[nfds].revents = 0;
                    nfds++;
                    log_info("[daemon] frame client connected fd=%d", client_fd);
                } else if (errno != EINTR) {
                    log_error("accept (frame): %s", strerror(errno));
                }
            }
        }

        /* ── Service clients with pending data (round-robin) ────────── */
        int i;
        /* First client index depends on how many server sockets we have */
        int first_client_idx;
        if (g_frame_server_fd >= 0) {
            first_client_idx = 2;  /* fds[0]=ctrl_server, fds[1]=frame_server, clients start at 2 */
        } else {
            first_client_idx = 1;  /* fds[0]=ctrl_server, clients start at 1 */
        }

        for (i = first_client_idx; i < nfds; i++) {
            if (!(fds[i].revents & (POLLIN | POLLHUP | POLLERR)))
                continue;

            /* Check if this is the frame client disconnecting */
            if (fds[i].fd == g_frame_client_fd && (fds[i].revents & (POLLHUP | POLLERR))) {
                log_info("[daemon] frame client disconnected fd=%d", fds[i].fd);
                cleanup_pending_for_fd(fds[i].fd);
                close(fds[i].fd);
                g_frame_client_fd = -1;
                /* Compact: move last entry into this slot */
                fds[i] = fds[nfds - 1];
                nfds--;
                i--; /* re-check this slot */
                continue;
            }

            int ret = process_request(fds[i].fd);
            if (ret == -1) {
                /* Client disconnected or I/O error */
                int client_fd = fds[i].fd;
                
                /* Check if it's the frame client */
                if (client_fd == g_frame_client_fd) {
                    g_frame_client_fd = -1;
                    log_info("[daemon] frame client fd=%d disconnected", client_fd);
                } else {
                    log_info("[daemon] control client fd=%d disconnected", client_fd);
                }
                
                cleanup_pending_for_fd(client_fd);
                release_control(client_fd);
                close(client_fd);
                
                /* Compact: move last entry into this slot */
                fds[i] = fds[nfds - 1];
                nfds--;
                i--; /* re-check this slot */
            } else if (ret == -2) {
                /* CMD_SHUTDOWN */
                g_shutdown = 1;
                break;
            }
        }
    }

    /* Clean up any remaining client connections */
    {
        int i;
        /* First client index depends on how many server sockets we had */
        int first_client_idx;
        if (g_frame_server_fd >= 0) {
            first_client_idx = 2;
        } else {
            first_client_idx = 1;
        }
        for (i = first_client_idx; i < nfds; i++) {
            cleanup_pending_for_fd(fds[i].fd);
            release_control(fds[i].fd);
            close(fds[i].fd);
        }
    }

shutdown:
    /* ================================================================
     * SHUTDOWN
     * ================================================================ */

    log_info("[daemon] shutting down");

    /* Close frame socket */
    if (g_frame_server_fd >= 0) {
        close(g_frame_server_fd);
        unlink(FRAME_SOCKET_PATH);
    }

    /* Close frame client */
    if (g_frame_client_fd >= 0) {
        close(g_frame_client_fd);
        g_frame_client_fd = -1;
    }

    /* Close control socket */
    if (g_ctrl_server_fd >= 0) {
        close(g_ctrl_server_fd);
        unlink(CTRL_SOCKET_PATH);
    }

    /* Clean up shared memory ring buffer */
    if (g_ring_buffer) {
        vd_ring_shutdown(g_ring_buffer);
        vd_ring_destroy(g_ring_buffer, 1);
        g_ring_buffer = NULL;
    }

    log_info("vendor-daemon stopped");
    log_info("========================================");

    /* Restore original stdout/stderr */
    if (g_saved_stdout >= 0) {
        dup2(g_saved_stdout, STDOUT_FILENO);
        close(g_saved_stdout);
    }
    if (g_saved_stderr >= 0) {
        dup2(g_saved_stderr, STDERR_FILENO);
        close(g_saved_stderr);
    }
    if (g_log_fp) {
        fclose(g_log_fp);
    }

    return 0;
}
