/**
 * @file rtsp_server.h
 * @brief Internal RTSP/RTP streaming server for H.264 (and optional audio).
 *
 * Provides a lightweight RTSP implementation sufficient for basic
 * DESCRIBE/SETUP/PLAY control and unicast RTP streaming over UDP or
 * TCP-interleaved (planned). Each client session is represented by
 * an rtsp_session_t and tracked in the rtsp_server instance. Video
 * frames are pulled from the Anyka video input / encoder pipeline.
 */
#ifndef RTSP_SERVER_H
#define RTSP_SERVER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <pthread.h>
#include <netinet/in.h>
#include "platform/platform.h"

// Remove conflicting macro definitions
// The actual values will be taken from the Anyka SDK headers

/* RTSP constants */
#define RTSP_MAX_CLIENTS 10
#define RTSP_BUFFER_SIZE 4096
#define RTSP_RTP_BUFFER_SIZE 1500
#define RTSP_MAX_URI_LEN 256
#define RTSP_SESSION_TIMEOUT_SEC 60
#define RTSP_RTCP_INTERVAL_SEC 5
#define RTSP_MAX_HEADER_LEN 1024
#define RTSP_MAX_USERNAME_LEN 64
#define RTSP_MAX_PASSWORD_LEN 64
#define RTSP_MAX_REALM_LEN 128
#define RTSP_MAX_NONCE_LEN 32
#define RTSP_MAX_RESPONSE_LEN 64

/* RTP transport modes */
#define RTP_TRANSPORT_UDP 0
#define RTP_TRANSPORT_TCP 1

/* Authentication types */
typedef enum {
    RTSP_AUTH_NONE = 0,
    RTSP_AUTH_BASIC = 1,
    RTSP_AUTH_DIGEST = 2
} rtsp_auth_type_t;

/* SDP media types */
typedef enum {
    SDP_MEDIA_VIDEO = 0,
    SDP_MEDIA_AUDIO = 1,
    SDP_MEDIA_APPLICATION = 2
} sdp_media_type_t;

/* SDP direction types */
typedef enum {
    SDP_DIR_SENDRECV = 0,
    SDP_DIR_SENDONLY = 1,
    SDP_DIR_RECVONLY = 2,
    SDP_DIR_INACTIVE = 3
} sdp_direction_t;

/* Audio RTP payload types */
#define RTP_PT_PCMU             0      /* G.711 μ-law */
#define RTP_PT_PCMA             8      /* G.711 A-law */
#define RTP_PT_AAC              97     /* AAC */

/* Video RTP payload types */
#define RTP_PT_H264             96     /* H.264 */

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
enum rtsp_session_state {
    RTSP_STATE_INVALID = -1,
    RTSP_STATE_INIT = 0,
    RTSP_STATE_READY = 1,
    RTSP_STATE_PLAYING = 2,
    RTSP_STATE_RECORDING = 3
};

struct rtsp_video_resolution {
    int width;
    int height;
};

/* RTCP packet types */
#define RTCP_SR 200
#define RTCP_RR 201
#define RTCP_SDES 202
#define RTCP_BYE 203
#define RTCP_APP 204

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
    uint16_t seq_num;           /* Alias for sequence */
    uint32_t timestamp;
    int rtp_sockfd;
    int rtcp_sockfd;
    uint16_t rtp_port;          /* RTP port number */
    uint16_t rtcp_port;         /* RTCP port number */
    int transport;              /* Transport protocol */
    struct sockaddr_in client_rtp_addr;
    struct sockaddr_in client_rtcp_addr;
    struct sockaddr_in client_addr;  /* Alias for client_rtp_addr */
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
    struct sockaddr_in client_addr;  /* Alias for client_rtp_addr */
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
    char name[64];
    char value[256];
    struct rtsp_header *next;
};

/* Authentication user structure */
struct rtsp_user {
    char username[RTSP_MAX_USERNAME_LEN];
    char password[RTSP_MAX_PASSWORD_LEN];
    struct rtsp_user *next;
};

/* Authentication configuration */
struct rtsp_auth_config {
    rtsp_auth_type_t auth_type;
    char realm[RTSP_MAX_REALM_LEN];
    char nonce[RTSP_MAX_NONCE_LEN];
    struct rtsp_user *users;
    bool enabled;
};

/* SDP media description */
struct sdp_media {
    sdp_media_type_t type;
    int port;
    char protocol[16];
    int payload_type;
    char encoding[32];
    int clock_rate;
    int channels;
    sdp_direction_t direction;
    char control[64];
    char fmtp[256];
    char rtcp_fb[256];
    char extmap[256];
    char mid[32];
    char ssrc[32];
    struct sdp_media *next;
};

/* SDP session description */
struct sdp_session {
    int version;
    char origin[128];
    char session_name[128];
    char session_info[256];
    char uri[256];
    char email[128];
    char phone[64];
    char connection[128];
    char bandwidth[64];
    char time_zone[32];
    char key[64];
    char attributes[512];
    struct sdp_media *media;
};

/* RTSP session structure */
typedef struct rtsp_session {
    int sockfd;
    struct sockaddr_in addr;
    enum rtsp_session_state state;
    bool active;
    pthread_t thread;
    char session_id[64];
    int cseq;                   /* RTSP sequence number */
    char uri[256];              /* Request URI */
    
    /* Session timeout */
    time_t last_activity;
    time_t created_time;
    int timeout_seconds;
    
    /* Headers */
    struct rtsp_header *headers;
    
    /* Authentication */
    bool authenticated;
    char auth_username[RTSP_MAX_USERNAME_LEN];
    char auth_nonce[RTSP_MAX_NONCE_LEN];
    
    /* Buffers */
    char *recv_buffer;
    char *send_buffer;
    size_t recv_pos;
    
    /* RTP session */
    struct rtp_session rtp_session;
    
    /* Audio RTP session */
    struct audio_rtp_session audio_rtp_session;
    bool audio_enabled;
    
    /* Linked list */
    struct rtsp_session *next;
    struct rtsp_session *prev;
    struct rtsp_server *server; /* back-pointer */
} rtsp_session_t;

/** Video stream encoding configuration */
typedef struct {
    int width;
    int height;
    int fps;
    int bitrate;
    int gop_size;
    int profile;
    int codec_type;
    int br_mode;
} video_config_t;

/** Audio stream encoding configuration */
typedef struct {
    int sample_rate;        /* Sample rate (8000, 16000, 44100, etc.) */
    int channels;           /* Number of channels (1 for mono, 2 for stereo) */
    int bits_per_sample;    /* Bits per sample (8, 16) */
    int codec_type;         /* Audio codec type (G.711 A-law, μ-law, AAC) */
    int bitrate;            /* Audio bitrate for AAC */
} audio_config_t;

/** Full stream config passed when creating server */
typedef struct {
    char stream_path[64];
    char stream_name[64];
    int port;
    bool enabled;
    void *vi_handle;
    video_config_t video_config;
    audio_config_t audio_config;
    bool audio_enabled;
} rtsp_stream_config_t;

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
    rtsp_session_t *sessions;
    pthread_mutex_t sessions_mutex;
    int sessions_count;
    
    /* Encoding using platform abstraction */
    platform_vi_handle_t vi_handle;
    platform_venc_handle_t venc_handle;
    bool encoder_initialized;
    
    /* Audio encoding using platform abstraction */
    platform_ai_handle_t ai_handle;
    platform_aenc_handle_t aenc_handle;
    bool audio_encoder_initialized;
    
    /* Statistics */
    uint64_t bytes_sent;
    uint64_t frames_sent;
    uint64_t audio_frames_sent;

    /* H.264 parameter sets (base64) learned at runtime */
    char h264_sps_b64[256];
    char h264_pps_b64[256];
    
    /* Authentication */
    struct rtsp_auth_config auth_config;
    
    /* SDP session */
    struct sdp_session sdp_session;
};

typedef struct rtsp_server rtsp_server_t;

/**
 * @brief Allocate and initialize a server instance (not started).
 * @param config Desired stream configuration (copied internally).
 * @return Opaque server pointer or NULL on failure.
 */
rtsp_server_t *rtsp_server_create(rtsp_stream_config_t *config);

/**
 * @brief Start the accept / encoding threads.
 * @param server Instance created via rtsp_server_create.
 * @return 0 on success, negative error code otherwise.
 */
int rtsp_server_start(rtsp_server_t *server);

/**
 * @brief Stop all threads and close client sessions (server reusable after start again).
 * @param server Server instance.
 * @return 0 on success, negative error code.
 */
int rtsp_server_stop(rtsp_server_t *server);

/**
 * @brief Destroy server and free all memory.
 * @param server Server instance (may be NULL).
 * @return 0 always; provided for symmetry.
 */
int rtsp_server_destroy(rtsp_server_t *server);

/**
 * @brief Get server statistics including RTCP data
 * @param server Server instance
 * @param bytes_sent Total bytes sent
 * @param frames_sent Total frames sent
 * @param sessions_count Number of active sessions
 * @return 0 on success, -1 on error
 */
int rtsp_server_get_stats(rtsp_server_t *server, uint64_t *bytes_sent, 
                         uint32_t *frames_sent, uint32_t *sessions_count);

/**
 * @brief Get stream URL
 * @param server Server instance
 * @param url Buffer to store URL
 * @param url_size Size of URL buffer
 * @return 0 on success, -1 on error
 */
int rtsp_server_get_stream_url(rtsp_server_t *server, char *url, size_t url_size);

/* Enhanced RTSP functions */
int rtsp_parse_headers(const char *request, struct rtsp_header **headers);
void rtsp_free_headers(struct rtsp_header *headers);
const char* rtsp_get_header(struct rtsp_header *headers, const char *name);
int rtsp_validate_request(const char *request, size_t len);
int rtsp_send_error_response(rtsp_session_t *session, enum rtsp_error_code code, const char *reason);
int rtsp_check_session_timeout(rtsp_session_t *session);
void rtsp_update_session_activity(rtsp_session_t *session);

/* RTCP functions */
int rtcp_init_session(struct rtp_session *rtp_session);
void rtcp_cleanup_session(struct rtp_session *rtp_session);
int rtcp_send_sr(struct rtp_session *rtp_session);
int rtcp_send_rr(struct rtp_session *rtp_session);
int rtcp_handle_packet(struct rtp_session *rtp_session, const uint8_t *data, size_t len);
void* rtcp_thread(void *arg);

/* Session management */
int rtsp_session_cleanup_timeout_sessions(rtsp_server_t *server);
int rtsp_session_set_timeout(rtsp_session_t *session, int timeout_seconds);
void rtsp_session_cleanup_all(rtsp_server_t *server);
int rtsp_session_get_count(rtsp_server_t *server);
int rtsp_session_add(rtsp_server_t *server, rtsp_session_t *session);
int rtsp_session_remove(rtsp_server_t *server, rtsp_session_t *session);
bool rtsp_session_has_timed_out(rtsp_session_t *session);

/* Authentication functions */
int rtsp_auth_init(struct rtsp_auth_config *auth_config);
void rtsp_auth_cleanup(struct rtsp_auth_config *auth_config);
int rtsp_auth_add_user(struct rtsp_auth_config *auth_config, const char *username, const char *password);
int rtsp_auth_remove_user(struct rtsp_auth_config *auth_config, const char *username);
int rtsp_auth_validate_basic(rtsp_session_t *session, const char *auth_header);
int rtsp_auth_validate_digest(rtsp_session_t *session, const char *auth_header, const char *method, const char *uri);
int rtsp_auth_require_auth(rtsp_session_t *session);
void rtsp_auth_generate_nonce(char *nonce, size_t nonce_size);
int rtsp_auth_parse_credentials(const char *auth_header, char *username, char *password, char *realm, char *nonce, char *response);
int rtsp_auth_verify_digest(const char *username, const char *password, const char *realm, const char *nonce, const char *method, const char *uri, const char *response);

/* SDP functions */
int sdp_init_session(struct sdp_session *sdp, const char *session_name, const char *origin);
void sdp_cleanup_session(struct sdp_session *sdp);
int sdp_add_media(struct sdp_session *sdp, sdp_media_type_t type, int port, const char *protocol, int payload_type, const char *encoding, int clock_rate, int channels);
int sdp_set_media_direction(struct sdp_session *sdp, sdp_media_type_t type, sdp_direction_t direction);
int sdp_set_media_control(struct sdp_session *sdp, sdp_media_type_t type, const char *control);
int sdp_set_media_fmtp(struct sdp_session *sdp, sdp_media_type_t type, const char *fmtp);
int sdp_set_media_rtcp_fb(struct sdp_session *sdp, sdp_media_type_t type, const char *rtcp_fb);
int sdp_set_media_extmap(struct sdp_session *sdp, sdp_media_type_t type, const char *extmap);
int sdp_set_media_mid(struct sdp_session *sdp, sdp_media_type_t type, const char *mid);
int sdp_set_media_ssrc(struct sdp_session *sdp, sdp_media_type_t type, const char *ssrc);
int sdp_generate(struct sdp_session *sdp, char *buffer, size_t buffer_size);
int sdp_parse(struct sdp_session *sdp, const char *sdp_text);
int sdp_validate(const char *sdp_text);

/* Enhanced RTSP functions */
int rtsp_parse_headers_enhanced(const char *request, struct rtsp_header **headers);
int rtsp_handle_auth_required(rtsp_session_t *session);
int rtsp_generate_www_authenticate_header(rtsp_session_t *session, char *header, size_t header_size);

/* RTP functions */
int rtsp_setup_encoder(rtsp_server_t *server);
void rtsp_cleanup_encoder(rtsp_server_t *server);
int rtsp_setup_audio_encoder(rtsp_server_t *server);
void rtsp_cleanup_audio_encoder(rtsp_server_t *server);
int rtsp_init_rtp_session(rtsp_session_t *session);
void rtsp_cleanup_rtp_session(rtsp_session_t *session);
int rtsp_init_audio_rtp_session(rtsp_session_t *session);
void rtsp_cleanup_audio_rtp_session(rtsp_session_t *session);
int rtsp_send_rtp_packet(rtsp_session_t *session, const uint8_t *data, size_t len, uint32_t timestamp);
int rtsp_send_audio_rtp_packet(rtsp_session_t *session, const uint8_t *data, size_t len, uint32_t timestamp);
int rtsp_send_rtp_packet_udp(rtsp_session_t *session, const uint8_t *data, size_t len, uint32_t timestamp);
int rtsp_send_rtp_packet_tcp(rtsp_session_t *session, const uint8_t *data, size_t len, uint32_t timestamp);

/* Session cleanup */
void rtsp_cleanup_session(rtsp_session_t *session);

/* Request validation */
int rtsp_validate_request(const char *request, size_t request_len);

#endif
