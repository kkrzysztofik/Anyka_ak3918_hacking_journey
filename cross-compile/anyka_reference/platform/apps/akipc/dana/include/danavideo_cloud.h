#ifndef DANA_VIDEO_IPC_CLOUD_UTILITY_H
#define DANA_VIDEO_IPC_CLOUD_UTILITY_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum _danavideo_cloud_alarm_e {
    DANAVIDEO_CLOUD_ALARM_NO    = 0,    // no alarm
    DANAVIDEO_CLOUD_ALARM_MD    = 1,    // motion detection alarm
    DANAVIDEO_CLOUD_ALARM_VB    = 2,    // voice/audio detection alarm
    DANAVIDEO_CLOUD_ALARM_UD_1  = 3,    // user-defined alarm 1
    DANAVIDEO_CLOUD_ALARM_UD_2  = 4,    // user-defined alarm 2
} danavideo_cloud_alarm_t;

typedef enum _danavideo_clodu_mode_e {
    DANAVIDEO_CLOUD_MODE_UNKNOWN = 0,
    DANAVIDEO_CLOUD_MODE_REALTIME,
    DANAVIDEO_CLOUD_MODE_ALARM,
} danavideo_cloud_mode_t;

uint32_t lib_danavideo_cloud_linked_version(void);
char *lib_danavideo_cloud_linked_version_str(uint32_t version);

// callback invoked when the cloud mode changes
// cloud_mode: the new cloud mode; chan_no: the channel number
typedef void (*danavideo_cloud_mode_changed_callback_t) (const danavideo_cloud_mode_t cloud_mode, const int32_t chan_no);
void lib_danavideo_cloud_set_cloud_mode_changed(danavideo_cloud_mode_changed_callback_t fun);

// initialize the cloud upload module
bool lib_danavideo_cloud_init(const uint32_t chan_num, const int32_t maximum_buffering_data, const int32_t package_size, danavideo_cloud_mode_t mode);

// enable realtime cloud upload
bool lib_danavideo_cloud_realtime_on();

bool lib_danavideo_cloud_realtime_off();

// msg_type: see danavideo_msg_type_t for valid values
/*
 * typedef enum _danavideo_msg_type {
 *    audio_stream = 0x20000000,
 *    video_stream = 0x40000000,
 *    extend_data   = 0x60000000,
 *    pic_stream   = 0x80000000,
 * } danavideo_msg_type_t;
 */
// codec: see danavidecodec_t for valid values
/*
 * typedef enum _danavideo_codec_type {
 *     H264    = 1,
 *     MPEG    = 2,
 *     MJPEG   = 3,
 *     H265    = 4,
 *     H265_HISILICON    = 5,
 *     MJPEG_DIFT        = 6,
 *     G711A   = 101,
 *     ULAW    = 102,
 *     G711U   = 103,
 *     PCM     = 104,
 *     ADPCM   = 105,
 *     G721    = 106,
 *     G723    = 107,
 *     G726_16 = 108,
 *     G726_24 = 109,
 *     G726_32 = 110,
 *     G726_40 = 111,
 *     AAC     = 112,
 *     JPG     = 200,
 * } danavidecodec_t;
 */
bool lib_danavideo_cloud_realtime_upload(const uint32_t chan_no, const uint32_t msg_type, const uint32_t codec, const uint32_t is_keyfram, const uint32_t timestamp, const danavideo_cloud_alarm_t alarm, const char *data, const uint32_t data_len, const uint32_t timeout_usec);

// realtime upload & file upload
// for file upload, register one upload task; it is different from storage path and far-end cloud upload path
// after upload completes (success or failure), this callback is invoked to notify the user which file was uploaded; the user's callback can then handle it (e.g., delete the file)
typedef void (*danavideo_cloud_customfile_upload_callback_t) (const int8_t retcode, const char *file_name, const char *file_path);

// register a custom file upload task
// after calling this interface, use save_site & save_path with lib_danavideo_util_pushmsg
bool lib_danavideo_cloud_customfile_async_upload_pre(const uint32_t ch_no, const char *file_name, const char *file_path, uint32_t *save_site, char *save_path, const size_t save_path_len);
// for alarm upload, it is recommended to record a local video file at the same time (currently supports mp4 files)
// after the upload completes, use lib_danavideo_cloud_realtime_upload callback to delete the alarm recording file to save space
bool lib_danavideo_cloud_customfile_async_upload(const uint32_t ch_no, const char *file_name, const char *file_path, danavideo_cloud_customfile_upload_callback_t fun);

bool lib_danavideo_cloud_deinit();

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif
