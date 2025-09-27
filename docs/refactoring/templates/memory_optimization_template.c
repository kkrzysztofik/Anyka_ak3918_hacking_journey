/**
 * @file memory_optimization_template.c
 * @brief Template for memory-optimized response building
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Project-specific headers (relative from src/)
#include "networking/common/buffer_pool.h"
#include "platform/platform.h"
#include "services/common/onvif_types.h"
#include "utils/memory/memory_manager.h"

// Global variables (MANDATORY: g_<module>_<variable_name> naming)
static int g_memory_optimization_enabled = 1;        // NOLINT
static char g_response_buffer_type[32] = "dynamic";  // NOLINT

/**
 * @brief Build SOAP response using existing dynamic buffer infrastructure
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content SOAP content to include (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Uses existing memory_manager utilities per AGENTS.md standards
 *       For small responses (<4KB)
 */
static int build_response_with_dynamic_buffer(onvif_response_t *response,
                                              const char *soap_content) {
  if (!response || !soap_content) {
    return -1;  // Input validation per AGENTS.md
  }

  dynamic_buffer_t response_buffer;
  dynamic_buffer_init(&response_buffer, 0);  // Default size

  // Build SOAP response safely
  dynamic_buffer_append_string(&response_buffer, "<?xml version=\"1.0\"?>");
  dynamic_buffer_appendf(&response_buffer, "<soap:Envelope>%s</soap:Envelope>",
                         soap_content);

  // Use exact-size allocation with proper error handling
  size_t response_length = dynamic_buffer_length(&response_buffer);
  if (response_length == 0) {
    dynamic_buffer_cleanup(&response_buffer);
    return -1;
  }

  response->body = ONVIF_MALLOC(response_length + 1);
  if (!response->body) {
    dynamic_buffer_cleanup(&response_buffer);
    return -1;  // Memory allocation failure
  }

  memcpy(response->body, dynamic_buffer_data(&response_buffer),
         response_length);
  response->body[response_length] = '\0';  // Ensure null termination
  response->body_length = response_length;

  dynamic_buffer_cleanup(&response_buffer);

  platform_log_debug("Dynamic response: %zu bytes (saved %zu bytes)",
                     response_length,
                     ONVIF_RESPONSE_BUFFER_SIZE - response_length);
  return 0;
}

/**
 * @brief Build SOAP response using existing buffer pool infrastructure
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content SOAP content to include (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Uses existing buffer_pool utilities per AGENTS.md standards
 *       For medium responses (4-32KB)
 */
static int build_response_with_buffer_pool(onvif_response_t *response,
                                           const char *soap_content) {
  if (!response || !soap_content) {
    return -1;  // Input validation per AGENTS.md
  }

  char *pool_buffer = buffer_pool_get(&g_networking_response_buffer_pool);
  if (pool_buffer) {
    // Build response in pool buffer with safe string functions
    int result =
        snprintf(pool_buffer, BUFFER_SIZE,
                 "<?xml version=\"1.0\"?><soap:Envelope>%s</soap:Envelope>",
                 soap_content);

    if (result < 0 || result >= BUFFER_SIZE) {
      buffer_pool_return(&g_networking_response_buffer_pool, pool_buffer);
      return -1;  // Buffer too small or encoding error
    }

    size_t actual_length = (size_t)result;
    response->body = ONVIF_MALLOC(actual_length + 1);
    if (!response->body) {
      buffer_pool_return(&g_networking_response_buffer_pool, pool_buffer);
      return -1;
    }

    // Use safe string copy
    strncpy(response->body, pool_buffer, actual_length + 1);
    response->body[actual_length] = '\0';  // Ensure null termination
    response->body_length = actual_length;

    // Return buffer to pool
    buffer_pool_return(&g_networking_response_buffer_pool, pool_buffer);

    platform_log_debug("Pool response: %zu bytes (saved %zu bytes)",
                       actual_length,
                       ONVIF_RESPONSE_BUFFER_SIZE - actual_length);
    return 0;
  } else {
    // Pool exhausted - fall back to exact allocation
    size_t needed = strlen(soap_content) + 64;  // XML wrapper overhead
    response->body = ONVIF_MALLOC(needed);
    if (!response->body) {
      return -1;
    }

    int result =
        snprintf(response->body, needed,
                 "<?xml version=\"1.0\"?><soap:Envelope>%s</soap:Envelope>",
                 soap_content);
    if (result < 0 || result >= (int)needed) {
      ONVIF_FREE(response->body);
      response->body = NULL;
      return -1;
    }

    response->body_length = (size_t)result;
    platform_log_debug("Fallback response: %zu bytes", response->body_length);
    return 0;
  }
}

/**
 * @brief Smart response builder that chooses optimal allocation strategy
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content SOAP content to include (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Automatically chooses between dynamic buffer, buffer pool, or direct
 * allocation based on content size for optimal memory usage
 */
int build_smart_response(onvif_response_t *response, const char *soap_content) {
  if (!response || !soap_content) {
    return -1;  // Input validation per AGENTS.md
  }

  // Estimate response size
  size_t estimated_size = strlen(soap_content) + 64;  // XML wrapper overhead

  if (estimated_size < 4096) {
    // Small response - use dynamic buffer
    return build_response_with_dynamic_buffer(response, soap_content);
  } else if (estimated_size < 32768) {
    // Medium response - use buffer pool
    return build_response_with_buffer_pool(response, soap_content);
  } else {
    // Large response - use direct allocation with tracking
    response->body = ONVIF_MALLOC(estimated_size + 1);
    if (!response->body) {
      return -1;
    }

    int result =
        snprintf(response->body, estimated_size + 1,
                 "<?xml version=\"1.0\"?><soap:Envelope>%s</soap:Envelope>",
                 soap_content);
    if (result < 0 || result >= (int)(estimated_size + 1)) {
      ONVIF_FREE(response->body);
      response->body = NULL;
      return -1;
    }

    response->body_length = (size_t)result;
    platform_log_debug("Direct response: %zu bytes", response->body_length);
    return 0;
  }
}

/**
 * @brief Cleanup response memory using tracked allocation
 * @param response Response structure to cleanup (must not be NULL)
 * @note Uses ONVIF_FREE for proper memory tracking
 */
void cleanup_response(onvif_response_t *response) {
  if (!response) {
    return;  // Input validation
  }

  if (response->body) {
    ONVIF_FREE(response->body);
    response->body = NULL;
    response->body_length = 0;
  }
}
