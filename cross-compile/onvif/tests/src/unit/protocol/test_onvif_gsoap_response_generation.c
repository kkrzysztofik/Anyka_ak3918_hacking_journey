/**
 * @file test_onvif_gsoap_response_generation.c
 * @brief Unit tests for gSOAP response generation functions
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"
#include "common/soap_test_helpers.h"
#include "data/response_test_data.h"
#include "mocks/gsoap_mock.h"
#include "protocol/gsoap/onvif_gsoap_device.h"
#include "protocol/gsoap/onvif_gsoap_media.h"
#include "protocol/gsoap/onvif_gsoap_ptz.h"
#include "protocol/gsoap/onvif_gsoap_response.h"
#include "services/common/onvif_types.h"
#include "services/media/onvif_media.h"
#include "utils/error/error_handling.h"

// ============================================================================
// Test Setup and Teardown
// ============================================================================

/**
 * @brief Setup function for response generation tests
 * @param state Test state
 * @return 0 on success
 */
static int response_generation_setup(void** state) {
  onvif_gsoap_context_t* ctx = calloc(1, sizeof(onvif_gsoap_context_t));
  if (!ctx) {
    return -1;
  }

  // Use real functions instead of mocks
  gsoap_mock_use_real_function(true);

  // Initialize the gSOAP context
  int result = onvif_gsoap_init(ctx);
  if (result != ONVIF_SUCCESS) {
    free(ctx);
    return -1;
  }

  *state = ctx;
  return 0;
}

/**
 * @brief Teardown function for response generation tests
 * @param state Test state
 * @return 0 on success
 */
static int response_generation_teardown(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  if (ctx) {
    // Cleanup the gSOAP context
    onvif_gsoap_cleanup(ctx);
    free(ctx);
  }

  // Re-enable mocks for other tests
  gsoap_mock_use_real_function(false);

  return 0;
}

// ============================================================================
// Device Service Response Generation Tests
// ============================================================================

/**
 * @brief Test successful device info response generation
 */
static void test_unit_onvif_gsoap_generate_device_info_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Test basic context validation
  assert_non_null(ctx);
  assert_int_equal(ctx->error_context.last_error_code, ONVIF_SUCCESS);

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_device_info_response(
    ctx, mock_device_info_valid.manufacturer, mock_device_info_valid.model,
    mock_device_info_valid.firmware_version, mock_device_info_valid.serial_number,
    mock_device_info_valid.hardware_id);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetDeviceInformationResponse* response;
  result = soap_test_parse_get_device_info_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Field-by-field comparison with test data
  assert_string_equal(response->Manufacturer, mock_device_info_valid.manufacturer);
  assert_string_equal(response->Model, mock_device_info_valid.model);
  assert_string_equal(response->FirmwareVersion, mock_device_info_valid.firmware_version);
  assert_string_equal(response->SerialNumber, mock_device_info_valid.serial_number);
  assert_string_equal(response->HardwareId, mock_device_info_valid.hardware_id);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test device info response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_device_info_response_null_context(void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_device_info_response(
    NULL, mock_device_info_valid.manufacturer, mock_device_info_valid.model,
    mock_device_info_valid.firmware_version, mock_device_info_valid.serial_number,
    mock_device_info_valid.hardware_id);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test device info response generation with NULL parameters
 */
static void test_unit_onvif_gsoap_generate_device_info_response_null_params(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;

  // Test with NULL manufacturer - the function should handle NULL gracefully
  int result = onvif_gsoap_generate_device_info_response(
    ctx, NULL, mock_device_info_valid.model, mock_device_info_valid.firmware_version,
    mock_device_info_valid.serial_number, mock_device_info_valid.hardware_id);

  // The function should succeed even with NULL parameters (it converts them to empty strings)
  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test device info response generation with empty parameters
 */
static void test_unit_onvif_gsoap_generate_device_info_response_empty_params(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Test with empty parameters using real functions
  int result = onvif_gsoap_generate_device_info_response(
    ctx, mock_device_info_empty.manufacturer, mock_device_info_empty.model,
    mock_device_info_empty.firmware_version, mock_device_info_empty.serial_number,
    mock_device_info_empty.hardware_id);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetDeviceInformationResponse* response;
  result = soap_test_parse_get_device_info_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Field-by-field comparison - empty strings should be properly serialized/deserialized
  assert_string_equal(response->Manufacturer, mock_device_info_empty.manufacturer);
  assert_string_equal(response->Model, mock_device_info_empty.model);
  assert_string_equal(response->FirmwareVersion, mock_device_info_empty.firmware_version);
  assert_string_equal(response->SerialNumber, mock_device_info_empty.serial_number);
  assert_string_equal(response->HardwareId, mock_device_info_empty.hardware_id);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test successful system reboot response generation
 */
static void test_unit_onvif_gsoap_generate_system_reboot_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];
  const char* test_message = "System will reboot in 5 seconds";

  // Test basic context validation
  assert_non_null(ctx);
  assert_int_equal(ctx->error_context.last_error_code, ONVIF_SUCCESS);

  // Prepare callback data
  system_reboot_callback_data_t callback_data = {.message = test_message};

  // Generate response using callback
  int result = onvif_gsoap_generate_response_with_callback(ctx, system_reboot_response_callback,
                                                           &callback_data);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__SystemRebootResponse* response;
  result = soap_test_parse_system_reboot_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Field-by-field comparison with test data
  assert_non_null(response->Message);
  assert_string_equal(response->Message, test_message);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test system reboot response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_system_reboot_response_null_context(void** state) {
  (void)state; // Unused parameter
  const char* test_message = "System will reboot";

  // Prepare callback data
  system_reboot_callback_data_t callback_data = {.message = test_message};

  // Test with NULL context
  int result = onvif_gsoap_generate_response_with_callback(NULL, system_reboot_response_callback,
                                                           &callback_data);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test system reboot response generation with NULL user_data
 */
static void test_unit_onvif_gsoap_generate_system_reboot_response_null_params(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;

  // Test with NULL message (NULL user_data) - callback should handle gracefully
  system_reboot_callback_data_t callback_data = {.message = NULL};

  int result = onvif_gsoap_generate_response_with_callback(ctx, system_reboot_response_callback,
                                                           &callback_data);

  // The callback should handle NULL message by converting to empty string and succeed
  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test successful GetCapabilities response generation
 */
static void test_unit_onvif_gsoap_generate_capabilities_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[4096];
  const char* test_device_ip = "192.168.1.100";
  const int test_http_port = 80;

  // Test basic context validation
  assert_non_null(ctx);
  assert_int_equal(ctx->error_context.last_error_code, ONVIF_SUCCESS);

  // Test successful response generation using real functions with NULL capabilities (default)
  int result = onvif_gsoap_generate_capabilities_response(ctx, NULL, test_device_ip, test_http_port);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetCapabilitiesResponse* response;
  result = soap_test_parse_get_capabilities_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify capabilities structure exists
  assert_non_null(response->Capabilities);

  // Verify Device capabilities
  assert_non_null(response->Capabilities->Device);
  assert_non_null(response->Capabilities->Device->XAddr);
  assert_string_equal(response->Capabilities->Device->XAddr,
                     "http://192.168.1.100:80/onvif/device_service");

  // Verify Media capabilities
  assert_non_null(response->Capabilities->Media);
  assert_non_null(response->Capabilities->Media->XAddr);
  assert_string_equal(response->Capabilities->Media->XAddr,
                     "http://192.168.1.100:80/onvif/media_service");

  // Verify PTZ capabilities
  assert_non_null(response->Capabilities->PTZ);
  assert_non_null(response->Capabilities->PTZ->XAddr);
  assert_string_equal(response->Capabilities->PTZ->XAddr,
                     "http://192.168.1.100:80/onvif/ptz_service");

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test GetCapabilities response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_capabilities_response_null_context(void** state) {
  (void)state; // Unused parameter
  const char* test_device_ip = "192.168.1.100";
  const int test_http_port = 80;

  // Test with NULL context
  int result = onvif_gsoap_generate_capabilities_response(NULL, NULL, test_device_ip, test_http_port);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test GetCapabilities response generation with NULL parameters
 */
static void test_unit_onvif_gsoap_generate_capabilities_response_null_params(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  const int test_http_port = 80;

  // Test with NULL device_ip - the function should handle NULL gracefully
  int result = onvif_gsoap_generate_capabilities_response(ctx, NULL, NULL, test_http_port);

  // The function should succeed even with NULL device_ip (it converts it to empty string)
  assert_int_equal(result, ONVIF_SUCCESS);
}

/**
 * @brief Test GetCapabilities response generation with real capabilities data
 */
static void test_unit_onvif_gsoap_generate_capabilities_response_with_real_data(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[4096];
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
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tds__GetCapabilitiesResponse* response;
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

// ============================================================================
// Media Service Response Generation Tests
// ============================================================================

/**
 * @brief Test successful profiles response generation
 */
static void test_unit_onvif_gsoap_generate_profiles_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[4096];

  // Create a simple test profile that matches struct media_profile
  struct media_profile test_profile = {0};
  strncpy(test_profile.token, "TestProfile", sizeof(test_profile.token) - 1);
  strncpy(test_profile.name, "Test Profile", sizeof(test_profile.name) - 1);
  test_profile.fixed = 0;

  // Set up video source
  strncpy(test_profile.video_source.source_token, "VideoSource0",
          sizeof(test_profile.video_source.source_token) - 1);
  test_profile.video_source.bounds.width = 1920;
  test_profile.video_source.bounds.height = 1080;
  test_profile.video_source.bounds.x = 0;
  test_profile.video_source.bounds.y = 0;

  // Set up video encoder
  strncpy(test_profile.video_encoder.token, "VideoEncoder0",
          sizeof(test_profile.video_encoder.token) - 1);
  strncpy(test_profile.video_encoder.encoding, "H264",
          sizeof(test_profile.video_encoder.encoding) - 1);
  test_profile.video_encoder.resolution.width = 1920;
  test_profile.video_encoder.resolution.height = 1080;
  test_profile.video_encoder.quality = 0.5f;
  test_profile.video_encoder.framerate_limit = 30;
  test_profile.video_encoder.encoding_interval = 1;
  test_profile.video_encoder.bitrate_limit = 2000000;
  test_profile.video_encoder.gov_length = 30;

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_profiles_response(ctx, &test_profile, 1);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__GetProfilesResponse* response;
  result = soap_test_parse_get_profiles_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Validate we got one profile back
  assert_int_equal(response->__sizeProfiles, 1);
  assert_non_null(response->Profiles);

  // Field-by-field comparison with test profile
  assert_string_equal(response->Profiles[0].token, test_profile.token);
  assert_string_equal(response->Profiles[0].Name, test_profile.name);
  assert_non_null(response->Profiles[0].fixed);
  assert_int_equal(*response->Profiles[0].fixed, test_profile.fixed);

  // Validate video source configuration
  assert_non_null(response->Profiles[0].VideoSourceConfiguration);
  assert_string_equal(response->Profiles[0].VideoSourceConfiguration->SourceToken,
                     test_profile.video_source.source_token);
  assert_int_equal(response->Profiles[0].VideoSourceConfiguration->Bounds->width,
                  test_profile.video_source.bounds.width);
  assert_int_equal(response->Profiles[0].VideoSourceConfiguration->Bounds->height,
                  test_profile.video_source.bounds.height);

  // Validate video encoder configuration
  assert_non_null(response->Profiles[0].VideoEncoderConfiguration);
  // Note: Encoding is an enum in gSOAP, skip string comparison
  assert_int_equal(response->Profiles[0].VideoEncoderConfiguration->Resolution->Width,
                  test_profile.video_encoder.resolution.width);
  assert_int_equal(response->Profiles[0].VideoEncoderConfiguration->Resolution->Height,
                  test_profile.video_encoder.resolution.height);
  assert_float_equal(response->Profiles[0].VideoEncoderConfiguration->Quality,
                    test_profile.video_encoder.quality, 0.01f);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test profiles response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_profiles_response_null_context(void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_profiles_response(NULL, mock_profiles, mock_profile_count);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful stream URI response generation
 */
static void test_unit_onvif_gsoap_generate_stream_uri_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Create a simple test stream URI that matches struct stream_uri
  struct stream_uri test_uri = {0};
  strncpy(test_uri.uri, "rtsp://192.168.1.100:554/stream", sizeof(test_uri.uri) - 1);
  test_uri.timeout = 30;
  test_uri.invalid_after_connect = 0;
  test_uri.invalid_after_reboot = 0;

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_stream_uri_response(ctx, &test_uri);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__GetStreamUriResponse* response;
  result = soap_test_parse_get_stream_uri_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Field-by-field comparison with test URI
  assert_non_null(response->MediaUri);
  assert_string_equal(response->MediaUri->Uri, test_uri.uri);
  assert_int_equal(response->MediaUri->InvalidAfterConnect, test_uri.invalid_after_connect);
  assert_int_equal(response->MediaUri->InvalidAfterReboot, test_uri.invalid_after_reboot);
  // Timeout is an ISO 8601 duration string (e.g., "PT60S"), just verify it's present and valid format
  assert_non_null(response->MediaUri->Timeout);
  assert_non_null(strstr(response->MediaUri->Timeout, "PT"));
  assert_non_null(strstr(response->MediaUri->Timeout, "S"));

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test stream URI response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_stream_uri_response_null_context(void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_stream_uri_response(NULL, &mock_stream_uri_valid);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful create profile response generation
 */
static void test_unit_onvif_gsoap_generate_create_profile_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Create a simple test profile that matches struct media_profile
  struct media_profile test_profile = {0};
  strncpy(test_profile.token, "NewProfile", sizeof(test_profile.token) - 1);
  strncpy(test_profile.name, "New Test Profile", sizeof(test_profile.name) - 1);
  test_profile.fixed = 0;

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_create_profile_response(ctx, &test_profile);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__CreateProfileResponse* response;
  result = soap_test_parse_create_profile_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Field-by-field comparison with test profile
  assert_non_null(response->Profile);
  assert_string_equal(response->Profile->token, test_profile.token);
  assert_string_equal(response->Profile->Name, test_profile.name);
  assert_non_null(response->Profile->fixed);
  assert_int_equal(*response->Profile->fixed, test_profile.fixed);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test create profile response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_create_profile_response_null_context(void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_create_profile_response(NULL, &mock_profiles[0]);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful set video source configuration response generation
 */
static void test_unit_onvif_gsoap_generate_set_video_source_configuration_response_success(
  void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_set_video_source_configuration_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__SetVideoSourceConfigurationResponse* response;
  result = soap_test_parse_set_video_source_config_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test set video source configuration response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_set_video_source_configuration_response_null_context(
  void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_set_video_source_configuration_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful set video encoder configuration response generation
 */
static void test_unit_onvif_gsoap_generate_set_video_encoder_configuration_response_success(
  void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_set_video_encoder_configuration_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__SetVideoEncoderConfigurationResponse* response;
  result = soap_test_parse_set_video_encoder_config_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test set video encoder configuration response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_set_video_encoder_configuration_response_null_context(
  void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_set_video_encoder_configuration_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful start multicast streaming response generation
 */
static void test_unit_onvif_gsoap_generate_start_multicast_streaming_response_success(
  void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_start_multicast_streaming_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__StartMulticastStreamingResponse* response;
  result = soap_test_parse_start_multicast_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test start multicast streaming response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_start_multicast_streaming_response_null_context(
  void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_start_multicast_streaming_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful stop multicast streaming response generation
 */
static void test_unit_onvif_gsoap_generate_stop_multicast_streaming_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_stop_multicast_streaming_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__StopMulticastStreamingResponse* response;
  result = soap_test_parse_stop_multicast_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test stop multicast streaming response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_stop_multicast_streaming_response_null_context(
  void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_stop_multicast_streaming_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful get metadata configurations response generation
 */
static void test_unit_onvif_gsoap_generate_get_metadata_configurations_response_success(
  void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Create a simple test metadata configuration that matches struct metadata_configuration
  struct metadata_configuration test_config = {0};
  strncpy(test_config.token, "MetadataConfig0", sizeof(test_config.token) - 1);
  strncpy(test_config.name, "Metadata Configuration", sizeof(test_config.name) - 1);
  test_config.use_count = 1;
  test_config.session_timeout = 30;
  test_config.analytics = 0;
  strncpy(test_config.multicast.address, "239.255.255.250",
          sizeof(test_config.multicast.address) - 1);
  test_config.multicast.port = 3702;
  test_config.multicast.ttl = 1;
  test_config.multicast.auto_start = 0;

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_get_metadata_configurations_response(ctx, &test_config, 1);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__GetMetadataConfigurationsResponse* response;
  result = soap_test_parse_get_metadata_configs_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Validate we got one configuration back
  assert_int_equal(response->__sizeConfigurations, 1);
  assert_non_null(response->Configurations);

  // Field-by-field comparison with test configuration
  assert_string_equal(response->Configurations[0].token, test_config.token);
  assert_string_equal(response->Configurations[0].Name, test_config.name);
  assert_int_equal(response->Configurations[0].UseCount, test_config.use_count);
  // SessionTimeout is xsd:duration string format (default is "PT60S" for 60 seconds)
  assert_non_null(response->Configurations[0].SessionTimeout);
  assert_string_equal(response->Configurations[0].SessionTimeout, "PT60S");

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test get metadata configurations response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_get_metadata_configurations_response_null_context(
  void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  struct metadata_configuration test_config = {0};
  int result = onvif_gsoap_generate_get_metadata_configurations_response(NULL, &test_config, 1);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful set metadata configuration response generation
 */
static void test_unit_onvif_gsoap_generate_set_metadata_configuration_response_success(
  void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_set_metadata_configuration_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__SetMetadataConfigurationResponse* response;
  result = soap_test_parse_set_metadata_config_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test set metadata configuration response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_set_metadata_configuration_response_null_context(
  void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_set_metadata_configuration_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful delete profile response generation
 */
static void test_unit_onvif_gsoap_generate_delete_profile_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_delete_profile_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _trt__DeleteProfileResponse* response;
  result = soap_test_parse_delete_profile_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test delete profile response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_delete_profile_response_null_context(void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
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
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_absolute_move_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tptz__AbsoluteMoveResponse* response;
  result = soap_test_parse_absolute_move_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test absolute move response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_absolute_move_response_null_context(void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
  int result = onvif_gsoap_generate_absolute_move_response(NULL);

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test successful goto preset response generation
 */
static void test_unit_onvif_gsoap_generate_goto_preset_response_success(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char response_buffer[2048];

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_goto_preset_response(ctx);

  assert_int_equal(result, ONVIF_SUCCESS);

  // Extract serialized response using helper
  int response_size = get_serialized_response(ctx, response_buffer, sizeof(response_buffer));
  assert_true(response_size > 0);
  assert_true(response_size < (int)sizeof(response_buffer));

  // Create http_response_t wrapper for helper function
  http_response_t http_resp = {
    .body = response_buffer,
    .body_length = strlen(response_buffer),
    .status_code = 200
  };

  // Deserialize response back to gSOAP structure for proper validation
  onvif_gsoap_context_t parse_ctx;
  result = soap_test_init_response_parsing(&parse_ctx, &http_resp);
  assert_int_equal(result, ONVIF_SUCCESS);

  struct _tptz__GotoPresetResponse* response;
  result = soap_test_parse_goto_preset_response(&parse_ctx, &response);
  assert_int_equal(result, ONVIF_SUCCESS);

  // This is an empty response (no data fields), just validate it parsed successfully
  assert_non_null(response);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test goto preset response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_goto_preset_response_null_context(void** state) {
  (void)state; // Unused parameter

  // Test with NULL context
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
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char output_buffer[2048];

  // Test successful response generation using real functions
  int result = onvif_gsoap_generate_fault_response(
    ctx, mock_fault_invalid_parameter.fault_code, mock_fault_invalid_parameter.fault_string,
    "test_actor", mock_fault_invalid_parameter.fault_detail, output_buffer, sizeof(output_buffer));

  // API returns byte length on success, not ONVIF_SUCCESS
  assert_true(result > 0);
  assert_true(strlen(output_buffer) > 0);

  // Create http_response_t wrapper for fault parsing
  http_response_t fault_http_resp = {
    .body = output_buffer,
    .body_length = strlen(output_buffer),
    .status_code = 500
  };

  // Parse SOAP Fault using helper function
  onvif_gsoap_context_t parse_ctx;
  int init_result = soap_test_init_response_parsing(&parse_ctx, &fault_http_resp);
  assert_int_equal(init_result, ONVIF_SUCCESS);

  struct SOAP_ENV__Fault* fault;
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

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test fault response generation with NULL context
 */
static void test_unit_onvif_gsoap_generate_fault_response_null_context(void** state) {
  (void)state; // Unused parameter
  char output_buffer[2048];

  // Test with NULL context - should still work with temporary context
  int result = onvif_gsoap_generate_fault_response(
    NULL, mock_fault_invalid_parameter.fault_code, mock_fault_invalid_parameter.fault_string,
    "test_actor", mock_fault_invalid_parameter.fault_detail, output_buffer, sizeof(output_buffer));

  // NULL context creates temporary context, so should succeed
  assert_true(result > 0);
  assert_true(strlen(output_buffer) > 0);

  // Create http_response_t wrapper for fault parsing
  http_response_t fault_http_resp = {
    .body = output_buffer,
    .body_length = strlen(output_buffer),
    .status_code = 500
  };

  // Parse SOAP Fault using helper function
  onvif_gsoap_context_t parse_ctx;
  int init_result = soap_test_init_response_parsing(&parse_ctx, &fault_http_resp);
  assert_int_equal(init_result, ONVIF_SUCCESS);

  struct SOAP_ENV__Fault* fault;
  int parse_result = soap_test_parse_soap_fault(&parse_ctx, &fault);
  assert_int_equal(parse_result, ONVIF_SUCCESS);

  // Validate SOAP 1.2 fault fields
  assert_non_null(fault->SOAP_ENV__Code);
  assert_non_null(fault->SOAP_ENV__Code->SOAP_ENV__Value);
  assert_string_equal(fault->SOAP_ENV__Code->SOAP_ENV__Value, mock_fault_invalid_parameter.fault_code);
  assert_non_null(fault->SOAP_ENV__Reason);
  assert_non_null(fault->SOAP_ENV__Reason->SOAP_ENV__Text);
  assert_string_equal(fault->SOAP_ENV__Reason->SOAP_ENV__Text, mock_fault_invalid_parameter.fault_string);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test fault response generation with NULL fault code
 */
static void test_unit_onvif_gsoap_generate_fault_response_null_fault_code(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char output_buffer[2048];

  // Test with NULL fault code - should use default
  int result = onvif_gsoap_generate_fault_response(
    ctx, NULL, mock_fault_invalid_parameter.fault_string, "test_actor",
    mock_fault_invalid_parameter.fault_detail, output_buffer, sizeof(output_buffer));

  // NULL fault_code uses default, so should succeed
  assert_true(result > 0);
  assert_true(strlen(output_buffer) > 0);

  // Parse SOAP Fault to validate it's well-formed
  http_response_t fault_http_resp = {
    .body = output_buffer,
    .body_length = strlen(output_buffer),
    .status_code = 500
  };

  onvif_gsoap_context_t parse_ctx;
  int init_result = soap_test_init_response_parsing(&parse_ctx, &fault_http_resp);
  assert_int_equal(init_result, ONVIF_SUCCESS);

  struct SOAP_ENV__Fault* fault;
  int parse_result = soap_test_parse_soap_fault(&parse_ctx, &fault);
  assert_int_equal(parse_result, ONVIF_SUCCESS);

  // Validate SOAP 1.2 fault fields
  assert_non_null(fault->SOAP_ENV__Code);
  assert_non_null(fault->SOAP_ENV__Code->SOAP_ENV__Value);
  assert_non_null(fault->SOAP_ENV__Reason);
  assert_non_null(fault->SOAP_ENV__Reason->SOAP_ENV__Text);

  // Cleanup
  onvif_gsoap_cleanup(&parse_ctx);
}

/**
 * @brief Test fault response generation with NULL fault string
 */
static void test_unit_onvif_gsoap_generate_fault_response_null_fault_string(void** state) {
  onvif_gsoap_context_t* ctx = (onvif_gsoap_context_t*)*state;
  char output_buffer[1024];

  // Test with NULL fault string
  int result = onvif_gsoap_generate_fault_response(
    ctx, mock_fault_invalid_parameter.fault_code, NULL, "test_actor",
    mock_fault_invalid_parameter.fault_detail, output_buffer, sizeof(output_buffer));

  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

// ============================================================================
// Test Array Definition
// ============================================================================

const struct CMUnitTest response_generation_tests[] = {
  // Device Service Tests
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_null_context,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_null_params,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_device_info_response_empty_params,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_system_reboot_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_system_reboot_response_null_context, response_generation_setup,
    response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_system_reboot_response_null_params, response_generation_setup,
    response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_capabilities_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_capabilities_response_null_context, response_generation_setup,
    response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_capabilities_response_null_params, response_generation_setup,
    response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_capabilities_response_with_real_data, response_generation_setup,
    response_generation_teardown),

  // Media Service Tests
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_profiles_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_profiles_response_null_context,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_stream_uri_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_stream_uri_response_null_context,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_create_profile_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_create_profile_response_null_context, response_generation_setup,
    response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_set_video_source_configuration_response_success,
    response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_set_video_source_configuration_response_null_context,
    response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_set_video_encoder_configuration_response_success,
    response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_set_video_encoder_configuration_response_null_context,
    response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_start_multicast_streaming_response_success,
    response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_start_multicast_streaming_response_null_context,
    response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_stop_multicast_streaming_response_success,
    response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_stop_multicast_streaming_response_null_context,
    response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_get_metadata_configurations_response_success,
    response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_get_metadata_configurations_response_null_context,
    response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_set_metadata_configuration_response_success,
    response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_set_metadata_configuration_response_null_context,
    response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_delete_profile_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_delete_profile_response_null_context, response_generation_setup,
    response_generation_teardown),

  // PTZ Service Tests
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_absolute_move_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(
    test_unit_onvif_gsoap_generate_absolute_move_response_null_context, response_generation_setup,
    response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_goto_preset_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_goto_preset_response_null_context,
                                  response_generation_setup, response_generation_teardown),

  // Error Response Tests
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_success,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_null_context,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_null_fault_code,
                                  response_generation_setup, response_generation_teardown),
  cmocka_unit_test_setup_teardown(test_unit_onvif_gsoap_generate_fault_response_null_fault_string,
                                  response_generation_setup, response_generation_teardown),
};
