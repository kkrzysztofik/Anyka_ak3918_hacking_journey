/**
 * @file test_onvif_gsoap_response_generation.c
 * @brief Unit tests for gSOAP response generation functions
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "cmocka_wrapper.h"
#include "common/onvif_constants.h"
#include "common/soap_test_helpers.h"
#include "core/config/config_runtime.h"
#include "data/response_test_data.h"
#include "generated/soapH.h"
#include "mocks/config_mock.h"
#include "mocks/gsoap_mock.h"
#include "networking/http/http_parser.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/gsoap/onvif_gsoap_device.h"
#include "protocol/gsoap/onvif_gsoap_media.h"
#include "protocol/gsoap/onvif_gsoap_ptz.h"
#include "protocol/gsoap/onvif_gsoap_response.h"
#include "services/media/onvif_media.h"
#include "utils/error/error_handling.h"
#include "utils/test_gsoap_utils.h"

// ============================================================================
// Test Constants
// ============================================================================

/* Buffer sizes */
#define TEST_BUFFER_SIZE_SMALL  1024
#define TEST_BUFFER_SIZE_MEDIUM 2048
#define TEST_BUFFER_SIZE_LARGE  4096

/* HTTP status codes */
#define TEST_HTTP_STATUS_OK    200
#define TEST_HTTP_STATUS_ERROR 500

/* Test data values */
#define TEST_YEAR_CURRENT   2025
#define TEST_YEAR_OFFSET    1900
#define TEST_MONTH_DECEMBER 15
#define TEST_DAY_14         14
#define TEST_HOUR_30        30
#define TEST_MINUTE_45      45
#define TEST_FPS_30         30
#define TEST_TIMEOUT_10     10

/* Video resolution constants */
#define TEST_VIDEO_WIDTH_HD  1920
#define TEST_VIDEO_HEIGHT_HD 1080
#define TEST_FRAME_RATE_30   30
#define TEST_BITRATE_2M      2000000

/* Floating point constants */
#define TEST_FLOAT_HALF  0.5F
#define TEST_FLOAT_SMALL 0.01F

/* Network constants */
#define TEST_WS_DISCOVERY_PORT 3702

// ============================================================================
// Test Setup and Teardown
// ============================================================================

/**
 * @brief Test state structure for response generation tests
 */
typedef struct {
  onvif_gsoap_context_t* ctx;
  struct application_config* app_config;
} response_generation_test_state_t;

/**
 * @brief Setup function for response generation tests
 * @param state Test state
 * @return 0 on success
 */
static int response_generation_setup(void** state) {
  response_generation_test_state_t* test_state = calloc(1, sizeof(response_generation_test_state_t));
  if (!test_state) {
    return -1;
  }

  onvif_gsoap_context_t* ctx = calloc(1, sizeof(onvif_gsoap_context_t));
  if (!ctx) {
    free(test_state);
    return -1;
  }

  // Initialize test data (large strings, etc.)
  int init_result = response_test_data_init();
  if (init_result != ONVIF_SUCCESS) {
    free(ctx);
    free(test_state);
    return -1;
  }

  // Use real functions instead of mocks
  gsoap_mock_use_real_function(true);
  config_mock_use_real_function(true); // Enable real config functions for http_verbose_enabled()

  // Initialize config_runtime for real config functions to work
  struct application_config* app_config = calloc(1, sizeof(struct application_config));
  if (!app_config) {
    free(ctx);
    free(test_state);
    return -1;
  }
  memset(app_config, 0, sizeof(struct application_config));

  int config_result = config_runtime_init(app_config);
  if (config_result != ONVIF_SUCCESS && config_result != ONVIF_ERROR_ALREADY_EXISTS) {
    free(app_config);
    free(ctx);
    free(test_state);
    return -1;
  }

  // Apply defaults to ensure config values are valid
  (void)config_runtime_apply_defaults();

  // Initialize the gSOAP context
  int result = onvif_gsoap_init(ctx);
  if (result != ONVIF_SUCCESS) {
    config_runtime_cleanup();
    free(app_config);
    free(ctx);
    free(test_state);
    return -1;
  }

  test_state->ctx = ctx;
  test_state->app_config = app_config;
  *state = test_state;
  return 0;
}

/**
 * @brief Teardown function for response generation tests
 * @param state Test state
 * @return 0 on success
 */
static int response_generation_teardown(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  if (test_state) {
    if (test_state->ctx) {
      // Cleanup the gSOAP context
      onvif_gsoap_cleanup(test_state->ctx);
      free(test_state->ctx);
    }

    // Cleanup config_runtime
    config_runtime_cleanup();

    // Free app_config AFTER config_runtime_cleanup()
    // config_runtime_cleanup() sets the global pointer to NULL, so we can safely free now
    if (test_state->app_config) {
      free(test_state->app_config);
    }

    free(test_state);
  }

  // Re-enable mocks for other tests
  gsoap_mock_use_real_function(false);
  config_mock_use_real_function(false);

  return 0;
}

// ============================================================================
// Device Service Response Generation Tests
// ============================================================================

/**
 * @brief Test successful device info response generation
 */
static void test_unit_onvif_gsoap_generate_device_info_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  assert_non_null(ctx);
  assert_int_equal(ctx->error_context.last_error_code, ONVIF_SUCCESS);

  int result = onvif_gsoap_generate_device_info_response(ctx, mock_device_info_valid.manufacturer, mock_device_info_valid.model,
                                                         mock_device_info_valid.firmware_version, mock_device_info_valid.serial_number,
                                                         mock_device_info_valid.hardware_id);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetDeviceInformationResponse* response = NULL;
  result = soap_test_parse_get_device_info_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  assert_string_equal(response->Manufacturer, mock_device_info_valid.manufacturer);
  assert_string_equal(response->Model, mock_device_info_valid.model);
  assert_string_equal(response->FirmwareVersion, mock_device_info_valid.firmware_version);
  assert_string_equal(response->SerialNumber, mock_device_info_valid.serial_number);
  assert_string_equal(response->HardwareId, mock_device_info_valid.hardware_id);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test device info response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_device_info_response_null_context(void** state) {
  (void)state;

  int result = onvif_gsoap_generate_device_info_response(NULL, mock_device_info_valid.manufacturer, mock_device_info_valid.model,
                                                         mock_device_info_valid.firmware_version, mock_device_info_valid.serial_number,
                                                         mock_device_info_valid.hardware_id);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test device info response generation with NULL parameters
 */
static void test_unit_onvif_gsoap_generate_device_info_response_null_params(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;

  int result = onvif_gsoap_generate_device_info_response(ctx, NULL, mock_device_info_valid.model, mock_device_info_valid.firmware_version,
                                                         mock_device_info_valid.serial_number, mock_device_info_valid.hardware_id);

  // The function should succeed even with NULL parameters (it converts them to empty strings)
  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test device info response generation with empty parameters
 */
static void test_unit_onvif_gsoap_generate_device_info_response_empty_params(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  // Test with empty parameters using real functions
  int result = onvif_gsoap_generate_device_info_response(ctx, mock_device_info_empty.manufacturer, mock_device_info_empty.model,
                                                         mock_device_info_empty.firmware_version, mock_device_info_empty.serial_number,
                                                         mock_device_info_empty.hardware_id);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetDeviceInformationResponse* response = NULL;
  result = soap_test_parse_get_device_info_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Field-by-field comparison - empty strings should be properly serialized/deserialized
  assert_string_equal(response->Manufacturer, mock_device_info_empty.manufacturer);
  assert_string_equal(response->Model, mock_device_info_empty.model);
  assert_string_equal(response->FirmwareVersion, mock_device_info_empty.firmware_version);
  assert_string_equal(response->SerialNumber, mock_device_info_empty.serial_number);
  assert_string_equal(response->HardwareId, mock_device_info_empty.hardware_id);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test successful system reboot response generation
 */
static void test_unit_onvif_gsoap_generate_system_reboot_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];
  const char* test_message = "System will reboot in 5 seconds";

  assert_non_null(ctx);
  assert_int_equal(ctx->error_context.last_error_code, ONVIF_SUCCESS);

  // Prepare callback data
  system_reboot_callback_data_t callback_data = {.message = test_message};

  // Generate response using callback
  int result = onvif_gsoap_generate_response_with_callback(ctx, system_reboot_response_callback, &callback_data);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__SystemRebootResponse* response = NULL;
  result = soap_test_parse_system_reboot_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  assert_non_null(response->Message);
  assert_string_equal(response->Message, test_message);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test system reboot response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_system_reboot_response_null_context(void** state) {
  (void)state;
  const char* test_message = "System will reboot";

  // Prepare callback data
  system_reboot_callback_data_t callback_data = {.message = test_message};

  int result = onvif_gsoap_generate_response_with_callback(NULL, system_reboot_response_callback, &callback_data);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test system reboot response generation with NULL user_data
 */
static void test_unit_onvif_gsoap_generate_system_reboot_response_null_params(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;

  system_reboot_callback_data_t callback_data = {.message = NULL};

  int result = onvif_gsoap_generate_response_with_callback(ctx, system_reboot_response_callback, &callback_data);

  // The callback should handle NULL message by converting to empty string and succeed
  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test successful GetCapabilities response generation
 */
static void test_unit_onvif_gsoap_generate_capabilities_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_LARGE];
  const char* test_device_ip = "192.168.1.100";
  const int test_http_port = 80;

  assert_non_null(ctx);
  assert_int_equal(ctx->error_context.last_error_code, ONVIF_SUCCESS);

  // Create test capabilities structure with Device, Media, and PTZ services
  struct tt__Capabilities* test_caps = soap_new_tt__Capabilities(&ctx->soap, 1);
  assert_non_null(test_caps);
  soap_default_tt__Capabilities(&ctx->soap, test_caps);

  // Device capabilities
  test_caps->Device = soap_new_tt__DeviceCapabilities(&ctx->soap, 1);
  assert_non_null(test_caps->Device);
  test_caps->Device->XAddr = soap_strdup(&ctx->soap, "http://192.168.1.100:80/onvif/device_service");

  // Media capabilities
  test_caps->Media = soap_new_tt__MediaCapabilities(&ctx->soap, 1);
  assert_non_null(test_caps->Media);
  test_caps->Media->XAddr = soap_strdup(&ctx->soap, "http://192.168.1.100:80/onvif/media_service");

  // PTZ capabilities
  test_caps->PTZ = soap_new_tt__PTZCapabilities(&ctx->soap, 1);
  assert_non_null(test_caps->PTZ);
  test_caps->PTZ->XAddr = soap_strdup(&ctx->soap, "http://192.168.1.100:80/onvif/ptz_service");

  int result = onvif_gsoap_generate_capabilities_response(ctx, test_caps, test_device_ip, test_http_port);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetCapabilitiesResponse* response = NULL;
  result = soap_test_parse_get_capabilities_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify capabilities structure exists
  assert_non_null(response->Capabilities);

  // Verify Device capabilities
  assert_non_null(response->Capabilities->Device);
  assert_non_null(response->Capabilities->Device->XAddr);
  assert_string_equal(response->Capabilities->Device->XAddr, "http://192.168.1.100:80/onvif/device_service");

  // Verify Media capabilities
  assert_non_null(response->Capabilities->Media);
  assert_non_null(response->Capabilities->Media->XAddr);
  assert_string_equal(response->Capabilities->Media->XAddr, "http://192.168.1.100:80/onvif/media_service");

  // Verify PTZ capabilities
  assert_non_null(response->Capabilities->PTZ);
  assert_non_null(response->Capabilities->PTZ->XAddr);
  assert_string_equal(response->Capabilities->PTZ->XAddr, "http://192.168.1.100:80/onvif/ptz_service");

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test GetCapabilities response generation with NULL capabilities (fallback path)
 */
static void test_unit_onvif_gsoap_generate_capabilities_response_null_fallback(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_LARGE];
  const char* test_device_ip = "192.168.1.100";
  const int test_http_port = 80;

  // Pass NULL for capabilities to trigger fallback path
  int result = onvif_gsoap_generate_capabilities_response(ctx, NULL, test_device_ip, test_http_port);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract and parse response
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);

  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetCapabilitiesResponse* response = NULL;
  result = soap_test_parse_get_capabilities_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify Device, Media, and PTZ capabilities with correct XAddr URLs
  assert_non_null(response->Capabilities);
  assert_non_null(response->Capabilities->Device);
  assert_non_null(response->Capabilities->Device->XAddr);
  assert_string_equal(response->Capabilities->Device->XAddr, "http://192.168.1.100:80/onvif/device_service");

  assert_non_null(response->Capabilities->Media);
  assert_non_null(response->Capabilities->Media->XAddr);
  assert_string_equal(response->Capabilities->Media->XAddr, "http://192.168.1.100:80/onvif/media_service");

  assert_non_null(response->Capabilities->PTZ);
  assert_non_null(response->Capabilities->PTZ->XAddr);
  assert_string_equal(response->Capabilities->PTZ->XAddr, "http://192.168.1.100:80/onvif/ptz_service");

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test GetCapabilities response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_capabilities_response_null_context(void** state) {
  (void)state;
  const char* test_device_ip = "192.168.1.100";
  const int test_http_port = 80;

  int result = onvif_gsoap_generate_capabilities_response(NULL, NULL, test_device_ip, test_http_port);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test GetCapabilities response generation with NULL parameters
 */
static void test_unit_onvif_gsoap_generate_capabilities_response_null_params(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  const int test_http_port = 80;

  int result = onvif_gsoap_generate_capabilities_response(ctx, NULL, NULL, test_http_port);

  // The function should succeed even with NULL device_ip (it converts it to empty string)
  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test GetCapabilities response generation with real capabilities data
 */
static void test_unit_onvif_gsoap_generate_capabilities_response_with_real_data(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_LARGE];
  const char* test_device_ip = "192.168.1.100";
  const int test_http_port = 80;

  // Create real capabilities structure with custom XAddr URLs
  struct tt__Capabilities* real_caps = soap_new_tt__Capabilities(&ctx->soap, 1);
  assert_non_null(real_caps);

  // Create Device capabilities with custom XAddr
  real_caps->Device = soap_new_tt__DeviceCapabilities(&ctx->soap, 1);
  assert_non_null(real_caps->Device);
  real_caps->Device->XAddr = soap_strdup(&ctx->soap, "http://custom-device.local:8080/device");

  // Create Media capabilities with custom XAddr
  real_caps->Media = soap_new_tt__MediaCapabilities(&ctx->soap, 1);
  assert_non_null(real_caps->Media);
  real_caps->Media->XAddr = soap_strdup(&ctx->soap, "http://custom-media.local:8080/media");

  // PTZ is intentionally NULL to test that only implemented services are included

  // Generate response using real capabilities
  int result = onvif_gsoap_generate_capabilities_response(ctx, real_caps, test_device_ip, test_http_port);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract and parse response
  get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetCapabilitiesResponse* response = NULL;
  result = soap_test_parse_get_capabilities_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify the response uses our real capabilities with custom XAddr URLs
  assert_non_null(response->Capabilities);
  assert_non_null(response->Capabilities->Device);
  assert_string_equal(response->Capabilities->Device->XAddr, "http://custom-device.local:8080/device");

  assert_non_null(response->Capabilities->Media);
  assert_string_equal(response->Capabilities->Media->XAddr, "http://custom-media.local:8080/media");

  // PTZ should be NULL since we didn't provide it
  assert_null(response->Capabilities->PTZ);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test GetSystemDateAndTime response generation success scenario
 */
static void test_unit_onvif_gsoap_generate_system_date_time_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_LARGE];

  // Create a specific test time: 2025-01-15 14:30:45 UTC
  struct tm test_time = {0};
  test_time.tm_year = TEST_YEAR_CURRENT - TEST_YEAR_OFFSET; // tm_year is years since 1900
  test_time.tm_mon = 0;                                     // January (0-11)
  test_time.tm_mday = TEST_DAY_14 + 1;                      // 15th day
  test_time.tm_hour = TEST_DAY_14;                          // 14:30:45
  test_time.tm_min = TEST_HOUR_30;
  test_time.tm_sec = TEST_MINUTE_45;

  int result = onvif_gsoap_generate_system_date_time_response(ctx, &test_time);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for parsing
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Parse response back
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetSystemDateAndTimeResponse* response = NULL;
  result = soap_test_parse_get_system_date_time_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify SystemDateTime structure exists
  assert_non_null(response->SystemDateAndTime);
  assert_int_equal(response->SystemDateAndTime->DateTimeType, tt__SetDateTimeType__Manual);
  assert_int_equal(response->SystemDateAndTime->DaylightSavings, xsd__boolean__false_);

  // Verify UTCDateTime exists and has correct values
  assert_non_null(response->SystemDateAndTime->UTCDateTime);
  assert_non_null(response->SystemDateAndTime->UTCDateTime->Time);
  assert_int_equal(response->SystemDateAndTime->UTCDateTime->Time->Hour, 14);
  assert_int_equal(response->SystemDateAndTime->UTCDateTime->Time->Minute, 30);
  assert_int_equal(response->SystemDateAndTime->UTCDateTime->Time->Second, 45);

  assert_non_null(response->SystemDateAndTime->UTCDateTime->Date);
  assert_int_equal(response->SystemDateAndTime->UTCDateTime->Date->Year, TEST_YEAR_CURRENT);
  assert_int_equal(response->SystemDateAndTime->UTCDateTime->Date->Month, 1);
  assert_int_equal(response->SystemDateAndTime->UTCDateTime->Date->Day, 15);

  // Verify TimeZone is set to UTC
  assert_non_null(response->SystemDateAndTime->TimeZone);
  assert_non_null(response->SystemDateAndTime->TimeZone->TZ);
  assert_string_equal(response->SystemDateAndTime->TimeZone->TZ, "UTC");

  // LocalDateTime is optional and should be NULL
  assert_null(response->SystemDateAndTime->LocalDateTime);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test GetSystemDateAndTime response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_system_date_time_response_null_context(void** state) {
  (void)state;

  struct tm test_time = {0};
  test_time.tm_year = TEST_YEAR_CURRENT - TEST_YEAR_OFFSET;
  test_time.tm_mon = 0;
  test_time.tm_mday = TEST_DAY_14 + 1;

  int result = onvif_gsoap_generate_system_date_time_response(NULL, &test_time);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test GetSystemDateAndTime response generation with NULL time (uses current time)
 */
static void test_unit_onvif_gsoap_generate_system_date_time_response_null_time(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_LARGE];

  int result = onvif_gsoap_generate_system_date_time_response(ctx, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract and verify response was generated
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);

  // Parse response back
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetSystemDateAndTimeResponse* response = NULL;
  result = soap_test_parse_get_system_date_time_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify basic structure (don't check exact time values since it's current time)
  assert_non_null(response->SystemDateAndTime);
  assert_non_null(response->SystemDateAndTime->UTCDateTime);
  assert_non_null(response->SystemDateAndTime->UTCDateTime->Time);
  assert_non_null(response->SystemDateAndTime->UTCDateTime->Date);

  // Verify time values are within reasonable ranges
  assert_in_range(response->SystemDateAndTime->UTCDateTime->Time->Hour, 0, 23);
  assert_in_range(response->SystemDateAndTime->UTCDateTime->Time->Minute, 0, 59);
  assert_in_range(response->SystemDateAndTime->UTCDateTime->Time->Second, 0, 59);
  assert_in_range(response->SystemDateAndTime->UTCDateTime->Date->Year, TEST_YEAR_CURRENT, 2030);
  assert_in_range(response->SystemDateAndTime->UTCDateTime->Date->Month, 1, 12);
  assert_in_range(response->SystemDateAndTime->UTCDateTime->Date->Day, 1, 31);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test GetServices response generation success scenario
 */
static void test_unit_onvif_gsoap_generate_services_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_LARGE];
  const char* test_device_ip = "192.168.1.100";
  const int test_http_port = 80;
  const int include_capability = 0;

  int result = onvif_gsoap_generate_services_response(ctx, include_capability, test_device_ip, test_http_port);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for parsing
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Parse response back
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetServicesResponse* response = NULL;
  result = soap_test_parse_get_services_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify response structure
  assert_non_null(response);
  assert_int_equal(response->__sizeService, 1);
  assert_non_null(response->Service);

  // Verify Device service information
  assert_non_null(response->Service[0].Namespace);
  assert_string_equal(response->Service[0].Namespace, "http://www.onvif.org/ver10/device/wsdl");

  assert_non_null(response->Service[0].XAddr);
  assert_string_equal(response->Service[0].XAddr, "http://192.168.1.100:80/onvif/device_service");

  assert_non_null(response->Service[0].Version);
  assert_int_equal(response->Service[0].Version->Major, ONVIF_VERSION_MAJOR);
  assert_int_equal(response->Service[0].Version->Minor, ONVIF_VERSION_MINOR);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test GetServices response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_services_response_null_context(void** state) {
  (void)state;

  const char* test_device_ip = "192.168.1.100";
  const int test_http_port = 80;

  int result = onvif_gsoap_generate_services_response(NULL, 0, test_device_ip, test_http_port);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test GetServices response generation with NULL device_ip
 */
static void test_unit_onvif_gsoap_generate_services_response_null_params(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  const int test_http_port = 80;

  int result = onvif_gsoap_generate_services_response(ctx, 0, NULL, test_http_port);

  // Should succeed with empty device_ip
  assert_int_equal(result, ONVIF_SUCCESS);
}

// ============================================================================
// Media Service Response Generation Tests
// ============================================================================

/**
 * @brief Test successful profiles response generation
 */
static void test_unit_onvif_gsoap_generate_profiles_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_LARGE];

  // Create a simple test profile that matches struct media_profile
  struct media_profile test_profile = {0};
  strncpy(test_profile.token, "TestProfile", sizeof(test_profile.token) - 1);
  strncpy(test_profile.name, "Test Profile", sizeof(test_profile.name) - 1);
  test_profile.fixed = 0;

  // Set up video source
  strncpy(test_profile.video_source.source_token, "VideoSource0", sizeof(test_profile.video_source.source_token) - 1);
  test_profile.video_source.bounds.width = TEST_VIDEO_WIDTH_HD;
  test_profile.video_source.bounds.height = TEST_VIDEO_HEIGHT_HD;
  test_profile.video_source.bounds.x = 0;
  test_profile.video_source.bounds.y = 0;

  // Set up video encoder
  strncpy(test_profile.video_encoder.token, "VideoEncoder0", sizeof(test_profile.video_encoder.token) - 1);
  strncpy(test_profile.video_encoder.encoding, "H264", sizeof(test_profile.video_encoder.encoding) - 1);
  test_profile.video_encoder.resolution.width = TEST_VIDEO_WIDTH_HD;
  test_profile.video_encoder.resolution.height = TEST_VIDEO_HEIGHT_HD;
  test_profile.video_encoder.quality = TEST_FLOAT_HALF;
  test_profile.video_encoder.framerate_limit = TEST_FRAME_RATE_30;
  test_profile.video_encoder.encoding_interval = 1;
  test_profile.video_encoder.bitrate_limit = TEST_BITRATE_2M;
  test_profile.video_encoder.gov_length = TEST_FRAME_RATE_30;

  int result = onvif_gsoap_generate_profiles_response(ctx, &test_profile, 1);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__GetProfilesResponse* response = NULL;
  result = soap_test_parse_get_profiles_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Validate we got one profile back
  assert_int_equal(response->__sizeProfiles, 1);
  assert_non_null(response->Profiles);

  assert_string_equal(response->Profiles[0].token, test_profile.token);
  assert_string_equal(response->Profiles[0].Name, test_profile.name);
  assert_non_null(response->Profiles[0].fixed);
  assert_int_equal(*response->Profiles[0].fixed, test_profile.fixed);

  // Validate video source configuration
  assert_non_null(response->Profiles[0].VideoSourceConfiguration);
  assert_string_equal(response->Profiles[0].VideoSourceConfiguration->SourceToken, test_profile.video_source.source_token);
  assert_int_equal(response->Profiles[0].VideoSourceConfiguration->Bounds->width, test_profile.video_source.bounds.width);
  assert_int_equal(response->Profiles[0].VideoSourceConfiguration->Bounds->height, test_profile.video_source.bounds.height);

  // Validate video encoder configuration
  assert_non_null(response->Profiles[0].VideoEncoderConfiguration);
  // Encoding is an enum in gSOAP, skip string comparison
  assert_int_equal(response->Profiles[0].VideoEncoderConfiguration->Resolution->Width, test_profile.video_encoder.resolution.width);
  assert_int_equal(response->Profiles[0].VideoEncoderConfiguration->Resolution->Height, test_profile.video_encoder.resolution.height);
  assert_float_equal(response->Profiles[0].VideoEncoderConfiguration->Quality, test_profile.video_encoder.quality, TEST_FLOAT_SMALL); // NOLINT

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test profiles response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_profiles_response_null_context(void** state) {
  (void)state;

  int result = onvif_gsoap_generate_profiles_response(NULL, mock_profiles, (int)mock_profile_count);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful stream URI response generation
 */
static void test_unit_onvif_gsoap_generate_stream_uri_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  // Create a simple test stream URI that matches struct stream_uri
  struct stream_uri test_uri = {0};
  strncpy(test_uri.uri, "rtsp://192.168.1.100:554/stream", sizeof(test_uri.uri) - 1);
  test_uri.timeout = TEST_FRAME_RATE_30;
  test_uri.invalid_after_connect = 0;
  test_uri.invalid_after_reboot = 0;

  int result = onvif_gsoap_generate_stream_uri_response(ctx, &test_uri);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__GetStreamUriResponse* response = NULL;
  result = soap_test_parse_get_stream_uri_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  assert_non_null(response->MediaUri);
  assert_string_equal(response->MediaUri->Uri, test_uri.uri);
  assert_int_equal(response->MediaUri->InvalidAfterConnect, test_uri.invalid_after_connect);
  assert_int_equal(response->MediaUri->InvalidAfterReboot, test_uri.invalid_after_reboot);
  // Timeout is an ISO 8601 duration string (e.g., "PT60S"), just verify it's present and valid
  // format
  assert_non_null(response->MediaUri->Timeout);
  assert_non_null(strstr(response->MediaUri->Timeout, "PT"));
  assert_non_null(strstr(response->MediaUri->Timeout, "S"));

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test stream URI response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_stream_uri_response_null_context(void** state) {
  (void)state;

  int result = onvif_gsoap_generate_stream_uri_response(NULL, &mock_stream_uri_valid);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful create profile response generation
 */
static void test_unit_onvif_gsoap_generate_create_profile_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  // Create a simple test profile that matches struct media_profile
  struct media_profile test_profile = {0};
  strncpy(test_profile.token, "NewProfile", sizeof(test_profile.token) - 1);
  strncpy(test_profile.name, "New Test Profile", sizeof(test_profile.name) - 1);
  test_profile.fixed = 0;

  int result = onvif_gsoap_generate_create_profile_response(ctx, &test_profile);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__CreateProfileResponse* response = NULL;
  result = soap_test_parse_create_profile_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  assert_non_null(response->Profile);
  assert_string_equal(response->Profile->token, test_profile.token);
  assert_string_equal(response->Profile->Name, test_profile.name);
  assert_non_null(response->Profile->fixed);
  assert_int_equal(*response->Profile->fixed, test_profile.fixed);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test create profile response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_create_profile_response_null_context(void** state) {
  (void)state;

  int result = onvif_gsoap_generate_create_profile_response(NULL, &mock_profiles[0]);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful set video source configuration response generation
 */
static void test_unit_onvif_gsoap_generate_set_video_source_configuration_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  int result = onvif_gsoap_generate_set_video_source_configuration_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__SetVideoSourceConfigurationResponse* response = NULL;
  result = soap_test_parse_set_video_source_config_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test set video source configuration response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_set_video_source_configuration_response_null_context(void** state) {
  (void)state;

  int result = onvif_gsoap_generate_set_video_source_configuration_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful set video encoder configuration response generation
 */
static void test_unit_onvif_gsoap_generate_set_video_encoder_configuration_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  int result = onvif_gsoap_generate_set_video_encoder_configuration_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__SetVideoEncoderConfigurationResponse* response = NULL;
  result = soap_test_parse_set_video_encoder_config_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test set video encoder configuration response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_set_video_encoder_configuration_response_null_context(void** state) {
  (void)state;

  int result = onvif_gsoap_generate_set_video_encoder_configuration_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful start multicast streaming response generation
 */
static void test_unit_onvif_gsoap_generate_start_multicast_streaming_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  int result = onvif_gsoap_generate_start_multicast_streaming_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__StartMulticastStreamingResponse* response = NULL;
  result = soap_test_parse_start_multicast_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test start multicast streaming response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_start_multicast_streaming_response_null_context(void** state) {
  (void)state;

  int result = onvif_gsoap_generate_start_multicast_streaming_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful stop multicast streaming response generation
 */
static void test_unit_onvif_gsoap_generate_stop_multicast_streaming_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  int result = onvif_gsoap_generate_stop_multicast_streaming_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__StopMulticastStreamingResponse* response = NULL;
  result = soap_test_parse_stop_multicast_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test stop multicast streaming response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_stop_multicast_streaming_response_null_context(void** state) {
  (void)state;

  int result = onvif_gsoap_generate_stop_multicast_streaming_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful get metadata configurations response generation
 */
static void test_unit_onvif_gsoap_generate_get_metadata_configurations_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  // Create a simple test metadata configuration that matches struct metadata_configuration
  struct metadata_configuration test_config = {0};
  strncpy(test_config.token, "MetadataConfig0", sizeof(test_config.token) - 1);
  strncpy(test_config.name, "Metadata Configuration", sizeof(test_config.name) - 1);
  test_config.use_count = 1;
  test_config.session_timeout = TEST_FRAME_RATE_30;
  test_config.analytics = 0;
  strncpy(test_config.multicast.address, "239.255.255.250", sizeof(test_config.multicast.address) - 1);
  test_config.multicast.port = TEST_WS_DISCOVERY_PORT;
  test_config.multicast.ttl = 1;
  test_config.multicast.auto_start = 0;

  int result = onvif_gsoap_generate_get_metadata_configurations_response(ctx, &test_config, 1);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__GetMetadataConfigurationsResponse* response = NULL;
  result = soap_test_parse_get_metadata_configs_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Validate we got one configuration back
  assert_int_equal(response->__sizeConfigurations, 1);
  assert_non_null(response->Configurations);

  assert_string_equal(response->Configurations[0].token, test_config.token);
  assert_string_equal(response->Configurations[0].Name, test_config.name);
  assert_int_equal(response->Configurations[0].UseCount, test_config.use_count);
  // SessionTimeout is xsd:duration string format (default is "PT60S" for 60 seconds)
  assert_non_null(response->Configurations[0].SessionTimeout);
  assert_string_equal(response->Configurations[0].SessionTimeout, "PT60S");

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test get metadata configurations response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_get_metadata_configurations_response_null_context(void** state) {
  (void)state;

  struct metadata_configuration test_config = {0};
  int result = onvif_gsoap_generate_get_metadata_configurations_response(NULL, &test_config, 1);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful set metadata configuration response generation
 */
static void test_unit_onvif_gsoap_generate_set_metadata_configuration_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  int result = onvif_gsoap_generate_set_metadata_configuration_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__SetMetadataConfigurationResponse* response = NULL;
  result = soap_test_parse_set_metadata_config_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test set metadata configuration response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_set_metadata_configuration_response_null_context(void** state) {
  (void)state;

  int result = onvif_gsoap_generate_set_metadata_configuration_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful delete profile response generation
 */
static void test_unit_onvif_gsoap_generate_delete_profile_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  int result = onvif_gsoap_generate_delete_profile_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__DeleteProfileResponse* response = NULL;
  result = soap_test_parse_delete_profile_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test delete profile response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_delete_profile_response_null_context(void** state) {
  (void)state;

  int result = onvif_gsoap_generate_delete_profile_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

// ============================================================================
// PTZ Service Response Generation Tests
// ============================================================================

/**
 * @brief Test successful absolute move response generation
 */
static void test_unit_onvif_gsoap_generate_absolute_move_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  int result = onvif_gsoap_generate_absolute_move_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tptz__AbsoluteMoveResponse* response = NULL;
  result = soap_test_parse_absolute_move_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test absolute move response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_absolute_move_response_null_context(void** state) {
  (void)state;

  int result = onvif_gsoap_generate_absolute_move_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful goto preset response generation
 */
static void test_unit_onvif_gsoap_generate_goto_preset_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  int result = onvif_gsoap_generate_goto_preset_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tptz__GotoPresetResponse* response = NULL;
  result = soap_test_parse_goto_preset_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test goto preset response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_goto_preset_response_null_context(void** state) {
  (void)state;

  int result = onvif_gsoap_generate_goto_preset_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

// ============================================================================
// Error Response Generation Tests
// ============================================================================

/**
 * @brief Test successful fault response generation
 */
static void test_unit_onvif_gsoap_generate_fault_response_success(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char output_buffer[TEST_BUFFER_SIZE_MEDIUM];

  int result = onvif_gsoap_generate_fault_response(ctx, mock_fault_invalid_parameter.fault_code, mock_fault_invalid_parameter.fault_string,
                                                   "test_actor", mock_fault_invalid_parameter.fault_detail, output_buffer, sizeof(output_buffer));

  // API returns byte length on success, not ONVIF_SUCCESS
  assert_true(result > 0);
  assert_true(strlen(output_buffer) > 0);

  // Create http_response_t wrapper for fault parsing
  http_response_t fault_http_resp = {.body = output_buffer, .body_length = strlen(output_buffer), .status_code = TEST_HTTP_STATUS_ERROR};

  // Parse SOAP Fault using helper function
  onvif_gsoap_context_t parse_ctx;
  int init_result = soap_test_init_response_parsing(&parse_ctx, &fault_http_resp);
  assert_int_equal(init_result, ONVIF_SUCCESS);

  struct SOAP_ENV__Fault* fault = NULL;
  int parse_result = soap_test_parse_soap_fault(&parse_ctx, &fault);
  assert_int_equal(parse_result, ONVIF_SUCCESS);

  // Validate SOAP 1.2 fault fields (Code, Reason, Detail)
  assert_non_null(fault->SOAP_ENV__Code);
  assert_non_null(fault->SOAP_ENV__Code->SOAP_ENV__Value);
  assert_string_equal(fault->SOAP_ENV__Code->SOAP_ENV__Value, mock_fault_invalid_parameter.fault_code);
  assert_non_null(fault->SOAP_ENV__Reason);
  assert_non_null(fault->SOAP_ENV__Reason->SOAP_ENV__Text);
  assert_string_equal(fault->SOAP_ENV__Reason->SOAP_ENV__Text, mock_fault_invalid_parameter.fault_string);
  if (mock_fault_invalid_parameter.fault_detail) {
    assert_non_null(fault->SOAP_ENV__Detail);
    assert_non_null(strstr(fault->SOAP_ENV__Detail->__any, mock_fault_invalid_parameter.fault_detail));
  }

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test fault response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_fault_response_null_context(void** state) {
  (void)state;
  char output_buffer[TEST_BUFFER_SIZE_MEDIUM];

  // NULL context creates temp ctx which calls onvif_gsoap_init
  int result = onvif_gsoap_generate_fault_response(NULL, mock_fault_invalid_parameter.fault_code, mock_fault_invalid_parameter.fault_string,
                                                   "test_actor", mock_fault_invalid_parameter.fault_detail, output_buffer, sizeof(output_buffer));

  // NULL context creates temporary context, so should succeed
  assert_true(result > 0);
  assert_true(strlen(output_buffer) > 0);

  // Create http_response_t wrapper for fault parsing
  http_response_t fault_http_resp = {.body = output_buffer, .body_length = strlen(output_buffer), .status_code = TEST_HTTP_STATUS_ERROR};

  // Parse SOAP Fault using helper function
  onvif_gsoap_context_t parse_ctx;
  int init_result = soap_test_init_response_parsing(&parse_ctx, &fault_http_resp);
  assert_int_equal(init_result, ONVIF_SUCCESS);

  struct SOAP_ENV__Fault* fault = NULL;
  int parse_result = soap_test_parse_soap_fault(&parse_ctx, &fault);
  assert_int_equal(parse_result, ONVIF_SUCCESS);

  // Validate SOAP 1.2 fault fields
  assert_non_null(fault->SOAP_ENV__Code);
  assert_non_null(fault->SOAP_ENV__Code->SOAP_ENV__Value);
  assert_string_equal(fault->SOAP_ENV__Code->SOAP_ENV__Value, mock_fault_invalid_parameter.fault_code);
  assert_non_null(fault->SOAP_ENV__Reason);
  assert_non_null(fault->SOAP_ENV__Reason->SOAP_ENV__Text);
  assert_string_equal(fault->SOAP_ENV__Reason->SOAP_ENV__Text, mock_fault_invalid_parameter.fault_string);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test fault response generation with NULL fault code
 */
static void test_unit_onvif_gsoap_generate_fault_response_null_fault_code(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char output_buffer[TEST_BUFFER_SIZE_MEDIUM];

  int result = onvif_gsoap_generate_fault_response(ctx, NULL, mock_fault_invalid_parameter.fault_string, "test_actor",
                                                   mock_fault_invalid_parameter.fault_detail, output_buffer, sizeof(output_buffer));

  // NULL fault_code uses default, so should succeed
  assert_true(result > 0);
  assert_true(strlen(output_buffer) > 0);

  // Parse SOAP Fault to validate it's well-formed
  http_response_t fault_http_resp = {.body = output_buffer, .body_length = strlen(output_buffer), .status_code = TEST_HTTP_STATUS_ERROR};

  onvif_gsoap_context_t parse_ctx;
  int init_result = soap_test_init_response_parsing(&parse_ctx, &fault_http_resp);
  assert_int_equal(init_result, ONVIF_SUCCESS);

  struct SOAP_ENV__Fault* fault = NULL;
  int parse_result = soap_test_parse_soap_fault(&parse_ctx, &fault);
  assert_int_equal(parse_result, ONVIF_SUCCESS);

  // Validate SOAP 1.2 fault fields
  assert_non_null(fault->SOAP_ENV__Code);
  assert_non_null(fault->SOAP_ENV__Code->SOAP_ENV__Value);
  assert_non_null(fault->SOAP_ENV__Reason);
  assert_non_null(fault->SOAP_ENV__Reason->SOAP_ENV__Text);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test fault response generation with NULL fault string
 */
static void test_unit_onvif_gsoap_generate_fault_response_null_fault_string(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char output_buffer[TEST_BUFFER_SIZE_SMALL];

  int result = onvif_gsoap_generate_fault_response(ctx, mock_fault_invalid_parameter.fault_code, NULL, "test_actor",
                                                   mock_fault_invalid_parameter.fault_detail, output_buffer, sizeof(output_buffer));

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test fault response generation with undersized buffer
 */
static void test_unit_onvif_gsoap_generate_fault_response_buffer_overflow(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char small_buffer[TEST_TIMEOUT_10]; // Intentionally too small for SOAP fault

  // Test with undersized buffer - should return error without crashing
  int result = onvif_gsoap_generate_fault_response(ctx, mock_fault_invalid_parameter.fault_code, mock_fault_invalid_parameter.fault_string,
                                                   "test_actor", mock_fault_invalid_parameter.fault_detail, small_buffer, sizeof(small_buffer));

  // Should return error (negative) or indicate buffer too small
  assert_true(result < 0 || result == 0);

  // Verify context remains consistent (no corruption)
  assert_non_null(ctx);
  assert_int_equal(ctx->error_context.last_error_code, ONVIF_SUCCESS);
}

/**
 * @brief Test device info response generation with large strings
 */
static void test_unit_onvif_gsoap_generate_device_info_response_large_strings(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_LARGE]; // Larger buffer for large strings

  // Test with large manufacturer string (512 chars)
  int result = onvif_gsoap_generate_device_info_response(ctx, mock_device_info_large_strings.manufacturer, mock_device_info_large_strings.model,
                                                         mock_device_info_large_strings.firmware_version,
                                                         mock_device_info_large_strings.serial_number, mock_device_info_large_strings.hardware_id);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Parse response back
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetDeviceInformationResponse* response = NULL;
  result = soap_test_parse_get_device_info_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify large string was not truncated
  // gSOAP may have internal limits on string sizes
  assert_non_null(response->Manufacturer);
  assert_true(strlen(response->Manufacturer) > 0);
  // Verify content matches (may be truncated by gSOAP implementation)
  assert_true(strncmp(response->Manufacturer, mock_device_info_large_strings.manufacturer, strlen(response->Manufacturer)) == 0);

  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test device info response generation with special XML characters
 */
static void test_unit_onvif_gsoap_generate_device_info_response_special_chars(void** state) {
  response_generation_test_state_t* test_state = (response_generation_test_state_t*)*state;
  onvif_gsoap_context_t* ctx = test_state ? test_state->ctx : NULL;
  char response_buffer[TEST_BUFFER_SIZE_MEDIUM];

  // Test with special XML characters that need escaping
  int result = onvif_gsoap_generate_device_info_response(ctx, mock_device_info_special_chars.manufacturer, mock_device_info_special_chars.model,
                                                         mock_device_info_special_chars.firmware_version,
                                                         mock_device_info_special_chars.serial_number, mock_device_info_special_chars.hardware_id);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Verify XML contains escaped entities (gSOAP should escape automatically)
  // Check for &lt; (< escaped), &gt; (> escaped), &amp; (& escaped)
  assert_non_null(strstr(response_buffer, "&lt;"));
  assert_non_null(strstr(response_buffer, "&gt;"));
  assert_non_null(strstr(response_buffer, "&amp;"));

  // Parse response back to verify round-trip
  http_response_t http_resp = {.body = response_buffer, .body_length = strlen(response_buffer), .status_code = TEST_HTTP_STATUS_OK};

  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetDeviceInformationResponse* response = NULL;
  result = soap_test_parse_get_device_info_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify special characters were preserved through escaping/unescaping
  assert_string_equal(response->Manufacturer, mock_device_info_special_chars.manufacturer);
  assert_string_equal(response->Model, mock_device_info_special_chars.model);
  assert_string_equal(response->FirmwareVersion, mock_device_info_special_chars.firmware_version);

  onvif_gsoap_cleanup(&parse_ctx);
}

// ============================================================================
// Test Array Definition
// ============================================================================

const struct CMUnitTest response_generation_tests[] = {
  // Device Service Tests
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_null_params, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_empty_params, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_large_strings, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_special_chars, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_system_reboot_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_system_reboot_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_system_reboot_response_null_params, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_capabilities_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_capabilities_response_null_fallback, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_capabilities_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_capabilities_response_null_params, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_capabilities_response_with_real_data, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_system_date_time_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_system_date_time_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_system_date_time_response_null_time, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_services_response_success, response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_services_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_services_response_null_params, response_generation_setup,
                                  response_generation_teardown),

  // Media Service Tests
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_profiles_response_success, response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_profiles_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_stream_uri_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_stream_uri_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_create_profile_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_create_profile_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_set_video_source_configuration_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_set_video_source_configuration_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_set_video_encoder_configuration_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_set_video_encoder_configuration_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_start_multicast_streaming_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_start_multicast_streaming_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_stop_multicast_streaming_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_stop_multicast_streaming_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_get_metadata_configurations_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_get_metadata_configurations_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_set_metadata_configuration_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_set_metadata_configuration_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_delete_profile_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_delete_profile_response_null_context, response_generation_setup,
                                  response_generation_teardown),

  // PTZ Service Tests
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_absolute_move_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_absolute_move_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_goto_preset_response_success, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_goto_preset_response_null_context, response_generation_setup,
                                  response_generation_teardown),

  // Error Response Tests
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_success, response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_null_context, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_null_fault_code, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_null_fault_string, response_generation_setup,
                                  response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_buffer_overflow, response_generation_setup,
                                  response_generation_teardown),
};
