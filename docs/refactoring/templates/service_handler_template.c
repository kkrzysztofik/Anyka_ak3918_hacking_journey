/**
 * @file service_handler_template.c
 * @brief Template for ONVIF service handler implementation
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
#include "utils/validation/input_validation.h"

// Global variables (MANDATORY: g_<module>_<variable_name> naming)
static int g_service_handler_initialized = 0;        // NOLINT
static char g_service_name[64] = "TemplateService";  // NOLINT

/**
 * @brief Handle ONVIF service request with proper validation and response
 * building
 * @param config Service handler configuration (must not be NULL)
 * @param request HTTP request structure (must not be NULL)
 * @param response Response structure to populate (must not be NULL)
 * @param gsoap_ctx gSOAP context for XML processing (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Uses direct HTTP request access per refactoring guide
 *       Follows security and validation standards from AGENTS.md
 */
static int handle_service_request(const service_handler_config_t *config,
                                  const http_request_t *request,
                                  onvif_response_t *response,
                                  onvif_gsoap_context_t *gsoap_ctx) {
  // Input validation per AGENTS.md requirements
  if (!config || !request || !response || !gsoap_ctx) {
    platform_log_error("Invalid parameters to service handler");
    return -1;
  }

  // Security validation
  if (security_validate_request(request) != 0) {
    platform_log_warn("Security validation failed for service request");
    return -1;
  }

  // Service-specific validation
  if (validate_service_request(request, config->service_type) != 0) {
    platform_log_warn("Service validation failed for %s", g_service_name);
    return -1;
  }

  // Process request based on operation type
  const char *operation = extract_operation_from_request(request);
  if (!operation) {
    platform_log_error("Failed to extract operation from request");
    return -1;
  }

  // Route to appropriate handler
  if (strcmp(operation, "GetInformation") == 0) {
    return handle_get_information(request, response, gsoap_ctx);
  } else if (strcmp(operation, "GetCapabilities") == 0) {
    return handle_get_capabilities(request, response, gsoap_ctx);
  } else if (strcmp(operation, "GetStatus") == 0) {
    return handle_get_status(request, response, gsoap_ctx);
  } else {
    platform_log_warn("Unknown operation: %s", operation);
    return -1;
  }
}

/**
 * @brief Handle GetInformation operation
 * @param request HTTP request structure (must not be NULL)
 * @param response Response structure to populate (must not be NULL)
 * @param gsoap_ctx gSOAP context for XML processing (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Implements ONVIF GetInformation operation with memory optimization
 */
static int handle_get_information(const http_request_t *request,
                                  onvif_response_t *response,
                                  onvif_gsoap_context_t *gsoap_ctx) {
  if (!request || !response || !gsoap_ctx) {
    return -1;  // Input validation
  }

  // Build response content
  char soap_content[2048];
  int result = snprintf(soap_content, sizeof(soap_content),
                        "<soap:Body>"
                        "<tds:GetDeviceInformationResponse>"
                        "<tds:Manufacturer>Anyka</tds:Manufacturer>"
                        "<tds:Model>AK3918</tds:Model>"
                        "<tds:FirmwareVersion>1.0.0</tds:FirmwareVersion>"
                        "<tds:SerialNumber>123456789</tds:SerialNumber>"
                        "<tds:HardwareId>AK3918-IP-CAM</tds:HardwareId>"
                        "</tds:GetDeviceInformationResponse>"
                        "</soap:Body>");

  if (result < 0 || result >= (int)sizeof(soap_content)) {
    platform_log_error("Failed to build GetInformation response");
    return -1;
  }

  // Use smart response building for memory optimization
  return build_smart_response(response, soap_content);
}

/**
 * @brief Handle GetCapabilities operation
 * @param request HTTP request structure (must not be NULL)
 * @param response Response structure to populate (must not be NULL)
 * @param gsoap_ctx gSOAP context for XML processing (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Implements ONVIF GetCapabilities operation with memory optimization
 */
static int handle_get_capabilities(const http_request_t *request,
                                   onvif_response_t *response,
                                   onvif_gsoap_context_t *gsoap_ctx) {
  if (!request || !response || !gsoap_ctx) {
    return -1;  // Input validation
  }

  // Build response content
  char soap_content[4096];
  int result =
      snprintf(soap_content, sizeof(soap_content),
               "<soap:Body>"
               "<tds:GetCapabilitiesResponse>"
               "<tds:Capabilities>"
               "<tt:Analytics>"
               "<tt:XAddr>http://%s/onvif/analytics_service</tt:XAddr>"
               "<tt:RuleSupport>true</tt:RuleSupport>"
               "<tt:AnalyticsModuleSupport>true</tt:AnalyticsModuleSupport>"
               "</tt:Analytics>"
               "<tt:Device>"
               "<tt:XAddr>http://%s/onvif/device_service</tt:XAddr>"
               "<tt:Network>"
               "<tt:IPFilter>false</tt:IPFilter>"
               "<tt:ZeroConfiguration>true</tt:ZeroConfiguration>"
               "<tt:IPVersion6>false</tt:IPVersion6>"
               "<tt:DynDNS>false</tt:DynDNS>"
               "</tt:Network>"
               "</tt:Device>"
               "</tds:Capabilities>"
               "</tds:GetCapabilitiesResponse>"
               "</soap:Body>",
               request->host, request->host);

  if (result < 0 || result >= (int)sizeof(soap_content)) {
    platform_log_error("Failed to build GetCapabilities response");
    return -1;
  }

  // Use smart response building for memory optimization
  return build_smart_response(response, soap_content);
}

/**
 * @brief Handle GetStatus operation
 * @param request HTTP request structure (must not be NULL)
 * @param response Response structure to populate (must not be NULL)
 * @param gsoap_ctx gSOAP context for XML processing (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Implements ONVIF GetStatus operation with memory optimization
 */
static int handle_get_status(const http_request_t *request,
                             onvif_response_t *response,
                             onvif_gsoap_context_t *gsoap_ctx) {
  if (!request || !response || !gsoap_ctx) {
    return -1;  // Input validation
  }

  // Build response content
  char soap_content[1024];
  int result = snprintf(soap_content, sizeof(soap_content),
                        "<soap:Body>"
                        "<tds:GetSystemDateAndTimeResponse>"
                        "<tds:SystemDateAndTime>"
                        "<tt:DateTimeType>Manual</tt:DateTimeType>"
                        "<tt:DaylightSavings>false</tt:DaylightSavings>"
                        "<tt:TimeZone>"
                        "<tt:TZ>UTC</tt:TZ>"
                        "</tt:TimeZone>"
                        "<tt:UTCDateTime>"
                        "<tt:Time>"
                        "<tt:Hour>12</tt:Hour>"
                        "<tt:Minute>0</tt:Minute>"
                        "<tt:Second>0</tt:Second>"
                        "</tt:Time>"
                        "<tt:Date>"
                        "<tt:Year>2025</tt:Year>"
                        "<tt:Month>1</tt:Month>"
                        "<tt:Day>1</tt:Day>"
                        "</tt:Date>"
                        "</tt:UTCDateTime>"
                        "</tds:SystemDateAndTime>"
                        "</tds:GetSystemDateAndTimeResponse>"
                        "</soap:Body>");

  if (result < 0 || result >= (int)sizeof(soap_content)) {
    platform_log_error("Failed to build GetStatus response");
    return -1;
  }

  // Use smart response building for memory optimization
  return build_smart_response(response, soap_content);
}

/**
 * @brief Extract operation name from SOAP request
 * @param request HTTP request structure (must not be NULL)
 * @return Operation name on success, NULL on error
 * @note Parses SOAP body to extract operation name
 */
static const char *extract_operation_from_request(
    const http_request_t *request) {
  if (!request || !request->body) {
    return NULL;
  }

  // Simple operation extraction - in real implementation, use proper XML
  // parsing
  if (strstr(request->body, "GetDeviceInformation") != NULL) {
    return "GetInformation";
  } else if (strstr(request->body, "GetCapabilities") != NULL) {
    return "GetCapabilities";
  } else if (strstr(request->body, "GetSystemDateAndTime") != NULL) {
    return "GetStatus";
  }

  return NULL;
}

/**
 * @brief Build smart response using optimal memory allocation strategy
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content SOAP content to include (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Automatically chooses between dynamic buffer, buffer pool, or direct
 * allocation based on content size for optimal memory usage
 */
static int build_smart_response(onvif_response_t *response,
                                const char *soap_content) {
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
