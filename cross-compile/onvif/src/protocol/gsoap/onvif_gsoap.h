/**
 * @file onvif_gsoap.h
 * @brief Proper gSOAP interface using generated structures and serialization
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides a proper gSOAP interface that uses the generated
 * gSOAP structures and serialization functions instead of manual XML building.
 * This ensures proper ONVIF compliance and eliminates buffer overflow risks.
 */

#ifndef ONVIF_GSOAP_H
#define ONVIF_GSOAP_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "generated/soapH.h"    //NOLINT
#include "generated/soapStub.h" //NOLINT
#include "services/common/onvif_imaging_types.h"
#include "services/media/onvif_media.h"
#include "services/ptz/onvif_ptz.h"

/* ============================================================================
 * Error Codes
 * ============================================================================
 */

#define ONVIF_XML_SUCCESS                 0
#define ONVIF_XML_ERROR_INVALID_INPUT     -1
#define ONVIF_XML_ERROR_PARSE_FAILED      -2
#define ONVIF_XML_ERROR_MEMORY_ALLOCATION -4
#define ONVIF_XML_ERROR_NOT_FOUND         -7

/* ============================================================================
 * SOAP Version and Fault Types
 * ============================================================================
 */

typedef enum {
  SOAP_VERSION_1_1, /**< SOAP 1.1 */
  SOAP_VERSION_1_2  /**< SOAP 1.2 */
} soap_version_t;

/* ============================================================================
 * Response Generation Callback Types
 * ============================================================================
 */

/**
 * @brief Callback function type for endpoint-specific response generation
 * @param soap gSOAP context
 * @param user_data User-provided data for the callback
 * @return 0 on success, negative error code on failure
 */
typedef int (*onvif_response_callback_t)(struct soap* soap, void* user_data);

typedef enum {
  SOAP_FAULT_VERSION_MISMATCH, /**< VersionMismatch */
  SOAP_FAULT_MUST_UNDERSTAND,  /**< MustUnderstand */
  SOAP_FAULT_CLIENT,           /**< Client */
  SOAP_FAULT_SERVER            /**< Server */
} soap_fault_type_t;

/* ============================================================================
 * gSOAP Context Structure
 * ============================================================================
 */

/**
 * @brief gSOAP context using proper gSOAP structures and serialization
 */
typedef struct {
  // gSOAP context
  struct soap* soap;

  // Statistics
  size_t total_bytes_written;     // Total bytes written
  uint64_t generation_start_time; // Generation start timestamp
  uint64_t generation_end_time;   // Generation end timestamp

  // User data
  void* user_data; // User-defined data
} onvif_gsoap_context_t;

/* ============================================================================
 * gSOAP Context Management Functions
 * ============================================================================
 */

/**
 * @brief Initialize gSOAP context with proper soap structure
 * @param ctx Pointer to gSOAP context structure
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_init(onvif_gsoap_context_t* ctx);

/**
 * @brief Clean up gSOAP context and free resources
 * @param ctx Pointer to gSOAP context structure
 */
void onvif_gsoap_cleanup(onvif_gsoap_context_t* ctx);

/**
 * @brief Reset gSOAP context to initial state
 * @param ctx Pointer to gSOAP context structure
 */
void onvif_gsoap_reset(onvif_gsoap_context_t* ctx);

/* ============================================================================
 * Response Generation Functions
 * ============================================================================
 */

/**
 * @brief Start SOAP response serialization
 * @param ctx Pointer to gSOAP context structure
 * @param response_data Pointer to response data structure
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_serialize_response(onvif_gsoap_context_t* ctx, void* response_data);

/**
 * @brief Finalize SOAP response serialization
 * @param ctx Pointer to gSOAP context structure
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_finalize_response(onvif_gsoap_context_t* ctx);

/* ============================================================================
 * ONVIF Service Helper Functions
 * ============================================================================
 */

/**
 * @brief Generic SOAP response generation with callback
 * @param ctx gSOAP context
 * @param callback Endpoint-specific response generation callback
 * @param user_data User data passed to callback
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_response_with_callback(onvif_gsoap_context_t* ctx,
                                                onvif_response_callback_t callback,
                                                void* user_data);

/**
 * @brief Generate GetDeviceInformation response
 * @param ctx Pointer to gSOAP context structure
 * @param manufacturer Device manufacturer
 * @param model Device model
 * @param firmware_version Firmware version
 * @param serial_number Serial number
 * @param hardware_id Hardware ID
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_device_info_response(onvif_gsoap_context_t* ctx, const char* manufacturer,
                                              const char* model, const char* firmware_version,
                                              const char* serial_number, const char* hardware_id);

/**
 * @brief Callback data structure for device info response
 */
typedef struct {
  char manufacturer[64];
  char model[64];
  char firmware_version[32];
  char serial_number[64];
  char hardware_id[32];
} device_info_callback_data_t;

/**
 * @brief Device info response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to device_info_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int device_info_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for capabilities response
 */
typedef struct {
  const struct device_capabilities* capabilities;
} capabilities_callback_data_t;

/**
 * @brief Capabilities response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to capabilities_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int capabilities_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for system datetime response
 */
typedef struct {
  const struct tm* tm_info;
} system_datetime_callback_data_t;

/**
 * @brief System datetime response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to system_datetime_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int system_datetime_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for services response
 */
typedef struct {
  int include_capability;
} services_callback_data_t;

/**
 * @brief Services response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to services_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int services_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for system reboot response
 */
typedef struct {
  const char* message;
} system_reboot_callback_data_t;

/**
 * @brief System reboot response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to system_reboot_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int system_reboot_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for imaging settings response
 */
typedef struct {
  const struct imaging_settings* settings;
} imaging_settings_callback_data_t;

/**
 * @brief Imaging settings response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to imaging_settings_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int imaging_settings_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for set imaging settings response
 */
typedef struct {
  const char* message;
} set_imaging_settings_callback_data_t;

/**
 * @brief Set imaging settings response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to set_imaging_settings_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int set_imaging_settings_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for PTZ nodes response
 */
typedef struct {
  const struct ptz_node* nodes;
  int count;
} ptz_nodes_callback_data_t;

/**
 * @brief PTZ nodes response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to ptz_nodes_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int ptz_nodes_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for PTZ absolute move response
 */
typedef struct {
  const char* message;
} ptz_absolute_move_callback_data_t;

/**
 * @brief PTZ absolute move response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to ptz_absolute_move_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int ptz_absolute_move_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for PTZ presets response
 */
typedef struct {
  const struct ptz_preset* presets;
  int count;
} ptz_presets_callback_data_t;

/**
 * @brief PTZ presets response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to ptz_presets_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int ptz_presets_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for PTZ set preset response
 */
typedef struct {
  const char* preset_token;
} ptz_set_preset_callback_data_t;

/**
 * @brief PTZ set preset response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to ptz_set_preset_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int ptz_set_preset_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for PTZ goto preset response
 */
typedef struct {
  const char* message;
} ptz_goto_preset_callback_data_t;

/**
 * @brief PTZ goto preset response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to ptz_goto_preset_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int ptz_goto_preset_response_callback(struct soap* soap, void* user_data);

/* ============================================================================
 * Media Service Callback Data Structures and Functions
 * ============================================================================
 */

/**
 * @brief Callback data structure for media profiles response
 */
typedef struct {
  const struct media_profile* profiles;
  int profile_count;
} media_profiles_callback_data_t;

/**
 * @brief Media profiles response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to media_profiles_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_profiles_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for stream URI response
 */
typedef struct {
  const struct stream_uri* uri;
} media_stream_uri_callback_data_t;

/**
 * @brief Stream URI response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to media_stream_uri_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_stream_uri_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for create profile response
 */
typedef struct {
  const struct media_profile* profile;
} media_create_profile_callback_data_t;

/**
 * @brief Create profile response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to media_create_profile_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_create_profile_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for delete profile response
 */
typedef struct {
  const char* message;
} media_delete_profile_callback_data_t;

/**
 * @brief Delete profile response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to media_delete_profile_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_delete_profile_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for set video source configuration response
 */
typedef struct {
  const char* message;
} media_set_video_source_config_callback_data_t;

/**
 * @brief Set video source configuration response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to media_set_video_source_config_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_set_video_source_config_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for set video encoder configuration response
 */
typedef struct {
  const char* message;
} media_set_video_encoder_config_callback_data_t;

/**
 * @brief Set video encoder configuration response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to media_set_video_encoder_config_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_set_video_encoder_config_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for start multicast streaming response
 */
typedef struct {
  const char* message;
} media_start_multicast_callback_data_t;

/**
 * @brief Start multicast streaming response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to media_start_multicast_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_start_multicast_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for stop multicast streaming response
 */
typedef struct {
  const char* message;
} media_stop_multicast_callback_data_t;

/**
 * @brief Stop multicast streaming response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to media_stop_multicast_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_stop_multicast_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for get metadata configurations response
 */
typedef struct {
  const struct metadata_configuration* configs;
  int config_count;
} media_get_metadata_configs_callback_data_t;

/**
 * @brief Get metadata configurations response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to media_get_metadata_configs_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_get_metadata_configs_response_callback(struct soap* soap, void* user_data);

/**
 * @brief Callback data structure for set metadata configuration response
 */
typedef struct {
  const char* message;
} media_set_metadata_config_callback_data_t;

/**
 * @brief Set metadata configuration response callback function
 * @param soap gSOAP context
 * @param user_data Pointer to media_set_metadata_config_callback_data_t
 * @return 0 on success, negative error code on failure
 */
int media_set_metadata_config_response_callback(struct soap* soap, void* user_data);

/* ============================================================================
 * Utility Functions
 * ============================================================================
 */

/**
 * @brief Get response data pointer
 * @param ctx Pointer to gSOAP context structure
 * @return Pointer to response data, or NULL if not available
 */
const char* onvif_gsoap_get_response_data(const onvif_gsoap_context_t* ctx);

/**
 * @brief Get response data length
 * @param ctx Pointer to gSOAP context structure
 * @return Response data length in bytes
 */
size_t onvif_gsoap_get_response_length(const onvif_gsoap_context_t* ctx);

/**
 * @brief Check if context has error
 * @param ctx Pointer to gSOAP context structure
 * @return true if context has error, false otherwise
 */
bool onvif_gsoap_has_error(const onvif_gsoap_context_t* ctx);

/**
 * @brief Get error message
 * @param ctx Pointer to gSOAP context structure
 * @return Error message string, or NULL if no error
 */
const char* onvif_gsoap_get_error(const onvif_gsoap_context_t* ctx);

/**
 * @brief Validate response completeness
 * @param ctx Pointer to gSOAP context structure
 * @return 0 if valid, negative error code if invalid
 */
int onvif_gsoap_validate_response(const onvif_gsoap_context_t* ctx);

/* ============================================================================
 * Error Handling and Fault Generation
 * ============================================================================
 */

/**
 * @brief Generate GetProfiles response using gSOAP serialization
 * @param ctx Pointer to gSOAP context structure
 * @param profiles Array of media profiles
 * @param profile_count Number of profiles
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_profiles_response(onvif_gsoap_context_t* ctx,
                                           const struct media_profile* profiles, int profile_count);

/**
 * @brief Generate GetStreamUri response using gSOAP serialization
 * @param ctx Pointer to gSOAP context structure
 * @param uri Stream URI information
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_stream_uri_response(onvif_gsoap_context_t* ctx,
                                             const struct stream_uri* uri);

/**
 * @brief Generate CreateProfile response using gSOAP serialization
 * @param ctx Pointer to gSOAP context structure
 * @param profile Created profile information
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_create_profile_response(onvif_gsoap_context_t* ctx,
                                                 const struct media_profile* profile);

/**
 * @brief Generate DeleteProfile response using gSOAP serialization
 * @param ctx Pointer to gSOAP context structure
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_delete_profile_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generate SetVideoSourceConfiguration response using gSOAP
 * serialization
 * @param ctx Pointer to gSOAP context structure
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_set_video_source_configuration_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generate SetVideoEncoderConfiguration response using gSOAP
 * serialization
 * @param ctx Pointer to gSOAP context structure
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_set_video_encoder_configuration_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generate StartMulticastStreaming response using gSOAP
 * serialization
 * @param ctx Pointer to gSOAP context structure
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_start_multicast_streaming_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generate StopMulticastStreaming response using gSOAP
 * serialization
 * @param ctx Pointer to gSOAP context structure
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_stop_multicast_streaming_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generate GetMetadataConfigurations response using gSOAP
 * serialization
 * @param ctx Pointer to gSOAP context structure
 * @param configs Array of metadata configurations
 * @param count Number of configurations
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_get_metadata_configurations_response(
  onvif_gsoap_context_t* ctx, const struct metadata_configuration* configs, int count);

/**
 * @brief Generate SetMetadataConfiguration response using gSOAP
 * serialization
 * @param ctx Pointer to gSOAP context structure
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_set_metadata_configuration_response(onvif_gsoap_context_t* ctx);

/* ============================================================================
 * PTZ Service Response Generation Functions
 * ============================================================================
 */

/**
 * @brief Generate GetNodes response using gSOAP serialization
 * @param ctx Pointer to gSOAP context structure
 * @param nodes Array of PTZ nodes
 * @param count Number of nodes in the array
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_get_nodes_response(onvif_gsoap_context_t* ctx,
                                            const struct ptz_node* nodes, int count);

/**
 * @brief Generate AbsoluteMove response using gSOAP serialization
 * @param ctx Pointer to gSOAP context structure
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_absolute_move_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generate GetPresets response using gSOAP serialization
 * @param ctx Pointer to gSOAP context structure
 * @param presets Array of PTZ presets
 * @param count Number of presets in the array
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_get_presets_response(onvif_gsoap_context_t* ctx,
                                              const struct ptz_preset* presets, int count);

/**
 * @brief Generate SetPreset response using gSOAP serialization
 * @param ctx Pointer to gSOAP context structure
 * @param preset_token The token of the created preset
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_set_preset_response(onvif_gsoap_context_t* ctx, const char* preset_token);

/**
 * @brief Generate GotoPreset response using gSOAP serialization
 * @param ctx Pointer to gSOAP context structure
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_goto_preset_response(onvif_gsoap_context_t* ctx);

/* ============================================================================
 * Imaging Service Response Generation Functions
 * ============================================================================
 */

/* ============================================================================
 * Request Parsing Functions
 * ============================================================================
 */

/**
 * @brief Initialize gSOAP context with request data for parsing
 * @param ctx Pointer to gSOAP context structure
 * @param request_data SOAP request data to parse
 * @param request_size Size of the request data
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_init_request_parsing(onvif_gsoap_context_t* ctx, const char* request_data,
                                     size_t request_size);

/**
 * @brief Parse profile token from SOAP request using gSOAP
 * @param ctx Pointer to gSOAP context structure
 * @param token Buffer to store the extracted token
 * @param token_size Size of the token buffer
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_parse_profile_token(onvif_gsoap_context_t* ctx, char* token, size_t token_size);

/**
 * @brief Parse configuration token from SOAP request using gSOAP
 * @param ctx Pointer to gSOAP context structure
 * @param token Buffer to store the extracted token
 * @param token_size Size of the token buffer
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_parse_configuration_token(onvif_gsoap_context_t* ctx, char* token,
                                          size_t token_size);

/**
 * @brief Parse protocol from SOAP request using gSOAP
 * @param ctx Pointer to gSOAP context structure
 * @param protocol Buffer to store the extracted protocol
 * @param protocol_size Size of the protocol buffer
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_parse_protocol(onvif_gsoap_context_t* ctx, char* protocol, size_t protocol_size);

/**
 * @brief Parse value from SOAP request using gSOAP XPath
 * @param ctx Pointer to gSOAP context structure
 * @param xpath XPath expression to find the value
 * @param value Buffer to store the extracted value
 * @param value_size Size of the value buffer
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_parse_value(onvif_gsoap_context_t* ctx, const char* xpath, char* value,
                            size_t value_size);

/**
 * @brief Parse boolean value from SOAP request using gSOAP XPath
 * @param ctx Pointer to gSOAP context structure
 * @param xpath XPath expression to find the value
 * @param value Pointer to store the extracted boolean value
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_parse_boolean(onvif_gsoap_context_t* ctx, const char* xpath, int* value);

/**
 * @brief Parse integer value from SOAP request using gSOAP XPath
 * @param ctx Pointer to gSOAP context structure
 * @param xpath XPath expression to find the value
 * @param value Pointer to store the extracted integer value
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_parse_integer(onvif_gsoap_context_t* ctx, const char* xpath, int* value);

/**
 * @brief Extract ONVIF operation name from SOAP request using gSOAP parsing
 * @param request_data Raw SOAP request data
 * @param request_size Size of the request data
 * @param operation_name Buffer to store the extracted operation name
 * @param operation_name_size Size of the operation name buffer
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_extract_operation_name(const char* request_data, size_t request_size,
                                       char* operation_name, size_t operation_name_size);

/* ============================================================================
 * Error Handling and Fault Generation
 * ============================================================================
 */

/**
 * @brief Generate SOAP fault response
 * @param ctx Pointer to gSOAP context structure
 * @param fault_code Fault code
 * @param fault_string Fault string message
 * @return 0 on success, negative error code on failure
 */
int onvif_gsoap_generate_fault_response(onvif_gsoap_context_t* ctx, int fault_code,
                                        const char* fault_string);

#endif /* ONVIF_GSOAP_H */
