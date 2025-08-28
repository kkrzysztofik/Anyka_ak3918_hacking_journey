/* rtsp_utils.c - Basic RTSP utility functions */

#include "rtsp_utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>

static int rtsp_initialized = 0;

int rtsp_utils_init(void) {
    if (rtsp_initialized) {
        return 0;
    }
    
    /* Initialize RTSP subsystem */
    rtsp_initialized = 1;
    return 0;
}

int rtsp_utils_cleanup(void) {
    if (!rtsp_initialized) {
        return 0;
    }
    
    /* Cleanup RTSP subsystem */
    rtsp_initialized = 0;
    return 0;
}

int rtsp_utils_create_session(rtsp_session_t *session, const char *uri) {
    if (!session || !uri) {
        return -1;
    }
    
    /* Initialize session structure */
    memset(session, 0, sizeof(rtsp_session_t));
    session->state = RTSP_STATE_INIT;
    strncpy(session->stream_uri, uri, sizeof(session->stream_uri) - 1);
    snprintf(session->session_id, sizeof(session->session_id), "sess_%ld", time(NULL));
    
    /* Set default RTP/RTCP ports */
    session->rtp_port = 5004;
    session->rtcp_port = 5005;
    
    return 0;
}

int rtsp_utils_destroy_session(rtsp_session_t *session) {
    if (!session) {
        return -1;
    }
    
    /* Cleanup session */
    memset(session, 0, sizeof(rtsp_session_t));
    return 0;
}

int rtsp_utils_get_stream_uri(char *uri, size_t max_len) {
    if (!uri || max_len == 0) {
        return -1;
    }
    
    /* Return default stream URI */
    snprintf(uri, max_len, "rtsp://localhost:554/vs0");
    return 0;
}
