/**
 * @file response_buffer.c
 * @brief RAII response buffer management implementation.
 */

#include "response_buffer.h"
#include "platform/platform.h"
#include "memory_debug.h"
#include "constants_clean.h"
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <stdio.h>

int response_buffer_init(response_buffer_t *buffer, onvif_response_t *response) {
    if (!buffer) {
        return -1;
    }
    
    buffer->response = response;
    buffer->owned = (response == NULL);
    
    if (buffer->owned) {
        buffer->response = MEMORY_DEBUG_MALLOC(sizeof(onvif_response_t));
        if (!buffer->response) {
            platform_log_error("Failed to allocate response structure\n");
            return -1;
        }
        
        memset(buffer->response, 0, sizeof(onvif_response_t));
        buffer->response->status_code = 200;
        buffer->response->content_type = "application/soap+xml";
    }
    
    return 0;
}

int response_buffer_create(response_buffer_t *buffer) {
    return response_buffer_init(buffer, NULL);
}

void response_buffer_cleanup(response_buffer_t *buffer) {
    if (!buffer) return;
    
    if (buffer->response) {
        if (buffer->response->body) {
            MEMORY_DEBUG_FREE(buffer->response->body);
            buffer->response->body = NULL;
        }
        
        if (buffer->response->content_type && buffer->owned) {
            // Only free content_type if we own the response
            MEMORY_DEBUG_FREE(buffer->response->content_type);
            buffer->response->content_type = NULL;
        }
        
        if (buffer->owned) {
            MEMORY_DEBUG_FREE(buffer->response);
        }
        buffer->response = NULL;
    }
    
    buffer->owned = 0;
}

onvif_response_t *response_buffer_get(response_buffer_t *buffer) {
    if (!buffer) return NULL;
    return buffer->response;
}

int response_buffer_set_body(response_buffer_t *buffer, const char *body, size_t length) {
    if (!buffer || !buffer->response) {
        return -1;
    }
    
    // Free existing body
    if (buffer->response->body) {
        MEMORY_DEBUG_FREE(buffer->response->body);
        buffer->response->body = NULL;
    }
    
    if (!body || length == 0) {
        buffer->response->body_length = 0;
        return 0;
    }
    
    // Allocate new body
    buffer->response->body = MEMORY_DEBUG_MALLOC(length + 1);
    if (!buffer->response->body) {
        platform_log_error("Failed to allocate response body\n");
        return -1;
    }
    
    memcpy(buffer->response->body, body, length);
    buffer->response->body[length] = '\0';
    buffer->response->body_length = length;
    
    return 0;
}

int response_buffer_set_body_printf(response_buffer_t *buffer, const char *format, ...) {
    if (!buffer || !buffer->response || !format) {
        return -1;
    }
    
    // Free existing body
    if (buffer->response->body) {
        MEMORY_DEBUG_FREE(buffer->response->body);
        buffer->response->body = NULL;
    }
    
    // Allocate buffer for formatted string
    buffer->response->body = MEMORY_DEBUG_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);
    if (!buffer->response->body) {
        platform_log_error("Failed to allocate response body buffer\n");
        return -1;
    }
    
    va_list args;
    va_start(args, format);
    int len = vsnprintf(buffer->response->body, ONVIF_RESPONSE_BUFFER_SIZE, format, args);
    va_end(args);
    
    if (len < 0) {
        platform_log_error("Failed to format response body\n");
        free(buffer->response->body);
        buffer->response->body = NULL;
        return -1;
    }
    
    if (len >= ONVIF_RESPONSE_BUFFER_SIZE) {
        // Buffer too small, reallocate with exact size
        char *new_body = MEMORY_DEBUG_REALLOC(buffer->response->body, len + 1);
        if (!new_body) {
            platform_log_error("Failed to reallocate response body\n");
            MEMORY_DEBUG_FREE(buffer->response->body);
            buffer->response->body = NULL;
            return -1;
        }
        buffer->response->body = new_body;
        
        va_start(args, format);
        vsnprintf(buffer->response->body, len + 1, format, args);
        va_end(args);
    }
    
    buffer->response->body_length = len;
    return 0;
}
