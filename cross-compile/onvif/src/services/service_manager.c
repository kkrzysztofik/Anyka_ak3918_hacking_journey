/**
 * @file service_manager.c
 * @brief ONVIF service manager implementation.
 */

#include "service_manager.h"
#include "services/ptz/onvif_ptz.h"
#include "services/imaging/onvif_imaging.h"
#include "platform.h"
#include <stdio.h>

static bool g_services_initialized = false;

int onvif_services_init(platform_vi_handle_t vi_handle) {
    if (g_services_initialized) {
        return 0; // Already initialized
    }
    
    platform_log_info("Initializing ONVIF services...\n");
    
    // Initialize PTZ service
    if (ptz_adapter_init() != 0) {
        platform_log_warning("Failed to initialize PTZ service\n");
        // PTZ is optional, continue with other services
    } else {
        platform_log_info("PTZ service initialized\n");
    }
    
    // Initialize Imaging service
    if (onvif_imaging_init(vi_handle) != 0) {
        platform_log_warning("Failed to initialize Imaging service\n");
        // Imaging is optional, continue with other services
    } else {
        platform_log_info("Imaging service initialized\n");
    }
    
    g_services_initialized = true;
    platform_log_info("ONVIF services initialization completed\n");
    
    return 0;
}

void onvif_services_cleanup(void) {
    if (!g_services_initialized) {
        return; // Already cleaned up
    }
    
    platform_log_info("Cleaning up ONVIF services...\n");
    
    // Cleanup services in reverse order
    onvif_imaging_cleanup();
    ptz_adapter_shutdown();
    
    g_services_initialized = false;
    platform_log_info("ONVIF services cleanup completed\n");
}

bool onvif_services_initialized(void) {
    return g_services_initialized;
}
