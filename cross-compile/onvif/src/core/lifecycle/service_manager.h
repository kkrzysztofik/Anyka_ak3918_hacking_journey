/**
 * @file service_manager.h
 * @brief ONVIF service manager for centralized service lifecycle management
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_SERVICE_MANAGER_H
#define ONVIF_SERVICE_MANAGER_H

#include "platform/platform_common.h"

#include <stdbool.h>

/**
 * @brief Initialize all ONVIF services
 * @param vi_handle Video input handle (can be NULL if not available)
 * @return 0 on success, negative error code on failure
 */
int onvif_services_init(platform_vi_handle_t vi_handle);

/**
 * @brief Cleanup all ONVIF services
 */
void onvif_services_cleanup(void);

/**
 * @brief Check if services are initialized
 * @return true if services are initialized, false otherwise
 */
bool onvif_services_initialized(void);

#endif /* ONVIF_SERVICE_MANAGER_H */
