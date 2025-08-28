#ifndef RTSP_UTILS_H
#define RTSP_UTILS_H

#include <stddef.h>  /* for size_t */

/* RTSP session states */
enum rtsp_state {
    RTSP_STATE_INIT = 0,
    RTSP_STATE_READY = 1,
    RTSP_STATE_PLAYING = 2,
    RTSP_STATE_RECORDING = 3
};

/* RTSP session structure */
struct rtsp_session {
    char session_id[64];
    enum rtsp_state state;
    char stream_uri[256];
    int rtp_port;
    int rtcp_port;
    void *context;
};

typedef struct rtsp_session rtsp_session_t;

/* RTSP server structure */
struct rtsp_server {
    int port;
    int socket_fd;
    int running;
    void *sessions;
    int max_sessions;
};

typedef struct rtsp_server rtsp_server_t;

/* Function declarations */
int rtsp_utils_init(void);
int rtsp_utils_cleanup(void);
int rtsp_utils_create_session(rtsp_session_t *session, const char *uri);
int rtsp_utils_destroy_session(rtsp_session_t *session);
int rtsp_utils_get_stream_uri(char *uri, size_t max_len);

#endif /* RTSP_UTILS_H */
