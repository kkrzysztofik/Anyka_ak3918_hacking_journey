/**
 * @file http_constants.h
 * @brief HTTP server constants and configuration values
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef HTTP_CONSTANTS_H
#define HTTP_CONSTANTS_H

/* ============================================================================
 * HTTP Protocol Constants
 * ============================================================================
 */

/** @brief Length of "Authorization:" header prefix */
#define HTTP_AUTH_HEADER_PREFIX_LEN 8

/** @brief Length of "Basic " authentication prefix */
#define HTTP_BASIC_AUTH_PREFIX_LEN 6

/** @brief Maximum length for operation names */
#define HTTP_OPERATION_NAME_MAX_LEN 64

/** @brief Buffer size for operation name storage */
#define HTTP_OPERATION_NAME_BUFFER_SIZE 65

/** @brief Maximum length for credentials buffer */
#define HTTP_CREDENTIALS_BUFFER_SIZE 128

/** @brief Maximum length for client IP address string */
#define HTTP_CLIENT_IP_MAX_LEN 16

/** @brief Maximum length for client IP address with null terminator */
#define HTTP_CLIENT_IP_BUFFER_SIZE 17

/** @brief Maximum length for enhanced action name with client IP */
#define HTTP_ENHANCED_ACTION_MAX_LEN 256

/** @brief Socket listen backlog size */
#define HTTP_SOCKET_BACKLOG_SIZE 10

/** @brief HTTP status code for successful authentication */
#define HTTP_AUTH_SUCCESS_CODE 200

/** @brief HTTP status code for authentication failure */
#define HTTP_AUTH_FAILURE_CODE 401

/* ============================================================================
 * HTTP Status Codes
 * ============================================================================
 */

/** @brief HTTP 200 OK */
#define HTTP_STATUS_OK 200

/** @brief HTTP 400 Bad Request */
#define HTTP_STATUS_BAD_REQUEST 400

/** @brief HTTP 401 Unauthorized */
#define HTTP_STATUS_UNAUTHORIZED 401

/** @brief HTTP 403 Forbidden */
#define HTTP_STATUS_FORBIDDEN 403

/** @brief HTTP 404 Not Found */
#define HTTP_STATUS_NOT_FOUND 404

/** @brief HTTP 408 Request Timeout */
#define HTTP_STATUS_REQUEST_TIMEOUT 408

/** @brief HTTP 507 Insufficient Storage */
#define HTTP_STATUS_INSUFFICIENT_STORAGE 507

/** @brief HTTP 503 Service Unavailable */
#define HTTP_STATUS_SERVICE_UNAVAILABLE 503

/** @brief HTTP 500 Internal Server Error */
#define HTTP_STATUS_INTERNAL_SERVER_ERROR 500

/* ============================================================================
 * HTTP Status Code Ranges
 * ============================================================================
 */

/** @brief HTTP status code range constants */
#define HTTP_STATUS_SUCCESS_MIN      200
#define HTTP_STATUS_SUCCESS_MAX      299
#define HTTP_STATUS_CLIENT_ERROR_MIN 400
#define HTTP_STATUS_CLIENT_ERROR_MAX 499
#define HTTP_STATUS_SERVER_ERROR_MIN 500

/* ============================================================================
 * SOAP/XML Parsing Constants
 * ============================================================================
 */

/** @brief Length of "<Action>" tag */
#define SOAP_ACTION_TAG_LEN 8

/** @brief Length of "</Action>" tag */
#define SOAP_ACTION_END_TAG_LEN 9

/** @brief Length of "<soapenv:Body>" tag */
#define SOAP_BODY_TAG_LEN 13

/** @brief Length of "<Body>" tag */
#define SOAP_SIMPLE_BODY_TAG_LEN 6

/** @brief Length of "</Body>" tag */
#define SOAP_BODY_END_TAG_LEN 7

/* ============================================================================
 * HTTP Request Field Sizes
 * ============================================================================
 */

/** @brief Maximum size for HTTP method string (GET, POST, PUT, DELETE, etc.) */
#define HTTP_METHOD_SIZE 16

/** @brief Maximum size for HTTP request path/URI */
#define HTTP_PATH_SIZE 256

/** @brief Maximum size for HTTP version string (HTTP/1.1, HTTP/1.0) */
#define HTTP_VERSION_SIZE 16

/* ============================================================================
 * HTTP Request Processing Constants
 * ============================================================================
 */

/** @brief Buffer size for error message strings */
#define HTTP_ERROR_MSG_BUFFER_SIZE 256

/** @brief Chunk size for reading HTTP request data (4KB) */
#define HTTP_READ_CHUNK_SIZE 4096

/** @brief Buffer size for debug log messages */
#define HTTP_DEBUG_LOG_BUFFER_SIZE 512

/** @brief Maximum number of read retry attempts */
#define HTTP_MAX_READ_RETRIES 10

/** @brief Read timeout in milliseconds */
#define HTTP_READ_TIMEOUT_MS 5000

#endif /* HTTP_CONSTANTS_H */
