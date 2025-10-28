/**
 * @file rtsp_types.h
 * @brief Common RTSP types, constants, and structures
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all common types, constants, and data structures
 * used across the RTSP implementation modules.
 */

#ifndef RTSP_TYPES_H
#define RTSP_TYPES_H

#include <bits/pthreadtypes.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/socket.h>
#include <time.h>

#include "platform/platform_common.h"
#include "services/common/video_config_types.h"

/* RTSP constants */
#define RTSP_MAX_CLIENTS         10
#define RTSP_BUFFER_SIZE         4096
#define RTSP_RTP_BUFFER_SIZE     1500
#define RTSP_MAX_URI_LEN         256
#define RTSP_SESSION_TIMEOUT_SEC 60
#define RTSP_RTCP_INTERVAL_SEC   5
#define RTSP_MAX_HEADER_LEN      1024
#define RTSP_MAX_USERNAME_LEN    64
#define RTSP_MAX_PASSWORD_LEN    64
#define RTSP_MAX_REALM_LEN       128
#define RTSP_MAX_NONCE_LEN       32
#define RTSP_MAX_RESPONSE_LEN    64
#define RTSP_SESSION_ID_SIZE     64  /* Maximum size for RTSP session identifier */

/* RTSP header field sizes */
#define RTSP_HEADER_NAME_SIZE    64  /* Maximum size for RTSP header field name */
#define RTSP_HEADER_VALUE_SIZE   256 /* Maximum size for RTSP header field value */

/* SDP media description field sizes */
#define SDP_PROTOCOL_SIZE        16  /* SDP media protocol string size */
#define SDP_ENCODING_SIZE        32  /* SDP media encoding name size */
#define SDP_CONTROL_SIZE         64  /* SDP media control URL size */
#define SDP_ATTRIBUTE_SIZE       256 /* SDP media attribute string size (fmtp, rtcp-fb, extmap) */
#define SDP_MID_SIZE             32  /* SDP media identifier size */
#define SDP_SSRC_SIZE            32  /* SDP media SSRC string size */

/* SDP session description field sizes */
#define SDP_ORIGIN_SIZE          128 /* SDP session origin string size */
#define SDP_SESSION_NAME_SIZE    128 /* SDP session name string size */
#define SDP_SESSION_INFO_SIZE    256 /* SDP session information string size */
#define SDP_URI_SIZE             256 /* SDP URI string size */
#define SDP_EMAIL_SIZE           128 /* SDP email address string size */
#define SDP_PHONE_SIZE           64  /* SDP phone number string size */
#define SDP_CONNECTION_SIZE      128 /* SDP connection information string size */
#define SDP_BANDWIDTH_SIZE       64  /* SDP bandwidth information string size */
#define SDP_TIMEZONE_SIZE        32  /* SDP timezone string size */
#define SDP_KEY_SIZE             64  /* SDP encryption key string size */
#define SDP_ATTRIBUTES_SIZE      512 /* SDP session attributes string size */

/* Stream configuration field sizes */
#define RTSP_STREAM_PATH_SIZE    64  /* Maximum size for RTSP stream path */
#define RTSP_STREAM_NAME_SIZE    64  /* Maximum size for RTSP stream name */
#define RTSP_H264_PARAM_B64_SIZE 256 /* Maximum size for H.264 parameter sets in base64 */

/* H.264 NAL unit constants */
#define H264_NAL_TYPE_MASK       0x1F /* Mask for extracting NAL unit type (5 bits) */
#define H264_NAL_SPS             7    /* Sequence Parameter Set NAL type */
#define H264_NAL_PPS             8    /* Picture Parameter Set NAL type */
#define H264_START_CODE_SIZE     4    /* Size of H.264 start code (0x00 0x00 0x00 0x01) */
#define H264_FU_INDICATOR_SIZE   1    /* Size of FU indicator byte */
#define H264_FU_HEADER_SIZE      1    /* Size of FU header byte */
#define H264_FU_A_TYPE           28   /* Fragmentation Unit A type */
#define H264_FU_START_BIT        0x80 /* FU header start bit */
#define H264_FU_END_BIT          0x40 /* FU header end bit */
#define H264_FU_TYPE_MASK        0x1F /* FU header NAL type mask */

/* Multistream timing constants */
#define RTSP_FRAME_ENCODING_WAIT_MS     200   /* Wait time for first frame encoding */
#define RTSP_TIMEOUT_CHECK_INTERVAL_S   10    /* Interval for checking session timeouts */
#define RTSP_MAX_RETRY_DELAY_MS         100   /* Maximum retry delay for operations */
#define RTSP_SHUTDOWN_TIMEOUT_MS        10    /* Timeout during server shutdown */
#define RTSP_RETRY_DELAY_SHUTDOWN_MS    5     /* Retry delay during shutdown */
#define RTSP_RETRY_DELAY_NORMAL_MS      20    /* Retry delay during normal operation */
#define RTSP_THREAD_MUTEX_RETRY_MS      10    /* Retry delay for mutex lock failures */
#define RTSP_THREAD_POLL_DELAY_MS       10    /* Thread polling delay to prevent busy waiting */
#define RTSP_THREAD_POLL_DELAY_1MS      1     /* Fine-grained thread polling delay */
#define RTSP_THREAD_POLL_ITERATIONS     10    /* Number of fine-grained polling iterations */
#define RTSP_AUDIO_STREAM_TIMEOUT_MS    100   /* Timeout for getting audio stream */
#define RTSP_FRAME_INTERVAL_MIN_US      5     /* Minimum frame interval in microseconds */
#define RTSP_FRAME_INTERVAL_MAX_US      20    /* Maximum frame interval in microseconds */
#define RTSP_STATS_UPDATE_INTERVAL_US   10000 /* Statistics update interval in microseconds */
#define RTSP_STATS_UPDATE_INTERVAL_MS   10000 /* Statistics update interval in milliseconds */
#define RTSP_DECIMAL_BASE               10    /* Decimal number base for conversions */
#define RTSP_HTTP_OK                    200   /* HTTP 200 OK status code */
#define RTSP_LISTEN_BACKLOG             10    /* TCP listen queue size */

/* RTSP protocol string lengths */
#define RTSP_PREFIX_LEN                 5     /* Length of "RTSP/" prefix */
#define RTSP_VERSION_1_0_LEN            8     /* Length of "RTSP/1.0" string */

/* Base64 encoding constants */
#define BASE64_BITS_PER_CHAR            6     /* Number of bits per base64 character */
#define BASE64_CHAR_MASK                0x3F  /* Mask for 6-bit base64 character value */
#define BASE64_TRIPLE_SHIFT_HIGH        18    /* Bit shift for first base64 character */
#define BASE64_TRIPLE_SHIFT_MID_HIGH    12    /* Bit shift for second base64 character */

/* RTP transport modes */
#define RTP_TRANSPORT_UDP 0
#define RTP_TRANSPORT_TCP 1

/* RTP/RTCP protocol constants */
#define RTP_VERSION             2
#define RTP_HEADER_SIZE         12
#define RTP_MAX_PACKET_SIZE     1500 // Standard MTU
#define RTP_TCP_OVERHEAD        4    // TCP interleaving header
#define RTP_TCP_MAX_PACKET_SIZE (RTP_MAX_PACKET_SIZE + RTP_TCP_OVERHEAD)
#define RTP_MARKER_BIT          0x80
#define RTP_PAYLOAD_TYPE_MASK   0x7F

/* RTP header bit masks and shifts */
#define RTP_VERSION_MASK   0xC0
#define RTP_VERSION_SHIFT  6
#define RTP_PADDING_MASK   0x20
#define RTP_EXTENSION_MASK 0x10
#define RTP_CSRC_MASK      0x0F
#define RTP_MARKER_MASK    0x80
#define RTP_PT_MASK        0x7F
#define RTP_BYTE_MASK      0xFF
#define RTP_VERSION_BITS_MASK 0x03 // Mask for 2-bit version field after shift

/* Bit shift values for multi-byte fields */
#define SHIFT_8_BITS  8
#define SHIFT_16_BITS 16
#define SHIFT_24_BITS 24

/* RTP header construction constants */
#define RTP_VERSION_FLAGS        0x80 /* RTP header: Version=2, Padding=0, Extension=0, CC=0 */
#define AUDIO_BITS_PER_SAMPLE_16 16   /* Audio bits per sample for 16-bit audio */

/* RTCP constants */
#define RTCP_HEADER_SIZE         8
#define RTCP_VERSION             2
#define RTCP_THREAD_POLL_DELAY_MS 100 // Polling delay in RTCP thread to prevent busy waiting
#define RTCP_PT_SR             200 // Sender Report
#define RTCP_PT_RR             201 // Receiver Report
#define RTCP_PT_SDES           202 // Source Description
#define RTCP_PT_BYE            203 // Goodbye
#define RTCP_PT_APP            204 // Application-defined
#define RTCP_SR_PACKET_SIZE    28  // Sender Report packet size
#define RTCP_REPORT_COUNT_MASK 0x1F
#define RTCP_PT_MASK           0x7F

/* RTCP header construction constants */
#define RTCP_VERSION_BYTE      0x80 // Version 2 (bits 6-7 = 10b)
#define RTCP_SR_LENGTH_WORDS   0x06 // SR packet length in 32-bit words minus 1
#define RTCP_RR_PACKET_SIZE    32   // Receiver Report packet size (with one report block)
#define RTCP_RR_VERSION_RC1    0x81 // Version 2 with Report Count = 1
#define RTCP_RR_LENGTH_WORDS   0x07 // RR packet length in 32-bit words minus 1
#define RTCP_REPORT_BLOCK_SIZE 24   // Size of one RTCP report block in bytes

/* NTP constants */
#define NTP_FRAC_SHIFT_56 56
#define NTP_FRAC_SHIFT_48 48
#define NTP_FRAC_SHIFT_40 40
#define NTP_FRAC_SHIFT_32 32

/* Authentication constants */
#define MD5_HASH_SIZE              16  /* MD5 hash size in bytes */
#define HEX_DIGIT_MASK             0x0F /* Mask for extracting hex digit value (4 bits) */
#define HEX_DIGIT_SHIFT            4   /* Bit shift for high nibble in hex conversion */
#define AUTH_BASIC_PREFIX_LEN      6   /* Length of "Basic " prefix (including space) */
#define AUTH_DIGEST_PREFIX_LEN     7   /* Length of "Digest " prefix (including space) */
#define AUTH_USERNAME_KEY_LEN      9   /* Length of "username=" key */
#define AUTH_REALM_KEY_LEN         6   /* Length of "realm=" key */
#define AUTH_NONCE_KEY_LEN         6   /* Length of "nonce=" key */
#define AUTH_RESPONSE_KEY_LEN      9   /* Length of "response=" key */
#define DIGEST_AUTH_BUFFER_SIZE    512 /* Buffer size for digest authentication intermediate strings */
#define WWW_AUTH_HEADER_SIZE       512 /* Buffer size for WWW-Authenticate header */
#define WWW_AUTH_MIN_SIZE          64  /* Minimum WWW-Authenticate header buffer size */
#define MD5_HEX_STRING_SIZE        64  /* MD5 hash as hex string (32 bytes * 2 + null terminator) */

/* Audio sample rates */
#define AUDIO_SAMPLE_RATE_8KHZ    8000
#define AUDIO_SAMPLE_RATE_16KHZ   16000
#define AUDIO_SAMPLE_RATE_44_1KHZ 44100
#define AUDIO_SAMPLE_RATE_48KHZ   48000

/* Audio frame sizes */
#define AUDIO_FRAME_SIZE_G711 160 // 20ms at 8kHz
#define AUDIO_FRAME_SIZE_AAC  1024

/* Time conversion constants */
#define MS_TO_US               1000         // Milliseconds to microseconds
#define S_TO_MS                1000         // Seconds to milliseconds
#define S_TO_US                1000000      // Seconds to microseconds
#define RTP_TIMESTAMP_HZ_VIDEO 90000        // 90 kHz for video
#define NTP_OFFSET             2208988800UL // NTP epoch offset (1900 to 1970)

/* Authentication types */
typedef enum { RTSP_AUTH_NONE = 0, RTSP_AUTH_BASIC = 1, RTSP_AUTH_DIGEST = 2 } rtsp_auth_type_t;

/* SDP media types */
typedef enum { SDP_MEDIA_VIDEO = 0, SDP_MEDIA_AUDIO = 1, SDP_MEDIA_APPLICATION = 2 } sdp_media_type_t;

/* SDP direction types */
typedef enum { SDP_DIR_SENDRECV = 0, SDP_DIR_SENDONLY = 1, SDP_DIR_RECVONLY = 2, SDP_DIR_INACTIVE = 3 } sdp_direction_t;

/* Audio RTP payload types */
#define RTP_PT_PCMU 0  /* G.711 μ-law */
#define RTP_PT_PCMA 8  /* G.711 A-law */
#define RTP_PT_AAC  97 /* AAC */

/* Video RTP payload types */
#define RTP_PT_H264 96 /* H.264 */

/* RTSP method enumeration */
enum rtsp_method {
  RTSP_METHOD_UNKNOWN = 0,
  RTSP_METHOD_OPTIONS,
  RTSP_METHOD_DESCRIBE,
  RTSP_METHOD_SETUP,
  RTSP_METHOD_PLAY,
  RTSP_METHOD_PAUSE,
  RTSP_METHOD_TEARDOWN,
  RTSP_METHOD_GET_PARAMETER,
  RTSP_METHOD_SET_PARAMETER,
  RTSP_METHOD_ANNOUNCE,
  RTSP_METHOD_RECORD,
  RTSP_METHOD_REDIRECT
};

/* RTSP error codes */
enum rtsp_error_code {
  RTSP_OK = 200,
  RTSP_MOVED_PERMANENTLY = 301,
  RTSP_MOVED_TEMPORARILY = 302,
  RTSP_SEE_OTHER = 303,
  RTSP_NOT_MODIFIED = 304,
  RTSP_USE_PROXY = 305,
  RTSP_TEMPORARY_REDIRECT = 307,
  RTSP_BAD_REQUEST = 400,
  RTSP_UNAUTHORIZED = 401,
  RTSP_PAYMENT_REQUIRED = 402,
  RTSP_FORBIDDEN = 403,
  RTSP_NOT_FOUND = 404,
  RTSP_METHOD_NOT_ALLOWED = 405,
  RTSP_NOT_ACCEPTABLE = 406,
  RTSP_PROXY_AUTHENTICATION_REQUIRED = 407,
  RTSP_REQUEST_TIMEOUT = 408,
  RTSP_GONE = 410,
  RTSP_LENGTH_REQUIRED = 411,
  RTSP_PRECONDITION_FAILED = 412,
  RTSP_REQUEST_ENTITY_TOO_LARGE = 413,
  RTSP_REQUEST_URI_TOO_LARGE = 414,
  RTSP_UNSUPPORTED_MEDIA_TYPE = 415,
  RTSP_PARAMETER_NOT_UNDERSTOOD = 451,
  RTSP_CONFERENCE_NOT_FOUND = 452,
  RTSP_NOT_ENOUGH_BANDWIDTH = 453,
  RTSP_SESSION_NOT_FOUND = 454,
  RTSP_METHOD_NOT_VALID_IN_THIS_STATE = 455,
  RTSP_HEADER_FIELD_NOT_VALID_FOR_RESOURCE = 456,
  RTSP_INVALID_RANGE = 457,
  RTSP_PARAMETER_IS_READ_ONLY = 458,
  RTSP_AGGREGATE_OPERATION_NOT_ALLOWED = 459,
  RTSP_ONLY_AGGREGATE_OPERATION_ALLOWED = 460,
  RTSP_UNSUPPORTED_TRANSPORT = 461,
  RTSP_DESTINATION_UNREACHABLE = 462,
  RTSP_KEY_MANAGEMENT_FAILURE = 463,
  RTSP_CONNECTION_AUTHORIZATION_REQUIRED = 470,
  RTSP_CONNECTION_CREDENTIALS_NOT_ACCEPTED = 471,
  RTSP_FAILURE_TO_ESTABLISH_CONNECTION = 472,
  RTSP_CONNECTION_TIMEOUT = 473,
  RTSP_READ_TIMEOUT = 474,
  RTSP_WRITE_TIMEOUT = 475,
  RTSP_CONTROL_TIMEOUT = 476,
  RTSP_INSUFFICIENT_STORAGE = 507,
  RTSP_LOOP_DETECTED = 508,
  RTSP_NOT_EXTENDED = 510,
  RTSP_INTERNAL_ERROR = 500,
  RTSP_NOT_IMPLEMENTED = 501,
  RTSP_BAD_GATEWAY = 502,
  RTSP_SERVICE_UNAVAILABLE = 503,
  RTSP_GATEWAY_TIMEOUT = 504,
  RTSP_VERSION_NOT_SUPPORTED = 505
};

/* RTSP session states */
enum rtsp_session_state { RTSP_STATE_INVALID = -1, RTSP_STATE_INIT = 0, RTSP_STATE_READY = 1, RTSP_STATE_PLAYING = 2, RTSP_STATE_RECORDING = 3 };

struct rtsp_video_resolution {
  int width;
  int height;
};

/* RTCP packet types */
#define RTCP_SR   200
#define RTCP_RR   201
#define RTCP_SDES 202
#define RTCP_BYE  203
#define RTCP_APP  204

/* RTCP statistics structure */
struct rtcp_stats {
  uint32_t packets_sent;
  uint32_t octets_sent;
  uint32_t packets_lost;
  uint32_t fraction_lost;
  uint32_t jitter;
  uint32_t last_sr_timestamp;
  uint32_t delay_since_last_sr;
  uint64_t ntp_timestamp;
  uint32_t rtp_timestamp;
};

/* RTP session structure */
struct rtp_session {
  uint32_t ssrc;
  uint16_t sequence;
  uint16_t seq_num; /* Alias for sequence */
  uint32_t timestamp;
  int rtp_sockfd;
  int rtcp_sockfd;
  uint16_t rtp_port;  /* RTP port number */
  uint16_t rtcp_port; /* RTCP port number */
  int transport;      /* Transport protocol */
  struct sockaddr_in client_rtp_addr;
  struct sockaddr_in client_rtcp_addr;
  struct sockaddr_in client_addr; /* Alias for client_rtp_addr */
  /* TCP interleaved channels when transport == RTP_TRANSPORT_TCP */
  int tcp_channel_rtp;
  int tcp_channel_rtcp;

  /* RTCP support */
  struct rtcp_stats stats;
  time_t last_rtcp_sent;
  time_t last_rtcp_received;
  bool rtcp_enabled;
  pthread_t rtcp_thread;
};

/* Audio RTP session structure */
struct audio_rtp_session {
  uint32_t ssrc;
  uint16_t sequence;
  uint32_t timestamp;
  int rtp_sockfd;
  int rtcp_sockfd;
  uint16_t rtp_port;
  uint16_t rtcp_port;
  int transport;
  struct sockaddr_in client_rtp_addr;
  struct sockaddr_in client_rtcp_addr;
  struct sockaddr_in client_addr; /* Alias for client_rtp_addr */
  int tcp_channel_rtp;
  int tcp_channel_rtcp;

  /* RTCP support */
  struct rtcp_stats stats;
  time_t last_rtcp_sent;
  time_t last_rtcp_received;
  bool rtcp_enabled;
  pthread_t rtcp_thread;
};

/* RTSP header structure */
struct rtsp_header {
  char name[RTSP_HEADER_NAME_SIZE];
  char value[RTSP_HEADER_VALUE_SIZE];
  struct rtsp_header* next;
};

/* Authentication user structure */
struct rtsp_user {
  char username[RTSP_MAX_USERNAME_LEN];
  char password[RTSP_MAX_PASSWORD_LEN];
  struct rtsp_user* next;
};

/* Authentication configuration */
struct rtsp_auth_config {
  rtsp_auth_type_t auth_type;
  char realm[RTSP_MAX_REALM_LEN];
  char nonce[RTSP_MAX_NONCE_LEN];
  struct rtsp_user* users;
  bool enabled;
};

/* SDP media description */
struct sdp_media {
  sdp_media_type_t type;
  int port;
  char protocol[SDP_PROTOCOL_SIZE];
  int payload_type;
  char encoding[SDP_ENCODING_SIZE];
  int clock_rate;
  int channels;
  sdp_direction_t direction;
  char control[SDP_CONTROL_SIZE];
  char fmtp[SDP_ATTRIBUTE_SIZE];
  char rtcp_fb[SDP_ATTRIBUTE_SIZE];
  char extmap[SDP_ATTRIBUTE_SIZE];
  char mid[SDP_MID_SIZE];
  char ssrc[SDP_SSRC_SIZE];
  struct sdp_media* next;
};

/* SDP session description */
struct sdp_session {
  int version;
  char origin[SDP_ORIGIN_SIZE];
  char session_name[SDP_SESSION_NAME_SIZE];
  char session_info[SDP_SESSION_INFO_SIZE];
  char uri[SDP_URI_SIZE];
  char email[SDP_EMAIL_SIZE];
  char phone[SDP_PHONE_SIZE];
  char connection[SDP_CONNECTION_SIZE];
  char bandwidth[SDP_BANDWIDTH_SIZE];
  char time_zone[SDP_TIMEZONE_SIZE];
  char key[SDP_KEY_SIZE];
  char attributes[SDP_ATTRIBUTES_SIZE];
  struct sdp_media* media;
};

/** Audio stream encoding configuration */
typedef struct {
  int sample_rate;     /* Sample rate (8000, 16000, 44100, etc.) */
  int channels;        /* Number of channels (1 for mono, 2 for stereo) */
  int bits_per_sample; /* Bits per sample (8, 16) */
  int codec_type;      /* Audio codec type (G.711 A-law, μ-law, AAC) */
  int bitrate;         /* Audio bitrate for AAC */
} audio_config_t;

/** Full stream config passed when creating server */
typedef struct {
  char stream_path[RTSP_STREAM_PATH_SIZE];
  char stream_name[RTSP_STREAM_NAME_SIZE];
  int port;
  bool enabled;
  void* vi_handle;
  video_config_t video_config;
  audio_config_t audio_config;
  bool audio_enabled;
} rtsp_stream_config_t;

/* Forward declarations for opaque types */
typedef struct rtsp_session rtsp_session_t;
typedef struct rtsp_server rtsp_server_t;

/* RTSP server structure */
struct rtsp_server {
  /* Configuration */
  rtsp_stream_config_t config;

  /* Network */
  int listen_sockfd;
  bool running;

  /* Threading */
  pthread_t accept_thread;
  pthread_t encoder_thread;
  pthread_t audio_thread;
  pthread_t timeout_thread;

  /* Sessions */
  rtsp_session_t* sessions;
  pthread_mutex_t sessions_mutex;
  int sessions_count;

  /* Encoding using platform abstraction */
  platform_vi_handle_t vi_handle;
  platform_venc_handle_t venc_handle;
  platform_venc_stream_handle_t venc_stream_handle;
  bool encoder_initialized;

  /* Audio encoding using platform abstraction */
  platform_ai_handle_t ai_handle;
  platform_aenc_stream_handle_t aenc_handle;
  bool audio_encoder_initialized;

  /* Statistics */
  uint64_t bytes_sent;
  uint64_t frames_sent;
  uint64_t audio_frames_sent;

  /* H.264 parameter sets (base64) learned at runtime */
  char h264_sps_b64[RTSP_H264_PARAM_B64_SIZE];
  char h264_pps_b64[RTSP_H264_PARAM_B64_SIZE];

  /* Authentication */
  struct rtsp_auth_config auth_config;

  /* SDP session */
  struct sdp_session sdp_session;
};

#endif /* RTSP_TYPES_H */
