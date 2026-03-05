#ifndef DANA_VIDEO_H
#define DANA_VIDEO_H

#include <stdint.h>
#include <stdbool.h>
#include <pthread.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum _dana_video_cmd_e {
    DANAVIDEOCMD_DEVDEF,		// 0
    DANAVIDEOCMD_DEVREBOOT, 	// 1
    DANAVIDEOCMD_GETSCREEN,		// 2
    DANAVIDEOCMD_GETALARM,		// 3
    DANAVIDEOCMD_GETBASEINFO,	// 4
    DANAVIDEOCMD_GETCOLOR,		// 5
    DANAVIDEOCMD_GETFLIP,		// 6
    DANAVIDEOCMD_GETFUNLIST,	// 7
    DANAVIDEOCMD_GETNETINFO,	// 8
    DANAVIDEOCMD_GETPOWERFREQ,	// 9
    DANAVIDEOCMD_GETTIME,		// 10
    DANAVIDEOCMD_GETWIFIAP,		// 11
    DANAVIDEOCMD_GETWIFI,		// 12
    DANAVIDEOCMD_PTZCTRL,		// 13
    DANAVIDEOCMD_SDCFORMAT,		// 14
    DANAVIDEOCMD_SETALARM,		// 15
    DANAVIDEOCMD_SETCHAN,		// 16
    DANAVIDEOCMD_SETCOLOR,		// 17
    DANAVIDEOCMD_SETFLIP,		// 18
    DANAVIDEOCMD_SETNETINFO,	// 19
    DANAVIDEOCMD_SETPOWERFREQ,	// 20
    DANAVIDEOCMD_SETTIME,		// 21
    DANAVIDEOCMD_SETVIDEO,		// 22
    DANAVIDEOCMD_SETWIFIAP,		// 23
    DANAVIDEOCMD_SETWIFI,		// 24
    DANAVIDEOCMD_STARTAUDIO,	// 25
    DANAVIDEOCMD_STARTTALKBACK,	// 26
    DANAVIDEOCMD_STARTVIDEO,	// 27
    DANAVIDEOCMD_STOPAUDIO, 	// 28
    DANAVIDEOCMD_STOPTALKBACK,	// 29
    DANAVIDEOCMD_STOPVIDEO,		// 30
    DANAVIDEOCMD_RECLIST,		// 31
    DANAVIDEOCMD_RECPLAY,		// 32
    DANAVIDEOCMD_RECSTOP,		// 33
    DANAVIDEOCMD_RECACTION,		// 34
    DANAVIDEOCMD_RECSETRATE,	// 35
    DANAVIDEOCMD_RECPLANGET,	// 36
    DANAVIDEOCMD_RECPLANSET,	// 37
    DANAVIDEOCMD_EXTENDMETHOD,	// 38
    DANAVIDEOCMD_SETOSD,		// 39
    DANAVIDEOCMD_GETOSD,		// 40
    DANAVIDEOCMD_SETCHANNAME,	// 41
    DANAVIDEOCMD_GETCHANNAME,	// 42

    DANAVIDEOCMD_RESOLVECMDFAILED,// 43

    DANAVIDEOCMD_CALLPSP = 66,
    DANAVIDEOCMD_GETPSP,
    DANAVIDEOCMD_SETPSP,
    DANAVIDEOCMD_SETPSPDEF,
    
    DANAVIDEOCMD_GETLAYOUT = 70,
    DANAVIDEOCMD_SETCHANADV,

    DANAVIDEOCMD_SETICR = 78,
    DANAVIDEOCMD_GETICR,

    DANAVIDEOCMD_SETBC = 80,
    DANAVIDEOCMD_GETBC = 81,

    DANAVIDEOCMD_GETSDCSTATUS = 82,
    DANAVIDEOCMD_GETVIDEO  = 83,

    DANAVIDEOCMD_GETUSERPASS = 85,
    DANAVIDEOCMD_SETUSERPASS,
    DANAVIDEOCMD_CONTROLLED = 87,
    //20170927
    DANAVIDEOCMD_GETMOTRACK = 90,
    DANAVIDEOCMD_SETMOTRACK = 91,

} dana_video_cmd_t;

typedef void* pdana_video_conn_t;

typedef struct _dana_video_callback_funs_s {
    uint32_t (*danavideoconn_created)(pdana_video_conn_t danavideoconn);
    void (*danavideoconn_aborted)(pdana_video_conn_t danavideoconn);
    void (*danavideoconn_command_handler)(pdana_video_conn_t danavideoconn, dana_video_cmd_t cmd, uint32_t trans_id, void* cmd_arg, uint32_t cmd_arg_len); 

} dana_video_callback_funs_t;

// called when ipc recved p2pserver's heartbeat response
typedef void (*danavideo_hb_is_ok_callback_t) (void);

// called when ipc check that the conn with p2p2server is not avliable
typedef void (*danavideo_hb_is_not_ok_callback_t) (void);

// called when need upgrade rom
typedef void (*danavideo_upgrade_rom_callback_t) (const char* rom_path,  const char *rom_md5, const uint32_t rom_size); // unit: Byte

// called when need autoset
typedef void (*danavideo_autoset_callback_t) (const uint32_t power_freq, const int64_t now_time, const char *time_zone, const char *ntp_server1, const char *ntp_server2);
// power frequency: 0 = 50Hz, 1 = 60Hz
// now_time unit: s
// time_zone: time zone
// NTP server 1 and 2


// called when need setwifi
// setwifi has four cases: one is receiving this command via command_handler
//                         one is via callback function (local mode)
//                         there is also smart voice control mode
//                         DanaAirLink
//              ip_type  0: fixed 1:dhcp
//              enc_type see danavideo_cmd.h
typedef void (*danavideo_local_setwifiap_callback_t) (const uint32_t ch_no, const uint32_t ip_type, const char *ipaddr, const char *netmask, const char *gateway, const char *dns_name1, const char *dns_name2, const char *essid, const char *auth_key, const uint32_t enc_type);

// called when need local auth
// 0 succeeded
// 1 failed
typedef uint32_t (*danavideo_local_auth_callback_t) (const char *user_name, const char *user_pass);

// called when create a new conf or update a exited conf
typedef void (*danavideo_conf_created_or_updated_t) (const char *conf_absolute_pathname);

// called when need get the connected wifi wifi-quality [0-100] (if no wifi set to 0)
typedef uint32_t (*danavideo_get_connected_wifi_quality_callback_t) ();

// called when recv productsetdeviceinfo 
typedef void (*danavideo_productsetdeviceinfo_callback_t) (const char *model, const char *sn, const char *hw_mac);

typedef enum _danavideo_msg_type {
   audio_stream = 0x20000000,
   video_stream = 0x40000000,
   extend_data   = 0x60000000,
   pic_stream   = 0x80000000,
   dana_data      = 0xa0000000,
} danavideo_msg_type_t;

typedef enum _danavideo_codec_type {
    H264    = 1,
    MPEG    = 2,
    MJPEG   = 3,
    H265    = 4,
    H265_HISILICON    = 5,
    MJPEG_DIFT        = 6,
    G711A   = 101,
    ULAW    = 102,
    G711U   = 103,
    PCM     = 104,
    ADPCM   = 105,
    G721    = 106,
    G723    = 107,
    G726_16 = 108,
    G726_24 = 109,
    G726_32 = 110,
    G726_40 = 111,
    AAC     = 112,
    JPG     = 200,
} danavidecodec_t;

typedef struct _dana_packet_s {
    char    *data;
    int32_t   len;
} dana_packet_t;

typedef struct _dana_audio_packet_s {
    char  *data;
    int32_t len;
    danavidecodec_t  codec; // audio encoding
} dana_audio_packet_t;


// used to set video send buffer
// default 2M, minimum 2M
void lib_danavideo_set_maximum_buffering_data_size(const int32_t size);

// used to set lib_danavideo_start timeout
void lib_danavideo_set_startup_timeout(const uint32_t timeout_sec);

void lib_danavideo_set_hb_is_ok(danavideo_hb_is_ok_callback_t fun);
void lib_danavideo_set_hb_is_not_ok(danavideo_hb_is_not_ok_callback_t fun);

void lib_danavideo_set_upgrade_rom(danavideo_upgrade_rom_callback_t fun);

void lib_danavideo_set_autoset(danavideo_autoset_callback_t fun);

void lib_danavideo_set_local_setwifiap(danavideo_local_setwifiap_callback_t fun);

void lib_danavideo_set_local_auth(danavideo_local_auth_callback_t fun);

void lib_danavideo_set_conf_created_or_updated(danavideo_conf_created_or_updated_t fun);

void lib_danavideo_set_get_connected_wifi_quality(danavideo_get_connected_wifi_quality_callback_t fun);

void lib_danavideo_set_productsetdeviceinfo(danavideo_productsetdeviceinfo_callback_t fun);

uint32_t lib_danavideo_linked_version(void);
char * lib_danavideo_linked_version_str(uint32_t version);

char * lib_danavideo_deviceid_from_conf(const char *danale_path);

char * lib_danavideo_deviceid();

// ***local listen port***
// if called before lib_danavideo_start, the configured port will be used (fixed mode or freely chosen in non-fixed mode)
// if called after lib_danavideo_start, it will default to starting 34102, then check if the configured port matches 34102; if different it will close 34102 and start the user-configured port, otherwise return directly

void lib_danavideo_set_listen_port(const bool listen_port_fixed, const uint16_t listen_port);
uint16_t lib_danavideo_get_listen_port(); 


// create config file
// must be called after lib_danavideo_init
// if the config file in the specified directory exists and is valid, it will not be created
// otherwise a new config file will be created
// non-blocking
bool lib_danavideo_create_on_check_conf();

// update send data interface
// add lib_danavideoconn_send
// cancel danavideo_msg_max_size
// cancel lib_danavideo_packet_create
//     lib_danavideo_packet_destroy
//     lib_danavideoconn_sendvideomsg
//     lib_danavideoconn_sendaudiomsg
//     lib_danavideoconn_sendpicmsg
// update danavideo_msg_type_t: change rec_stream to extend_data to support sending extend_data
// support sending video audio pic extend_data
bool lib_danavideoconn_send(pdana_video_conn_t danavideoconn, const danavideo_msg_type_t msg_type, const danavidecodec_t codec, const uint8_t ch_no, const uint8_t is_keyframe, const uint32_t timestamp, const char* data, const uint32_t data_len, const uint32_t timeout_usec);

typedef void (*danadata_read_callback_t)(pdana_video_conn_t danavideoconn, const char *data, const int32_t len);

void lib_danadata_set_read(danadata_read_callback_t fun);

bool lib_danadataconn_send(pdana_video_conn_t danavideoconn, const danavideo_msg_type_t msg_type, const char* data, const uint32_t data_len, const uint32_t timeout_usec);


dana_audio_packet_t* lib_danavideoconn_readaudio(pdana_video_conn_t danavideoconn, uint32_t timeout_usec); // call lib_danavideo_audio_packet_destroy to free
void lib_danavideo_audio_packet_destroy(dana_audio_packet_t *danaaudiomsg);

// lib_danavideo_init no longer automatically creates the config file; when the config file does not exist (or is invalid), it returns false directly
bool lib_danavideo_init(const char *danale_path, const char *agent_user, const char *agent_pass, const char *chip_code, const char *scheme_code, const char *product_code, dana_video_callback_funs_t *danavideocallbackfuns);

uint32_t lib_danavideo_set_userdata(pdana_video_conn_t danavideoconn, void *userdata);

uint32_t lib_danavideo_get_userdata(pdana_video_conn_t danavideoconn, void **userdata);

bool lib_danavideo_start();

bool lib_danavideo_cmd_exec_response(pdana_video_conn_t danavideoconn, const dana_video_cmd_t cmd, char* error_method, const uint32_t trans_id, const uint32_t code, const char* code_msg); 

bool lib_danavideo_cmd_setvideo_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t fps);
bool lib_danavideo_cmd_startvideo_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t fps);
// audio_codec default G711A  (danavidecodec_t)
// sample_rate default 8000Hz
// sample_bit  default 16bit
// track       default mono   (1 mono; 2 stereo)
bool lib_danavideo_cmd_startaudio_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t *audio_codec, const uint32_t *sample_rate, const uint32_t *sample_bit, const uint32_t *track);
bool lib_danavideo_cmd_getalarm_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t motion_detection, const uint32_t opensound_detection, const uint32_t openi2o_detection, const uint32_t smoke_detection, const uint32_t shadow_detection, const uint32_t other_detection, const uint32_t *pir_detection);
bool lib_danavideo_cmd_getbaseinfo_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const char *dana_id, const char *api_ver, const char *sn, const char *device_name, const char *rom_ver, const uint32_t device_type, const uint32_t ch_num, const uint64_t sdc_size, const uint64_t sdc_free, const size_t work_channel_count, const uint32_t *work_channel); // dana_id < 49; api_ver < 129; sn < 129; device_name < 129; rom_ver < 129 work_channel <=48// DONE
bool lib_danavideo_cmd_getcolor_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t brightness, const uint32_t contrast, const uint32_t saturation, const uint32_t hue);
bool lib_danavideo_cmd_getflip_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t flip_type);
bool lib_danavideo_cmd_getfunlist_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t methodes_count, const char **methodes); // methodes_count <= 160
bool lib_danavideo_cmd_getnetinfo_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t ip_type, const char *ipaddr, const char *netmask, const char *gateway, const uint32_t dns_type, const char *dns_name1, const char *dns_name2, const uint32_t http_port); // ipaddr < 16; netmask < 16; gataway < 16; dns_type < 10; dns_name1 < 16; dns_name2 < 16
bool lib_danavideo_cmd_getpowerfreq_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t freq);
bool lib_danavideo_cmd_gettime_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const int64_t now_time, const char *time_zone, const char *ntp_server_1, const char *ntp_server_2); // time_zone < 65

typedef struct _libdanavideo_wifiinfo_s {
    char essid[33];
    uint32_t enc_type;
    uint32_t quality;
} libdanavideo_wifiinfo_t;

bool lib_danavideo_cmd_getwifiap_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t wifi_device, const uint32_t wifi_list_count, const libdanavideo_wifiinfo_t *wifi_list); // wifi_list_count <= 20;  essid < 33
bool lib_danavideo_cmd_getwifi_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const char *essid, const char *auth_key, const uint32_t enc_type); // essid < 33; auth_key < 65
bool lib_danavideo_cmd_starttalkback_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t audio_codec, const uint32_t * sample_rate, const uint32_t *sample_bit, const uint32_t *track);


typedef struct _libdanavideo_reclist_recordinfo_s {
    int64_t start_time;
    uint32_t length;
    uint32_t record_type;
} libdanavideo_reclist_recordinfo_t;

bool lib_danavideo_cmd_reclist_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t rec_lists_count, const libdanavideo_reclist_recordinfo_t *rec_lists); // rec_lists_count <= 35

typedef struct _libdanavideo_recplanget_recplan_s {
    uint32_t record_no;
    size_t week_count; // <= 7  indicates how many days are in the week array
    uint32_t week[7];
    char start_time[33]; // hour:minute:second 12:12:12
    char end_time[33];
    uint32_t status;
} libdanavideo_recplanget_recplan_t;

bool lib_danavideo_cmd_recplanget_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t rec_plans_count, const libdanavideo_recplanget_recplan_t *rec_plans); // rec_plans_count <= 3

typedef struct _libdanavideo_osdinfo_s {
    uint32_t chan_name_show;
    uint32_t show_name_x;
    uint32_t show_name_y;

    uint32_t datetime_show;
    uint32_t show_datetime_x;
    uint32_t show_datetime_y;
    uint32_t show_format;
    uint32_t hour_format;
    uint32_t show_week;
    uint32_t datetime_attr;

    uint32_t custom1_show;
    char     show_custom1_str[45];
    uint32_t show_custom1_x;
    uint32_t show_custom1_y;

    uint32_t custom2_show;
    char     show_custom2_str[45];
    uint32_t show_custom2_x;
    uint32_t show_custom2_y;
} libdanavideo_osdinfo_t;

bool lib_danavideo_cmd_getosd_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char * code_msg, const libdanavideo_osdinfo_t *osdinfo);

bool lib_danavideo_cmd_getchanname_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char * code_msg, const char *chan_name);

typedef struct _libdanavideo_pspinfo_s {
    uint32_t psp_id;
    char psp_name[60];
    bool psp_default;
    bool is_set;
} libdanavideo_pspinfo_t;

bool lib_danavideo_cmd_getpsp_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char * code_msg, const uint32_t total, const uint32_t psp_count, const libdanavideo_pspinfo_t *psp); // psp_count <=10

bool lib_danavideo_cmd_getlayout_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char * code_msg, const uint32_t matrix_x, const uint32_t matrix_y, const size_t chans_count, const uint32_t chans[64], const uint32_t layout_change, const uint32_t chan_pos_change, const size_t use_chs_count, const uint32_t use_chs[98]);

bool lib_danavideo_cmd_geticr_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char * code_msg, const uint32_t mode);

typedef struct dana_bcfilter_t {
    size_t size;
    uint8_t bytes[66];
} dana_bcfilter_t;

typedef struct _dana_bcinfo_s {
    uint32_t bc_id;
    bool has_bc_filter;
    dana_bcfilter_t bc_filter;
} dana_bcinfo_t;

bool lib_danavideo_cmd_getbc_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char * code_msg, const size_t bc_count, const dana_bcinfo_t bc[4]);

bool lib_danavideo_cmd_getsdcstatus_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char * code_msg, const uint32_t status, const uint32_t *format_progress, const uint32_t *sd_size, const uint32_t *sd_free);
bool lib_danavideo_cmd_getvideo_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char * code_msg, const uint32_t video_quality);

bool lib_danavideo_cmd_getuserpass_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const char *user_id, const char *user_pass);
//20170927
bool lib_danavideo_cmd_getmotrack_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t status);
bool lib_danavideo_cmd_controlled_response(pdana_video_conn_t danavideoconn, const uint32_t trans_id, uint32_t code, const char* code_msg, const uint32_t onoff);

//dangerous interface, use with caution
bool lib_danavideo_setdanaconn_kernel_buffer(pdana_video_conn_t danavideoconn, const uint32_t buffer_size);

bool lib_danavideo_stop();

bool lib_danavideo_deinit();


/////////////////////////////////////// utility class ////////////////////////////////////////

typedef enum _danavideo_device_type_e {
    DANAVIDEO_DEVICE_IPC = 1,
    DANAVIDEO_DEVICE_DVR,
    DANAVIDEO_DEVICE_NVR,
    DANAVIDEO_DEVICE_DVR_NO_MIX_NO_MULTI_CHANNEL, // does not support channel 0 (multi-stream video compositing), does not support simultaneous multi-stream transmission
    DANAVIDEO_DEVICE_NVR_NO_MIX_NO_MULTI_CHANNEL, // does not support channel 0 (multi-stream video compositing), does not support simultaneous multi-stream transmission
    DANAVIDEO_DEVICE_DVR_NO_MIX_MULTI_CHANNEL,   // does not support channel 0 (multi-stream video compositing), supports simultaneous multi-stream transmission
    DANAVIDEO_DEVICE_NVR_NO_MIX_MULTI_CHANNEL,   // does not support channel 0 (multi-stream video compositing), supports simultaneous multi-stream transmission
    DANAVIDEO_DEVICE_DVR_SPLIT,     // supports channel 0 (multi-stream video compositing), supports irregular split screen
    DANAVIDEO_DEVICE_NVR_SPLIT,     // supports channel 0 (multi-stream video compositing), supports irregular split screen

//cancel external release
#if 0
    DANA_DEVICE_RING = 60004,   // ring device type
    DANA_DEVICE_GARAGE_DOOR_OPENER_WITH_CAMERA = 60005, // camera-enabled garage door
#endif
    DANA_DEVICE_NAS = 60006,	

    DANAVIDEO_DEVICE_Unknown_type = -1,
} danavideo_device_type_t;

bool lib_danavideo_util_setdeviceinfo(const danavideo_device_type_t device_type, const uint32_t channel_num, const char *rom_ver, const char *api_ver, const char *rom_md5); // rom_ver < 129; api_ver < 129; rom_md5 < 65

typedef enum _dana_video_feature_e {
    // hardware features
    DANAVIDEO_FEATURE_HAVE_BATTERY          = 4097, // battery
    DANAVIDEO_FEATURE_HAVE_GPS              = 4098, // GPS
    DANAVIDEO_FEATURE_HAVE_PIR              = 4099, // PIR infrared (smoke, possibly rf433)
    DANAVIDEO_FEATURE_HAVE_G_SENSOR         = 4100, // accelerometer sensor
    DANAVIDEO_FEATURE_HAVE_GYRO_SENSOR      = 4101, // gyroscope sensor
    DANAVIDEO_FEATURE_HAVE_TEMP_SENSOR      = 4102, // temperature sensor
    DANAVIDEO_FEATURE_HAVE_HUMIDITY_SENSOR  = 4103, // humidity sensor
    DANAVIDEO_FEATURE_HAVE_MOBILENET        = 4104, // 3G/4G support
    DANAVIDEO_FEATURE_HAVE_PTZ_L_R          = 4105, // PTZ (pan-tilt-zoom) support (left-right)
    DANAVIDEO_FEATURE_HAVE_PTZ_U_D          = 4106, // PTZ (pan-tilt-zoom) support (up-down)
    DANAVIDEO_FEATURE_HAVE_PTZ_L_R_U_D      = 4107, // PTZ (pan-tilt-zoom) support (up-down, left-right)
    DANAVIDEO_FEATURE_HAVE_PTZ_DD           = 4108, // PTZ (pan-tilt-zoom) support (8-direction support)
    DANAVIDEO_FEATURE_HAVE_WIPER            = 4109, // wiper
    DANAVIDEO_FEATURE_HAVE_ZOOM_LENS        = 4110, // zoom lens ?
    DANAVIDEO_FEATURE_HAVE_ENLARGING_LENS   = 4111, // magnification lens ?
    DANAVIDEO_FEATURE_HAVE_SD               = 4112, // SD card support
    DANAVIDEO_FEATURE_HAVE_MIC              = 4113, // MIC support
    DANAVIDEO_FEATURE_HAVE_SPEAKER          = 4114, // intercom support (speaker)
    DANAVIDEO_FEATURE_HAVE_BLE_HEADSET      = 4115, // Bluetooth headset support
    DANAVIDEO_FEATURE_HAVE_RF315            = 4116, // support RF315
    DANAVIDEO_FEATURE_HAVE_RF433            = 4117, // support RF433
    DANAVIDEO_FEATURE_HAVE_RF866            = 4118, // support RF866
    DANAVIDEO_FEATURE_HAVE_ZIGBEE           = 4119, // support Zigbee
    DANAVIDEO_FEATURE_HAVE_BELL             = 4120, // doorbell
    DANAVIDEO_FEATURE_HAVE_2_WAY_VOICE      = 4121, // two-way voice intercom [echo cancellation]
    DANAVIDEO_FEATURE_HAVE_PSP              = 4138, // support preset points
    DANAVIDEO_FEATURE_HAVE_FISHEYE_INSCRIBCIRCLE     = 4139, // support fisheye inscribed circle
    DANAVIDEO_FEATURE_HAVE_1_WAY_VOICE               = 4146, //one-way voice intercom
    DANAVIDEO_FEATURE_HAVE_FISHEYE_CIRCUMCIRCLE      = 4147, // support Jiegao fisheye 2 circumcircle
    DANAVIDEO_FEATURE_HAVE_SOUND_ALARM      = 4149, // sound detection alarm (SOUND DETECTION ALARM)
    DANAVIDEO_FEATURE_HAVE_MOTION_ALARM     = 4150, // support motion detection alarm - non-doorbell (MOTION DETECTION ALARM)

    // extended features
    DANA_VIDEO_HAVE_CLOUD_STORAGE           = 8193, // cloud storage
    DANA_VIDEO_HAVE_PERSONAL_STORAGE        = 8194, // personal storage
    DANA_VIDEO_HAVE_SMART_HOME              = 8195, // smart home central control function (manages home smart security devices)
    DANA_VIDEO_HAVE_UPGRADE                 = 8196, // firmware upgrade function
    DANA_VIDEO_HAVE_UPGRADE_OTA             = 8200, // firmware upgrade function OTA (original 8196 is LOCAL)
    DANA_VIDEO_HAVE_VOICE_OPTIMIZED_MODE    = 8201, //support audio optimization feature
} dana_video_feature_t;

bool lib_danavideo_util_setdevicefeature(const size_t feature_list_count, const dana_video_feature_t feature_list[152]); // feature_list <= 152


// push notification info
// add alarm message level option  alarm_level
// standardize definition of alarm message type  msg_type
typedef enum _dana_video_pushmsg_alarm_level_e {
   DANA_VIDEO_PUSHMSG_ALARM_LEVEL_1 = 1,    //  low
   DANA_VIDEO_PUSHMSG_ALARM_LEVEL_2 = 2,    //  medium
   DANA_VIDEO_PUSHMSG_ALARM_LEVEL_3 = 3,    //  high
} dana_video_pushmsg_alarm_level_t;
typedef enum _dana_video_pushmsg_msg_type_e {
    DANA_VIDEO_PUSHMSG_MSG_TYPE_MOTION_DETECT   = 1,    //  motion detection
    DANA_VIDEO_PUSHMSG_MSG_TYPE_SOUND_DETECT    = 2,    //  sound detection
    DANA_VIDEO_PUSHMSG_MSG_TYPE_IR              = 3,    //  infrared
    DANA_VIDEO_PUSHMSG_MSG_TYPE_OTHER           = 4,    //  other
    DANA_VIDEO_PUSHMSG_MSG_TYPE_BODY_SENSOR     = 10,   //  body/human sensing
    DANA_VIDEO_PUSHMSG_MSG_TYPE_SMOKE_SENSOR    = 11,   //  smoke detection
    DANA_VIDEO_PUSHMSG_MSG_TYPE_DOOR_SENSOR     = 12,   //  door magnet opened
    DANA_VIDEO_PUSHMSG_MSG_TYPE_GLASS_BREAK     = 13,   //  glass break
    DANA_VIDEO_PUSHMSG_MSG_TYPE_COMBUSTIBLEGAS_EXCEEDED = 14,   // combustible gas exceeded threshold
    DANA_VIDEO_PUSHMSG_MSG_TYPE_DOOR_BELL       = 15,   // doorbell triggered / need to report cloud storage recording
    DANA_VIDEO_PUSHMSG_MSG_TYPE_DEMOLITION      = 16,   // tamper alarm
    DANA_VIDEO_PUSHMSG_MSG_TYPE_LOW_BATTERY     = 17,   // low battery alarm
    DANA_VIDEO_PUSHMSG_MSG_TYPE_PASSWD_INCORRECT = 18,   // incorrect password alarm
    DANA_VIDEO_PUSHMSG_MSG_TYPE_SOS             = 19,   // SOS
    DANA_VIDEO_PUSHMSG_MSG_TYPE_WATERLOGGING    = 20,   // water leak alarm
    DANA_VIDEO_PUSHMSG_MSG_TYPE_DEV_OFFLINE     = 21,   // device offline alarm
    DANA_VIDEO_PUSHMSG_MSG_TYPE_DEV_ONLINE      = 22,   // device online notification
    DANA_VIDEO_PUSHMSG_MSG_TYPE_BATTERY_POWERED = 23,   // device battery power notification
    DANA_VIDEO_PUSHMSG_MSG_TYPE_SENSOR_DETECT   = 31,   // sensor alarm
    DANA_VIDEO_PUSHMSG_MSG_TYPE_VLOST           = 32,   // video loss alarm
    DANA_VIDEO_PUSHMSG_MSG_TYPE_VMASK           = 33,   // video occlusion alarm
    DANA_VIDEO_PUSHMSG_MSG_TYPE_DISKFULL        = 34,   // disk full alarm
    DANA_VIDEO_PUSHMSG_MSG_TYPE_DISKERR         = 35,   // disk error alarm
    DANA_VIDEO_PUSHMSG_MSG_TYPE_DISK_NO_FORMAT  = 36,   // disk not formatted alarm
    DANA_VIDEO_PUSHMSG_MSG_TYPE_GARAGE_DOOR_TOGGLE  =  41,  // garage door state toggle [open/close state unknown]
    DANA_VIDEO_PUSHMSG_MSG_TYPE_GARAGE_DOOR_CLOSE   =  42,  // garage door closed
    DANA_VIDEO_PUSHMSG_MSG_TYPE_GARAGE_DOOR_OPEN    =  43,  // garage door opened
    DANA_VIDEO_PUSHMSG_MSG_TYPE_DOOR_BELL_PUSH      =  51,   // doorbell pressed / unrelated to cloud storage, only used to trigger App to open call interface
    DANA_VIDEO_PUSHMSG_MSG_TYPE_SYS_MSG         = 99,   //  system message (software upgrade, etc.)
} dana_video_pushmsg_msg_type_t;


bool lib_danavideo_cloud_realtime_lastsavepath(const uint32_t chan_no, const int64_t cur_time, char *save_path, const size_t save_path_len, uint32_t *offset);

bool lib_danavideo_util_pushmsg(const uint32_t ch_no, const uint32_t alarm_level, const uint32_t msg_type, const char *msg_title, const char *msg_body, const int64_t cur_time, const uint32_t att_flag, const char *att_path, const char *att_type, const uint32_t record_flag, const int64_t start_time, const uint32_t time_len, const uint32_t save_site, const char *save_path);

bool lib_danavideo_local_searchapp(const char *check_data, const uint32_t encrypt_flag);


// smart voice control
void lib_danavideo_smart_conf_init();
void lib_danavideo_smart_conf_parse(const int16_t *audio, int32_t size); // PCM audio data: 16-bit signed integer, sampling rate must be 48000 or 44100
// latest version requires 44100, and supports two-way communication

// user plays the sound directly
//
// note: user must first set the danale config file path
// or execute lib_danavideo_init once
void lib_danavideo_smart_conf_set_danalepath(const char *danale_path);

// set volume; each manufacturer needs to tune this on their own board; recommended value is 0x2400, not exceeding 0x8000
void lib_danavideo_smart_conf_set_volume(const uint32_t volume);

typedef void (*danavideo_smart_conf_response_callback_t) (const char *audio, size_t size);
void lib_danavideo_set_smart_conf_response(danavideo_smart_conf_response_callback_t fun);



// DanaAirLink
// need to first set the danale config file path (or execute lib_danavideo_init once)
bool danaairlink_set_danalepath(const char *danale_path);

typedef enum _danaairlink_chip_type_e {
    DANAAIRLINK_CHIP_TYPE_NORMAL,
    DANAAIRLINK_CHIP_TYPE_MT7601,
    DANAAIRLINK_CHIP_TYPE_RLT8188,
    DANAAIRLINK_CHIP_TYPE_AP6212,
    DANAAIRLINK_CHIP_TYPE_RTL8189,
    DANAAIRLINK_CHIP_TYPE_RT5572,
    DANAAIRLINK_CHIP_TYPE_RTL8811,
    DANAAIRLINK_CHIP_TYPE_MT7610,
    DANAAIRLINK_CHIP_TYPE_MRL8801,
    DANAAIRLINK_CHIP_TYPE_MT7628,
    DANAAIRLINK_CHIP_TYPE_UNKNOW = -1,
} danaairlink_chip_type_t;
bool danaairlink_init(const danaairlink_chip_type_t chip_type, const char *if_name);

// every time you need to enter the DanaAirLink configuration state, call this method once
// i.e., when the callback function (danavideo_local_setwifiap_callback_t) has been called and you need to enter the configuration state again, call this method
bool danaairlink_start_once(); 
bool danaairlink_stop();
bool danaairlink_deinit();

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif
