/**
 * @file onvif_snapshot.c
 * @brief ONVIF Snapshot service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <pthread.h>

#include "platform.h"
#include "onvif_snapshot.h"
#include "common/onvif_constants.h"
#include "utils/error/error_handling.h"
#include "utils/network/network_utils.h"
#include "utils/memory/memory_manager.h"
#include "protocol/soap/onvif_soap.h"
#include "protocol/xml/unified_xml.h"
#include "protocol/response/onvif_service_handler.h"

/* Snapshot service state */
static bool g_snapshot_initialized = false;
static platform_snapshot_handle_t g_snapshot_handle = NULL;
static platform_vi_handle_t g_vi_handle = NULL;
static pthread_mutex_t g_snapshot_mutex = PTHREAD_MUTEX_INITIALIZER;

/* Service handler instance */
static onvif_service_handler_instance_t g_snapshot_handler;
static int g_handler_initialized = 0;

/* Forward declarations for helper functions */
static int handle_snapshot_validation_error(const error_context_t *context, const error_result_t *result, onvif_response_t *response);
static int handle_snapshot_system_error(const error_context_t *context, const error_result_t *result, onvif_response_t *response);

/* Default snapshot resolution */
#define DEFAULT_SNAPSHOT_WIDTH  640
#define DEFAULT_SNAPSHOT_HEIGHT 480

int onvif_snapshot_init(void) {
    if (g_snapshot_initialized) {
        return ONVIF_SUCCESS;
    }
    
    platform_log_info("Initializing ONVIF Snapshot service\n");
    
    // Open video input for snapshots
    platform_result_t result = platform_vi_open(&g_vi_handle);
    if (result != PLATFORM_SUCCESS) {
        platform_log_error("Failed to open video input for snapshots\n");
        return ONVIF_ERROR;
    }
    
    // Initialize snapshot capture
    result = platform_snapshot_init(&g_snapshot_handle, g_vi_handle, 
                                   DEFAULT_SNAPSHOT_WIDTH, DEFAULT_SNAPSHOT_HEIGHT);
    if (result != PLATFORM_SUCCESS) {
        platform_log_error("Failed to initialize snapshot capture\n");
        platform_vi_close(g_vi_handle);
        g_vi_handle = NULL;
        return ONVIF_ERROR;
    }
    
    g_snapshot_initialized = true;
    platform_log_info("ONVIF Snapshot service initialized successfully\n");
    return ONVIF_SUCCESS;
}

void onvif_snapshot_cleanup(void) {
    if (!g_snapshot_initialized) {
        return;
    }
    
    platform_log_info("Cleaning up ONVIF Snapshot service\n");
    
    pthread_mutex_lock(&g_snapshot_mutex);
    
    if (g_snapshot_handle) {
        platform_snapshot_cleanup(g_snapshot_handle);
        g_snapshot_handle = NULL;
    }
    
    if (g_vi_handle) {
        platform_vi_close(g_vi_handle);
        g_vi_handle = NULL;
    }
    
    g_snapshot_initialized = false;
    
    pthread_mutex_unlock(&g_snapshot_mutex);
}

int onvif_snapshot_capture(int width, int height, uint8_t **data, size_t *size) {
    if (!g_snapshot_initialized || !data || !size) {
        return ONVIF_ERROR_NULL;
    }
    
    pthread_mutex_lock(&g_snapshot_mutex);
    
    platform_snapshot_t snapshot;
    platform_result_t result = platform_snapshot_capture(g_snapshot_handle, &snapshot, 5000);
    
    if (result != PLATFORM_SUCCESS) {
        platform_log_error("Failed to capture snapshot\n");
        pthread_mutex_unlock(&g_snapshot_mutex);
        return ONVIF_ERROR;
    }
    
    // Allocate memory for the snapshot data using safe memory management
    *data = ONVIF_MALLOC(snapshot.len);
    if (!*data) {
        platform_log_error("Failed to allocate memory for snapshot data\n");
        platform_snapshot_release(g_snapshot_handle, &snapshot);
        pthread_mutex_unlock(&g_snapshot_mutex);
        return ONVIF_ERROR_MEMORY;
    }
    
    // Copy snapshot data safely
    if (!memory_safe_memcpy(*data, snapshot.len, snapshot.data, snapshot.len)) {
        platform_log_error("Failed to copy snapshot data safely\n");
        MEMORY_SAFE_FREE(*data);
        platform_snapshot_release(g_snapshot_handle, &snapshot);
        pthread_mutex_unlock(&g_snapshot_mutex);
        return ONVIF_ERROR;
    }
    *size = snapshot.len;
    
    // Release the platform snapshot
    platform_snapshot_release(g_snapshot_handle, &snapshot);
    
    pthread_mutex_unlock(&g_snapshot_mutex);
    
    platform_log_info("Snapshot captured successfully: %zu bytes\n", *size);
    return ONVIF_SUCCESS;
}

void onvif_snapshot_release(uint8_t *data) {
    MEMORY_SAFE_FREE(data);
}

int onvif_snapshot_get_uri(const char *profile_token, struct stream_uri *uri) {
    if (!profile_token || !uri) {
        return ONVIF_ERROR_NULL;
    }
    
    // Generate snapshot URI
    build_device_url("http", ONVIF_SNAPSHOT_PORT_DEFAULT, SNAPSHOT_PATH, 
                    uri->uri, sizeof(uri->uri));
    uri->invalid_after_connect = 0;
    uri->invalid_after_reboot = 0;
    uri->timeout = 60;
    
    return ONVIF_SUCCESS;
}

/* Service handler action implementations */
static int handle_get_snapshot_uri(const service_handler_config_t *config,
                                  const onvif_request_t *request,
                                  onvif_response_t *response,
                                  onvif_xml_builder_t *xml_builder) {
    // Initialize error context
    error_context_t error_ctx;
    error_context_init(&error_ctx, "Snapshot", "GetSnapshotUri", "uri_retrieval");
    
    // Enhanced parameter validation
    if (!config) {
        return error_handle_parameter(&error_ctx, "config", "missing", response);
    }
    if (!response) {
        return error_handle_parameter(&error_ctx, "response", "missing", response);
    }
    if (!xml_builder) {
        return error_handle_parameter(&error_ctx, "xml_builder", "missing", response);
    }
    
    // Parse profile token from request using common XML parser
    char profile_token[32];
    if (onvif_xml_parse_profile_token(request->body, profile_token, sizeof(profile_token)) != 0) {
        return error_handle_parameter(&error_ctx, "profile_token", "invalid", response);
    }
    
    // Get snapshot URI using existing function
    struct stream_uri uri;
    int result = onvif_snapshot_get_uri(profile_token, &uri);
    
    if (result != ONVIF_SUCCESS) {
        return onvif_generate_fault_response(response, SOAP_FAULT_RECEIVER, "Internal server error");
    }
    
    // Build snapshot URI XML
    onvif_xml_builder_start_element(xml_builder, "timg:MediaUri", NULL);
    onvif_xml_builder_element_with_text(xml_builder, "tt:Uri", uri.uri, NULL);
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:InvalidAfterConnect", "%s", 
                                           uri.invalid_after_connect ? "true" : "false");
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:InvalidAfterReboot", "%s", 
                                           uri.invalid_after_reboot ? "true" : "false");
    onvif_xml_builder_element_with_formatted_text(xml_builder, "tt:Timeout", "PT%dS", uri.timeout);
    onvif_xml_builder_end_element(xml_builder, "timg:MediaUri");
    
    // Generate success response using consistent SOAP response utility
    const char *xml_content = onvif_xml_builder_get_string(xml_builder);
    return onvif_generate_complete_response(response, ONVIF_SERVICE_IMAGING, "GetSnapshotUri", xml_content);
}

/* Action definitions */
static const service_action_def_t snapshot_actions[] = {
    {ONVIF_ACTION_GET_SNAPSHOT_URI, "GetSnapshotUri", handle_get_snapshot_uri, 1}
};

/* Service handler functions */
int onvif_snapshot_service_init(config_manager_t *config) {
    if (g_handler_initialized) {
        return ONVIF_SUCCESS;
    }
    
    service_handler_config_t handler_config = {
        .service_type = ONVIF_SERVICE_IMAGING,  // Snapshot is part of Imaging service
        .service_name = "Snapshot",
        .config = config,
        .enable_validation = 1,
        .enable_logging = 1
    };
    
    int result = onvif_service_handler_init(&g_snapshot_handler, &handler_config,
                                     snapshot_actions, sizeof(snapshot_actions) / sizeof(snapshot_actions[0]));
    
    if (result == ONVIF_SUCCESS) {
        // Register snapshot-specific error handlers
        // Error handler registration not implemented yet
        
        g_handler_initialized = 1;
    }
    
    return result;
}

void onvif_snapshot_service_cleanup(void) {
    if (g_handler_initialized) {
        error_context_t error_ctx;
        error_context_init(&error_ctx, "Snapshot", "Cleanup", "service_cleanup");
        
        onvif_service_handler_cleanup(&g_snapshot_handler);
        
        // Unregister error handlers
        // Error handler unregistration not implemented yet
        
        g_handler_initialized = 0;
        
        // Check for memory leaks
        memory_manager_check_leaks();
    }
}

int onvif_snapshot_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response) {
    if (!g_handler_initialized) {
        return ONVIF_ERROR;
    }
    return onvif_service_handler_handle_request(&g_snapshot_handler, action, request, response);
}

/* Error handler implementations */
static int handle_snapshot_validation_error(const error_context_t *context, const error_result_t *result, onvif_response_t *response) {
    if (!context || !result || !response) {
        return ONVIF_ERROR_INVALID;
    }
    
    platform_log_error("Snapshot validation failed: %s", result->error_message);
    return onvif_generate_fault_response(response, result->soap_fault_code, result->soap_fault_string);
}

static int handle_snapshot_system_error(const error_context_t *context, const error_result_t *result, onvif_response_t *response) {
    if (!context || !result || !response) {
        return ONVIF_ERROR_INVALID;
    }
    
    platform_log_error("Snapshot system error: %s", result->error_message);
    return onvif_generate_fault_response(response, result->soap_fault_code, result->soap_fault_string);
}
