/**
 * @file soap_test_helpers.h
 * @brief Helper functions for SOAP request/response testing
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef SOAP_TEST_HELPERS_H
#define SOAP_TEST_HELPERS_H

#include "networking/http/http_parser.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include <stddef.h>

/* Forward declarations for gSOAP structures */
struct _trt__GetProfilesResponse;
struct _onvif3__GetNodesResponse;
struct _tds__GetDeviceInformationResponse;

/* Media Service Response Structures */
struct _trt__GetStreamUriResponse;
struct _trt__CreateProfileResponse;
struct _trt__DeleteProfileResponse;
struct _trt__SetVideoSourceConfigurationResponse;
struct _trt__SetVideoEncoderConfigurationResponse;
struct _trt__GetMetadataConfigurationsResponse;
struct _trt__SetMetadataConfigurationResponse;
struct _trt__StartMulticastStreamingResponse;
struct _trt__StopMulticastStreamingResponse;

/* PTZ Service Response Structures */
struct _onvif3__AbsoluteMoveResponse;
struct _onvif3__GetPresetsResponse;
struct _onvif3__SetPresetResponse;
struct _onvif3__GotoPresetResponse;
struct _onvif3__RemovePresetResponse;

/* Device Service Response Structures */
struct _tds__GetCapabilitiesResponse;
struct _tds__GetSystemDateAndTimeResponse;
struct _tds__GetServicesResponse;
struct _tds__SystemRebootResponse;

/* Imaging Service Response Structures */
struct _onvif4__SetImagingSettingsResponse;

/* ============================================================================
 * HTTP Request Builder
 * ============================================================================ */

/**
 * @brief Create HTTP request with SOAP envelope
 * @param action_name ONVIF action name (e.g., "GetProfiles")
 * @param soap_envelope SOAP XML envelope string
 * @param service_path Service endpoint path (e.g., "/onvif/media_service")
 * @return Allocated HTTP request structure (caller must free with soap_test_free_request)
 * @note Returns NULL on allocation failure or invalid parameters
 */
http_request_t* soap_test_create_request(const char* action_name, const char* soap_envelope,
                                         const char* service_path);

/**
 * @brief Free HTTP request created by soap_test_create_request
 * @param request Request to free (safe to pass NULL)
 */
void soap_test_free_request(http_request_t* request);

/* ============================================================================
 * SOAP Response Parser
 * ============================================================================ */

/**
 * @brief Initialize gSOAP context for parsing response
 * @param ctx gSOAP context to initialize (output)
 * @param response HTTP response containing SOAP envelope
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Caller must call soap_test_cleanup_response_parsing() and onvif_gsoap_cleanup() when done
 */
int soap_test_init_response_parsing(onvif_gsoap_context_t* ctx,
                                    const http_response_t* response);

/**
 * @brief Cleanup response parsing resources
 * @param ctx gSOAP context to cleanup
 * @note Must be called before onvif_gsoap_cleanup()
 */
void soap_test_cleanup_response_parsing(onvif_gsoap_context_t* ctx);

/**
 * @brief Parse SOAP response for Media GetProfiles
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_get_profiles_response(onvif_gsoap_context_t* ctx,
                                          struct _trt__GetProfilesResponse** response);

/**
 * @brief Parse SOAP response for PTZ GetNodes
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_get_nodes_response(onvif_gsoap_context_t* ctx,
                                       struct _onvif3__GetNodesResponse** response);

/**
 * @brief Parse SOAP response for Device GetDeviceInformation
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_get_device_info_response(onvif_gsoap_context_t* ctx,
                                             struct _tds__GetDeviceInformationResponse** response);

/* Media Service - Additional Response Parsers */

/**
 * @brief Parse SOAP response for Media GetStreamUri
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_get_stream_uri_response(onvif_gsoap_context_t* ctx,
                                            struct _trt__GetStreamUriResponse** response);

/**
 * @brief Parse SOAP response for Media CreateProfile
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_create_profile_response(onvif_gsoap_context_t* ctx,
                                            struct _trt__CreateProfileResponse** response);

/**
 * @brief Parse SOAP response for Media DeleteProfile
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_delete_profile_response(onvif_gsoap_context_t* ctx,
                                            struct _trt__DeleteProfileResponse** response);

/**
 * @brief Parse SOAP response for Media SetVideoSourceConfiguration
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_set_video_source_config_response(onvif_gsoap_context_t* ctx,
                                                     struct _trt__SetVideoSourceConfigurationResponse** response);

/**
 * @brief Parse SOAP response for Media SetVideoEncoderConfiguration
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_set_video_encoder_config_response(onvif_gsoap_context_t* ctx,
                                                      struct _trt__SetVideoEncoderConfigurationResponse** response);

/**
 * @brief Parse SOAP response for Media GetMetadataConfigurations
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_get_metadata_configs_response(onvif_gsoap_context_t* ctx,
                                                  struct _trt__GetMetadataConfigurationsResponse** response);

/**
 * @brief Parse SOAP response for Media SetMetadataConfiguration
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_set_metadata_config_response(onvif_gsoap_context_t* ctx,
                                                 struct _trt__SetMetadataConfigurationResponse** response);

/**
 * @brief Parse SOAP response for Media StartMulticastStreaming
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_start_multicast_response(onvif_gsoap_context_t* ctx,
                                             struct _trt__StartMulticastStreamingResponse** response);

/**
 * @brief Parse SOAP response for Media StopMulticastStreaming
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_stop_multicast_response(onvif_gsoap_context_t* ctx,
                                            struct _trt__StopMulticastStreamingResponse** response);

/* PTZ Service - Additional Response Parsers */

/**
 * @brief Parse SOAP response for PTZ AbsoluteMove
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_absolute_move_response(onvif_gsoap_context_t* ctx,
                                           struct _onvif3__AbsoluteMoveResponse** response);

/**
 * @brief Parse SOAP response for PTZ GetPresets
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_get_presets_response(onvif_gsoap_context_t* ctx,
                                         struct _onvif3__GetPresetsResponse** response);

/**
 * @brief Parse SOAP response for PTZ SetPreset
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_set_preset_response(onvif_gsoap_context_t* ctx,
                                        struct _onvif3__SetPresetResponse** response);

/**
 * @brief Parse SOAP response for PTZ GotoPreset
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_goto_preset_response(onvif_gsoap_context_t* ctx,
                                         struct _onvif3__GotoPresetResponse** response);

/**
 * @brief Parse SOAP response for PTZ RemovePreset
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_remove_preset_response(onvif_gsoap_context_t* ctx,
                                           struct _onvif3__RemovePresetResponse** response);

/* Device Service - Additional Response Parsers */

/**
 * @brief Parse SOAP response for Device GetCapabilities
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_get_capabilities_response(onvif_gsoap_context_t* ctx,
                                              struct _tds__GetCapabilitiesResponse** response);

/**
 * @brief Parse SOAP response for Device GetSystemDateAndTime
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_get_system_date_time_response(onvif_gsoap_context_t* ctx,
                                                  struct _tds__GetSystemDateAndTimeResponse** response);

/**
 * @brief Parse SOAP response for Device GetServices
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_get_services_response(onvif_gsoap_context_t* ctx,
                                          struct _tds__GetServicesResponse** response);

/**
 * @brief Parse SOAP response for Device SystemReboot
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_system_reboot_response(onvif_gsoap_context_t* ctx,
                                           struct _tds__SystemRebootResponse** response);

/* Imaging Service - Additional Response Parsers */

/**
 * @brief Parse SOAP response for Imaging SetImagingSettings
 * @param ctx gSOAP context (already initialized with soap_test_init_response_parsing)
 * @param response Parsed response structure (output)
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Response structure is allocated from gSOAP managed memory
 */
int soap_test_parse_set_imaging_settings_response(onvif_gsoap_context_t* ctx,
                                                  struct _onvif4__SetImagingSettingsResponse** response);

/* ============================================================================
 * Response Validation
 * ============================================================================ */

/**
 * @brief Validate HTTP response structure
 * @param response HTTP response to validate
 * @param expected_status Expected HTTP status code
 * @param expected_content_type Expected Content-Type header (substring match)
 * @return ONVIF_SUCCESS if valid, ONVIF_ERROR otherwise
 * @note expected_content_type can be NULL to skip content type check
 */
int soap_test_validate_http_response(const http_response_t* response, int expected_status,
                                     const char* expected_content_type);

/**
 * @brief Check if SOAP response contains fault
 * @param response HTTP response to check
 * @param fault_code Output buffer for fault code (can be NULL)
 * @param fault_string Output buffer for fault string (can be NULL)
 * @return 1 if fault exists, 0 if no fault, negative on error
 * @note If fault_code/fault_string are provided, they must be at least 256/512 bytes
 */
int soap_test_check_soap_fault(const http_response_t* response, char* fault_code,
                               char* fault_string);

/* ============================================================================
 * XML Field Extraction (Simple XPath-like)
 * ============================================================================ */

/**
 * @brief Extract text value from XML element
 * @param xml XML string to search
 * @param element_name Element name (e.g., "token")
 * @param value Output buffer for extracted value
 * @param value_size Size of output buffer
 * @return ONVIF_SUCCESS if found, ONVIF_ERROR_NOT_FOUND if not found
 * @note Searches for first occurrence of &lt;element_name&gt;text&lt;/element_name&gt;
 */
int soap_test_extract_element_text(const char* xml, const char* element_name, char* value,
                                   size_t value_size);

/**
 * @brief Extract attribute value from XML element
 * @param xml XML string to search
 * @param element_name Element name (e.g., "Configuration")
 * @param attribute_name Attribute name (e.g., "token")
 * @param value Output buffer for extracted value
 * @param value_size Size of output buffer
 * @return ONVIF_SUCCESS if found, ONVIF_ERROR_NOT_FOUND if not found
 * @note Searches for first occurrence of &lt;element_name attribute_name="value"&gt;
 */
int soap_test_extract_attribute(const char* xml, const char* element_name,
                                const char* attribute_name, char* value, size_t value_size);

#endif /* SOAP_TEST_HELPERS_H */
