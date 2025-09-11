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

// Remove conflicting macro definitions
// The actual values will be taken from the Anyka SDK headers

/* RTSP constants */
#define RTSP_MAX_CLIENTS 10
#define RTSP_BUFFER_SIZE 4096
#define RTSP_RTP_BUFFER_SIZE 1500
#define RTSP_MAX_URI_LEN 256

/* RTP transport modes */
#define RTP_TRANSPORT_UDP 0
#define RTP_TRANSPORT_TCP 1

/* Audio RTP payload types */
#define RTP_PT_PCMU             0      /* G.711 μ-law */
#define RTP_PT_PCMA             8      /* G.711 A-law */
#define RTP_PT_AAC              97     /* AAC */

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
    RTSP_METHOD_SET_PARAMETER
};

/* RTSP session states */
enum rtsp_session_state {
    RTSP_STATE_INIT = 0,
    RTSP_STATE_READY = 1,
    RTSP_STATE_PLAYING = 2,
    RTSP_STATE_RECORDING = 3
};

struct rtsp_video_resolution {
    int width;
    int height;
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
    int tcp_channel_rtp;
    int tcp_channel_rtcp;
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
    
    /* Sessions */
    rtsp_session_t *sessions;
    pthread_mutex_t sessions_mutex;
    int sessions_count;
    
    /* Encoding */
    void *vi_handle;
    void *venc_handle;
    void *stream_handle;
    bool encoder_initialized;
    
    /* Audio encoding */
    void *ai_handle;
    void *aenc_handle;
    void *audio_stream_handle;
    bool audio_encoder_initialized;
    
    /* Statistics */
    uint64_t bytes_sent;
    uint64_t frames_sent;
    uint64_t audio_frames_sent;

    /* H.264 parameter sets (base64) learned at runtime */
    char h264_sps_b64[256];
    char h264_pps_b64[256];
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

#endif
