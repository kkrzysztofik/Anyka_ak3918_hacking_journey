/**
 * @file network_mock.h
 * @brief Mock functions for network operations
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef NETWORK_MOCK_H
#define NETWORK_MOCK_H

#include <stddef.h>
#include <stdint.h>
#include <sys/socket.h>
#include <sys/types.h>

/* ============================================================================
 * Mock State Management
 * ============================================================================ */

/**
 * @brief Initialize network mock
 */
void network_mock_init(void);

/**
 * @brief Cleanup network mock
 */
void network_mock_cleanup(void);

/**
 * @brief Reset network mock state
 */
void network_mock_reset(void);

/* ============================================================================
 * HTTP Mock Functions
 * ============================================================================ */

/**
 * @brief Set mock HTTP response
 * @param status_code HTTP status code to return
 * @param response_body Response body to return
 * @param content_type Content type header
 */
void network_mock_set_http_response(int status_code, const char* response_body,
                                    const char* content_type);

/**
 * @brief Get last HTTP request details
 * @param method Output parameter for HTTP method
 * @param url Output parameter for URL
 * @param headers Output parameter for headers
 * @param body Output parameter for request body
 * @return 1 if request was made, 0 if not
 */
int network_mock_get_last_http_request(char* method, size_t method_size, char* url, size_t url_size,
                                       char* headers, size_t headers_size, char* body,
                                       size_t body_size);

/**
 * @brief Set mock HTTP error
 * @param error_code Error code to return
 */
void network_mock_set_http_error(int error_code);

/**
 * @brief Enable/disable HTTP error simulation
 * @param enabled 1 to enable, 0 to disable
 */
void network_mock_set_http_error_enabled(int enabled);

/* ============================================================================
 * RTSP Mock Functions
 * ============================================================================ */

/**
 * @brief Set mock RTSP response
 * @param status_code RTSP status code to return
 * @param session_id Session ID to return
 * @param transport Transport header to return
 */
void network_mock_set_rtsp_response(int status_code, const char* session_id, const char* transport);

/**
 * @brief Get last RTSP request details
 * @param method Output parameter for RTSP method
 * @param url Output parameter for URL
 * @param headers Output parameter for headers
 * @return 1 if request was made, 0 if not
 */
int network_mock_get_last_rtsp_request(char* method, size_t method_size, char* url, size_t url_size,
                                       char* headers, size_t headers_size);

/**
 * @brief Set mock RTSP error
 * @param error_code Error code to return
 */
void network_mock_set_rtsp_error(int error_code);

/* ============================================================================
 * UDP Mock Functions
 * ============================================================================ */

/**
 * @brief Set mock UDP response
 * @param response_data Response data to return
 * @param response_size Size of response data
 */
void network_mock_set_udp_response(const uint8_t* response_data, size_t response_size);

/**
 * @brief Get last UDP request details
 * @param data Output parameter for request data
 * @param size Output parameter for request size
 * @param dest_addr Output parameter for destination address
 * @param dest_port Output parameter for destination port
 * @return 1 if request was made, 0 if not
 */
int network_mock_get_last_udp_request(uint8_t* data, size_t* size, char* dest_addr,
                                      size_t addr_size, uint16_t* dest_port);

/**
 * @brief Set mock UDP error
 * @param error_code Error code to return
 */
void network_mock_set_udp_error(int error_code);

/* ============================================================================
 * Socket Mock Functions
 * ============================================================================ */

/**
 * @brief Set mock socket creation result
 * @param result 0 on success, -1 on failure
 */
void network_mock_set_socket_result(int result);

/**
 * @brief Set mock socket bind result
 * @param result 0 on success, -1 on failure
 */
void network_mock_set_bind_result(int result);

/**
 * @brief Set mock socket listen result
 * @param result 0 on success, -1 on failure
 */
void network_mock_set_listen_result(int result);

/**
 * @brief Set mock socket accept result
 * @param result 0 on success, -1 on failure
 */
void network_mock_set_accept_result(int result);

/**
 * @brief Set mock socket connect result
 * @param result 0 on success, -1 on failure
 */
void network_mock_set_connect_result(int result);

/**
 * @brief Set mock socket send result
 * @param result Number of bytes sent, -1 on failure
 */
void network_mock_set_send_result(int result);

/**
 * @brief Set mock socket receive result
 * @param result Number of bytes received, -1 on failure
 */
void network_mock_set_recv_result(int result);

/* ============================================================================
 * Network Statistics
 * ============================================================================ */

/**
 * @brief Get HTTP request count
 * @return Number of HTTP requests made
 */
int network_mock_get_http_request_count(void);

/**
 * @brief Get RTSP request count
 * @return Number of RTSP requests made
 */
int network_mock_get_rtsp_request_count(void);

/**
 * @brief Get UDP request count
 * @return Number of UDP requests made
 */
int network_mock_get_udp_request_count(void);

/**
 * @brief Reset all request counters
 */
void network_mock_reset_counters(void);

/* ============================================================================
 * Network Error Simulation
 * ============================================================================ */

/**
 * @brief Enable network error simulation
 * @param error_type Type of error to simulate
 * @param error_code Error code to return
 */
void network_mock_enable_error(int error_type, int error_code);

/**
 * @brief Disable network error simulation
 */
void network_mock_disable_error(void);

/**
 * @brief Check if network error simulation is enabled
 * @return 1 if enabled, 0 if disabled
 */
int network_mock_is_error_enabled(void);

/* ============================================================================
 * Mock Network Function Declarations
 * ============================================================================ */

// These are the actual mock functions that replace real network functions
// during testing

/**
 * @brief Mock HTTP GET request
 * @param url URL to request
 * @param response Output buffer for response
 * @param response_size Size of response buffer
 * @return 0 on success, -1 on failure
 */
int mock_http_get(const char* url, char* response, size_t response_size);

/**
 * @brief Mock HTTP POST request
 * @param url URL to request
 * @param data POST data
 * @param data_size Size of POST data
 * @param response Output buffer for response
 * @param response_size Size of response buffer
 * @return 0 on success, -1 on failure
 */
int mock_http_post(const char* url, const char* data, size_t data_size, char* response,
                   size_t response_size);

/**
 * @brief Mock RTSP request
 * @param url RTSP URL
 * @param method RTSP method
 * @param headers RTSP headers
 * @param response Output buffer for response
 * @param response_size Size of response buffer
 * @return 0 on success, -1 on failure
 */
int mock_rtsp_request(const char* url, const char* method, const char* headers, char* response,
                      size_t response_size);

/**
 * @brief Mock UDP send
 * @param data Data to send
 * @param size Size of data
 * @param dest_addr Destination address
 * @param dest_port Destination port
 * @return Number of bytes sent, -1 on failure
 */
int mock_udp_send(const uint8_t* data, size_t size, const char* dest_addr, uint16_t dest_port);

/**
 * @brief Mock UDP receive
 * @param data Buffer for received data
 * @param size Size of buffer
 * @param src_addr Output parameter for source address
 * @param src_port Output parameter for source port
 * @return Number of bytes received, -1 on failure
 */
int mock_udp_recv(uint8_t* data, size_t size, char* src_addr, size_t addr_size, uint16_t* src_port);

/**
 * @brief Mock socket creation
 * @param domain Socket domain
 * @param type Socket type
 * @param protocol Protocol
 * @return Socket file descriptor, -1 on failure
 */
int mock_socket(int domain, int type, int protocol);

/**
 * @brief Mock socket bind
 * @param sockfd Socket file descriptor
 * @param addr Address structure
 * @param addrlen Address length
 * @return 0 on success, -1 on failure
 */
int mock_bind(int sockfd, const struct sockaddr* addr, socklen_t addrlen);

/**
 * @brief Mock socket listen
 * @param sockfd Socket file descriptor
 * @param backlog Backlog size
 * @return 0 on success, -1 on failure
 */
int mock_listen(int sockfd, int backlog);

/**
 * @brief Mock socket accept
 * @param sockfd Socket file descriptor
 * @param addr Address structure
 * @param addrlen Address length
 * @return New socket file descriptor, -1 on failure
 */
int mock_accept(int sockfd, struct sockaddr* addr, socklen_t* addrlen);

/**
 * @brief Mock socket connect
 * @param sockfd Socket file descriptor
 * @param addr Address structure
 * @param addrlen Address length
 * @return 0 on success, -1 on failure
 */
int mock_connect(int sockfd, const struct sockaddr* addr, socklen_t addrlen);

/**
 * @brief Mock socket send
 * @param sockfd Socket file descriptor
 * @param buf Buffer to send
 * @param len Length of buffer
 * @param flags Send flags
 * @return Number of bytes sent, -1 on failure
 */
int mock_send(int sockfd, const void* buf, size_t len, int flags);

/**
 * @brief Mock socket receive
 * @param sockfd Socket file descriptor
 * @param buf Buffer for received data
 * @param len Length of buffer
 * @param flags Receive flags
 * @return Number of bytes received, -1 on failure
 */
int mock_recv(int sockfd, void* buf, size_t len, int flags);

/**
 * @brief Mock socket close
 * @param sockfd Socket file descriptor
 * @return 0 on success, -1 on failure
 */
int mock_close(int sockfd);

/* ============================================================================
 * HTTP Response Mock Functions
 * ============================================================================ */

/**
 * @brief Mock HTTP response structure (matches http_response_t)
 */
typedef struct {
  int status_code;
  char* content_type;
  char* body;
  size_t body_length;
  struct {
    char* name;
    char* value;
  }* headers;
  size_t header_count;
} mock_http_response_t;

/**
 * @brief Mock HTTP response functions
 */
void mock_reset_last_header(void);
const char* mock_get_last_header_name(void);
const char* mock_get_last_header_value(void);
void http_response_free(mock_http_response_t* response);
int http_response_add_header(mock_http_response_t* response, const char* name, const char* value);

#endif // NETWORK_MOCK_H
