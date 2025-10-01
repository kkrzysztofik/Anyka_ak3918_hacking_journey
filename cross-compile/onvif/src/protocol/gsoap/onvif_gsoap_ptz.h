/**
 * @file onvif_gsoap_ptz.h
 * @brief PTZ service SOAP request parsing using gSOAP deserialization
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides PTZ service request parsing functions that use
 * gSOAP's generated deserialization functions for proper ONVIF compliance.
 * PTZ (Pan-Tilt-Zoom) operations include position control, speed settings,
 * and preset management.
 */

#ifndef ONVIF_GSOAP_PTZ_H
#define ONVIF_GSOAP_PTZ_H

#include "generated/soapH.h"    //NOLINT
#include "generated/soapStub.h" //NOLINT
#include "protocol/gsoap/onvif_gsoap_core.h"

/**
 * @brief Parse GetNodes ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetNodes structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Retrieves PTZ node information including capabilities and supported features
 * @note Output structure is allocated with soap_new__onvif3__GetNodes()
 */
int onvif_gsoap_parse_get_nodes(onvif_gsoap_context_t* ctx,
                                 struct _onvif3__GetNodes** out);

/**
 * @brief Parse AbsoluteMove ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed AbsoluteMove structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken, Position (PanTilt and Zoom coordinates), and optional Speed fields
 * @note Position coordinates include x, y, and zoom values in PTZ coordinate space
 * @note Output structure is allocated with soap_new__onvif3__AbsoluteMove()
 */
int onvif_gsoap_parse_absolute_move(onvif_gsoap_context_t* ctx,
                                     struct _onvif3__AbsoluteMove** out);

/**
 * @brief Parse GetPresets ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetPresets structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken to retrieve all configured presets for the profile
 * @note Output structure is allocated with soap_new__onvif3__GetPresets()
 */
int onvif_gsoap_parse_get_presets(onvif_gsoap_context_t* ctx,
                                   struct _onvif3__GetPresets** out);

/**
 * @brief Parse SetPreset ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SetPreset structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken and optional PresetToken/PresetName for preset creation/update
 * @note If PresetToken is NULL, creates a new preset; otherwise updates existing preset
 * @note Output structure is allocated with soap_new__onvif3__SetPreset()
 */
int onvif_gsoap_parse_set_preset(onvif_gsoap_context_t* ctx,
                                  struct _onvif3__SetPreset** out);

/**
 * @brief Parse GotoPreset ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GotoPreset structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken, PresetToken, and optional Speed for preset recall
 * @note PresetToken identifies which preset position to move to
 * @note Output structure is allocated with soap_new__onvif3__GotoPreset()
 */
int onvif_gsoap_parse_goto_preset(onvif_gsoap_context_t* ctx,
                                   struct _onvif3__GotoPreset** out);

/**
 * @brief Parse RemovePreset ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed RemovePreset structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken and PresetToken for preset deletion
 * @note PresetToken identifies which preset to remove from the profile
 * @note Output structure is allocated with soap_new__onvif3__RemovePreset()
 */
int onvif_gsoap_parse_remove_preset(onvif_gsoap_context_t* ctx,
                                     struct _onvif3__RemovePreset** out);

#endif /* ONVIF_GSOAP_PTZ_H */
