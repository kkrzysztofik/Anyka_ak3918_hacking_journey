#ifndef VENDOR_DAEMON_PROTOCOL_H
#define VENDOR_DAEMON_PROTOCOL_H

/* ---- IPC socket paths ---------------------------------------------------- */
#define CTRL_SOCKET_PATH        "/tmp/vd-ctrl.sock"   /* Formerly /tmp/vendor-daemon.sock */
#define FRAME_MAIN_SOCKET_PATH  "/tmp/vd-frame-main.sock"
#define FRAME_SUB_SOCKET_PATH   "/tmp/vd-frame-sub.sock"

/* ---- Request / response constants --------------------------------------- */
#define MAX_REQUEST_SIZE        (1 * 1024 * 1024)  /* 1 MB – for frame data */
#define STATUS_OK               0
#define STATUS_ERROR            (-1)
/* Handle belongs to a dead daemon generation, a reused slot, or the wrong
 * object kind. Distinct from STATUS_ERROR so the client logs "stale handle"
 * rather than a confusing argument error. */
#define VD_STATUS_STALE_EPOCH   (-2)

/* ---- Connection limits -------------------------------------------------- */
/* Maximum simultaneous clients (control + streaming + snapshot + spare) */
#define MAX_CLIENTS             4

/* ---- Logging ------------------------------------------------------------ */
#define LOG_FILE_PATH_DEFAULT   "/mnt/logs/vendor_daemon.log"
#define LOG_FILE_PATH_MAX       512

/* ---- SDK diagnostics ---------------------------------------------------- */
#define SDK_ERROR_NO_DATA       23

/* ---- Command IDs -------------------------------------------------------- */

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
    CMD_VENC_GET_STREAM           = 16,  /* reserved (pull path removed) */
    CMD_VENC_RELEASE_STREAM       = 17,  /* reserved (pull path removed) */
    CMD_VENC_CANCEL_STREAM        = 18,
    /* Push-based frame delivery (Fix 0) */
    CMD_VENC_START_PUSH           = 19,
    CMD_VENC_STOP_PUSH            = 20,
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

    /* ---- Session ---------------------------------------------------------
     * CMD_HELLO is the client's attach handshake.  It is the only command a
     * client may send before the epoch gate is satisfied, so it must never
     * require an existing session -- it is deliberately absent from
     * is_lifecycle_cmd(), which is what exempts it from acquire_control().
     * Response: [u32 epoch][u32 shm_version] = 8 bytes.
     */
    CMD_HELLO                     = 300,
    CMD_SHUTDOWN                  = 255
};

#endif /* VENDOR_DAEMON_PROTOCOL_H */
