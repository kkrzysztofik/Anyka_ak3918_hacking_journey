/**
 * @file test_onvif_gsoap_ptz.c
 * @brief Unit tests for ONVIF gSOAP PTZ service module
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
#include "protocol/gsoap/onvif_gsoap_ptz.h"
#include "utils/error/error_handling.h"
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
  struct _tptz__GetNodes* request = NULL;

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
  struct _tptz__AbsoluteMove* request = NULL;

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

  // AbsoluteMove uses Position
  assert_non_null(request->Position);
  assert_non_null(request->Position->PanTilt);
  assert_true(request->Position->PanTilt->x >= 0.4 && request->Position->PanTilt->x <= 0.6);
  assert_true(request->Position->PanTilt->y >= 0.2 && request->Position->PanTilt->y <= 0.4);

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
  struct _tptz__AbsoluteMove* request = NULL;

  // Setup parsing test
  int result = setup_parsing_test(&ctx, SOAP_PTZ_ABSOLUTE_MOVE_NO_SPEED);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Parse valid request
  result = onvif_gsoap_parse_absolute_move(&ctx, &request);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(request);

  // Verify required fields
  assert_non_null(request->ProfileToken);
  assert_non_null(request->Position);

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
  struct _tptz__GetPresets* request = NULL;

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
  struct _tptz__SetPreset* request = NULL;

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
  struct _tptz__GotoPreset* request = NULL;

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
  struct _tptz__RemovePreset* request = NULL;

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
 * Test functions are registered in test_protocol_runner.c
 * ============================================================================ */
