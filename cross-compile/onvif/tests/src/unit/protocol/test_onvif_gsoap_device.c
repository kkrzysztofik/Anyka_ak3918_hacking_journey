/**
 * @file test_onvif_gsoap_device.c
 * @brief Unit tests for ONVIF gSOAP device service module
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
#include "protocol/gsoap/onvif_gsoap_device.h"
#include "utils/error/error_handling.h"
#include "utils/test_gsoap_utils.h"

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
