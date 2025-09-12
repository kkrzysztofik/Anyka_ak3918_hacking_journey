/**
 * @file http_onvif_adapter.c
 * @brief HTTP to ONVIF request/response adapter implementation.
 */

#include "http_onvif_adapter.h"
#include "common/onvif_types.h"
#include "utils/logging_utils.h"
#include <stdlib.h>
#include <string.h>

int http_to_onvif_request(const http_request_t *http_req, onvif_request_t *onvif_req) {
    if (!http_req || !onvif_req) {
        return -1;
    }
    
    // Initialize ONVIF request
    memset(onvif_req, 0, sizeof(onvif_request_t));
    
    // Copy body data
    if (http_req->body && http_req->body_length > 0) {
        onvif_req->body = malloc(http_req->body_length + 1);
        if (!onvif_req->body) {
            return -1;
        }
        memcpy(onvif_req->body, http_req->body, http_req->body_length);
        onvif_req->body[http_req->body_length] = '\0';
        onvif_req->body_length = http_req->body_length;
    }
    
    // Copy headers data
    if (http_req->headers) {
        size_t headers_len = strlen(http_req->headers);
        onvif_req->headers = malloc(headers_len + 1);
        if (!onvif_req->headers) {
            if (onvif_req->body) free(onvif_req->body);
            return -1;
        }
        strcpy(onvif_req->headers, http_req->headers);
        onvif_req->headers_length = headers_len;
    }
    
    // Store transport data (HTTP request pointer)
    onvif_req->transport_data = (void*)http_req;
    
    // Action will be determined by the service handler based on the body content
    onvif_req->action = ONVIF_ACTION_UNKNOWN;
    
    return 0;
}

int onvif_to_http_response(const onvif_response_t *onvif_resp, http_response_t *http_resp) {
    if (!onvif_resp || !http_resp) {
        return -1;
    }
    
    // Initialize HTTP response
    memset(http_resp, 0, sizeof(http_response_t));
    
    // Copy status code
    http_resp->status_code = onvif_resp->status_code;
    
    // Copy body data
    if (onvif_resp->body && onvif_resp->body_length > 0) {
        http_resp->body = malloc(onvif_resp->body_length + 1);
        if (!http_resp->body) {
            return -1;
        }
        memcpy(http_resp->body, onvif_resp->body, onvif_resp->body_length);
        http_resp->body[onvif_resp->body_length] = '\0';
        http_resp->body_length = onvif_resp->body_length;
    }
    
    // Copy content type
    if (onvif_resp->content_type) {
        size_t content_type_len = strlen(onvif_resp->content_type);
        http_resp->content_type = malloc(content_type_len + 1);
        if (!http_resp->content_type) {
            if (http_resp->body) free(http_resp->body);
            return -1;
        }
        strcpy(http_resp->content_type, onvif_resp->content_type);
    }
    
    return 0;
}

void onvif_request_cleanup(onvif_request_t *onvif_req) {
    if (!onvif_req) return;
    
    if (onvif_req->body) {
        free(onvif_req->body);
        onvif_req->body = NULL;
    }
    
    if (onvif_req->headers) {
        free(onvif_req->headers);
        onvif_req->headers = NULL;
    }
    
    onvif_req->transport_data = NULL;
    onvif_req->body_length = 0;
    onvif_req->headers_length = 0;
}

void onvif_response_cleanup(onvif_response_t *onvif_resp) {
    if (!onvif_resp) return;
    
    if (onvif_resp->body) {
        free(onvif_resp->body);
        onvif_resp->body = NULL;
    }
    
    if (onvif_resp->content_type) {
        free(onvif_resp->content_type);
        onvif_resp->content_type = NULL;
    }
    
    onvif_resp->transport_data = NULL;
    onvif_resp->body_length = 0;
}
