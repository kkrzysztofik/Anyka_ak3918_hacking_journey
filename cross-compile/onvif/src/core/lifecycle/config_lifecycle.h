/**
 * @file config_lifecycle.h
 * @brief Configuration management lifecycle
 *
 * This module provides centralized management for configuration loading,
 * memory allocation for configuration structures, and configuration cleanup.
 *
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_CONFIG_LIFECYCLE_H
#define ONVIF_CONFIG_LIFECYCLE_H

#include "core/config/config.h"

#include <stdbool.h>

/**
 * @brief Allocate memory for configuration structures
 * @param cfg Configuration structure to initialize
 * @return 0 on success, -1 on failure
 */
int config_lifecycle_allocate_memory(struct application_config* cfg);

/**
 * @brief Load configuration from file and initialize stream configs
 * @param cfg Configuration structure to populate
 * @return 0 on success, -1 on failure
 */
int config_lifecycle_load_configuration(struct application_config* cfg);

/**
 * @brief Free all allocated configuration memory
 * @param cfg Configuration structure to clean up
 */
void config_lifecycle_free_memory(struct application_config* cfg);

/**
 * @brief Check if configuration is loaded
 * @return true if configuration is ready, false otherwise
 */
bool config_lifecycle_loaded(void);

/**
 * @brief Get configuration summary for debugging
 * @param summary Buffer to store configuration summary
 * @param size Size of the summary buffer
 * @return 0 on success, -1 on failure
 */
int config_lifecycle_get_summary(char* summary, size_t size);

#endif /* ONVIF_CONFIG_LIFECYCLE_H */
