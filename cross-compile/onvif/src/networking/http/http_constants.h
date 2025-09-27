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

/** @brief Socket listen backlog size */
#define HTTP_SOCKET_BACKLOG_SIZE 10

/** @brief HTTP status code for successful authentication */
#define HTTP_AUTH_SUCCESS_CODE 200

/** @brief HTTP status code for authentication failure */
#define HTTP_AUTH_FAILURE_CODE 401

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

#endif /* HTTP_CONSTANTS_H */
