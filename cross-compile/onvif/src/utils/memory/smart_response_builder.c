/**
 * @file smart_response_builder.c
 * @brief Smart response builder utilities with dynamic allocation strategies
 * @author kkrzysztofik
 * @date 2025
 */

#include "smart_response_builder.h"

#include "/home/kmk/anyka-dev/cross-compile/onvif/src/networking/common/buffer_pool.h"
#include "/home/kmk/anyka-dev/cross-compile/onvif/src/networking/http/http_parser.h"
#include "common/onvif_constants.h"
#include "platform/platform.h"
#include "utils/memory/memory_manager.h"

#include <string.h>

/* ============================================================================
 * Constants
 * ============================================================================
 */

/* Buffer pool size threshold for allocation strategy selection */
#define BUFFER_POOL_SIZE_THRESHOLD 32768

/* ============================================================================
 * Smart Response Builder Functions
 * ============================================================================
 */

/**
 * @brief Build SOAP response using existing dynamic buffer infrastructure
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content Complete SOAP response content (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Uses existing memory_manager utilities per AGENTS.md standards
 * @note soap_content should already contain complete SOAP envelope from gSOAP
 */
int smart_response_build_with_dynamic_buffer(http_response_t* response, const char* soap_content) {
  if (!response || !soap_content) {
    return -1; // Input validation per AGENTS.md
  }

  // gSOAP already provides complete SOAP response, no need to wrap
  size_t response_length = strlen(soap_content);
  response->body = ONVIF_MALLOC(response_length + 1);
  if (!response->body) {
    return -1;
  }

  // Single copy from gSOAP response to final response
  memcpy(response->body, soap_content, response_length);
  response->body[response_length] = '\0';
  response->body_length = response_length;

  platform_log_debug("Response allocated: %zu bytes (saved %zu bytes)", response_length,
                     ONVIF_RESPONSE_BUFFER_SIZE - response_length);
  return 0;
}

/**
 * @brief Build SOAP response using existing buffer pool infrastructure
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content Complete SOAP response content (must not be NULL)
 * @param buffer_pool Buffer pool to use for allocation (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Uses existing buffer_pool utilities per AGENTS.md standards
 * @note soap_content should already contain complete SOAP envelope from gSOAP
 */
int smart_response_build_with_buffer_pool(http_response_t* response, const char* soap_content,
                                          buffer_pool_t* buffer_pool) {
  if (!response || !soap_content || !buffer_pool) {
    return -1; // Input validation per AGENTS.md
  }

  size_t content_length = strlen(soap_content);

  // Check if response fits in buffer pool
  if (content_length < BUFFER_SIZE) {
    char* pool_buffer = buffer_pool_get(buffer_pool);
    if (pool_buffer) {
      // Use pool buffer directly - zero-copy for small responses
      memcpy(pool_buffer, soap_content, content_length);
      pool_buffer[content_length] = '\0';

      // Allocate final response buffer
      response->body = ONVIF_MALLOC(content_length + 1);
      if (!response->body) {
        buffer_pool_return(buffer_pool, pool_buffer);
        return -1;
      }

      // Single copy from pool to final response
      memcpy(response->body, pool_buffer, content_length);
      response->body[content_length] = '\0';
      response->body_length = content_length;

      // Return buffer to pool
      buffer_pool_return(buffer_pool, pool_buffer);

      platform_log_debug("Pool response: %zu bytes (saved %zu bytes)", content_length,
                         ONVIF_RESPONSE_BUFFER_SIZE - content_length);
      return 0;
    }
    // Pool exhausted - fall back to direct allocation
    platform_log_warning("Buffer pool exhausted, falling back to direct allocation for %zu "
                         "bytes",
                         content_length);
  } else {
    // Response too large for buffer pool - use direct allocation
    platform_log_debug("Response too large for buffer pool (%zu bytes), using direct "
                       "allocation",
                       content_length);
  }

  // Direct allocation fallback
  response->body = ONVIF_MALLOC(content_length + 1);
  if (!response->body) {
    return -1;
  }

  // Single copy from gSOAP response to final response
  memcpy(response->body, soap_content, content_length);
  response->body[content_length] = '\0';
  response->body_length = content_length;

  platform_log_debug("Direct response: %zu bytes", content_length);
  return 0;
}

/**
 * @brief Smart response builder with dynamic allocation strategy selection
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content Complete SOAP response content (must not be NULL)
 * @param estimated_size Estimated response size for strategy selection
 * @param buffer_pool Buffer pool to use for medium responses (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Automatically selects optimal allocation strategy based on size
 * @note soap_content should already contain complete SOAP envelope from gSOAP
 */
int smart_response_build(http_response_t* response, const char* soap_content, size_t estimated_size,
                         buffer_pool_t* buffer_pool) {
  if (!response || !soap_content || !buffer_pool) {
    return -1; // Input validation per AGENTS.md
  }

  // Strategy selection based on estimated size - optimized for buffer pool
  // utilization
  if (estimated_size <= BUFFER_POOL_SIZE_THRESHOLD) {
    // Small to medium response (≤32KB) - use buffer pool for maximum
    // utilization
    platform_log_debug("Using buffer pool for response: %zu bytes (≤32KB)", estimated_size);
    return smart_response_build_with_buffer_pool(response, soap_content, buffer_pool);
  }

  // Large response (>32KB) - use direct allocation with tracking
  platform_log_debug("Using direct allocation for large response: %zu bytes (>32KB)",
                     estimated_size);

  // gSOAP already provides complete SOAP response, no need to wrap
  size_t content_length = strlen(soap_content);
  response->body = ONVIF_MALLOC(content_length + 1);
  if (!response->body) {
    return -1;
  }

  // Single copy from gSOAP response to final response
  memcpy(response->body, soap_content, content_length);
  response->body[content_length] = '\0';
  response->body_length = content_length;

  platform_log_debug("Large response: %zu bytes (direct allocation)", response->body_length);
  return 0;
}

/**
 * @brief Estimate response size for strategy selection
 * @param soap_content Complete SOAP response content to estimate (must not be
 * NULL)
 * @return Estimated response size in bytes
 * @note gSOAP already provides complete response, no additional overhead needed
 */
size_t smart_response_estimate_size(const char* soap_content) {
  if (!soap_content) {
    return 0;
  }

  // gSOAP already provides complete SOAP response, use actual length
  return strlen(soap_content);
}
