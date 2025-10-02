/**
 * @file soap_test_helpers.c
 * @brief Implementation of SOAP request/response testing helper functions
 * @author kkrzysztofik
 * @date 2025
 */

#define _XOPEN_SOURCE 700

#include "soap_test_helpers.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "generated/soapH.h"
#include "utils/error/error_handling.h"

/* ============================================================================
 * SOAP Response Parsing Support
 * ============================================================================ */

/**
 * @brief State for parsing SOAP response from memory buffer
 */
typedef struct {
  const char* data;
  size_t length;
  size_t position;
} soap_buffer_state_t;

/**
 * @brief Custom frecv callback for gSOAP to read from memory buffer
 */
static size_t soap_frecv_from_buffer(struct soap* soap, char* buf, size_t len) {
  soap_buffer_state_t* state = (soap_buffer_state_t*)soap->user;
  if (!state || !buf || len == 0) {
    return 0;
  }

  size_t remaining = state->length - state->position;
  if (remaining == 0) {
    return 0;  // EOF
  }

  size_t to_copy = (len < remaining) ? len : remaining;
  memcpy(buf, state->data + state->position, to_copy);
  state->position += to_copy;

  return to_copy;
}

/* ============================================================================
 * HTTP Request Builder Implementation
 * ============================================================================ */

http_request_t* soap_test_create_request(const char* action_name, const char* soap_envelope,
                                         const char* service_path) {
  if (!action_name || !soap_envelope || !service_path) {
    return NULL;
  }

  // Allocate request structure
  http_request_t* request = (http_request_t*)calloc(1, sizeof(http_request_t));
  if (!request) {
    return NULL;
  }

  // Set HTTP method
  strncpy(request->method, "POST", sizeof(request->method) - 1);

  // Set service path
  strncpy(request->path, service_path, sizeof(request->path) - 1);

  // Set HTTP version
  strncpy(request->version, "HTTP/1.1", sizeof(request->version) - 1);

  // Set SOAP body
  request->body_length = strlen(soap_envelope);
  request->body = (char*)malloc(request->body_length + 1);
  if (!request->body) {
    free(request);
    return NULL;
  }
  strcpy(request->body, soap_envelope);
  request->content_length = request->body_length;

  // Add required headers
  request->header_count = 2;
  request->headers = (http_header_t*)calloc(request->header_count, sizeof(http_header_t));
  if (!request->headers) {
    free(request->body);
    free(request);
    return NULL;
  }

  // Content-Type header
  request->headers[0].name = strdup("Content-Type");
  request->headers[0].value = strdup("application/soap+xml; charset=utf-8");
  if (!request->headers[0].name || !request->headers[0].value) {
    soap_test_free_request(request);
    return NULL;
  }

  // SOAPAction header
  request->headers[1].name = strdup("SOAPAction");
  char action_header[256];
  snprintf(action_header, sizeof(action_header), "\"%s\"", action_name);
  request->headers[1].value = strdup(action_header);
  if (!request->headers[1].name || !request->headers[1].value) {
    soap_test_free_request(request);
    return NULL;
  }

  return request;
}

void soap_test_free_request(http_request_t* request) {
  if (!request) {
    return;
  }

  if (request->body) {
    free(request->body);
  }

  if (request->headers) {
    for (size_t i = 0; i < request->header_count; i++) {
      if (request->headers[i].name) {
        free(request->headers[i].name);
      }
      if (request->headers[i].value) {
        free(request->headers[i].value);
      }
    }
    free(request->headers);
  }

  free(request);
}

/* ============================================================================
 * SOAP Response Parser Implementation
 * ============================================================================ */

int soap_test_init_response_parsing(onvif_gsoap_context_t* ctx, const http_response_t* response) {
  if (!ctx || !response || !response->body) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize gSOAP context
  int result = onvif_gsoap_init(ctx);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Allocate and initialize buffer state for parsing
  soap_buffer_state_t* state = (soap_buffer_state_t*)malloc(sizeof(soap_buffer_state_t));
  if (!state) {
    onvif_gsoap_cleanup(ctx);
    return ONVIF_ERROR_MEMORY;
  }

  state->data = response->body;
  state->length = response->body_length;
  state->position = 0;

  // Set up gSOAP to use our buffer reading callback
  ctx->soap.user = state;
  ctx->soap.frecv = soap_frecv_from_buffer;

  return ONVIF_SUCCESS;
}

void soap_test_cleanup_response_parsing(onvif_gsoap_context_t* ctx) {
  if (!ctx) {
    return;
  }

  // Free the buffer state that was allocated in init_response_parsing
  if (ctx->soap.user) {
    free(ctx->soap.user);
    ctx->soap.user = NULL;
  }
}

int soap_test_parse_get_profiles_response(onvif_gsoap_context_t* ctx,
                                          struct _trt__GetProfilesResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _trt__GetProfilesResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _trt__GetProfilesResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__trt__GetProfilesResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__trt__GetProfilesResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_get_nodes_response(onvif_gsoap_context_t* ctx,
                                       struct _onvif3__GetNodesResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure (PTZ uses onvif3 namespace)
  *response = (struct _onvif3__GetNodesResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _onvif3__GetNodesResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__onvif3__GetNodesResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__onvif3__GetNodesResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_get_device_info_response(onvif_gsoap_context_t* ctx,
                                             struct _tds__GetDeviceInformationResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _tds__GetDeviceInformationResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _tds__GetDeviceInformationResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__tds__GetDeviceInformationResponse(&ctx->soap, *response);

  // Begin receiving and parse the complete message
  if (soap_begin_recv(&ctx->soap) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  // Navigate to the Body element
  if (soap_envelope_begin_in(&ctx->soap) != SOAP_OK || soap_body_begin_in(&ctx->soap) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  // Deserialize the response element from the body
  if (soap_in__tds__GetDeviceInformationResponse(&ctx->soap, NULL, *response, NULL) == NULL) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  // Close body and envelope
  if (soap_body_end_in(&ctx->soap) != SOAP_OK || soap_envelope_end_in(&ctx->soap) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  // Finish receiving
  if (soap_end_recv(&ctx->soap) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Media Service - Additional Response Parsers
 * ============================================================================ */

int soap_test_parse_get_stream_uri_response(onvif_gsoap_context_t* ctx,
                                            struct _trt__GetStreamUriResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _trt__GetStreamUriResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _trt__GetStreamUriResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__trt__GetStreamUriResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__trt__GetStreamUriResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_create_profile_response(onvif_gsoap_context_t* ctx,
                                            struct _trt__CreateProfileResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _trt__CreateProfileResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _trt__CreateProfileResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__trt__CreateProfileResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__trt__CreateProfileResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_delete_profile_response(onvif_gsoap_context_t* ctx,
                                            struct _trt__DeleteProfileResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _trt__DeleteProfileResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _trt__DeleteProfileResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__trt__DeleteProfileResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__trt__DeleteProfileResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_set_video_source_config_response(
  onvif_gsoap_context_t* ctx, struct _trt__SetVideoSourceConfigurationResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _trt__SetVideoSourceConfigurationResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _trt__SetVideoSourceConfigurationResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__trt__SetVideoSourceConfigurationResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__trt__SetVideoSourceConfigurationResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_set_video_encoder_config_response(
  onvif_gsoap_context_t* ctx, struct _trt__SetVideoEncoderConfigurationResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _trt__SetVideoEncoderConfigurationResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _trt__SetVideoEncoderConfigurationResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__trt__SetVideoEncoderConfigurationResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__trt__SetVideoEncoderConfigurationResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_get_metadata_configs_response(
  onvif_gsoap_context_t* ctx, struct _trt__GetMetadataConfigurationsResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _trt__GetMetadataConfigurationsResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _trt__GetMetadataConfigurationsResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__trt__GetMetadataConfigurationsResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__trt__GetMetadataConfigurationsResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_set_metadata_config_response(
  onvif_gsoap_context_t* ctx, struct _trt__SetMetadataConfigurationResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _trt__SetMetadataConfigurationResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _trt__SetMetadataConfigurationResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__trt__SetMetadataConfigurationResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__trt__SetMetadataConfigurationResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_start_multicast_response(
  onvif_gsoap_context_t* ctx, struct _trt__StartMulticastStreamingResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _trt__StartMulticastStreamingResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _trt__StartMulticastStreamingResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__trt__StartMulticastStreamingResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__trt__StartMulticastStreamingResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_stop_multicast_response(
  onvif_gsoap_context_t* ctx, struct _trt__StopMulticastStreamingResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _trt__StopMulticastStreamingResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _trt__StopMulticastStreamingResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__trt__StopMulticastStreamingResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__trt__StopMulticastStreamingResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * PTZ Service - Additional Response Parsers
 * ============================================================================ */

int soap_test_parse_absolute_move_response(onvif_gsoap_context_t* ctx,
                                           struct _onvif3__AbsoluteMoveResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _onvif3__AbsoluteMoveResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _onvif3__AbsoluteMoveResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__onvif3__AbsoluteMoveResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__onvif3__AbsoluteMoveResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_get_presets_response(onvif_gsoap_context_t* ctx,
                                         struct _onvif3__GetPresetsResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _onvif3__GetPresetsResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _onvif3__GetPresetsResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__onvif3__GetPresetsResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__onvif3__GetPresetsResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_set_preset_response(onvif_gsoap_context_t* ctx,
                                        struct _onvif3__SetPresetResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _onvif3__SetPresetResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _onvif3__SetPresetResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__onvif3__SetPresetResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__onvif3__SetPresetResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_goto_preset_response(onvif_gsoap_context_t* ctx,
                                         struct _onvif3__GotoPresetResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _onvif3__GotoPresetResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _onvif3__GotoPresetResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__onvif3__GotoPresetResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__onvif3__GotoPresetResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_remove_preset_response(onvif_gsoap_context_t* ctx,
                                           struct _onvif3__RemovePresetResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _onvif3__RemovePresetResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _onvif3__RemovePresetResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__onvif3__RemovePresetResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__onvif3__RemovePresetResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Device Service - Additional Response Parsers
 * ============================================================================ */

int soap_test_parse_get_capabilities_response(onvif_gsoap_context_t* ctx,
                                              struct _tds__GetCapabilitiesResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _tds__GetCapabilitiesResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _tds__GetCapabilitiesResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__tds__GetCapabilitiesResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__tds__GetCapabilitiesResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_get_system_date_time_response(
  onvif_gsoap_context_t* ctx, struct _tds__GetSystemDateAndTimeResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _tds__GetSystemDateAndTimeResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _tds__GetSystemDateAndTimeResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__tds__GetSystemDateAndTimeResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__tds__GetSystemDateAndTimeResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_get_services_response(onvif_gsoap_context_t* ctx,
                                          struct _tds__GetServicesResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _tds__GetServicesResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _tds__GetServicesResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__tds__GetServicesResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__tds__GetServicesResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

int soap_test_parse_system_reboot_response(onvif_gsoap_context_t* ctx,
                                           struct _tds__SystemRebootResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _tds__SystemRebootResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _tds__SystemRebootResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__tds__SystemRebootResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__tds__SystemRebootResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Imaging Service - Additional Response Parsers
 * ============================================================================ */

int soap_test_parse_set_imaging_settings_response(
  onvif_gsoap_context_t* ctx, struct _onvif4__SetImagingSettingsResponse** response) {
  if (!ctx || !response) {
    return ONVIF_ERROR_INVALID;
  }

  // Allocate response structure
  *response = (struct _onvif4__SetImagingSettingsResponse*)soap_malloc(
    &ctx->soap, sizeof(struct _onvif4__SetImagingSettingsResponse));
  if (!*response) {
    return ONVIF_ERROR_MEMORY;
  }

  // Initialize response structure
  soap_default__onvif4__SetImagingSettingsResponse(&ctx->soap, *response);

  // Parse SOAP response
  if (soap_read__onvif4__SetImagingSettingsResponse(&ctx->soap, *response) != SOAP_OK) {
    return ONVIF_ERROR_PARSE_FAILED;
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Response Validation Implementation
 * ============================================================================ */

int soap_test_validate_http_response(const http_response_t* response, int expected_status,
                                     const char* expected_content_type) {
  if (!response) {
    return ONVIF_ERROR_INVALID;
  }

  // Check status code
  if (response->status_code != expected_status) {
    return ONVIF_ERROR;
  }

  // Check content type (if specified)
  if (expected_content_type && response->content_type) {
    if (strstr(response->content_type, expected_content_type) == NULL) {
      return ONVIF_ERROR;
    }
  }

  // Check body exists
  if (!response->body || response->body_length == 0) {
    return ONVIF_ERROR;
  }

  return ONVIF_SUCCESS;
}

int soap_test_check_soap_fault(const http_response_t* response, char* fault_code,
                               char* fault_string) {
  if (!response || !response->body) {
    return -1;
  }

  // Simple check for SOAP fault
  if (strstr(response->body, "<soap:Fault>") || strstr(response->body, "<s:Fault>")) {
    // Extract fault code if buffer provided
    if (fault_code) {
      soap_test_extract_element_text(response->body, "faultcode", fault_code, 256);
    }

    // Extract fault string if buffer provided
    if (fault_string) {
      soap_test_extract_element_text(response->body, "faultstring", fault_string, 512);
    }

    return 1; // Fault exists
  }

  return 0; // No fault
}

/* ============================================================================
 * XML Field Extraction Implementation
 * ============================================================================ */

int soap_test_extract_element_text(const char* xml, const char* element_name, char* value,
                                   size_t value_size) {
  if (!xml || !element_name || !value) {
    return ONVIF_ERROR_INVALID;
  }

  // Build opening tag
  char open_tag[128];
  snprintf(open_tag, sizeof(open_tag), "<%s>", element_name);

  // Build closing tag
  char close_tag[128];
  snprintf(close_tag, sizeof(close_tag), "</%s>", element_name);

  // Find opening tag
  const char* start = strstr(xml, open_tag);
  if (!start) {
    return ONVIF_ERROR_NOT_FOUND;
  }
  start += strlen(open_tag);

  // Find closing tag
  const char* end = strstr(start, close_tag);
  if (!end) {
    return ONVIF_ERROR_NOT_FOUND;
  }

  // Extract value
  size_t text_len = end - start;
  if (text_len >= value_size) {
    text_len = value_size - 1;
  }

  strncpy(value, start, text_len);
  value[text_len] = '\0';

  return ONVIF_SUCCESS;
}

int soap_test_extract_attribute(const char* xml, const char* element_name,
                                const char* attribute_name, char* value, size_t value_size) {
  if (!xml || !element_name || !attribute_name || !value) {
    return ONVIF_ERROR_INVALID;
  }

  // Build element pattern
  char pattern[128];
  snprintf(pattern, sizeof(pattern), "<%s ", element_name);

  // Find element
  const char* element_start = strstr(xml, pattern);
  if (!element_start) {
    return ONVIF_ERROR_NOT_FOUND;
  }

  // Find attribute within element (before closing >)
  const char* element_end = strchr(element_start, '>');
  if (!element_end) {
    return ONVIF_ERROR_NOT_FOUND;
  }

  // Build attribute pattern
  char attr_pattern[128];
  snprintf(attr_pattern, sizeof(attr_pattern), "%s=\"", attribute_name);

  // Find attribute
  const char* attr_start = strstr(element_start, attr_pattern);
  if (!attr_start || attr_start > element_end) {
    return ONVIF_ERROR_NOT_FOUND;
  }
  attr_start += strlen(attr_pattern);

  // Find end quote
  const char* attr_end = strchr(attr_start, '"');
  if (!attr_end) {
    return ONVIF_ERROR_NOT_FOUND;
  }

  // Extract value
  size_t attr_len = attr_end - attr_start;
  if (attr_len >= value_size) {
    attr_len = value_size - 1;
  }

  strncpy(value, attr_start, attr_len);
  value[attr_len] = '\0';

  return ONVIF_SUCCESS;
}
