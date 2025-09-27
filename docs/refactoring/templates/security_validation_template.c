/**
 * @file security_validation_template.c
 * @brief Template for secure request validation and response building
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Project-specific headers (relative from src/)
#include "platform/platform.h"
#include "services/common/onvif_types.h"
#include "utils/validation/input_validation.h"
#include "utils/xml/xml_utils.h"

// Global variables (MANDATORY: g_<module>_<variable_name> naming)
static int g_security_validation_enabled = 1;  // NOLINT
static int g_rate_limit_max_requests = 100;    // NOLINT

/**
 * @brief Validate request authentication and authorization
 * @param request HTTP request to validate (must not be NULL)
 * @return 0 on success, -1 on authentication failure
 * @note Implements ONVIF security requirements per specification
 */
static int security_validate_request(const http_request_t *request) {
  if (!request) {
    return -1;  // Input validation
  }

  // Check for valid authentication header
  if (!request->auth_header || strlen(request->auth_header) == 0) {
    platform_log_warn("Request missing authentication header");
    return -1;
  }

  // Validate authentication credentials
  if (validate_credentials(request->auth_header) != 0) {
    platform_log_warn("Invalid authentication credentials");
    return -1;
  }

  // Check request rate limiting
  if (check_rate_limit(request->client_ip) != 0) {
    platform_log_warn("Rate limit exceeded for %s", request->client_ip);
    return -1;
  }

  return 0;
}

/**
 * @brief Validate service request parameters
 * @param request HTTP request to validate (must not be NULL)
 * @param service_type Type of service being accessed
 * @return 0 on success, -1 on validation failure
 * @note Implements comprehensive input validation per security guidelines
 */
static int validate_service_request(const http_request_t *request,
                                    onvif_service_type_t service_type) {
  if (!request) {
    return -1;  // Input validation
  }

  // Validate HTTP method
  if (strcmp(request->method, "POST") != 0) {
    platform_log_warn("Invalid HTTP method: %s", request->method);
    return -1;
  }

  // Validate content type
  if (!request->content_type ||
      strstr(request->content_type, "text/xml") == NULL) {
    platform_log_warn("Invalid content type: %s", request->content_type);
    return -1;
  }

  // Validate request body
  if (!request->body || strlen(request->body) == 0) {
    platform_log_warn("Empty request body");
    return -1;
  }

  // Validate XML structure
  if (xml_util_validate_xml(request->body) != 0) {
    platform_log_warn("Invalid XML in request body");
    return -1;
  }

  // Service-specific validation
  switch (service_type) {
    case ONVIF_SERVICE_DEVICE:
      return validate_device_request(request);
    case ONVIF_SERVICE_MEDIA:
      return validate_media_request(request);
    case ONVIF_SERVICE_PTZ:
      return validate_ptz_request(request);
    case ONVIF_SERVICE_IMAGING:
      return validate_imaging_request(request);
    default:
      platform_log_error("Unknown service type: %d", service_type);
      return -1;
  }
}

/**
 * @brief Safely copy string with bounds checking
 * @param dest Destination buffer (must not be NULL)
 * @param src Source string (must not be NULL)
 * @param dest_size Size of destination buffer
 * @return 0 on success, -1 on error
 * @note Ensures null termination and prevents buffer overflow
 */
static int safe_string_copy(char *dest, const char *src, size_t dest_size) {
  if (!dest || !src || dest_size == 0) {
    return -1;  // Input validation
  }

  size_t src_len = strlen(src);
  if (src_len >= dest_size) {
    platform_log_error("String too long for destination buffer");
    return -1;  // Buffer too small
  }

  strncpy(dest, src, dest_size - 1);
  dest[dest_size - 1] = '\0';  // Ensure null termination

  return 0;
}

/**
 * @brief Build SOAP response with proper XML escaping
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content SOAP content to include (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Uses existing XML utilities to prevent injection attacks
 */
static int build_secure_soap_response(onvif_response_t *response,
                                      const char *soap_content) {
  if (!response || !soap_content) {
    return -1;  // Input validation
  }

  // Use existing XML utilities for safe building
  char *escaped_content = NULL;
  size_t escaped_size = 0;

  if (xml_util_escape_string(soap_content, NULL, 0) < 0) {
    // Calculate required size
    escaped_size = xml_util_escape_string(soap_content, NULL, 0);
    if (escaped_size <= 0) {
      return -1;
    }
  }

  escaped_content = ONVIF_MALLOC(escaped_size + 1);
  if (!escaped_content) {
    return -1;
  }

  if (xml_util_escape_string(soap_content, escaped_content, escaped_size + 1) !=
      0) {
    ONVIF_FREE(escaped_content);
    return -1;
  }

  // Build SOAP envelope safely
  if (xml_util_build_soap_envelope(escaped_content, response->body,
                                   response->body_capacity) != 0) {
    ONVIF_FREE(escaped_content);
    return -1;
  }

  ONVIF_FREE(escaped_content);
  return 0;
}

/**
 * @brief Validate device service request parameters
 * @param request HTTP request to validate (must not be NULL)
 * @return 0 on success, -1 on validation failure
 * @note Device-specific validation logic
 */
static int validate_device_request(const http_request_t *request) {
  if (!request) {
    return -1;
  }

  // Check for required device service parameters
  if (strstr(request->body, "GetDeviceInformation") == NULL &&
      strstr(request->body, "GetCapabilities") == NULL &&
      strstr(request->body, "GetSystemDateAndTime") == NULL) {
    platform_log_warn("Invalid device service operation");
    return -1;
  }

  return 0;
}

/**
 * @brief Validate media service request parameters
 * @param request HTTP request to validate (must not be NULL)
 * @return 0 on success, -1 on validation failure
 * @note Media-specific validation logic
 */
static int validate_media_request(const http_request_t *request) {
  if (!request) {
    return -1;
  }

  // Check for required media service parameters
  if (strstr(request->body, "GetProfiles") == NULL &&
      strstr(request->body, "GetStreamUri") == NULL &&
      strstr(request->body, "GetVideoEncoderConfiguration") == NULL) {
    platform_log_warn("Invalid media service operation");
    return -1;
  }

  return 0;
}

/**
 * @brief Validate PTZ service request parameters
 * @param request HTTP request to validate (must not be NULL)
 * @return 0 on success, -1 on validation failure
 * @note PTZ-specific validation logic
 */
static int validate_ptz_request(const http_request_t *request) {
  if (!request) {
    return -1;
  }

  // Check for required PTZ service parameters
  if (strstr(request->body, "GetPresets") == NULL &&
      strstr(request->body, "GotoPreset") == NULL &&
      strstr(request->body, "ContinuousMove") == NULL) {
    platform_log_warn("Invalid PTZ service operation");
    return -1;
  }

  return 0;
}

/**
 * @brief Validate imaging service request parameters
 * @param request HTTP request to validate (must not be NULL)
 * @return 0 on success, -1 on validation failure
 * @note Imaging-specific validation logic
 */
static int validate_imaging_request(const http_request_t *request) {
  if (!request) {
    return -1;
  }

  // Check for required imaging service parameters
  if (strstr(request->body, "GetImagingSettings") == NULL &&
      strstr(request->body, "SetImagingSettings") == NULL) {
    platform_log_warn("Invalid imaging service operation");
    return -1;
  }

  return 0;
}
