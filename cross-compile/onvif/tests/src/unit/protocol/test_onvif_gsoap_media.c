/**
 * @file test_onvif_gsoap_media.c
 * @brief Unit tests for ONVIF gSOAP media service module
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "data/soap_test_envelopes.h"
#include "generated/soapH.h"
#include "mocks/gsoap_mock.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_media.h"
#include "services/common/transport_types.h"
#include "utils/error/error_handling.h"
#include "utils/error/error_translation.h"
#include "utils/test_gsoap_utils.h"

/* ============================================================================
 * Test Suite Setup/Teardown
 * ============================================================================ */

// Setup/teardown functions are defined in test_onvif_gsoap_core.c
// and shared across all gSOAP test modules

/* ============================================================================
 * Helper Functions
 * ============================================================================ */

// setup_parsing_test is now defined in test_gsoap_utils.c and shared across all test files

/* ============================================================================
 * Media Service Parsing Tests
 * ============================================================================ */

/**
 * @brief Test parsing GetProfiles request
 * @param state Test state (unused)
 */
void test_unit_onvif_gsoap_parse_get_profiles(void** state) {
  (void)state;

  printf("DEBUG: Starting test_unit_onvif_gsoap_parse_get_profiles\n");

  onvif_gsoap_context_t ctx;
  memset(&ctx, 0, sizeof(ctx));
  struct _trt__GetProfiles* request = NULL;

  printf("DEBUG: Testing with NULL context\n");
  // Test with NULL context
  int result = onvif_gsoap_parse_get_profiles(NULL, &request);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  printf("DEBUG: Testing with NULL output pointer\n");
  // Test with NULL output pointer
  result = onvif_gsoap_parse_get_profiles(&ctx, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  printf("DEBUG: Setting up parsing test with SOAP_MEDIA_GET_PROFILES\n");
  // Setup parsing test
  result = setup_parsing_test(&ctx, SOAP_MEDIA_GET_PROFILES);
  if (result != ONVIF_SUCCESS) {
    // Debug: Print error details
    int error_code;
    const char* location;
    int soap_error;
    const char* error_msg =
      onvif_gsoap_get_detailed_error(&ctx, &error_code, &location, &soap_error);
    printf("\nDEBUG: setup_parsing_test failed\n");
    printf("  Error code: %d\n", error_code);
    printf("  Location: %s\n", location ? location : "NULL");
    printf("  SOAP error: %d (%s)\n", soap_error, soap_error_to_string(soap_error));
    printf("  Message: %s\n", error_msg ? error_msg : "NULL");
    printf("  SOAP envelope length: %zu\n", strlen(SOAP_MEDIA_GET_PROFILES));
    printf(
      "DEBUG: setup_parsing_test failed - Error code: %d (%s), Location: %s, SOAP error: %d (%s), "
      "Message: %s\n",
      error_code, onvif_error_to_string(error_code), location ? location : "NULL", soap_error,
      soap_error_to_string(soap_error), error_msg ? error_msg : "NULL");
  }
  assert_int_equal(result, ONVIF_SUCCESS);

  printf("DEBUG: setup_parsing_test succeeded, now parsing GetProfiles request\n");
  // Parse valid request
  result = onvif_gsoap_parse_get_profiles(&ctx, &request);
  if (result != ONVIF_SUCCESS) {
    // Debug: Print parsing error details
    int error_code;
    const char* location;
    int soap_error;
    const char* error_msg =
      onvif_gsoap_get_detailed_error(&ctx, &error_code, &location, &soap_error);
    printf("\nDEBUG: onvif_gsoap_parse_get_profiles failed\n");
    printf("  Error code: %d (%s)\n", error_code, onvif_error_to_string(error_code));
    printf("  Location: %s\n", location ? location : "NULL");
    printf("  SOAP error: %d (%s)\n", soap_error, soap_error_to_string(soap_error));
    printf("  Message: %s\n", error_msg ? error_msg : "NULL");
    printf("DEBUG: onvif_gsoap_parse_get_profiles failed - Error code: %d (%s), Location: %s, SOAP "
           "error: %d (%s), Message: %s\n",
           error_code, onvif_error_to_string(error_code), location ? location : "NULL", soap_error,
           soap_error_to_string(soap_error), error_msg ? error_msg : "NULL");
  }
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  printf("DEBUG: GetProfiles parsing succeeded, verifying operation name\n");
  // Verify operation name was set
  assert_string_equal(ctx.request_state.operation_name, "GetProfiles");

  printf("DEBUG: Test completed successfully, cleaning up\n");
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
  assert_int_equal(request->StreamSetup->Transport->Protocol, ONVIF_TRANSPORT_RTSP); // RTSP

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
 * Test functions are registered in test_protocol_runner.c
 * ============================================================================ */
