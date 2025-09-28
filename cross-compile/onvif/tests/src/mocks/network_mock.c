/**
 * @file network_mock.c
 * @brief Mock implementation for network functions
 * @author kkrzysztofik
 * @date 2025
 */

#include "network_mock.h"

#include <arpa/inet.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>

/* ============================================================================
 * Constants
 * ============================================================================ */

// Header pair structure to match the one in network_mock.h
// This must match the anonymous struct in network_mock.h exactly
typedef struct {
  char* name;
  char* value;
} mock_header_pair_t;

// Buffer sizes
#define NETWORK_MOCK_HTTP_RESPONSE_BODY_SIZE 4096
#define NETWORK_MOCK_HTTP_CONTENT_TYPE_SIZE  256
#define NETWORK_MOCK_HTTP_METHOD_SIZE        32
#define NETWORK_MOCK_HTTP_URL_SIZE           512
#define NETWORK_MOCK_HTTP_HEADERS_SIZE       1024
#define NETWORK_MOCK_HTTP_BODY_SIZE          2048

#define NETWORK_MOCK_RTSP_SESSION_ID_SIZE 64
#define NETWORK_MOCK_RTSP_TRANSPORT_SIZE  256
#define NETWORK_MOCK_RTSP_METHOD_SIZE     32
#define NETWORK_MOCK_RTSP_URL_SIZE        512
#define NETWORK_MOCK_RTSP_HEADERS_SIZE    1024

#define NETWORK_MOCK_UDP_RESPONSE_DATA_SIZE 4096
#define NETWORK_MOCK_UDP_DATA_SIZE          4096
#define NETWORK_MOCK_UDP_DEST_ADDR_SIZE     64

#define NETWORK_MOCK_HEADER_NAME_SIZE  256
#define NETWORK_MOCK_HEADER_VALUE_SIZE 256

// Default values
#define NETWORK_MOCK_DEFAULT_HTTP_STATUS     200
#define NETWORK_MOCK_DEFAULT_RTSP_STATUS     200
#define NETWORK_MOCK_DEFAULT_RTSP_SESSION_ID "12345678"
#define NETWORK_MOCK_DEFAULT_RTSP_TRANSPORT  "RTP/AVP;unicast;client_port=1234-1235"
#define NETWORK_MOCK_DEFAULT_UDP_SRC_PORT    1234
#define NETWORK_MOCK_DEFAULT_UDP_SRC_ADDR    "127.0.0.1"

// HTTP response constants
#define NETWORK_MOCK_HTTP_OK_RESPONSE          "OK"
#define NETWORK_MOCK_HTTP_DEFAULT_CONTENT_TYPE "text/plain"

/* ============================================================================
 * Mock State Management
 * ============================================================================ */

// Global network mock state
static struct {
  int initialized;
  int error_simulation_enabled;
  int error_type;
  int error_code;

  // HTTP state
  int http_status_code;
  char http_response_body[NETWORK_MOCK_HTTP_RESPONSE_BODY_SIZE];
  char http_content_type[NETWORK_MOCK_HTTP_CONTENT_TYPE_SIZE];
  int http_error_enabled;
  int http_request_count;

  char last_http_method[NETWORK_MOCK_HTTP_METHOD_SIZE];
  char last_http_url[NETWORK_MOCK_HTTP_URL_SIZE];
  char last_http_headers[NETWORK_MOCK_HTTP_HEADERS_SIZE];
  char last_http_body[NETWORK_MOCK_HTTP_BODY_SIZE];
  int last_http_request_made;

  // RTSP state
  int rtsp_status_code;
  char rtsp_session_id[NETWORK_MOCK_RTSP_SESSION_ID_SIZE];
  char rtsp_transport[NETWORK_MOCK_RTSP_TRANSPORT_SIZE];
  int rtsp_request_count;

  char last_rtsp_method[NETWORK_MOCK_RTSP_METHOD_SIZE];
  char last_rtsp_url[NETWORK_MOCK_RTSP_URL_SIZE];
  char last_rtsp_headers[NETWORK_MOCK_RTSP_HEADERS_SIZE];
  int last_rtsp_request_made;

  // UDP state
  uint8_t udp_response_data[NETWORK_MOCK_UDP_RESPONSE_DATA_SIZE];
  size_t udp_response_size;
  int udp_request_count;

  uint8_t last_udp_data[NETWORK_MOCK_UDP_DATA_SIZE];
  size_t last_udp_size;
  char last_udp_dest_addr[NETWORK_MOCK_UDP_DEST_ADDR_SIZE];
  uint16_t last_udp_dest_port;
  int last_udp_request_made;

  // Socket state
  int socket_result;
  int bind_result;
  int listen_result;
  int accept_result;
  int connect_result;
  int send_result;
  int recv_result;

  pthread_mutex_t mutex;
} g_network_mock_state = {0}; // NOLINT

/* ============================================================================
 * Mock Initialization and Cleanup
 * ============================================================================ */

void network_mock_init(void) {
  pthread_mutex_init(&g_network_mock_state.mutex, NULL);
  g_network_mock_state.initialized = 1;
  g_network_mock_state.error_simulation_enabled = 0;
  g_network_mock_state.error_type = 0;
  g_network_mock_state.error_code = 0;

  // Initialize HTTP state
  g_network_mock_state.http_status_code = NETWORK_MOCK_DEFAULT_HTTP_STATUS;
  strcpy(g_network_mock_state.http_response_body, NETWORK_MOCK_HTTP_OK_RESPONSE);
  strcpy(g_network_mock_state.http_content_type, NETWORK_MOCK_HTTP_DEFAULT_CONTENT_TYPE);
  g_network_mock_state.http_error_enabled = 0;
  g_network_mock_state.http_request_count = 0;
  g_network_mock_state.last_http_request_made = 0;

  // Initialize RTSP state
  g_network_mock_state.rtsp_status_code = NETWORK_MOCK_DEFAULT_RTSP_STATUS;
  strcpy(g_network_mock_state.rtsp_session_id, NETWORK_MOCK_DEFAULT_RTSP_SESSION_ID);
  strcpy(g_network_mock_state.rtsp_transport, NETWORK_MOCK_DEFAULT_RTSP_TRANSPORT);
  g_network_mock_state.rtsp_request_count = 0;
  g_network_mock_state.last_rtsp_request_made = 0;

  // Initialize UDP state
  g_network_mock_state.udp_response_size = 0;
  g_network_mock_state.udp_request_count = 0;
  g_network_mock_state.last_udp_request_made = 0;

  // Initialize socket state
  g_network_mock_state.socket_result = 0;
  g_network_mock_state.bind_result = 0;
  g_network_mock_state.listen_result = 0;
  g_network_mock_state.accept_result = 0;
  g_network_mock_state.connect_result = 0;
  g_network_mock_state.send_result = 0;
  g_network_mock_state.recv_result = 0;
}

void network_mock_cleanup(void) {
  pthread_mutex_destroy(&g_network_mock_state.mutex);
  memset(&g_network_mock_state, 0, sizeof(g_network_mock_state));
}

void network_mock_reset(void) {
  network_mock_cleanup();
  network_mock_init();
}

/* ============================================================================
 * HTTP Mock Functions
 * ============================================================================ */

void network_mock_set_http_response(int status_code, const char* response_body,
                                    const char* content_type) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.http_status_code = status_code;

  if (response_body) {
    strncpy(g_network_mock_state.http_response_body, response_body,
            sizeof(g_network_mock_state.http_response_body) - 1);
    g_network_mock_state.http_response_body[sizeof(g_network_mock_state.http_response_body) - 1] =
      '\0';
  }

  if (content_type) {
    strncpy(g_network_mock_state.http_content_type, content_type,
            sizeof(g_network_mock_state.http_content_type) - 1);
    g_network_mock_state.http_content_type[sizeof(g_network_mock_state.http_content_type) - 1] =
      '\0';
  }

  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

int network_mock_get_last_http_request(char* method, size_t method_size, char* url, size_t url_size,
                                       char* headers, size_t headers_size, char* body,
                                       size_t body_size) {
  if (!method || !url || !headers || !body || method_size == 0 || url_size == 0 ||
      headers_size == 0 || body_size == 0) {
    return 0;
  }

  pthread_mutex_lock(&g_network_mock_state.mutex);
  int made = g_network_mock_state.last_http_request_made;
  if (made) {
    strncpy(method, g_network_mock_state.last_http_method, method_size - 1);
    method[method_size - 1] = '\0';

    strncpy(url, g_network_mock_state.last_http_url, url_size - 1);
    url[url_size - 1] = '\0';

    strncpy(headers, g_network_mock_state.last_http_headers, headers_size - 1);
    headers[headers_size - 1] = '\0';

    strncpy(body, g_network_mock_state.last_http_body, body_size - 1);
    body[body_size - 1] = '\0';
  }
  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return made;
}

void network_mock_set_http_error(int error_code) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.error_code = error_code;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

void network_mock_set_http_error_enabled(int enabled) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.http_error_enabled = enabled;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

/* ============================================================================
 * RTSP Mock Functions
 * ============================================================================ */

void network_mock_set_rtsp_response(int status_code, const char* session_id,
                                    const char* transport) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.rtsp_status_code = status_code;

  if (session_id) {
    strncpy(g_network_mock_state.rtsp_session_id, session_id,
            sizeof(g_network_mock_state.rtsp_session_id) - 1);
    g_network_mock_state.rtsp_session_id[sizeof(g_network_mock_state.rtsp_session_id) - 1] = '\0';
  }

  if (transport) {
    strncpy(g_network_mock_state.rtsp_transport, transport,
            sizeof(g_network_mock_state.rtsp_transport) - 1);
    g_network_mock_state.rtsp_transport[sizeof(g_network_mock_state.rtsp_transport) - 1] = '\0';
  }

  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

int network_mock_get_last_rtsp_request(char* method, size_t method_size, char* url, size_t url_size,
                                       char* headers, size_t headers_size) {
  if (!method || !url || !headers || method_size == 0 || url_size == 0 || headers_size == 0) {
    return 0;
  }

  pthread_mutex_lock(&g_network_mock_state.mutex);
  int made = g_network_mock_state.last_rtsp_request_made;
  if (made) {
    strncpy(method, g_network_mock_state.last_rtsp_method, method_size - 1);
    method[method_size - 1] = '\0';

    strncpy(url, g_network_mock_state.last_rtsp_url, url_size - 1);
    url[url_size - 1] = '\0';

    strncpy(headers, g_network_mock_state.last_rtsp_headers, headers_size - 1);
    headers[headers_size - 1] = '\0';
  }
  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return made;
}

void network_mock_set_rtsp_error(int error_code) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.error_code = error_code;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

/* ============================================================================
 * UDP Mock Functions
 * ============================================================================ */

void network_mock_set_udp_response(const uint8_t* response_data, size_t response_size) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  if (response_data && response_size > 0) {
    size_t copy_size = (response_size < sizeof(g_network_mock_state.udp_response_data))
                         ? response_size
                         : sizeof(g_network_mock_state.udp_response_data) - 1;
    memcpy(g_network_mock_state.udp_response_data, response_data, copy_size);
    g_network_mock_state.udp_response_size = copy_size;
  } else {
    g_network_mock_state.udp_response_size = 0;
  }
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

int network_mock_get_last_udp_request(uint8_t* data, size_t* size, char* dest_addr,
                                      size_t addr_size, uint16_t* dest_port) {
  if (!data || !size || !dest_addr || !dest_port || addr_size == 0) {
    return 0;
  }

  pthread_mutex_lock(&g_network_mock_state.mutex);
  int made = g_network_mock_state.last_udp_request_made;
  if (made) {
    *size = g_network_mock_state.last_udp_size;
    memcpy(data, g_network_mock_state.last_udp_data, *size);

    strncpy(dest_addr, g_network_mock_state.last_udp_dest_addr, addr_size - 1);
    dest_addr[addr_size - 1] = '\0';

    *dest_port = g_network_mock_state.last_udp_dest_port;
  }
  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return made;
}

void network_mock_set_udp_error(int error_code) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.error_code = error_code;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

/* ============================================================================
 * Socket Mock Functions
 * ============================================================================ */

void network_mock_set_socket_result(int result) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.socket_result = result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

void network_mock_set_bind_result(int result) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.bind_result = result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

void network_mock_set_listen_result(int result) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.listen_result = result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

void network_mock_set_accept_result(int result) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.accept_result = result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

void network_mock_set_connect_result(int result) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.connect_result = result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

void network_mock_set_send_result(int result) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.send_result = result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

void network_mock_set_recv_result(int result) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.recv_result = result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

/* ============================================================================
 * Network Statistics
 * ============================================================================ */

int network_mock_get_http_request_count(void) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  int count = g_network_mock_state.http_request_count;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return count;
}

int network_mock_get_rtsp_request_count(void) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  int count = g_network_mock_state.rtsp_request_count;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return count;
}

int network_mock_get_udp_request_count(void) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  int count = g_network_mock_state.udp_request_count;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return count;
}

void network_mock_reset_counters(void) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.http_request_count = 0;
  g_network_mock_state.rtsp_request_count = 0;
  g_network_mock_state.udp_request_count = 0;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

/* ============================================================================
 * Network Error Simulation
 * ============================================================================ */

void network_mock_enable_error(int error_type, int error_code) { // NOLINT
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.error_simulation_enabled = 1;
  g_network_mock_state.error_type = error_type;
  g_network_mock_state.error_code = error_code;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

void network_mock_disable_error(void) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.error_simulation_enabled = 0;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

int network_mock_is_error_enabled(void) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  int enabled = g_network_mock_state.error_simulation_enabled;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return enabled;
}

/* ============================================================================
 * Mock Network Function Implementations
 * ============================================================================ */

int mock_http_get(const char* url, char* response, size_t response_size) {
  if (!url || !response || response_size == 0) {
    return -1;
  }

  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.http_request_count++;

  // Store request details
  strncpy(g_network_mock_state.last_http_method, "GET",
          sizeof(g_network_mock_state.last_http_method) - 1);
  g_network_mock_state.last_http_method[sizeof(g_network_mock_state.last_http_method) - 1] = '\0';

  strncpy(g_network_mock_state.last_http_url, url, sizeof(g_network_mock_state.last_http_url) - 1);
  g_network_mock_state.last_http_url[sizeof(g_network_mock_state.last_http_url) - 1] = '\0';

  strcpy(g_network_mock_state.last_http_headers, "");
  strcpy(g_network_mock_state.last_http_body, "");
  g_network_mock_state.last_http_request_made = 1;

  // Check for errors
  if (g_network_mock_state.http_error_enabled || g_network_mock_state.error_simulation_enabled) {
    pthread_mutex_unlock(&g_network_mock_state.mutex);
    return g_network_mock_state.error_code;
  }

  // Generate response
  (void)snprintf(response, response_size, "HTTP/1.1 %d OK\r\nContent-Type: %s\r\n\r\n%s",
                 g_network_mock_state.http_status_code, g_network_mock_state.http_content_type,
                 g_network_mock_state.http_response_body);

  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return 0;
}

int mock_http_post(const char* url, const char* data, size_t data_size, char* response,
                   size_t response_size) {
  if (!url || !response || response_size == 0) {
    return -1;
  }

  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.http_request_count++;

  // Store request details
  strncpy(g_network_mock_state.last_http_method, "POST",
          sizeof(g_network_mock_state.last_http_method) - 1);
  g_network_mock_state.last_http_method[sizeof(g_network_mock_state.last_http_method) - 1] = '\0';

  strncpy(g_network_mock_state.last_http_url, url, sizeof(g_network_mock_state.last_http_url) - 1);
  g_network_mock_state.last_http_url[sizeof(g_network_mock_state.last_http_url) - 1] = '\0';

  strcpy(g_network_mock_state.last_http_headers, "");

  if (data && data_size > 0) {
    size_t copy_size = (data_size < sizeof(g_network_mock_state.last_http_body) - 1)
                         ? data_size
                         : sizeof(g_network_mock_state.last_http_body) - 1;
    strncpy(g_network_mock_state.last_http_body, data, copy_size);
    g_network_mock_state.last_http_body[copy_size] = '\0';
  } else {
    strcpy(g_network_mock_state.last_http_body, "");
  }

  g_network_mock_state.last_http_request_made = 1;

  // Check for errors
  if (g_network_mock_state.http_error_enabled || g_network_mock_state.error_simulation_enabled) {
    pthread_mutex_unlock(&g_network_mock_state.mutex);
    return g_network_mock_state.error_code;
  }

  // Generate response
  (void)snprintf(response, response_size, "HTTP/1.1 %d OK\r\nContent-Type: %s\r\n\r\n%s",
                 g_network_mock_state.http_status_code, g_network_mock_state.http_content_type,
                 g_network_mock_state.http_response_body);

  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return 0;
}

int mock_rtsp_request(const char* url, const char* method, const char* headers, char* response,
                      size_t response_size) {
  if (!url || !method || !response || response_size == 0) {
    return -1;
  }

  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.rtsp_request_count++;

  // Store request details
  strncpy(g_network_mock_state.last_rtsp_method, method,
          sizeof(g_network_mock_state.last_rtsp_method) - 1);
  g_network_mock_state.last_rtsp_method[sizeof(g_network_mock_state.last_rtsp_method) - 1] = '\0';

  strncpy(g_network_mock_state.last_rtsp_url, url, sizeof(g_network_mock_state.last_rtsp_url) - 1);
  g_network_mock_state.last_rtsp_url[sizeof(g_network_mock_state.last_rtsp_url) - 1] = '\0';

  if (headers) {
    strncpy(g_network_mock_state.last_rtsp_headers, headers,
            sizeof(g_network_mock_state.last_rtsp_headers) - 1);
    g_network_mock_state.last_rtsp_headers[sizeof(g_network_mock_state.last_rtsp_headers) - 1] =
      '\0';
  } else {
    strcpy(g_network_mock_state.last_rtsp_headers, "");
  }

  g_network_mock_state.last_rtsp_request_made = 1;

  // Check for errors
  if (g_network_mock_state.error_simulation_enabled) {
    pthread_mutex_unlock(&g_network_mock_state.mutex);
    return g_network_mock_state.error_code;
  }

  // Generate response
  (void)snprintf(response, response_size,
                 "RTSP/1.0 %d OK\r\nCSeq: 1\r\nSession: %s\r\nTransport: %s\r\n\r\n",
                 g_network_mock_state.rtsp_status_code, g_network_mock_state.rtsp_session_id,
                 g_network_mock_state.rtsp_transport);

  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return 0;
}

int mock_udp_send(const uint8_t* data, size_t size, const char* dest_addr, uint16_t dest_port) {
  if (!data || size == 0 || !dest_addr) {
    return -1;
  }

  pthread_mutex_lock(&g_network_mock_state.mutex);
  g_network_mock_state.udp_request_count++;

  // Store request details
  size_t copy_size = (size < sizeof(g_network_mock_state.last_udp_data))
                       ? size
                       : sizeof(g_network_mock_state.last_udp_data);
  memcpy(g_network_mock_state.last_udp_data, data, copy_size);
  g_network_mock_state.last_udp_size = copy_size;

  strncpy(g_network_mock_state.last_udp_dest_addr, dest_addr,
          sizeof(g_network_mock_state.last_udp_dest_addr) - 1);
  g_network_mock_state.last_udp_dest_addr[sizeof(g_network_mock_state.last_udp_dest_addr) - 1] =
    '\0';

  g_network_mock_state.last_udp_dest_port = dest_port;
  g_network_mock_state.last_udp_request_made = 1;

  // Check for errors
  if (g_network_mock_state.error_simulation_enabled) {
    pthread_mutex_unlock(&g_network_mock_state.mutex);
    return g_network_mock_state.error_code;
  }

  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return (int)size; // Return number of bytes sent
}

int mock_udp_recv(uint8_t* data, size_t size, char* src_addr, size_t addr_size,
                  uint16_t* src_port) {
  if (!data || size == 0 || !src_addr || !src_port || addr_size == 0) {
    return -1;
  }

  pthread_mutex_lock(&g_network_mock_state.mutex);

  // Check for errors
  if (g_network_mock_state.error_simulation_enabled) {
    pthread_mutex_unlock(&g_network_mock_state.mutex);
    return g_network_mock_state.error_code;
  }

  // Return mock response data
  size_t copy_size =
    (g_network_mock_state.udp_response_size < size) ? g_network_mock_state.udp_response_size : size;
  memcpy(data, g_network_mock_state.udp_response_data, copy_size);

  strncpy(src_addr, NETWORK_MOCK_DEFAULT_UDP_SRC_ADDR, addr_size - 1);
  src_addr[addr_size - 1] = '\0';
  *src_port = NETWORK_MOCK_DEFAULT_UDP_SRC_PORT;

  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return (int)copy_size;
}

/* ============================================================================
 * Socket Mock Function Implementations
 * ============================================================================ */

int mock_socket(int domain, int type, int protocol) { // NOLINT
  (void)domain;
  (void)type;
  (void)protocol;

  pthread_mutex_lock(&g_network_mock_state.mutex);
  int result = g_network_mock_state.socket_result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);

  return result;
}

int mock_bind(int sockfd, const struct sockaddr* addr, socklen_t addrlen) {
  (void)sockfd;
  (void)addr;
  (void)addrlen;

  pthread_mutex_lock(&g_network_mock_state.mutex);
  int result = g_network_mock_state.bind_result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);

  return result;
}

int mock_listen(int sockfd, int backlog) { // NOLINT
  (void)sockfd;
  (void)backlog;

  pthread_mutex_lock(&g_network_mock_state.mutex);
  int result = g_network_mock_state.listen_result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);

  return result;
}

int mock_accept(int sockfd, struct sockaddr* addr, socklen_t* addrlen) {
  (void)sockfd;
  (void)addr;
  (void)addrlen;

  pthread_mutex_lock(&g_network_mock_state.mutex);
  int result = g_network_mock_state.accept_result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);

  return result;
}

int mock_connect(int sockfd, const struct sockaddr* addr, socklen_t addrlen) {
  (void)sockfd;
  (void)addr;
  (void)addrlen;

  pthread_mutex_lock(&g_network_mock_state.mutex);
  int result = g_network_mock_state.connect_result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);

  return result;
}

int mock_send(int sockfd, const void* buf, size_t len, int flags) { // NOLINT
  (void)sockfd;
  (void)buf;
  (void)len;
  (void)flags;

  pthread_mutex_lock(&g_network_mock_state.mutex);
  int result = g_network_mock_state.send_result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);

  return result;
}

int mock_recv(int sockfd, void* buf, size_t len, int flags) { // NOLINT
  (void)sockfd;
  (void)buf;
  (void)len;
  (void)flags;

  pthread_mutex_lock(&g_network_mock_state.mutex);
  int result = g_network_mock_state.recv_result;
  pthread_mutex_unlock(&g_network_mock_state.mutex);

  return result;
}

int mock_close(int sockfd) {
  (void)sockfd;
  return 0; // Always succeed
}

/* ============================================================================
 * HTTP Response Mock Functions
 * ============================================================================ */

static char g_network_mock_last_header_name[NETWORK_MOCK_HEADER_NAME_SIZE] = {0};   // NOLINT
static char g_network_mock_last_header_value[NETWORK_MOCK_HEADER_VALUE_SIZE] = {0}; // NOLINT

void mock_reset_last_header(void) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  memset(g_network_mock_last_header_name, 0, sizeof(g_network_mock_last_header_name));
  memset(g_network_mock_last_header_value, 0, sizeof(g_network_mock_last_header_value));
  pthread_mutex_unlock(&g_network_mock_state.mutex);
}

const char* mock_get_last_header_name(void) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  const char* name = g_network_mock_last_header_name;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return name;
}

const char* mock_get_last_header_value(void) {
  pthread_mutex_lock(&g_network_mock_state.mutex);
  const char* value = g_network_mock_last_header_value;
  pthread_mutex_unlock(&g_network_mock_state.mutex);
  return value;
}

void http_response_free(mock_http_response_t* response) {
  if (!response) {
    return;
  }

  // Free headers
  if (response->headers) {
    for (size_t i = 0; i < response->header_count; i++) {
      if (response->headers[i].name) {
        free(response->headers[i].name);
      }
      if (response->headers[i].value) {
        free(response->headers[i].value);
      }
    }
    free(response->headers);
    response->headers = NULL;
  }

  if (response->body) {
    free(response->body);
    response->body = NULL;
  }

  if (response->content_type) {
    free(response->content_type);
    response->content_type = NULL;
  }

  response->body_length = 0;
  response->header_count = 0;
}

int http_response_add_header(mock_http_response_t* response, const char* name, const char* value) {
  if (!response || !name || !value) {
    return -1;
  }

  // Store the last header for testing
  pthread_mutex_lock(&g_network_mock_state.mutex);
  strncpy(g_network_mock_last_header_name, name, sizeof(g_network_mock_last_header_name) - 1);
  g_network_mock_last_header_name[sizeof(g_network_mock_last_header_name) - 1] = '\0';
  strncpy(g_network_mock_last_header_value, value, sizeof(g_network_mock_last_header_value) - 1);
  g_network_mock_last_header_value[sizeof(g_network_mock_last_header_value) - 1] = '\0';
  pthread_mutex_unlock(&g_network_mock_state.mutex);

  // Allocate or reallocate headers array
  size_t new_count = response->header_count + 1;
  mock_header_pair_t* new_headers =
    realloc(response->headers, new_count * sizeof(mock_header_pair_t));

  if (!new_headers) {
    return -1; // Memory allocation failed
  }

  response->headers = (void*)new_headers;

  // Allocate and copy header name
  response->headers[response->header_count].name = malloc(strlen(name) + 1);
  if (!response->headers[response->header_count].name) {
    return -1;
  }
  strcpy(response->headers[response->header_count].name, name);

  // Allocate and copy header value
  response->headers[response->header_count].value = malloc(strlen(value) + 1);
  if (!response->headers[response->header_count].value) {
    free(response->headers[response->header_count].name);
    return -1;
  }
  strcpy(response->headers[response->header_count].value, value);

  response->header_count = new_count;
  return 0;
}
