/**
 * @file http_onvif_adapter.c
 * @brief HTTP to ONVIF request/response adapter implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "http_onvif_adapter.h"

#include <stdlib.h>
#include <string.h>

#include "common/onvif_types.h"
#include "networking/http/http_parser.h"
#include "utils/error/error_handling.h"
#include "utils/logging/logging_utils.h"
#include "utils/memory/memory_manager.h"
#include "utils/string/string_shims.h"


/* Helper function to safely copy string data */
static int copy_string_data(const char *src, char **dest, size_t *dest_len) {
  if (!src) {
    *dest = NULL;
    if (dest_len) {
      *dest_len = 0;
    }
    return 0;
  }

  size_t src_len = strlen(src);
  *dest = (char *)ONVIF_MALLOC(src_len + 1);
  if (!*dest) {
    return -1;
  }

  strcpy(*dest, src);

  if (dest_len) {
    *dest_len = src_len;
  }
  return 0;
}

/* Helper function to safely copy binary data */
static int copy_binary_data(const char *src, size_t src_len, char **dest,
                            size_t *dest_len) {
  if (!src || src_len == 0) {
    *dest = NULL;
    *dest_len = 0;
    return 0;
  }

  *dest = (char *)ONVIF_MALLOC(src_len + 1);
  if (!*dest) {
    return -1;
  }

  memcpy(*dest, src, src_len);
  (*dest)[src_len] = '\0';
  *dest_len = src_len;
  return 0;
}

int http_to_onvif_request(const http_request_t *http_req,
                          onvif_request_t *onvif_req) {
  if (!http_req || !onvif_req) {
    return -1;
  }

  // Initialize ONVIF request
  memset(onvif_req, 0, sizeof(onvif_request_t));

  // Copy body data
  if (copy_binary_data(http_req->body, http_req->body_length, &onvif_req->body,
                       &onvif_req->body_length) != 0) {
    return -1;
  }

  // Copy headers data
  if (copy_string_data(http_req->headers, &onvif_req->headers,
                       &onvif_req->headers_length) != 0) {
    onvif_request_cleanup(onvif_req);
    return -1;
  }

  // Store transport data (HTTP request pointer)
  onvif_req->transport_data = (void *)http_req;

  // Action will be determined by the service handler based on the body content
  onvif_req->action = ONVIF_ACTION_UNKNOWN;

  return 0;
}

int onvif_to_http_response(const onvif_response_t *onvif_resp,
                           http_response_t *http_resp) {
  if (!onvif_resp || !http_resp) {
    return -1;
  }

  // Initialize HTTP response
  memset(http_resp, 0, sizeof(http_response_t));

  // Copy status code
  http_resp->status_code = onvif_resp->status_code;

  // Copy body data
  if (copy_binary_data(onvif_resp->body, onvif_resp->body_length,
                       &http_resp->body, &http_resp->body_length) != 0) {
    return -1;
  }

  // Copy content type
  if (copy_string_data(onvif_resp->content_type, &http_resp->content_type,
                       NULL) != 0) {
    http_response_cleanup(http_resp);
    return -1;
  }

  return 0;
}

void onvif_request_cleanup(onvif_request_t *onvif_req) {
  if (!onvif_req) return;

  if (onvif_req->body) {
    ONVIF_FREE(onvif_req->body);
    onvif_req->body = NULL;
  }

  if (onvif_req->headers) {
    ONVIF_FREE(onvif_req->headers);
    onvif_req->headers = NULL;
  }

  onvif_req->transport_data = NULL;
  onvif_req->body_length = 0;
  onvif_req->headers_length = 0;
}

void http_response_cleanup(http_response_t *http_resp) {
  if (!http_resp) return;

  if (http_resp->body) {
    ONVIF_FREE(http_resp->body);
    http_resp->body = NULL;
  }

  if (http_resp->content_type) {
    ONVIF_FREE(http_resp->content_type);
    http_resp->content_type = NULL;
  }

  http_resp->body_length = 0;
}

/* onvif_response_cleanup is now provided by utils/response_helpers.c */
