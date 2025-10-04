/**
 * @file test_onvif_gsoap_imaging.c
 * @brief Unit tests for ONVIF gSOAP imaging service module
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
#include "protocol/gsoap/onvif_gsoap_imaging.h"
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
  struct _timg__GetImagingSettings* request = NULL;

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
  struct _timg__SetImagingSettings* request = NULL;

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
 * Test functions are registered in test_protocol_runner.c
 * ============================================================================ */
