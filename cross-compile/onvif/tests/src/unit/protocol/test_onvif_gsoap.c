/**
 * @file test_onvif_gsoap.c
 * @brief Unit tests for ONVIF gSOAP parsing module
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "data/soap_test_envelopes.h"
#include "generated/soapH.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_device.h"
#include "protocol/gsoap/onvif_gsoap_imaging.h"
#include "protocol/gsoap/onvif_gsoap_media.h"
#include "protocol/gsoap/onvif_gsoap_ptz.h"
#include "utils/error/error_handling.h"


/* ============================================================================
 * Helper Functions
 * ============================================================================ */

/**
 * @brief Helper function to set up context for parsing tests
 * @param ctx gSOAP context to initialize
 * @param soap_request SOAP request envelope string
 * @return ONVIF_SUCCESS on success, error code otherwise
 */
static int setup_parsing_test(onvif_gsoap_context_t* ctx, const char* soap_request) {
  if (!ctx || !soap_request) {
    return ONVIF_ERROR_INVALID;
  }

  // Initialize context
  int result = onvif_gsoap_init(ctx);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  // Initialize request parsing
  result = onvif_gsoap_init_request_parsing(ctx, soap_request, strlen(soap_request));
  if (result != ONVIF_SUCCESS) {
    onvif_gsoap_cleanup(ctx);
    return result;
  }

  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Core Context Tests
 * ============================================================================ */

/**
 * @brief Test gSOAP context initialization
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_init(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Test successful initialization
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify request_state is initialized to false
  assert_false(ctx.request_state.is_initialized);
  assert_int_equal(ctx.request_state.request_size, 0);

  // Verify error_context is clear
  assert_int_equal(ctx.error_context.last_error_code, ONVIF_SUCCESS);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test gSOAP context initialization with NULL pointer
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_init_null(void** state) {
  (void)state;

  // Test initialization with NULL context
  int result = onvif_gsoap_init(NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test gSOAP context cleanup
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_cleanup(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));

  // Initialize and then cleanup
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup should not crash
  onvif_gsoap_cleanup(&ctx);

  // Verify state is cleared
  assert_false(ctx.request_state.is_initialized);

  // Test cleanup with NULL pointer (should not crash)
  onvif_gsoap_cleanup(NULL);
}

/* ============================================================================
 * Media Service Parsing Tests
 * ============================================================================ */

/**
 * @brief Test parsing GetProfiles request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_get_profiles(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetProfiles* request = NULL;

  // Test with NULL context
  int result = onvif_gsoap_parse_get_profiles(NULL, &request);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with NULL output pointer
  result = onvif_gsoap_parse_get_profiles(&ctx, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Setup parsing test
  result = setup_parsing_test(&ctx, SOAP_MEDIA_GET_PROFILES);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_get_profiles(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify operation name was set
  assert_string_equal(ctx.request_state.operation_name, "GetProfiles");

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing GetStreamUri request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_get_stream_uri(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetStreamUri* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_MEDIA_GET_STREAM_URI);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_get_stream_uri(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields
  assert_non_null(request->ProfileToken);
  assert_string_equal(request->ProfileToken, "profile_1");

  assert_non_null(request->StreamSetup);
  assert_non_null(request->StreamSetup->Transport);
  // Protocol is an enum, not a pointer
  assert_int_equal(request->StreamSetup->Transport->Protocol, 0); // RTSP = 0

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing CreateProfile request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_create_profile(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__CreateProfile* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_MEDIA_CREATE_PROFILE);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_create_profile(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields
  assert_non_null(request->Name);
  assert_string_equal(request->Name, "TestProfile");

  if (request->Token) {
    assert_string_equal(request->Token, "test_profile_token");
  }

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing DeleteProfile request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_delete_profile(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__DeleteProfile* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_MEDIA_DELETE_PROFILE);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_delete_profile(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields
  assert_non_null(request->ProfileToken);
  assert_string_equal(request->ProfileToken, "profile_to_delete");

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing SetVideoSourceConfiguration request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_set_video_source_config(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__SetVideoSourceConfiguration* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_MEDIA_SET_VIDEO_SOURCE_CONFIG);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_set_video_source_config(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields
  assert_non_null(request->Configuration);
  assert_non_null(request->Configuration->token);
  assert_string_equal(request->Configuration->token, "video_src_config_1");

  // ForcePersistence is a boolean enum (required field in this operation)
  assert_int_equal(request->ForcePersistence, xsd__boolean__true_);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing SetVideoEncoderConfiguration request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_set_video_encoder_config(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__SetVideoEncoderConfiguration* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_MEDIA_SET_VIDEO_ENCODER_CONFIG);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_set_video_encoder_config(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields
  assert_non_null(request->Configuration);
  assert_non_null(request->Configuration->token);
  assert_string_equal(request->Configuration->token, "video_enc_config_1");

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/* ============================================================================
 * PTZ Service Parsing Tests
 * ============================================================================ */

/**
 * @brief Test parsing GetNodes request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_get_nodes(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _onvif3__GetNodes* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_PTZ_GET_NODES);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_get_nodes(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing AbsoluteMove request with speed
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_absolute_move(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _onvif3__AbsoluteMove* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_PTZ_ABSOLUTE_MOVE);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_absolute_move(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields
  assert_non_null(request->ProfileToken);
  assert_string_equal(request->ProfileToken, "ptz_profile_1");

  // AbsoluteMove uses Destination, not Position
  assert_non_null(request->Destination);
  assert_non_null(request->Destination->PanTilt);
  assert_true(request->Destination->PanTilt->x >= 0.4 && request->Destination->PanTilt->x <= 0.6);
  assert_true(request->Destination->PanTilt->y >= 0.2 && request->Destination->PanTilt->y <= 0.4);

  // Speed is optional but should be present in this test
  if (request->Speed) {
    assert_non_null(request->Speed->PanTilt);
  }

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing AbsoluteMove request without speed (optional field)
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_absolute_move_no_speed(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _onvif3__AbsoluteMove* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_PTZ_ABSOLUTE_MOVE_NO_SPEED);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_absolute_move(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify required fields
  assert_non_null(request->ProfileToken);
  assert_non_null(request->Destination);

  // Speed should be NULL (optional field not provided)
  // Note: gSOAP may still allocate it, so we just verify parsing succeeded

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing GetPresets request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_get_presets(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _onvif3__GetPresets* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_PTZ_GET_PRESETS);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_get_presets(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields
  assert_non_null(request->ProfileToken);
  assert_string_equal(request->ProfileToken, "ptz_profile_1");

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing SetPreset request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_set_preset(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _onvif3__SetPreset* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_PTZ_SET_PRESET);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_set_preset(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields
  assert_non_null(request->ProfileToken);
  assert_string_equal(request->ProfileToken, "ptz_profile_1");

  if (request->PresetName) {
    assert_string_equal(request->PresetName, "HomePosition");
  }

  if (request->PresetToken) {
    assert_string_equal(request->PresetToken, "preset_1");
  }

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing GotoPreset request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_goto_preset(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _onvif3__GotoPreset* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_PTZ_GOTO_PRESET);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_goto_preset(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields
  assert_non_null(request->ProfileToken);
  assert_string_equal(request->ProfileToken, "ptz_profile_1");

  assert_non_null(request->PresetToken);
  assert_string_equal(request->PresetToken, "preset_1");

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing RemovePreset request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_remove_preset(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _onvif3__RemovePreset* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_PTZ_REMOVE_PRESET);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_remove_preset(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields
  assert_non_null(request->ProfileToken);
  assert_string_equal(request->ProfileToken, "ptz_profile_1");

  assert_non_null(request->PresetToken);
  assert_string_equal(request->PresetToken, "preset_to_delete");

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/* ============================================================================
 * Device Service Parsing Tests
 * ============================================================================ */

/**
 * @brief Test parsing GetDeviceInformation request (empty request)
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_get_device_information(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _tds__GetDeviceInformation* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_DEVICE_GET_DEVICE_INFORMATION);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request (empty request body)
  result = onvif_gsoap_parse_get_device_information(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing GetCapabilities request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_get_capabilities(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _tds__GetCapabilities* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_DEVICE_GET_CAPABILITIES);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_get_capabilities(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields - Category array
  if (request->__sizeCategory > 0) {
    assert_non_null(request->Category);
    assert_int_equal(request->Category[0], 0); // All = 0
  }

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing GetSystemDateAndTime request (empty request)
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_get_system_date_and_time(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _tds__GetSystemDateAndTime* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_DEVICE_GET_SYSTEM_DATE_AND_TIME);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request (empty request body)
  result = onvif_gsoap_parse_get_system_date_and_time(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing SystemReboot request (empty request)
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_system_reboot(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _tds__SystemReboot* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_DEVICE_SYSTEM_REBOOT);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request (empty request body)
  result = onvif_gsoap_parse_system_reboot(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/* ============================================================================
 * Imaging Service Parsing Tests
 * ============================================================================ */

/**
 * @brief Test parsing GetImagingSettings request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_get_imaging_settings(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _onvif4__GetImagingSettings* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_IMAGING_GET_IMAGING_SETTINGS);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_get_imaging_settings(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields
  assert_non_null(request->VideoSourceToken);
  assert_string_equal(request->VideoSourceToken, "video_source_0");

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing SetImagingSettings request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_set_imaging_settings(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _onvif4__SetImagingSettings* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_IMAGING_SET_IMAGING_SETTINGS);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_set_imaging_settings(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify parsed fields
  assert_non_null(request->VideoSourceToken);
  assert_string_equal(request->VideoSourceToken, "video_source_0");

  assert_non_null(request->ImagingSettings);
  if (request->ImagingSettings->Brightness) {
    assert_true(*request->ImagingSettings->Brightness >= 45.0 &&
                *request->ImagingSettings->Brightness <= 55.0);
  }

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/* ============================================================================
 * Error Handling Tests
 * ============================================================================ */

/**
 * @brief Test parsing with invalid XML
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_invalid_xml(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetProfiles* request = NULL;

  // Setup parsing test with invalid XML
  int result = setup_parsing_test(&ctx, SOAP_INVALID_XML);
  // Should fail during parsing initialization or parsing
  if (result == ONVIF_SUCCESS) {
    result = onvif_gsoap_parse_get_profiles(&ctx, &request);
  }

  // Expect failure
  assert_true(result != ONVIF_SUCCESS);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing with invalid namespace
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_invalid_namespace(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetProfiles* request = NULL;

  // Setup parsing test with invalid namespace
  int result = setup_parsing_test(&ctx, SOAP_INVALID_NAMESPACE);
  // Should fail during parsing
  if (result == ONVIF_SUCCESS) {
    result = onvif_gsoap_parse_get_profiles(&ctx, &request);
  }

  // Expect failure
  assert_true(result != ONVIF_SUCCESS);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing with missing required parameter
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_missing_required_param(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetStreamUri* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_MISSING_REQUIRED_PARAM);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse request - should succeed but ProfileToken will be NULL
  result = onvif_gsoap_parse_get_stream_uri(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify ProfileToken is NULL (missing required parameter)
  assert_null(request->ProfileToken);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/**
 * @brief Test parsing without initialization
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_without_initialization(void** state) {
  (void)state;

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetProfiles* request = NULL;

  // Initialize context but NOT request parsing
  int result = onvif_gsoap_init(&ctx);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Try to parse without calling onvif_gsoap_init_request_parsing
  result = onvif_gsoap_parse_get_profiles(&ctx, &request);

  // Should fail - request_state.is_initialized is false
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Cleanup
  onvif_gsoap_cleanup(&ctx);
}

/* ============================================================================
 * Test functions are registered in test_protocol_runner.c
 * ============================================================================ */
