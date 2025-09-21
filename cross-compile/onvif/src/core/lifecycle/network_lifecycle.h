/**
 * @file network_lifecycle.h
 * @brief Network services lifecycle management
 *
 * This module provides centralized management for network services including
 * HTTP server, WS-Discovery, and snapshot service initialization and cleanup.
 *
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_NETWORK_LIFECYCLE_H
#define ONVIF_NETWORK_LIFECYCLE_H

#include <stdbool.h>

#include "core/config/config.h"

/**
 * @brief Initialize network services (HTTP server, WS-Discovery, snapshot)
 * @param cfg Application configuration containing network parameters
 * @return 0 on success, -1 on failure
 * @note HTTP server failure is fatal, other services are non-fatal
 */
int network_lifecycle_init(const struct application_config *cfg);

/**
 * @brief Cleanup network services
 * @note This function is idempotent and safe to call multiple times
 */
void network_lifecycle_cleanup(void);

/**
 * @brief Check if network services are initialized
 * @return true if network services are ready, false otherwise
 */
bool network_lifecycle_initialized(void);

/**
 * @brief Start optional network services (non-fatal if some fail)
 * @param cfg Application configuration
 */
void network_lifecycle_start_optional_services(
    const struct application_config *cfg);

#endif /* ONVIF_NETWORK_LIFECYCLE_H */
