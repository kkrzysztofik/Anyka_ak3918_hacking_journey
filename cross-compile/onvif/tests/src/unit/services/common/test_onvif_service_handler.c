/**
 * @file test_onvif_service_handler.c
 * @brief Unit tests for ONVIF service handler initialization and cleanup
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "common/test_helpers.h"
#include "networking/http/http_parser.h"
#include "protocol/gsoap/onvif_gsoap_core.h"
#include "protocol/response/onvif_service_handler.h"
#include "services/common/onvif_types.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"

/* ============================================================================
 * Test Mock Action Handlers
 * ============================================================================ */

static int g_mock_action_call_count = 0;
static int g_mock_action_result = ONVIF_SUCCESS;

/**
 * @brief Mock action handler for testing
 */
static int mock_action_handler(const service_handler_config_t* config,
                                const http_request_t* request, http_response_t* response,
                                onvif_gsoap_context_t* gsoap_ctx) {
  (void)config;
  (void)request;
  (void)response;
  (void)gsoap_ctx;

  g_mock_action_call_count++;
  return g_mock_action_result;
}

/* ============================================================================
 * Test Setup/Teardown
 * ============================================================================ */

static int setup_service_handler_tests(void** state) {
  (void)state;

  // Reset mock state
  g_mock_action_call_count = 0;
  g_mock_action_result = ONVIF_SUCCESS;

  return 0;
}

static int teardown_service_handler_tests(void** state) {
  (void)state;

  // Reset mock state
  g_mock_action_call_count = 0;
  g_mock_action_result = ONVIF_SUCCESS;

  return 0;
}

/* ============================================================================
 * Initialization Tests
 * ============================================================================ */

/**
 * @brief Test successful service handler initialization
 * @param state Test state (unused)
 */
void test_unit_service_handler_init_success(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1},
    {.action_name = "GetCapabilities", .handler = mock_action_handler, .requires_validation = 1}};

  size_t action_count = sizeof(actions) / sizeof(actions[0]);

  // Configure gSOAP mock expectations
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Test initialization
  int result = onvif_service_handler_init(&handler, &config, actions, action_count);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify handler structure was properly initialized
  assert_non_null(handler.actions);
  assert_non_null(handler.gsoap_ctx);
  assert_int_equal(handler.action_count, action_count);
  assert_string_equal(handler.config.service_name, "device");

  // Configure cleanup expectations
  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // Cleanup
  onvif_service_handler_cleanup(&handler);
}

/**
 * @brief Test service handler init with NULL handler parameter
 * @param state Test state (unused)
 */
void test_unit_service_handler_init_null_handler(void** state) {
  (void)state;

  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Test with NULL handler
  int result = onvif_service_handler_init(NULL, &config, actions, 1);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test service handler init with NULL config parameter
 * @param state Test state (unused)
 */
void test_unit_service_handler_init_null_config(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Test with NULL config
  int result = onvif_service_handler_init(&handler, NULL, actions, 1);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test service handler init with NULL actions parameter
 * @param state Test state (unused)
 */
void test_unit_service_handler_init_null_actions(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  // Test with NULL actions
  int result = onvif_service_handler_init(&handler, &config, NULL, 1);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test service handler init with zero action count
 * @param state Test state (unused)
 */
void test_unit_service_handler_init_zero_action_count(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Test with zero action count
  int result = onvif_service_handler_init(&handler, &config, actions, 0);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/* ============================================================================
 * Cleanup Tests
 * ============================================================================ */

/**
 * @brief Test successful service handler cleanup
 * @param state Test state (unused)
 */
void test_unit_service_handler_cleanup_success(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Configure gSOAP mock expectations
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Initialize first
  int result = onvif_service_handler_init(&handler, &config, actions, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Configure cleanup expectations
  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // Cleanup
  onvif_service_handler_cleanup(&handler);

  // Verify cleanup was successful
  assert_null(handler.actions);
  assert_null(handler.gsoap_ctx);
  assert_int_equal(handler.action_count, 0);
}

/**
 * @brief Test service handler cleanup with NULL handler
 * @param state Test state (unused)
 */
void test_unit_service_handler_cleanup_null_handler(void** state) {
  (void)state;

  // Cleanup with NULL should not crash
  onvif_service_handler_cleanup(NULL);
}

/**
 * @brief Test service handler cleanup after init
 * @param state Test state (unused)
 */
void test_unit_service_handler_cleanup_after_init(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1},
    {.action_name = "GetCapabilities", .handler = mock_action_handler, .requires_validation = 1},
    {.action_name = "GetServices", .handler = mock_action_handler, .requires_validation = 0}};

  size_t action_count = sizeof(actions) / sizeof(actions[0]);

  // Configure gSOAP mock expectations
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Initialize
  int result = onvif_service_handler_init(&handler, &config, actions, action_count);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify initialization
  assert_non_null(handler.actions);
  assert_non_null(handler.gsoap_ctx);
  assert_int_equal(handler.action_count, action_count);

  // Configure cleanup expectations
  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // Cleanup
  onvif_service_handler_cleanup(&handler);

  // Verify cleanup
  assert_null(handler.actions);
  assert_null(handler.gsoap_ctx);
  assert_int_equal(handler.action_count, 0);
}

/**
 * @brief Test double cleanup (should be idempotent)
 * @param state Test state (unused)
 */
void test_unit_service_handler_cleanup_double_cleanup(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Configure gSOAP mock expectations
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Initialize
  int result = onvif_service_handler_init(&handler, &config, actions, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Configure cleanup expectations
  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // First cleanup
  onvif_service_handler_cleanup(&handler);
  assert_null(handler.actions);
  assert_null(handler.gsoap_ctx);

  // Second cleanup (should not crash)
  onvif_service_handler_cleanup(&handler);
  assert_null(handler.actions);
  assert_null(handler.gsoap_ctx);
}

/* ============================================================================
 * Resource Allocation Tests
 * ============================================================================ */

/**
 * @brief Test that actions array is properly allocated
 * @param state Test state (unused)
 */
void test_unit_service_handler_init_actions_allocation(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1},
    {.action_name = "GetCapabilities", .handler = mock_action_handler, .requires_validation = 1}};

  size_t action_count = sizeof(actions) / sizeof(actions[0]);

  // Configure gSOAP mock expectations
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Initialize
  int result = onvif_service_handler_init(&handler, &config, actions, action_count);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify actions array is allocated and copied
  assert_non_null(handler.actions);
  assert_int_equal(handler.action_count, action_count);

  // Verify action data was copied correctly
  for (size_t i = 0; i < action_count; i++) {
    assert_string_equal(handler.actions[i].action_name, actions[i].action_name);
    assert_ptr_equal(handler.actions[i].handler, actions[i].handler);
    assert_int_equal(handler.actions[i].requires_validation, actions[i].requires_validation);
  }

  // Configure cleanup expectations
  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // Cleanup
  onvif_service_handler_cleanup(&handler);
}

/**
 * @brief Test that gSOAP context is properly allocated
 * @param state Test state (unused)
 */
void test_unit_service_handler_init_gsoap_allocation(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Configure gSOAP mock expectations
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Initialize
  int result = onvif_service_handler_init(&handler, &config, actions, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify gSOAP context is allocated
  assert_non_null(handler.gsoap_ctx);

  // Configure cleanup expectations
  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // Cleanup
  onvif_service_handler_cleanup(&handler);
}

/**
 * @brief Test full lifecycle (init -> cleanup)
 * @param state Test state (unused)
 */
void test_unit_service_handler_full_lifecycle(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_MEDIA,
                                     .service_name = "media",
                                     .config = NULL,
                                     .enable_validation = 0,
                                     .enable_logging = 0};

  service_action_def_t actions[] = {
    {.action_name = "GetProfiles", .handler = mock_action_handler, .requires_validation = 1},
    {.action_name = "GetStreamUri", .handler = mock_action_handler, .requires_validation = 1},
    {.action_name = "GetVideoSources", .handler = mock_action_handler, .requires_validation = 0}};

  size_t action_count = sizeof(actions) / sizeof(actions[0]);

  // Configure gSOAP mock expectations
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  // Test complete lifecycle
  int result = onvif_service_handler_init(&handler, &config, actions, action_count);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify initialized state
  assert_non_null(handler.actions);
  assert_non_null(handler.gsoap_ctx);
  assert_int_equal(handler.action_count, action_count);
  assert_string_equal(handler.config.service_name, "media");
  assert_int_equal(handler.config.service_type, ONVIF_SERVICE_MEDIA);

  // Configure cleanup expectations
  expect_function_call(__wrap_onvif_gsoap_cleanup);

  // Cleanup
  onvif_service_handler_cleanup(&handler);

  // Verify cleaned up state
  assert_null(handler.actions);
  assert_null(handler.gsoap_ctx);
  assert_int_equal(handler.action_count, 0);
}

/* ============================================================================
 * Request Handling Tests (Task 81)
 * ============================================================================ */

/**
 * @brief Test successful request handling
 * @param state Test state (unused)
 */
void test_unit_service_handler_handle_request_success(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Initialize handler
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  int result = onvif_service_handler_init(&handler, &config, actions, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test request handling
  http_request_t request = {0};
  http_response_t response = {0};

  // Expect gSOAP reset call before handling request
  expect_function_call(__wrap_onvif_gsoap_reset);

  int result_handle =
    onvif_service_handler_handle_request(&handler, "GetDeviceInformation", &request, &response);
  assert_int_equal(result_handle, ONVIF_SUCCESS);
  assert_int_equal(g_mock_action_call_count, 1);

  // Cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);
  onvif_service_handler_cleanup(&handler);
}

/**
 * @brief Test request handling with NULL handler
 * @param state Test state (unused)
 */
void test_unit_service_handler_handle_request_null_handler(void** state) {
  (void)state;

  http_request_t request = {0};
  http_response_t response = {0};

  int result = onvif_service_handler_handle_request(NULL, "GetDeviceInformation", &request, &response);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test request handling with NULL request
 * @param state Test state (unused)
 */
void test_unit_service_handler_handle_request_null_request(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler = {0};
  http_response_t response = {0};

  int result =
    onvif_service_handler_handle_request(&handler, "GetDeviceInformation", NULL, &response);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test request handling with NULL response
 * @param state Test state (unused)
 */
void test_unit_service_handler_handle_request_null_response(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler = {0};
  http_request_t request = {0};

  int result = onvif_service_handler_handle_request(&handler, "GetDeviceInformation", &request, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test request handling with unknown action
 * @param state Test state (unused)
 */
void test_unit_service_handler_handle_request_unknown_action(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Initialize handler
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  int result = onvif_service_handler_init(&handler, &config, actions, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with unknown action
  http_request_t request = {0};
  http_response_t response = {0};

  int result_handle = onvif_service_handler_handle_request(&handler, "UnknownAction", &request, &response);
  assert_int_equal(result_handle, ONVIF_SUCCESS);
  assert_int_equal(g_mock_action_call_count, 0); // Action should not be called

  // Cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);
  onvif_service_handler_cleanup(&handler);
}

/**
 * @brief Test request handling when action handler returns error
 * @param state Test state (unused)
 */
void test_unit_service_handler_handle_request_action_handler_error(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Initialize handler
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  int result = onvif_service_handler_init(&handler, &config, actions, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Set mock to return error
  g_mock_action_result = ONVIF_ERROR;

  // Test request handling with action error
  http_request_t request = {0};
  http_response_t response = {0};

  // Expect gSOAP reset call before handling request
  expect_function_call(__wrap_onvif_gsoap_reset);

  int result_handle =
    onvif_service_handler_handle_request(&handler, "GetDeviceInformation", &request, &response);
  assert_int_equal(result_handle, ONVIF_ERROR);
  assert_int_equal(g_mock_action_call_count, 1);

  // Cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);
  onvif_service_handler_cleanup(&handler);
}

/* ============================================================================
 * Request Validation Tests (Task 81)
 * ============================================================================ */

/**
 * @brief Test request validation with NULL handler
 * @param state Test state (unused)
 */
void test_unit_service_handler_validate_request_null_handler(void** state) {
  (void)state;

  http_request_t request = {0};
  const char* params[] = {"param1"};

  int result = onvif_service_handler_validate_request(NULL, &request, params, 1);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test request validation with NULL request
 * @param state Test state (unused)
 */
void test_unit_service_handler_validate_request_null_request(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler = {0};
  const char* params[] = {"param1"};

  int result = onvif_service_handler_validate_request(&handler, NULL, params, 1);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test request validation with NULL params
 * @param state Test state (unused)
 */
void test_unit_service_handler_validate_request_null_params(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler = {0};
  http_request_t request = {0};

  int result = onvif_service_handler_validate_request(&handler, &request, NULL, 1);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test request validation with zero param count
 * @param state Test state (unused)
 */
void test_unit_service_handler_validate_request_zero_param_count(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler = {0};
  http_request_t request = {0};
  const char* params[] = {"param1"};

  int result = onvif_service_handler_validate_request(&handler, &request, params, 0);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/* ============================================================================
 * Response Generation Tests (Task 81)
 * ============================================================================ */

/**
 * @brief Test success response generation with NULL handler
 * @param state Test state (unused)
 */
void test_unit_service_handler_generate_success_null_handler(void** state) {
  (void)state;

  http_response_t response = {0};

  int result = onvif_service_handler_generate_success(NULL, "GetDeviceInformation", "<body/>", &response);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test error response generation with NULL handler
 * @param state Test state (unused)
 */
void test_unit_service_handler_generate_error_null_handler(void** state) {
  (void)state;

  http_response_t response = {0};

  int result = onvif_service_handler_generate_error(NULL, "GetDeviceInformation",
                                                     ERROR_PATTERN_VALIDATION_FAILED, "Test error", &response);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test XML builder reset
 * @param state Test state (unused)
 */
void test_unit_service_handler_reset_xml_builder_success(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Initialize handler
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  int result = onvif_service_handler_init(&handler, &config, actions, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test reset
  expect_function_call(__wrap_onvif_gsoap_reset);

  int result_reset = onvif_service_handler_reset_xml_builder(&handler);
  assert_int_equal(result_reset, ONVIF_SUCCESS);

  // Cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);
  onvif_service_handler_cleanup(&handler);
}

/**
 * @brief Test XML builder reset with NULL handler
 * @param state Test state (unused)
 */
void test_unit_service_handler_reset_xml_builder_null_handler(void** state) {
  (void)state;

  int result = onvif_service_handler_reset_xml_builder(NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test gSOAP context retrieval
 * @param state Test state (unused)
 */
void test_unit_service_handler_get_gsoap_context(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Initialize handler
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  int result = onvif_service_handler_init(&handler, &config, actions, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test get context
  onvif_gsoap_context_t* ctx = onvif_service_handler_get_gsoap_context(&handler);
  assert_non_null(ctx);
  assert_ptr_equal(ctx, handler.gsoap_ctx);

  // Test with NULL handler
  ctx = onvif_service_handler_get_gsoap_context(NULL);
  assert_null(ctx);

  // Cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);
  onvif_service_handler_cleanup(&handler);
}

/* ============================================================================
 * Configuration Tests (Task 82)
 * ============================================================================ */

/**
 * @brief Test get config value with NULL handler
 * @param state Test state (unused)
 */
void test_unit_service_handler_get_config_value_null_handler(void** state) {
  (void)state;

  int value = 0;
  int result = onvif_service_handler_get_config_value(NULL, CONFIG_SECTION_DEVICE, "test_key",
                                                       &value, CONFIG_TYPE_INT);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test get config value with NULL key
 * @param state Test state (unused)
 */
void test_unit_service_handler_get_config_value_null_key(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler = {0};
  int value = 0;
  int result = onvif_service_handler_get_config_value(&handler, CONFIG_SECTION_DEVICE, NULL,
                                                       &value, CONFIG_TYPE_INT);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test get config value with NULL value pointer
 * @param state Test state (unused)
 */
void test_unit_service_handler_get_config_value_null_value_ptr(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler = {0};
  int result = onvif_service_handler_get_config_value(&handler, CONFIG_SECTION_DEVICE, "test_key",
                                                       NULL, CONFIG_TYPE_INT);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test set config value with NULL handler
 * @param state Test state (unused)
 */
void test_unit_service_handler_set_config_value_null_handler(void** state) {
  (void)state;

  int value = 100;
  int result = onvif_service_handler_set_config_value(NULL, CONFIG_SECTION_DEVICE, "test_key",
                                                       &value, CONFIG_TYPE_INT);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test set config value with NULL key
 * @param state Test state (unused)
 */
void test_unit_service_handler_set_config_value_null_key(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler = {0};
  int value = 100;
  int result = onvif_service_handler_set_config_value(&handler, CONFIG_SECTION_DEVICE, NULL,
                                                       &value, CONFIG_TYPE_INT);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test set config value with NULL value pointer
 * @param state Test state (unused)
 */
void test_unit_service_handler_set_config_value_null_value_ptr(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler = {0};
  int result = onvif_service_handler_set_config_value(&handler, CONFIG_SECTION_DEVICE, "test_key",
                                                       NULL, CONFIG_TYPE_INT);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/* ============================================================================
 * Statistics Tests (Task 82)
 * ============================================================================ */

/**
 * @brief Test get stats after handling requests
 * @param state Test state (unused)
 */
void test_unit_service_handler_get_stats_success(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Initialize handler
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  int result = onvif_service_handler_init(&handler, &config, actions, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Get stats before any requests
  service_stats_t stats = {0};
  int result_stats = onvif_service_handler_get_stats(&handler, &stats);
  assert_int_equal(result_stats, ONVIF_SUCCESS);
  assert_int_equal(stats.total_requests, 0);
  assert_int_equal(stats.total_errors, 0);
  assert_int_equal(stats.total_success, 0);

  // Cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);
  onvif_service_handler_cleanup(&handler);
}

/**
 * @brief Test get stats with NULL handler
 * @param state Test state (unused)
 */
void test_unit_service_handler_get_stats_null_handler(void** state) {
  (void)state;

  service_stats_t stats = {0};
  int result = onvif_service_handler_get_stats(NULL, &stats);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/**
 * @brief Test get stats with NULL stats pointer
 * @param state Test state (unused)
 */
void test_unit_service_handler_get_stats_null_stats(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler = {0};
  int result = onvif_service_handler_get_stats(&handler, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

/* ============================================================================
 * Action Registration Tests (Task 82)
 * ============================================================================ */

/**
 * @brief Test register action success
 * @param state Test state (unused)
 */
void test_unit_service_handler_register_action_success(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Initialize handler
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  int result = onvif_service_handler_init(&handler, &config, actions, 1);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(handler.action_count, 1);

  // Register new action
  service_action_def_t new_action = {
    .action_name = "GetCapabilities", .handler = mock_action_handler, .requires_validation = 1};

  int result_register = onvif_service_handler_register_action(&handler, &new_action);
  assert_int_equal(result_register, ONVIF_SUCCESS);
  assert_int_equal(handler.action_count, 2);

  // Cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);
  onvif_service_handler_cleanup(&handler);
}

/**
 * @brief Test register duplicate action
 * @param state Test state (unused)
 */
void test_unit_service_handler_register_action_duplicate(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1}};

  // Initialize handler
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  int result = onvif_service_handler_init(&handler, &config, actions, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Try to register duplicate action
  service_action_def_t duplicate_action = {
    .action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1};

  int result_register = onvif_service_handler_register_action(&handler, &duplicate_action);
  assert_int_equal(result_register, ONVIF_ERROR);
  assert_int_equal(handler.action_count, 1); // Count should not change

  // Cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);
  onvif_service_handler_cleanup(&handler);
}

/**
 * @brief Test unregister action success
 * @param state Test state (unused)
 */
void test_unit_service_handler_unregister_action_success(void** state) {
  (void)state;

  onvif_service_handler_instance_t handler;
  service_handler_config_t config = {.service_type = ONVIF_SERVICE_DEVICE,
                                     .service_name = "device",
                                     .config = NULL,
                                     .enable_validation = 1,
                                     .enable_logging = 1};

  service_action_def_t actions[] = {
    {.action_name = "GetDeviceInformation", .handler = mock_action_handler, .requires_validation = 1},
    {.action_name = "GetCapabilities", .handler = mock_action_handler, .requires_validation = 1}};

  // Initialize handler
  expect_function_call(__wrap_onvif_gsoap_init);
  will_return(__wrap_onvif_gsoap_init, ONVIF_SUCCESS);

  int result = onvif_service_handler_init(&handler, &config, actions, 2);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(handler.action_count, 2);

  // Unregister action
  int result_unregister = onvif_service_handler_unregister_action(&handler, "GetCapabilities");
  assert_int_equal(result_unregister, ONVIF_SUCCESS);
  assert_int_equal(handler.action_count, 1);

  // Cleanup
  expect_function_call(__wrap_onvif_gsoap_cleanup);
  onvif_service_handler_cleanup(&handler);
}

/* ============================================================================
 * Test Suite Definition
 * ============================================================================ */

const struct CMUnitTest service_handler_tests[] = {
  // Initialization tests
  cmocka_unit_test_setup_teardown(test_unit_service_handler_init_success,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_init_null_handler,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_init_null_config,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_init_null_actions,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_init_zero_action_count,
                                  setup_service_handler_tests, teardown_service_handler_tests),

  // Cleanup tests
  cmocka_unit_test_setup_teardown(test_unit_service_handler_cleanup_success,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_cleanup_null_handler,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_cleanup_after_init,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_cleanup_double_cleanup,
                                  setup_service_handler_tests, teardown_service_handler_tests),

  // Resource allocation tests
  cmocka_unit_test_setup_teardown(test_unit_service_handler_init_actions_allocation,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_init_gsoap_allocation,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_full_lifecycle,
                                  setup_service_handler_tests, teardown_service_handler_tests),

  // Request handling tests (Task 81)
  cmocka_unit_test_setup_teardown(test_unit_service_handler_handle_request_success,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_handle_request_null_handler,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_handle_request_null_request,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_handle_request_null_response,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_handle_request_unknown_action,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_handle_request_action_handler_error,
                                  setup_service_handler_tests, teardown_service_handler_tests),

  // Request validation tests (Task 81)
  cmocka_unit_test_setup_teardown(test_unit_service_handler_validate_request_null_handler,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_validate_request_null_request,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_validate_request_null_params,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_validate_request_zero_param_count,
                                  setup_service_handler_tests, teardown_service_handler_tests),

  // Response generation tests (Task 81)
  cmocka_unit_test_setup_teardown(test_unit_service_handler_generate_success_null_handler,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_generate_error_null_handler,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_reset_xml_builder_success,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_reset_xml_builder_null_handler,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_get_gsoap_context,
                                  setup_service_handler_tests, teardown_service_handler_tests),

  // Configuration tests (Task 82)
  cmocka_unit_test_setup_teardown(test_unit_service_handler_get_config_value_null_handler,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_get_config_value_null_key,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_get_config_value_null_value_ptr,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_set_config_value_null_handler,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_set_config_value_null_key,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_set_config_value_null_value_ptr,
                                  setup_service_handler_tests, teardown_service_handler_tests),

  // Statistics tests (Task 82)
  cmocka_unit_test_setup_teardown(test_unit_service_handler_get_stats_success,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_get_stats_null_handler,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_get_stats_null_stats,
                                  setup_service_handler_tests, teardown_service_handler_tests),

  // Action registration tests (Task 82)
  cmocka_unit_test_setup_teardown(test_unit_service_handler_register_action_success,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_register_action_duplicate,
                                  setup_service_handler_tests, teardown_service_handler_tests),
  cmocka_unit_test_setup_teardown(test_unit_service_handler_unregister_action_success,
                                  setup_service_handler_tests, teardown_service_handler_tests),
};

/**
 * @brief Get service handler unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnit tests
 */
const struct CMUnitTest* get_service_handler_unit_tests(size_t* count) {
  *count = sizeof(service_handler_tests) / sizeof(service_handler_tests[0]);
  return service_handler_tests;
}
