/**
 * @file smart_response_builder.h
 * @brief Smart response builder utilities with dynamic allocation strategies
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides smart response building utilities that automatically
 * select optimal memory allocation strategies based on response size, using
 * existing memory infrastructure for maximum efficiency.
 */

#ifndef SMART_RESPONSE_BUILDER_H
#define SMART_RESPONSE_BUILDER_H

#include "networking/common/buffer_pool.h"
#include "networking/http/http_parser.h"

#include <stddef.h>

/* ============================================================================
 * Smart Response Builder Functions
 * ============================================================================
 */

/**
 * @brief Build SOAP response using existing dynamic buffer infrastructure
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content SOAP content to include (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Uses existing memory_manager utilities per AGENTS.md standards
 */
int smart_response_build_with_dynamic_buffer(http_response_t* response, const char* soap_content);

/**
 * @brief Build SOAP response using existing buffer pool infrastructure
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content SOAP content to include (must not be NULL)
 * @param buffer_pool Buffer pool to use for allocation (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Uses existing buffer_pool utilities per AGENTS.md standards
 */
int smart_response_build_with_buffer_pool(http_response_t* response, const char* soap_content,
                                          buffer_pool_t* buffer_pool);

/**
 * @brief Smart response builder with dynamic allocation strategy selection
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content SOAP content to include (must not be NULL)
 * @param estimated_size Estimated response size for strategy selection
 * @param buffer_pool Buffer pool to use for medium responses (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Automatically selects optimal allocation strategy based on size
 */
int smart_response_build(http_response_t* response, const char* soap_content, size_t estimated_size,
                         buffer_pool_t* buffer_pool);

/**
 * @brief Estimate response size for strategy selection
 * @param soap_content SOAP content to estimate (must not be NULL)
 * @return Estimated response size in bytes
 * @note Includes XML wrapper overhead in estimation
 */
size_t smart_response_estimate_size(const char* soap_content);

#endif /* SMART_RESPONSE_BUILDER_H */
