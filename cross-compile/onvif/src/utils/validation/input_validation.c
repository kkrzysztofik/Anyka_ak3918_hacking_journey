/**
 * @file input_validation.c
 * @brief Comprehensive input validation utilities for ONVIF services
 * @author kkrzysztofik
 * @date 2025
 */

#include "input_validation.h"

#include "networking/http/http_parser.h"
#include "platform/platform.h"
#include "utils/error/error_handling.h"
#include "utils/security/base64_utils.h"
#include "utils/security/security_hardening.h"
#include "utils/string/string_shims.h"
#include "utils/validation/common_validation.h"

#include <ctype.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>

/* Utility macros */
#define MIN(a, b) ((a) < (b) ? (a) : (b))

/* HTTP Authentication constants for validation */
#define HTTP_MAX_USERNAME_LEN    64
#define HTTP_MAX_PASSWORD_LEN    64
#define HTTP_MAX_REALM_LEN       128
#define HTTP_MAX_AUTH_HEADER_LEN 1024

/* Fallback implementations for missing functions */

/* Centralized validation data - single source of truth */
static const char* const valid_http_methods[] = {"GET", "POST", "HEAD", "OPTIONS", NULL};

static const char* const valid_http_versions[] = {"HTTP/1.0", "HTTP/1.1", NULL};

static const char* const valid_onvif_paths[] = {"/onvif/device_service", "/onvif/media_service",
                                                "/onvif/ptz_service",    "/onvif/imaging_service",
                                                "/onvif/snapshot.jpeg",  NULL};

static const char* const valid_soap_actions[] = {"GetCapabilities",
                                                 "GetDeviceInformation",
                                                 "GetSystemDateAndTime",
                                                 "SetSystemDateAndTime",
                                                 "GetSystemLogging",
                                                 "GetScopes",
                                                 "SetScopes",
                                                 "AddScopes",
                                                 "RemoveScopes",
                                                 "GetDiscoveryMode",
                                                 "SetDiscoveryMode",
                                                 "GetRemoteDiscoveryMode",
                                                 "SetRemoteDiscoveryMode",
                                                 "GetDPAddresses",
                                                 "SetDPAddresses",
                                                 "GetNetworkInterfaces",
                                                 "SetNetworkInterfaces",
                                                 "GetNetworkProtocols",
                                                 "SetNetworkProtocols",
                                                 "GetNetworkDefaultGateway",
                                                 "SetNetworkDefaultGateway",
                                                 "GetZeroConfiguration",
                                                 "SetZeroConfiguration",
                                                 "GetIPAddressFilter",
                                                 "SetIPAddressFilter",
                                                 "AddIPAddressFilter",
                                                 "RemoveIPAddressFilter",
                                                 "GetAccessPolicy",
                                                 "SetAccessPolicy",
                                                 "CreateCertificate",
                                                 "GetCertificates",
                                                 "GetCertificateInformation",
                                                 "SetCertificate",
                                                 "DeleteCertificate",
                                                 "GetPkcs10Request",
                                                 "LoadCertificateWithPrivateKey",
                                                 "GetClientCertificateMode",
                                                 "SetClientCertificateMode",
                                                 "GetRelayOutputs",
                                                 "SetRelayOutputSettings",
                                                 "SetRelayOutputState",
                                                 "GetServiceCapabilities",
                                                 "SystemReboot",
                                                 "GetVideoSources",
                                                 "GetVideoOutputs",
                                                 "GetAudioSources",
                                                 "GetAudioOutputs",
                                                 "GetAudioSourceConfigurations",
                                                 "GetAudioOutputConfigurations",
                                                 "GetVideoSourceConfigurations",
                                                 "GetVideoOutputConfigurations",
                                                 "GetMetadataConfigurations",
                                                 "GetCompositeConfigurations",
                                                 "GetAudioDecoderConfigurations",
                                                 "GetVideoAnalyticsConfigurations",
                                                 "GetPTZConfigurations",
                                                 "GetVideoSourceConfiguration",
                                                 "GetVideoOutputConfiguration",
                                                 "GetAudioSourceConfiguration",
                                                 "GetAudioOutputConfiguration",
                                                 "GetMetadataConfiguration",
                                                 "GetCompositeConfiguration",
                                                 "GetAudioDecoderConfiguration",
                                                 "GetVideoAnalyticsConfiguration",
                                                 "GetPTZConfiguration",
                                                 "GetVideoSourceConfigurationOptions",
                                                 "GetVideoOutputConfigurationOptions",
                                                 "GetAudioSourceConfigurationOptions",
                                                 "GetAudioOutputConfigurationOptions",
                                                 "GetMetadataConfigurationOptions",
                                                 "GetCompositeConfigurationOptions",
                                                 "GetAudioDecoderConfigurationOptions",
                                                 "GetVideoAnalyticsConfigurationOptions",
                                                 "GetPTZConfigurationOptions",
                                                 "GetGuaranteedVideoItemBounds",
                                                 "GetStreamUri",
                                                 "GetSnapshotUri",
                                                 "GetProfiles",
                                                 "AddProfile",
                                                 "RemoveProfile",
                                                 "GetVideoSourceMode",
                                                 "SetVideoSourceMode",
                                                 "GetOSD",
                                                 "GetOSDOptions",
                                                 "SetOSD",
                                                 "CreateOSD",
                                                 "DeleteOSD",
                                                 "GetMoveOptions",
                                                 "GetStatus",
                                                 "GetConfiguration",
                                                 "GetConfigurations",
                                                 "GetCompatibleConfigurations",
                                                 "SetConfiguration",
                                                 "GetConfigurationOptions",
                                                 "Stop",
                                                 "AbsoluteMove",
                                                 "RelativeMove",
                                                 "ContinuousMove",
                                                 "GetPresets",
                                                 "SetPreset",
                                                 "RemovePreset",
                                                 "GotoPreset",
                                                 "GetImagingSettings",
                                                 "SetImagingSettings",
                                                 "GetOptions",
                                                 NULL};

/* Helper function to check if string is in array */
static int is_string_in_array(const char* str, const char* const array[]) {
  ONVIF_VALIDATE_NULL(str, "string");
  ONVIF_VALIDATE_NULL(array, "array");

  for (int i = 0; array[i] != NULL; i++) {
    if (strcmp(str, array[i]) == 0) {
      return ONVIF_VALIDATION_SUCCESS;
    }
  }
  return ONVIF_VALIDATION_FAILED;
}

/**
 * @brief Validate HTTP method
 * @param method HTTP method string
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_http_method(const char* method) {
  return is_string_in_array(method, valid_http_methods);
}

/**
 * @brief Validate HTTP path for security
 * @param path HTTP request path
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_http_path(const char* path) {
  ONVIF_VALIDATE_NULL(path, "path");

  // Check for path traversal attempts
  if (strstr(path, "..") != NULL) {
    ONVIF_LOG_ERROR("Path traversal attempt detected: %s\n", path);
    return ONVIF_VALIDATION_FAILED;
  }
  if (strstr(path, "//") != NULL) {
    ONVIF_LOG_ERROR("Double slash in path detected: %s\n", path);
    return ONVIF_VALIDATION_FAILED;
  }
  if (strstr(path, "\\") != NULL) {
    ONVIF_LOG_ERROR("Backslash in path detected: %s\n", path);
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for null bytes
  if (strlen(path) != strnlen(path, 1024)) {
    ONVIF_LOG_ERROR("Null byte in path detected: %s\n", path);
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for valid ONVIF paths only
  if (strncmp(path, "/onvif/", 7) != 0) {
    ONVIF_LOG_ERROR("Invalid ONVIF path: %s\n", path);
    return ONVIF_VALIDATION_FAILED;
  }

  // Check against centralized valid paths
  return is_string_in_array(path, valid_onvif_paths);
}

/**
 * @brief Validate HTTP version
 * @param version HTTP version string
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_http_version(const char* version) {
  return is_string_in_array(version, valid_http_versions);
}

/**
 * @brief Validate Content-Length header
 * @param content_length Content length value
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_content_length(size_t content_length) {
  // Reasonable limits for ONVIF requests
  const size_t MAX_CONTENT_LENGTH = 1024 * 1024; // 1MB
  const size_t MIN_CONTENT_LENGTH = 0;

  if (content_length < MIN_CONTENT_LENGTH) {
    ONVIF_LOG_ERROR("Content length too small: %zu\n", content_length);
    return ONVIF_VALIDATION_FAILED;
  }

  if (content_length > MAX_CONTENT_LENGTH) {
    ONVIF_LOG_ERROR("Content length too large: %zu (max: %zu)\n", content_length,
                    MAX_CONTENT_LENGTH);
    return ONVIF_VALIDATION_FAILED;
  }

  return ONVIF_VALIDATION_SUCCESS;
}

/**
 * @brief Validate SOAP action name
 * @param action SOAP action string
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_soap_action(const char* action) {
  ONVIF_VALIDATE_NULL(action, "action");

  // Check length
  size_t len = strlen(action);
  if (len == 0) {
    ONVIF_LOG_ERROR("SOAP action is empty\n");
    return ONVIF_VALIDATION_FAILED;
  }
  if (len > 64) {
    ONVIF_LOG_ERROR("SOAP action too long: %zu (max: 64)\n", len);
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for valid characters (alphanumeric and common XML characters)
  for (size_t i = 0; i < len; i++) {
    char character = action[i];
    if (!isalnum((unsigned char)character) && character != '_' && character != '-' &&
        character != '.') {
      ONVIF_LOG_ERROR("Invalid character in SOAP action: '%c' at position %zu\n", character, i);
      return ONVIF_VALIDATION_FAILED;
    }
  }

  // Check against centralized valid actions
  return is_string_in_array(action, valid_soap_actions);
}

/**
 * @brief Validate XML content for basic security
 * @param xml XML content string
 * @param max_length Maximum allowed length
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_xml_content(const char* xml, size_t max_length) {
  ONVIF_VALIDATE_NULL(xml, "xml");

  size_t len = strlen(xml);
  if (len > max_length) {
    ONVIF_LOG_ERROR("XML content too long: %zu (max: %zu)\n", len, max_length);
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for null bytes
  if (len != strnlen(xml, max_length)) {
    ONVIF_LOG_ERROR("Null byte detected in XML content\n");
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for basic XML structure
  // Accept XML declaration or SOAP envelope with various namespace prefixes
  if (strstr(xml, "<?xml") == NULL && strstr(xml, "<soap:") == NULL &&
      strstr(xml, "<s:Envelope") == NULL && strstr(xml, "<soapenv:") == NULL) {
    ONVIF_LOG_ERROR("Invalid XML structure: missing XML declaration or SOAP envelope\n");
    platform_log_debug("Invalid XML content (length=%zu): %.*s\n", len, (int)MIN(len, 200), xml);
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for dangerous patterns
  const char* dangerous_patterns[] = {"<script",  "javascript:", "vbscript:", "onload=", "onerror=",
                                      "onclick=", "eval(",       "exec(",     "system(", NULL};

  for (int i = 0; dangerous_patterns[i] != NULL; i++) {
    if (strcasestr(xml, dangerous_patterns[i]) != NULL) {
      ONVIF_LOG_ERROR("Dangerous pattern detected in XML: %s\n", dangerous_patterns[i]);
      return ONVIF_VALIDATION_FAILED;
    }
  }

  return ONVIF_VALIDATION_SUCCESS;
}

/**
 * @brief Sanitize string input
 * @param input Input string to sanitize
 * @param output Output buffer for sanitized string
 * @param output_size Size of output buffer
 * @return 1 on success, 0 on failure
 */
int sanitize_string_input(const char* input, char* output, size_t output_size) {
  if (!input || !output || output_size == 0) {
    return 0;
  }

  size_t input_len = strlen(input);
  if (input_len >= output_size) {
    return 0;
  }

  size_t out_pos = 0;
  for (size_t i = 0; i < input_len && out_pos < output_size - 1; i++) {
    char character = input[i];

    // Remove or escape dangerous characters
    switch (character) {
    case '<':
      if (out_pos + 4 < output_size - 1) {
        strcpy(output + out_pos, "&lt;");
        out_pos += 4;
      }
      break;
    case '>':
      if (out_pos + 4 < output_size - 1) {
        strcpy(output + out_pos, "&gt;");
        out_pos += 4;
      }
      break;
    case '&':
      if (out_pos + 5 < output_size - 1) {
        strcpy(output + out_pos, "&amp;");
        out_pos += 5;
      }
      break;
    case '"':
      if (out_pos + 6 < output_size - 1) {
        strcpy(output + out_pos, "&quot;");
        out_pos += 6;
      }
      break;
    case '\'':
      if (out_pos + 6 < output_size - 1) {
        strcpy(output + out_pos, "&apos;");
        out_pos += 6;
      }
      break;
    case '\0':
      // Skip null bytes
      break;
    default:
      if (isprint((unsigned char)character)) {
        output[out_pos++] = character;
      }
      break;
    }
  }

  output[out_pos] = '\0';
  return 1;
}

/* ==================== HTTP Authentication Validation Functions ==================== */

/**
 * @brief Validate username for security and format
 * @param username Username to validate
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_username_input(const char* username) {
  if (!username) {
    return ONVIF_VALIDATION_FAILED;
  }

  // Use common validation utilities
  validation_result_t result =
    validate_string("username", username, 1, HTTP_MAX_USERNAME_LEN - 1, 0);
  if (!validation_is_valid(&result)) {
    platform_log_warning("Invalid username: %s", validation_get_error_message(&result));
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for injection patterns
  if (security_detect_sql_injection(username) || security_detect_xss_attack(username)) {
    platform_log_warning("Username contains injection patterns: %s", username);
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for valid characters (alphanumeric, underscore, hyphen, dot)
  for (const char* p = username; *p; p++) {
    if (!isalnum(*p) && *p != '_' && *p != '-' && *p != '.') {
      platform_log_warning("Username contains invalid character '%c': %s", *p, username);
      return ONVIF_VALIDATION_FAILED;
    }
  }

  return ONVIF_VALIDATION_SUCCESS;
}

/**
 * @brief Validate password for security and format
 * @param password Password to validate
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_password_input(const char* password) {
  if (!password) {
    return ONVIF_VALIDATION_FAILED;
  }

  // Use common validation utilities
  validation_result_t result =
    validate_string("password", password, 1, HTTP_MAX_PASSWORD_LEN - 1, 0);
  if (!validation_is_valid(&result)) {
    platform_log_warning("Invalid password: %s", validation_get_error_message(&result));
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for injection patterns
  if (security_detect_sql_injection(password) || security_detect_xss_attack(password)) {
    platform_log_warning("Password contains injection patterns");
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for null bytes or control characters
  for (const char* p = password; *p; p++) {
    if (*p == '\0' || iscntrl(*p)) {
      platform_log_warning("Password contains invalid control character");
      return ONVIF_VALIDATION_FAILED;
    }
  }

  return ONVIF_VALIDATION_SUCCESS;
}

/**
 * @brief Validate Authorization header for security and format
 * @param auth_header Authorization header value
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_auth_header_input(const char* auth_header) {
  if (!auth_header) {
    return ONVIF_VALIDATION_FAILED;
  }

  // Use common validation utilities
  validation_result_t result =
    validate_string("auth_header", auth_header, 6, HTTP_MAX_AUTH_HEADER_LEN - 1, 0);
  if (!validation_is_valid(&result)) {
    platform_log_warning("Invalid Authorization header: %s", validation_get_error_message(&result));
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for injection patterns
  if (security_detect_sql_injection(auth_header) || security_detect_xss_attack(auth_header)) {
    platform_log_warning("Authorization header contains injection patterns");
    return ONVIF_VALIDATION_FAILED;
  }

  // Validate Basic auth format
  if (strnlen(auth_header, 6) < 6) {
    platform_log_warning("Authorization header too short");
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for "Basic " prefix (case-insensitive)
  char prefix[7] = {0};
  strncpy(prefix, auth_header, 6);
  if (strcasecmp(prefix, "Basic ") != 0) {
    platform_log_warning("Authorization header does not start with 'Basic ': %s", prefix);
    return ONVIF_VALIDATION_FAILED;
  }

  return ONVIF_VALIDATION_SUCCESS;
}

/**
 * @brief Validate realm for security and format
 * @param realm Realm to validate
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_realm_input(const char* realm) {
  if (!realm) {
    return ONVIF_VALIDATION_FAILED;
  }

  // Use common validation utilities
  validation_result_t result = validate_string("realm", realm, 1, HTTP_MAX_REALM_LEN - 1, 0);
  if (!validation_is_valid(&result)) {
    platform_log_warning("Invalid realm: %s", validation_get_error_message(&result));
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for injection patterns
  if (security_detect_sql_injection(realm) || security_detect_xss_attack(realm)) {
    platform_log_warning("Realm contains injection patterns: %s", realm);
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for valid characters (printable ASCII, no quotes)
  for (const char* p = realm; *p; p++) {
    if (*p < 32 || *p > 126 || *p == '"' || *p == '\\') {
      platform_log_warning("Realm contains invalid character '%c': %s", *p, realm);
      return ONVIF_VALIDATION_FAILED;
    }
  }

  return ONVIF_VALIDATION_SUCCESS;
}

/**
 * @brief Sanitize and validate Base64 encoded credentials
 * @param encoded Base64 encoded string
 * @param decoded Buffer for decoded data
 * @param decoded_size Size of decoded buffer
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_and_decode_base64(const char* encoded, char* decoded, size_t decoded_size) {
  if (!encoded || !decoded || decoded_size == 0) {
    return ONVIF_VALIDATION_FAILED;
  }

  // Validate Base64 format
  size_t encoded_len = strnlen(encoded, HTTP_MAX_AUTH_HEADER_LEN);
  if (encoded_len == 0) {
    platform_log_warning("Empty Base64 encoded string");
    return ONVIF_VALIDATION_FAILED;
  }

  // Decode using secure Base64 utility (includes character validation)
  if (onvif_util_base64_decode(encoded, decoded, decoded_size) != ONVIF_SUCCESS) {
    platform_log_warning("Failed to decode Base64 credentials");
    return ONVIF_VALIDATION_FAILED;
  }

  // Validate decoded content
  size_t decoded_len = strnlen(decoded, decoded_size);
  if (decoded_len == 0) {
    platform_log_warning("Empty decoded credentials");
    return ONVIF_VALIDATION_FAILED;
  }

  // Check for null bytes in decoded content
  for (size_t i = 0; i < decoded_len; i++) {
    if (decoded[i] == '\0' && i < decoded_len - 1) {
      platform_log_warning("Null byte found in decoded credentials at position %zu", i);
      return ONVIF_VALIDATION_FAILED;
    }
  }

  return ONVIF_VALIDATION_SUCCESS;
}

/* ==================== HTTP Request Validation (Execution Logic) ==================== */

/**
 * @brief Validate HTTP request comprehensively
 * @param request HTTP request structure
 * @return ONVIF_VALIDATION_SUCCESS if valid, ONVIF_VALIDATION_FAILED if invalid
 */
int validate_http_request(const http_request_t* request) {
  ONVIF_VALIDATE_NULL(request, "request");

  // Validate method
  if (validate_http_method(request->method) != ONVIF_VALIDATION_SUCCESS) {
    ONVIF_LOG_ERROR("Invalid HTTP method: %s\n", request->method);
    return ONVIF_VALIDATION_FAILED;
  }

  // Validate path
  if (validate_http_path(request->path) != ONVIF_VALIDATION_SUCCESS) {
    ONVIF_LOG_ERROR("Invalid HTTP path: %s\n", request->path);
    return ONVIF_VALIDATION_FAILED;
  }

  // Validate version
  if (validate_http_version(request->version) != ONVIF_VALIDATION_SUCCESS) {
    ONVIF_LOG_ERROR("Invalid HTTP version: %s\n", request->version);
    return ONVIF_VALIDATION_FAILED;
  }

  // Validate content length
  if (validate_content_length(request->content_length) != ONVIF_VALIDATION_SUCCESS) {
    ONVIF_LOG_ERROR("Invalid content length: %zu\n", request->content_length);
    return ONVIF_VALIDATION_FAILED;
  }

  // Validate body if present
  if (request->body && request->body_length > 0) {
    if (validate_xml_content(request->body, request->body_length) != ONVIF_VALIDATION_SUCCESS) {
      ONVIF_LOG_ERROR("Invalid XML content in request body\n");

      platform_log_debug("Full request body content (length=%zu): %.*s\n", request->body_length,
                         (int)request->body_length, request->body);
      return ONVIF_VALIDATION_FAILED;
    }
  }

  return ONVIF_VALIDATION_SUCCESS;
}
