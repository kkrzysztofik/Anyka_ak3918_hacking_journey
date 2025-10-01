/**
 * @file onvif_gsoap_media.h
 * @brief Media service SOAP request parsing and response generation using gSOAP
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides Media service request parsing and response generation
 * functions that use gSOAP's generated serialization/deserialization functions
 * for proper ONVIF compliance.
 */

#ifndef ONVIF_GSOAP_MEDIA_H
#define ONVIF_GSOAP_MEDIA_H

#include "generated/soapH.h"    //NOLINT
#include "generated/soapStub.h" //NOLINT
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_response.h"
#include "services/media/onvif_media.h"

/**
 * @brief Parse GetProfiles ONVIF Media service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetProfiles structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Caller must check request_state.is_initialized before calling
 * @note Output structure is allocated with soap_new__trt__GetProfiles()
 */
int onvif_gsoap_parse_get_profiles(onvif_gsoap_context_t* ctx,
                                    struct _trt__GetProfiles** out);

/**
 * @brief Parse GetStreamUri ONVIF Media service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetStreamUri structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken and StreamSetup (Protocol, Transport) fields
 * @note Output structure is allocated with soap_new__trt__GetStreamUri()
 */
int onvif_gsoap_parse_get_stream_uri(onvif_gsoap_context_t* ctx,
                                      struct _trt__GetStreamUri** out);

/**
 * @brief Parse CreateProfile ONVIF Media service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed CreateProfile structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts Name and Token fields for profile creation
 * @note Output structure is allocated with soap_new__trt__CreateProfile()
 */
int onvif_gsoap_parse_create_profile(onvif_gsoap_context_t* ctx,
                                      struct _trt__CreateProfile** out);

/**
 * @brief Parse DeleteProfile ONVIF Media service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed DeleteProfile structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken field for profile deletion
 * @note Output structure is allocated with soap_new__trt__DeleteProfile()
 */
int onvif_gsoap_parse_delete_profile(onvif_gsoap_context_t* ctx,
                                      struct _trt__DeleteProfile** out);

/**
 * @brief Parse SetVideoSourceConfiguration ONVIF Media service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SetVideoSourceConfiguration structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts Configuration (Name, Token, Bounds, SourceToken) and ForcePersistence
 * @note Output structure is allocated with soap_new__trt__SetVideoSourceConfiguration()
 */
int onvif_gsoap_parse_set_video_source_config(onvif_gsoap_context_t* ctx,
                                               struct _trt__SetVideoSourceConfiguration** out);

/**
 * @brief Parse SetVideoEncoderConfiguration ONVIF Media service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SetVideoEncoderConfiguration structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts Configuration (Name, Token, Encoding, Resolution, Quality, RateControl) and ForcePersistence
 * @note Output structure is allocated with soap_new__trt__SetVideoEncoderConfiguration()
 */
int onvif_gsoap_parse_set_video_encoder_config(onvif_gsoap_context_t* ctx,
                                                struct _trt__SetVideoEncoderConfiguration** out);

/* ============================================================================
 * Media Service Response Generation Functions
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

/**
 * @brief Generate Media GetProfiles response
 * @param ctx gSOAP context
 * @param profiles Array of media profiles
 * @param profile_count Number of profiles in array
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_profiles_response(onvif_gsoap_context_t* ctx,
                                           const struct media_profile* profiles,
                                           int profile_count);

/**
 * @brief Generate Media GetStreamUri response
 * @param ctx gSOAP context
 * @param uri Stream URI structure
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_stream_uri_response(onvif_gsoap_context_t* ctx,
                                             const struct stream_uri* uri);

/**
 * @brief Generate Media CreateProfile response
 * @param ctx gSOAP context
 * @param profile Created media profile
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_create_profile_response(onvif_gsoap_context_t* ctx,
                                                 const struct media_profile* profile);

/**
 * @brief Generate Media DeleteProfile response
 * @param ctx gSOAP context
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_delete_profile_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generate Media SetVideoSourceConfiguration response
 * @param ctx gSOAP context
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_set_video_source_configuration_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generate Media SetVideoEncoderConfiguration response
 * @param ctx gSOAP context
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_set_video_encoder_configuration_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generate Media StartMulticastStreaming response
 * @param ctx gSOAP context
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_start_multicast_streaming_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generate Media StopMulticastStreaming response
 * @param ctx gSOAP context
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_stop_multicast_streaming_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generate Media GetMetadataConfigurations response
 * @param ctx gSOAP context
 * @param configs Array of metadata configurations
 * @param count Number of configurations in array
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_get_metadata_configurations_response(
  onvif_gsoap_context_t* ctx, const struct metadata_configuration* configs, int count);

/**
 * @brief Generate Media SetMetadataConfiguration response
 * @param ctx gSOAP context
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_set_metadata_configuration_response(onvif_gsoap_context_t* ctx);

#endif /* ONVIF_GSOAP_MEDIA_H */
