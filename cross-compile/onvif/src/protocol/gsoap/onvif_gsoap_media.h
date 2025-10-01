/**
 * @file onvif_gsoap_media.h
 * @brief Media service SOAP request parsing using gSOAP deserialization
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides Media service request parsing functions that use
 * gSOAP's generated deserialization functions for proper ONVIF compliance.
 */

#ifndef ONVIF_GSOAP_MEDIA_H
#define ONVIF_GSOAP_MEDIA_H

#include "generated/soapH.h"    //NOLINT
#include "generated/soapStub.h" //NOLINT
#include "protocol/gsoap/onvif_gsoap_core.h"

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

#endif /* ONVIF_GSOAP_MEDIA_H */
