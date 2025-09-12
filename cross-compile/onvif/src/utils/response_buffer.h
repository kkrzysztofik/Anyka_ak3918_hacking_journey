/**
 * @file response_buffer.h
 * @brief RAII response buffer management for ONVIF services.
 * 
 * This module provides automatic memory management for ONVIF response buffers
 * using RAII patterns to prevent memory leaks and ensure proper cleanup.
 */

#ifndef RESPONSE_BUFFER_H
#define RESPONSE_BUFFER_H

#include "../common/onvif_request.h"
#include <stddef.h>

/**
 * @brief Response buffer with automatic cleanup
 */
typedef struct {
    onvif_response_t *response;    /**< Pointer to response structure */
    int owned;                     /**< Whether this structure owns the response */
} response_buffer_t;

/**
 * @brief Initialize response buffer with automatic cleanup
 * @param buffer Buffer structure to initialize
 * @param response Response structure to manage (can be NULL)
 * @return 0 on success, -1 on error
 */
int response_buffer_init(response_buffer_t *buffer, onvif_response_t *response);

/**
 * @brief Create new response buffer with allocated response
 * @param buffer Buffer structure to initialize
 * @return 0 on success, -1 on error
 */
int response_buffer_create(response_buffer_t *buffer);

/**
 * @brief Cleanup response buffer and free all resources
 * @param buffer Buffer structure to cleanup
 */
void response_buffer_cleanup(response_buffer_t *buffer);

/**
 * @brief Get response pointer from buffer
 * @param buffer Buffer structure
 * @return Response pointer or NULL if not initialized
 */
onvif_response_t *response_buffer_get(response_buffer_t *buffer);

/**
 * @brief Set response body with automatic memory management
 * @param buffer Buffer structure
 * @param body Body content to set
 * @param length Length of body content
 * @return 0 on success, -1 on error
 */
int response_buffer_set_body(response_buffer_t *buffer, const char *body, size_t length);

/**
 * @brief Set response body using printf-style formatting
 * @param buffer Buffer structure
 * @param format Printf format string
 * @param ... Format arguments
 * @return 0 on success, -1 on error
 */
int response_buffer_set_body_printf(response_buffer_t *buffer, const char *format, ...);

/**
 * @brief Macro for automatic cleanup using RAII pattern
 */
#define RESPONSE_BUFFER_AUTO(buffer) \
    response_buffer_t buffer; \
    response_buffer_init(&buffer, NULL); \
    __attribute__((cleanup(response_buffer_cleanup)))

#endif /* RESPONSE_BUFFER_H */
