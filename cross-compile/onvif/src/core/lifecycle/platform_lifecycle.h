/**
 * @file platform_lifecycle.h
 * @brief Platform initialization and cleanup management
 *
 * This module provides centralized management for platform initialization,
 * memory management, and overall system cleanup operations.
 *
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_PLATFORM_LIFECYCLE_H
#define ONVIF_PLATFORM_LIFECYCLE_H

#include <stdbool.h>

/**
 * @brief Initialize platform and memory management
 * @return 0 on success, -1 on failure
 */
int platform_lifecycle_init(void);

/**
 * @brief Perform full system cleanup
 * @note This function is idempotent and safe to call multiple times
 */
void platform_lifecycle_cleanup(void);

/**
 * @brief Check if platform is initialized
 * @return true if platform is ready, false otherwise
 */
bool platform_lifecycle_initialized(void);

/**
 * @brief Get platform initialization status
 * @return Platform status code
 */
int platform_lifecycle_get_status(void);

#endif /* ONVIF_PLATFORM_LIFECYCLE_H */
