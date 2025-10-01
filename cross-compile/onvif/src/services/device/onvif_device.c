/**
 * @file onvif_device.c
 * @brief ONVIF Device service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "onvif_device.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "core/config/config.h"
#include "networking/common/buffer_pool.h"
#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_device.h"
#include "protocol/gsoap/onvif_gsoap_response.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_service_common.h"
#include "services/common/onvif_types.h"
#include "services/common/service_dispatcher.h"
#include "utils/error/error_handling.h"
#include "utils/logging/service_logging.h"
#include "utils/memory/memory_manager.h"
#include "utils/memory/smart_response_builder.h"

/* ============================================================================
 * Device Service Specific Utility Functions
 * ============================================================================
 */

#define DEVICE_MANUFACTURER_DEFAULT "Anyka"
#define DEVICE_MODEL_DEFAULT        "AK3918 Camera"
#define DEVICE_FIRMWARE_VER_DEFAULT "1.0.0"
#define DEVICE_SERIAL_DEFAULT       "AK3918-001"
#define DEVICE_HARDWARE_ID_DEFAULT  "1.0"

#define DEFAULT_HTTP_PORT      8080
#define DEFAULT_MTU            1500
#define MAX_NETWORK_INTERFACES 8
#define MAX_NETWORK_PROTOCOLS  8
#define MAX_DEVICE_SERVICES    8

#define DEVICE_MANUFACTURER_LEN    64
#define DEVICE_MODEL_LEN           64
#define DEVICE_FIRMWARE_VER_LEN    32
#define DEVICE_SERIAL_LEN          64
#define DEVICE_HARDWARE_ID_LEN     32
#define NETWORK_INTERFACE_NAME_LEN 32
#define NETWORK_HW_ADDRESS_LEN     18
#define PROTOCOL_NAME_LEN          16
#define SERVICE_NAMESPACE_LEN      128
#define SERVICE_XADDR_LEN          256

// Global device capabilities - must be non-const as capabilities may change at
// runtime
static struct device_capabilities dev_caps = {.has_analytics = 0, // NOLINT
                                              .has_device = 1,
                                              .has_events = 0,
                                              .has_imaging = 1,
                                              .has_media = 1,
                                              .has_ptz = 1};

// Device service configuration and state
static struct {
  service_handler_config_t config;
  onvif_gsoap_context_t* gsoap_ctx;
} g_device_handler = // NOLINT(cppcoreguidelines-avoid-non-const-global-variables)
  {0};
static int g_handler_initialized = 0; // NOLINT

// Global buffer pool for medium-sized responses (4-32KB)
static buffer_pool_t g_device_response_buffer_pool; // NOLINT

/* ============================================================================
 * Static Helper/Utility Functions
 * ============================================================================
 */

/**
 * @brief System reboot utility function
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR on failure
 * @note This function executes the system reboot command
 */
int onvif_device_system_reboot(void) {
  service_log_context_t log_ctx;
  service_log_init_context(&log_ctx, "Device", "SystemReboot", SERVICE_LOG_INFO);
  service_log_info(&log_ctx, "System reboot requested via ONVIF");

  platform_log_notice("ONVIF SystemReboot requested - initiating system reboot\n");

  int result = -1;

  // Note: system() call is acceptable here for reboot functionality
  // as it's a controlled command with no user input
  result = system("reboot"); // NOLINT
  if (result == 0) {
    platform_log_info("System reboot initiated via 'reboot' command\n");
    return ONVIF_SUCCESS;
  }

  // If all methods failed, log error and return failure
  platform_log_error("All reboot methods failed - system may not support reboot\n");
  service_log_operation_failure(&log_ctx, "system_reboot", -1, "All reboot methods failed");
  return ONVIF_ERROR;
}

/* ============================================================================
 * Business Logic Callback Functions
 * ============================================================================
 */

/**
 * @brief Business logic callback for device information retrieval
 * @param config Service configuration
 * @param request ONVIF request
 * @param response ONVIF response
 * @param gsoap_ctx gSOAP context
 * @param log_ctx Service logging context
 * @param error_ctx Error context
 * @param callback_data Device info callback data structure
 * @return ONVIF_SUCCESS if business logic succeeds, error code if it fails
 */
static int get_device_information_business_logic(const service_handler_config_t* config,
                                                 const http_request_t* request,
                                                 http_response_t* response,
                                                 onvif_gsoap_context_t* gsoap_ctx,
                                                 service_log_context_t* log_ctx,
                                                 error_context_t* error_ctx, void* callback_data) {
  (void)config;
  (void)request;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  device_info_callback_data_t* device_info_data = (device_info_callback_data_t*)callback_data;

  // Retrieve configuration values with fallback directly into callback data
  int result = onvif_util_get_config_string_with_fallback(
    (onvif_service_handler_instance_t*)&g_device_handler, CONFIG_SECTION_DEVICE, "manufacturer",
    device_info_data->manufacturer, sizeof(device_info_data->manufacturer),
    DEVICE_MANUFACTURER_DEFAULT, log_ctx, error_ctx, response, "manufacturer");
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  result = onvif_util_get_config_string_with_fallback(
    (onvif_service_handler_instance_t*)&g_device_handler, CONFIG_SECTION_DEVICE, "model",
    device_info_data->model, sizeof(device_info_data->model), DEVICE_MODEL_DEFAULT, log_ctx,
    error_ctx, response, "model");
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  result = onvif_util_get_config_string_with_fallback(
    (onvif_service_handler_instance_t*)&g_device_handler, CONFIG_SECTION_DEVICE, "firmware_version",
    device_info_data->firmware_version, sizeof(device_info_data->firmware_version),
    DEVICE_FIRMWARE_VER_DEFAULT, log_ctx, error_ctx, response, "firmware_version");
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  result = onvif_util_get_config_string_with_fallback(
    (onvif_service_handler_instance_t*)&g_device_handler, CONFIG_SECTION_DEVICE, "serial_number",
    device_info_data->serial_number, sizeof(device_info_data->serial_number), DEVICE_SERIAL_DEFAULT,
    log_ctx, error_ctx, response, "serial_number");
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  result = onvif_util_get_config_string_with_fallback(
    (onvif_service_handler_instance_t*)&g_device_handler, CONFIG_SECTION_DEVICE, "hardware_id",
    device_info_data->hardware_id, sizeof(device_info_data->hardware_id),
    DEVICE_HARDWARE_ID_DEFAULT, log_ctx, error_ctx, response, "hardware_id");
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Log device information being returned
  service_log_info(log_ctx, "Manufacturer: %s, Model: %s, Firmware: %s, Serial: %s, Hardware: %s",
                   device_info_data->manufacturer, device_info_data->model,
                   device_info_data->firmware_version, device_info_data->serial_number,
                   device_info_data->hardware_id);

  // Generate response using gSOAP callback
  result = onvif_gsoap_generate_response_with_callback(gsoap_ctx, device_info_response_callback,
                                                       (void*)device_info_data);
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_response_generation", result,
                                  "Failed to generate gSOAP response");
    return ONVIF_ERROR;
  }

  // Get the generated SOAP response from gSOAP
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    service_log_operation_failure(log_ctx, "soap_response_retrieval", -1,
                                  "Failed to retrieve SOAP response data");
    return ONVIF_ERROR;
  }

  // Use smart response builder for final output with memory optimization
  size_t estimated_size = smart_response_estimate_size(soap_response);
  result =
    smart_response_build(response, soap_response, estimated_size, &g_device_response_buffer_pool);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "smart_response_build", result,
                                  "Failed to build smart response");
    return result;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for GetCapabilities operation
 * @param config Service handler configuration
 * @param request ONVIF request
 * @param response ONVIF response
 * @param gsoap_ctx gSOAP context
 * @param log_ctx Service logging context
 * @param error_ctx Error context
 * @param callback_data Capabilities callback data structure
 * @return ONVIF_SUCCESS if business logic succeeds, error code if it fails
 */
static int get_capabilities_business_logic(const service_handler_config_t* config,
                                           const http_request_t* request, http_response_t* response,
                                           onvif_gsoap_context_t* gsoap_ctx,
                                           service_log_context_t* log_ctx,
                                           error_context_t* error_ctx, void* callback_data) {
  (void)config;
  (void)request;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  capabilities_callback_data_t* capabilities_data = (capabilities_callback_data_t*)callback_data;

  // Get HTTP port from configuration with fallback
  int http_port = DEFAULT_HTTP_PORT;
  int result = onvif_util_get_config_int_with_fallback(
    (onvif_service_handler_instance_t*)&g_device_handler, CONFIG_SECTION_ONVIF, "http_port",
    &http_port, DEFAULT_HTTP_PORT, log_ctx, error_ctx, response, "http_port");
  if (result != ONVIF_SUCCESS) {
    return result;
  }
  service_log_info(log_ctx, "HTTP port configured as %d", http_port);

  // Set capabilities data
  capabilities_data->capabilities = &dev_caps;

  // Generate response using gSOAP callback
  result = onvif_gsoap_generate_response_with_callback(gsoap_ctx, capabilities_response_callback,
                                                       (void*)capabilities_data);
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_response_generation", result,
                                  "Failed to generate gSOAP response");
    return ONVIF_ERROR;
  }

  // Get the generated SOAP response from gSOAP
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    service_log_operation_failure(log_ctx, "soap_response_retrieval", -1,
                                  "Failed to retrieve SOAP response data");
    return ONVIF_ERROR;
  }

  // Use smart response builder for final output with memory optimization
  size_t estimated_size = smart_response_estimate_size(soap_response);
  result =
    smart_response_build(response, soap_response, estimated_size, &g_device_response_buffer_pool);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "smart_response_build", result,
                                  "Failed to build smart response");
    return result;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for GetSystemDateAndTime operation
 * @param config Service handler configuration
 * @param request ONVIF request
 * @param response ONVIF response
 * @param gsoap_ctx gSOAP context
 * @param log_ctx Service logging context
 * @param error_ctx Error context
 * @param callback_data System datetime callback data structure
 * @return ONVIF_SUCCESS if business logic succeeds, error code if it fails
 */
static int get_system_date_time_business_logic(const service_handler_config_t* config,
                                               const http_request_t* request,
                                               http_response_t* response,
                                               onvif_gsoap_context_t* gsoap_ctx,
                                               service_log_context_t* log_ctx,
                                               error_context_t* error_ctx, void* callback_data) {
  (void)config;
  (void)request;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  system_datetime_callback_data_t* datetime_data = (system_datetime_callback_data_t*)callback_data;

  // Get current system time
  time_t now = time(NULL);
  if (now == (time_t)-1) {
    service_log_operation_failure(log_ctx, "System time retrieval", -1, "time() function failed");
    return error_handle_system(error_ctx, ONVIF_ERROR_INVALID, "get_system_time", response);
  }

  struct tm* tm_info = localtime(&now);
  if (!tm_info) {
    service_log_operation_failure(log_ctx, "Time conversion", -1, "localtime() function failed");
    return error_handle_system(error_ctx, ONVIF_ERROR_INVALID, "convert_time", response);
  }

  // Set datetime data
  datetime_data->tm_info = *tm_info;

  // Generate response using gSOAP callback
  int result = onvif_gsoap_generate_response_with_callback(
    gsoap_ctx, system_datetime_response_callback, (void*)datetime_data);
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_response_generation", result,
                                  "Failed to generate gSOAP response");
    return ONVIF_ERROR;
  }

  // Get the generated SOAP response from gSOAP
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    service_log_operation_failure(log_ctx, "soap_response_retrieval", -1,
                                  "Failed to retrieve SOAP response data");
    return ONVIF_ERROR;
  }

  // Use smart response builder for final output with memory optimization
  size_t estimated_size = smart_response_estimate_size(soap_response);
  result =
    smart_response_build(response, soap_response, estimated_size, &g_device_response_buffer_pool);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "smart_response_build", result,
                                  "Failed to build smart response");
    return result;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for GetServices operation
 * @param config Service handler configuration
 * @param request ONVIF request
 * @param response ONVIF response
 * @param gsoap_ctx gSOAP context
 * @param log_ctx Service logging context
 * @param error_ctx Error context
 * @param callback_data Services callback data structure
 * @return ONVIF_SUCCESS if business logic succeeds, error code if it fails
 */
static int get_services_business_logic(const service_handler_config_t* config,
                                       const http_request_t* request, http_response_t* response,
                                       onvif_gsoap_context_t* gsoap_ctx,
                                       service_log_context_t* log_ctx, error_context_t* error_ctx,
                                       void* callback_data) {
  (void)config;
  (void)request;
  (void)error_ctx;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  services_callback_data_t* services_data = (services_callback_data_t*)callback_data;

  // Set services data - include capability information
  services_data->include_capability = 1;

  // Generate response using gSOAP callback
  int result = onvif_gsoap_generate_response_with_callback(gsoap_ctx, services_response_callback,
                                                           (void*)services_data);
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_response_generation", result,
                                  "Failed to generate gSOAP response");
    return ONVIF_ERROR;
  }

  // Get the generated SOAP response from gSOAP
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    service_log_operation_failure(log_ctx, "soap_response_retrieval", -1,
                                  "Failed to retrieve SOAP response data");
    return ONVIF_ERROR;
  }

  // Use smart response builder for final output with memory optimization
  size_t estimated_size = smart_response_estimate_size(soap_response);
  result =
    smart_response_build(response, soap_response, estimated_size, &g_device_response_buffer_pool);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "smart_response_build", result,
                                  "Failed to build smart response");
    return result;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Business logic callback for SystemReboot operation
 * @param config Service handler configuration
 * @param request ONVIF request
 * @param response ONVIF response
 * @param gsoap_ctx gSOAP context
 * @param log_ctx Service logging context
 * @param error_ctx Error context
 * @param callback_data System reboot callback data structure
 * @return ONVIF_SUCCESS if business logic succeeds, error code if it fails
 */
static int system_reboot_business_logic(const service_handler_config_t* config,
                                        const http_request_t* request, http_response_t* response,
                                        onvif_gsoap_context_t* gsoap_ctx,
                                        service_log_context_t* log_ctx, error_context_t* error_ctx,
                                        void* callback_data) {
  (void)config;
  (void)request;
  if (!callback_data) {
    return ONVIF_ERROR_INVALID;
  }

  system_reboot_callback_data_t* reboot_data = (system_reboot_callback_data_t*)callback_data;

  // Call the system reboot function
  int result = onvif_device_system_reboot();
  if (result != ONVIF_SUCCESS) {
    return error_handle_system(error_ctx, result, "system_reboot_execution", response);
  }

  // Set reboot message
  reboot_data->message = "System reboot initiated";

  // Generate response using gSOAP callback
  result = onvif_gsoap_generate_response_with_callback(gsoap_ctx, system_reboot_response_callback,
                                                       (void*)reboot_data);
  if (result != 0) {
    service_log_operation_failure(log_ctx, "gsoap_response_generation", result,
                                  "Failed to generate gSOAP response");
    return ONVIF_ERROR;
  }

  // Get the generated SOAP response from gSOAP
  const char* soap_response = onvif_gsoap_get_response_data(gsoap_ctx);
  if (!soap_response) {
    service_log_operation_failure(log_ctx, "soap_response_retrieval", -1,
                                  "Failed to retrieve SOAP response data");
    return ONVIF_ERROR;
  }

  // Use smart response builder for final output with memory optimization
  size_t estimated_size = smart_response_estimate_size(soap_response);
  result =
    smart_response_build(response, soap_response, estimated_size, &g_device_response_buffer_pool);
  if (result != ONVIF_SUCCESS) {
    service_log_operation_failure(log_ctx, "smart_response_build", result,
                                  "Failed to build smart response");
    return result;
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Service Operation Definitions
 * ============================================================================
 */

/**
 * @brief Device information service operation definition
 */
static const onvif_service_operation_t get_device_information_operation = {
  .service_name = "Device",
  .operation_name = "GetDeviceInformation",
  .operation_context = "device_info_retrieval",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = get_device_information_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

static const onvif_service_operation_t get_capabilities_operation = {
  .service_name = "Device",
  .operation_name = "GetCapabilities",
  .operation_context = "capabilities_retrieval",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = get_capabilities_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

static const onvif_service_operation_t get_system_date_time_operation = {
  .service_name = "Device",
  .operation_name = "GetSystemDateAndTime",
  .operation_context = "system_time_retrieval",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = get_system_date_time_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

static const onvif_service_operation_t get_services_operation = {
  .service_name = "Device",
  .operation_name = "GetServices",
  .operation_context = "service_list_retrieval",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = get_services_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

static const onvif_service_operation_t system_reboot_operation = {
  .service_name = "Device",
  .operation_name = "SystemReboot",
  .operation_context = "system_reboot",
  .callbacks = {.validate_parameters = onvif_util_validate_standard_parameters,
                .execute_business_logic = system_reboot_business_logic,
                .post_process_response = onvif_util_standard_post_process}};

/* ============================================================================
 * Handler Functions
 * ============================================================================
 */

static int handle_get_device_information(const service_handler_config_t* config,
                                         const http_request_t* request, http_response_t* response,
                                         onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for device information
  device_info_callback_data_t callback_data = {.manufacturer = {0},
                                               .model = {0},
                                               .firmware_version = {0},
                                               .serial_number = {0},
                                               .hardware_id = {0}};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &get_device_information_operation,
                                           device_info_response_callback, &callback_data);
}

static int handle_get_capabilities(const service_handler_config_t* config,
                                   const http_request_t* request, http_response_t* response,
                                   onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for capabilities
  capabilities_callback_data_t callback_data = {0};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &get_capabilities_operation,
                                           capabilities_response_callback, &callback_data);
}

static int handle_get_system_date_time(const service_handler_config_t* config,
                                       const http_request_t* request, http_response_t* response,
                                       onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for system datetime
  system_datetime_callback_data_t callback_data = {0};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &get_system_date_time_operation,
                                           system_datetime_response_callback, &callback_data);
}

static int handle_get_services(const service_handler_config_t* config,
                               const http_request_t* request, http_response_t* response,
                               onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for services
  services_callback_data_t callback_data = {0};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &get_services_operation, services_response_callback,
                                           &callback_data);
}

static int handle_system_reboot(const service_handler_config_t* config,
                                const http_request_t* request, http_response_t* response,
                                onvif_gsoap_context_t* gsoap_ctx) {
  // Prepare callback data for system reboot
  system_reboot_callback_data_t callback_data = {0};

  // Use the enhanced callback-based handler
  return onvif_util_handle_service_request(config, request, response, gsoap_ctx,
                                           &system_reboot_operation,
                                           system_reboot_response_callback, &callback_data);
}

/* ============================================================================
 * Service Registration using Standardized Callback Interface
 * ============================================================================
 */

/**
 * @brief Device service initialization handler
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int device_service_init_handler(void) {
  // Buffer pool and gSOAP context are already initialized in onvif_device_init
  return ONVIF_SUCCESS;
}

/**
 * @brief Device service cleanup handler
 */
static void device_service_cleanup_handler(void) {
  // Cleanup is handled in onvif_device_cleanup
}

/**
 * @brief Device service capabilities check handler
 * @param capability_name Capability name to check
 * @return 1 if capability is supported, 0 otherwise
 */
static int device_service_capabilities_handler(const char* capability_name) {
  if (!capability_name) {
    return 0;
  }

  // Check against known device service capabilities
  if (strcmp(capability_name, "GetDeviceInformation") == 0 ||
      strcmp(capability_name, "GetCapabilities") == 0 ||
      strcmp(capability_name, "GetSystemDateAndTime") == 0 ||
      strcmp(capability_name, "GetServices") == 0 || strcmp(capability_name, "SystemReboot") == 0) {
    return 1;
  }

  return 0;
}

/**
 * @brief Device service registration structure using standardized interface
 */
static const onvif_service_registration_t g_device_service_registration = {
  .service_name = "device",
  .namespace_uri = "http://www.onvif.org/ver10/device/wsdl",
  .operation_handler = onvif_device_handle_operation,
  .init_handler = device_service_init_handler,
  .cleanup_handler = device_service_cleanup_handler,
  .capabilities_handler = device_service_capabilities_handler,
  .reserved = {NULL, NULL, NULL, NULL}};

/* ============================================================================
 * Public API Functions
 * ============================================================================
 */

/**
 * @brief Initialize the ONVIF Device service
 * @param config Configuration manager instance
 * @return ONVIF_SUCCESS on success, error code on failure
 * @note This function initializes the device service handler and registers
 * with the standardized dispatcher
 */
int onvif_device_init(config_manager_t* config) {
  if (g_handler_initialized) {
    return ONVIF_SUCCESS;
  }

  // Initialize device handler configuration
  g_device_handler.config.service_type = ONVIF_SERVICE_DEVICE;
  g_device_handler.config.service_name = "Device";
  g_device_handler.config.config = config;
  g_device_handler.config.enable_validation = 1;
  g_device_handler.config.enable_logging = 1;

  // Initialize gSOAP context for device service
  g_device_handler.gsoap_ctx = ONVIF_MALLOC(sizeof(onvif_gsoap_context_t));
  if (!g_device_handler.gsoap_ctx) {
    return ONVIF_ERROR;
  }

  if (onvif_gsoap_init(g_device_handler.gsoap_ctx) != ONVIF_SUCCESS) {
    ONVIF_FREE(g_device_handler.gsoap_ctx);
    g_device_handler.gsoap_ctx = NULL;
    return ONVIF_ERROR;
  }

  // Initialize buffer pool for medium-sized responses
  if (buffer_pool_init(&g_device_response_buffer_pool) != 0) {
    onvif_gsoap_cleanup(g_device_handler.gsoap_ctx);
    ONVIF_FREE(g_device_handler.gsoap_ctx);
    g_device_handler.gsoap_ctx = NULL;
    return ONVIF_ERROR;
  }

  g_handler_initialized = 1;

  // Register with standardized service dispatcher
  int result = onvif_service_dispatcher_register_service(&g_device_service_registration);
  if (result != ONVIF_SUCCESS) {
    platform_log_error("Failed to register device service with dispatcher: %d\n", result);
    onvif_device_cleanup();
    return result;
  }

  // Log initial buffer pool statistics
  buffer_pool_stats_t stats = buffer_pool_get_stats(&g_device_response_buffer_pool);
  platform_log_info(
    "Device service initialized and registered - Buffer pool stats: %d/%d buffers used "
    "(%d%%), hits: %d, misses: %d\n",
    stats.current_used, BUFFER_POOL_SIZE, stats.utilization_percent, stats.hits, stats.misses);

  return ONVIF_SUCCESS;
}

/**
 * @brief Cleanup the ONVIF Device service
 * @return ONVIF_SUCCESS on success, error code on failure
 * @note This function cleans up all device service resources and unregisters from dispatcher
 */
int onvif_device_cleanup(void) {
  if (!g_handler_initialized) {
    return ONVIF_SUCCESS;
  }

  int result = ONVIF_SUCCESS;

  // Unregister from standardized service dispatcher
  int unregister_result = onvif_service_dispatcher_unregister_service("device");
  if (unregister_result != ONVIF_SUCCESS) {
    platform_log_error("Failed to unregister device service from dispatcher: %d\n",
                       unregister_result);
    // Don't fail cleanup for this, but log the error
  }

  // Cleanup buffer pool (returns void, so we can't check for errors)
  buffer_pool_cleanup(&g_device_response_buffer_pool);

  // Cleanup gSOAP context
  if (g_device_handler.gsoap_ctx) {
    onvif_gsoap_cleanup(g_device_handler.gsoap_ctx);
    ONVIF_FREE(g_device_handler.gsoap_ctx);
    g_device_handler.gsoap_ctx = NULL;
  }

  g_handler_initialized = 0;
  return result;
}

/* ============================================================================
 * Operation Dispatch Table
 * ============================================================================ */

/**
 * @brief Device operation handler function pointer type
 */
typedef int (*device_operation_handler_t)(const service_handler_config_t* config,
                                          const http_request_t* request, http_response_t* response,
                                          onvif_gsoap_context_t* gsoap_ctx);

/**
 * @brief Device operation dispatch entry
 */
typedef struct {
  const char* operation_name;
  device_operation_handler_t handler;
} device_operation_entry_t;

/**
 * @brief Device service operation dispatch table
 */
static const device_operation_entry_t g_device_operations[] = {
  {"GetDeviceInformation", handle_get_device_information},
  {"GetCapabilities", handle_get_capabilities},
  {"GetSystemDateAndTime", handle_get_system_date_time},
  {"GetServices", handle_get_services},
  {"SystemReboot", handle_system_reboot}};

#define DEVICE_OPERATIONS_COUNT (sizeof(g_device_operations) / sizeof(g_device_operations[0]))

/**
 * @brief Handle ONVIF Device service requests by operation name (Standardized Interface)
 * @param operation_name ONVIF operation name (e.g., "GetDeviceInformation")
 * @param request HTTP request structure
 * @param response HTTP response structure
 * @return ONVIF_SUCCESS on success, error code on failure
 * @note This function implements the standardized onvif_service_operation_handler_t interface
 */
int onvif_device_handle_operation(const char* operation_name, const http_request_t* request,
                                  http_response_t* response) {
  if (!g_handler_initialized || !operation_name || !request || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Dispatch using lookup table for O(n) performance with small constant factor
  for (size_t i = 0; i < DEVICE_OPERATIONS_COUNT; i++) {
    if (strcmp(operation_name, g_device_operations[i].operation_name) == 0) {
      return g_device_operations[i].handler(&g_device_handler.config, request, response,
                                            g_device_handler.gsoap_ctx);
    }
  }

  // Operation not found
  return ONVIF_ERROR_NOT_FOUND;
}
