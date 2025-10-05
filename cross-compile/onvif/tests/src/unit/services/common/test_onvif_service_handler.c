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
