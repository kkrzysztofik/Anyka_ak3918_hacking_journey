/**
 * @file onvif_gsoap_ptz.h
 * @brief PTZ service SOAP request parsing and response generation using gSOAP
 * @author kkrzysztofik
 * @date 2025
 *
 * This module provides PTZ service request parsing and response generation
 * functions that use gSOAP's generated serialization/deserialization functions
 * for proper ONVIF compliance. PTZ (Pan-Tilt-Zoom) operations include position
 * control, speed settings, and preset management.
 */

#ifndef ONVIF_GSOAP_PTZ_H
#define ONVIF_GSOAP_PTZ_H

#include "generated/soapH.h"    //NOLINT
#include "generated/soapStub.h" //NOLINT
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_response.h"
#include "services/ptz/onvif_ptz.h"

/**
 * @brief Parse GetNodes ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetNodes structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Retrieves PTZ node information including capabilities and supported features
 * @note Output structure is allocated with soap_new__onvif3__GetNodes()
 */
int onvif_gsoap_parse_get_nodes(onvif_gsoap_context_t* ctx, struct _onvif3__GetNodes** out);

/**
 * @brief Parse AbsoluteMove ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed AbsoluteMove structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken, Position (PanTilt and Zoom coordinates), and optional Speed fields
 * @note Position coordinates include x, y, and zoom values in PTZ coordinate space
 * @note Output structure is allocated with soap_new__onvif3__AbsoluteMove()
 */
int onvif_gsoap_parse_absolute_move(onvif_gsoap_context_t* ctx, struct _onvif3__AbsoluteMove** out);

/**
 * @brief Parse GetPresets ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GetPresets structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken to retrieve all configured presets for the profile
 * @note Output structure is allocated with soap_new__onvif3__GetPresets()
 */
int onvif_gsoap_parse_get_presets(onvif_gsoap_context_t* ctx, struct _onvif3__GetPresets** out);

/**
 * @brief Parse SetPreset ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed SetPreset structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken and optional PresetToken/PresetName for preset creation/update
 * @note If PresetToken is NULL, creates a new preset; otherwise updates existing preset
 * @note Output structure is allocated with soap_new__onvif3__SetPreset()
 */
int onvif_gsoap_parse_set_preset(onvif_gsoap_context_t* ctx, struct _onvif3__SetPreset** out);

/**
 * @brief Parse GotoPreset ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed GotoPreset structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken, PresetToken, and optional Speed for preset recall
 * @note PresetToken identifies which preset position to move to
 * @note Output structure is allocated with soap_new__onvif3__GotoPreset()
 */
int onvif_gsoap_parse_goto_preset(onvif_gsoap_context_t* ctx, struct _onvif3__GotoPreset** out);

/**
 * @brief Parse RemovePreset ONVIF PTZ service request
 * @param ctx gSOAP context with initialized request parsing
 * @param out Output pointer to receive parsed RemovePreset structure
 * @return ONVIF_SUCCESS on success, error code otherwise
 * @note Extracts ProfileToken and PresetToken for preset deletion
 * @note PresetToken identifies which preset to remove from the profile
 * @note Output structure is allocated with soap_new__onvif3__RemovePreset()
 */
int onvif_gsoap_parse_remove_preset(onvif_gsoap_context_t* ctx, struct _onvif3__RemovePreset** out);

/* ============================================================================
 * PTZ Service Response Generation Functions
 * ============================================================================
 */

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

/**
 * @brief Generate PTZ GetNodes response
 * @param ctx gSOAP context
 * @param nodes Array of PTZ nodes
 * @param count Number of nodes in array
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_get_nodes_response(onvif_gsoap_context_t* ctx,
                                            const struct ptz_node* nodes, int count);

/**
 * @brief Generate PTZ AbsoluteMove response
 * @param ctx gSOAP context
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_absolute_move_response(onvif_gsoap_context_t* ctx);

/**
 * @brief Generate PTZ GetPresets response
 * @param ctx gSOAP context
 * @param presets Array of PTZ presets
 * @param count Number of presets in array
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_get_presets_response(onvif_gsoap_context_t* ctx,
                                              const struct ptz_preset* presets, int count);

/**
 * @brief Generate PTZ SetPreset response
 * @param ctx gSOAP context
 * @param preset_token Token of the created/updated preset
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_set_preset_response(onvif_gsoap_context_t* ctx, const char* preset_token);

/**
 * @brief Generate PTZ GotoPreset response
 * @param ctx gSOAP context
 * @return 0 on success, error code on failure
 */
int onvif_gsoap_generate_goto_preset_response(onvif_gsoap_context_t* ctx);

#endif /* ONVIF_GSOAP_PTZ_H */
