/**
 * @file test_onvif_media_callbacks.c
 * @brief Media service callback lifecycle tests using real dispatcher patterns.
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "../../../mocks/buffer_pool_mock.h"
#include "../../../mocks/gsoap_mock.h"
#include "../../../mocks/mock_service_dispatcher.h"
#include "../../../utils/test_gsoap_utils.h"
#include "cmocka_wrapper.h"
#include "services/common/service_dispatcher.h"
#include "services/media/onvif_media.h"
#include "utils/error/error_handling.h"

#define TEST_MEDIA_SERVICE_NAME "Media"
#define TEST_MEDIA_NAMESPACE    "http://www.onvif.org/ver10/media/wsdl"

static config_manager_t g_mock_config;

static int dummy_operation_handler(const char* operation_name, const http_request_t* request, http_response_t* response);

static void media_dependencies_set_real(bool enable) {
  service_dispatcher_mock_use_real_function(enable);
  buffer_pool_mock_use_real_function(enable);
  gsoap_mock_use_real_function(enable);
}

static void media_reset_state(void) {
  onvif_media_cleanup();
  memset(&g_mock_config, 0, sizeof(g_mock_config));
}

static void media_pre_register_service(void) {
  onvif_service_registration_t registration = {.service_name = TEST_MEDIA_SERVICE_NAME,
                                               .namespace_uri = TEST_MEDIA_NAMESPACE,
                                               .operation_handler = dummy_operation_handler,
                                               .init_handler = NULL,
                                               .cleanup_handler = NULL,
                                               .capabilities_handler = NULL,
                                               .reserved = {NULL, NULL, NULL, NULL}};
  assert_int_equal(ONVIF_SUCCESS, onvif_service_dispatcher_register_service(&registration));
}

static int dummy_operation_handler(const char* operation_name, const http_request_t* request, http_response_t* response) {
  (void)operation_name;
  (void)request;
  (void)response;
  return ONVIF_SUCCESS;
}

int setup_media_callback_tests(void** state) {
  (void)state;

  mock_service_dispatcher_init();
  media_dependencies_set_real(true);

  onvif_service_dispatcher_init();
  media_reset_state();

  return 0;
}

int teardown_media_callback_tests(void** state) {
  (void)state;

  onvif_media_cleanup();
  onvif_service_dispatcher_cleanup();

  media_dependencies_set_real(false);
  mock_service_dispatcher_cleanup();

  return 0;
}

void test_unit_media_callback_registration_success(void** state) {
  (void)state;

  setup_http_verbose_mock();
  int result = onvif_media_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_MEDIA_SERVICE_NAME));
}

void test_unit_media_callback_registration_duplicate(void** state) {
  (void)state;

  setup_http_verbose_mock();
  assert_int_equal(ONVIF_SUCCESS, onvif_media_init());
  assert_int_equal(ONVIF_SUCCESS, onvif_media_init());
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_MEDIA_SERVICE_NAME));
}

void test_unit_media_callback_registration_null_config(void** state) {
  (void)state;

  setup_http_verbose_mock();
  int result = onvif_media_init();
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_MEDIA_SERVICE_NAME));
}

void test_unit_media_callback_registration_dispatcher_failure(void** state) {
  (void)state;

  media_pre_register_service();

  setup_http_verbose_mock();
  int result = onvif_media_init();
  assert_int_equal(ONVIF_ERROR_ALREADY_EXISTS, result);
  // Service should remain registered with original handler after ALREADY_EXISTS error
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_MEDIA_SERVICE_NAME));
}

void test_unit_media_callback_double_initialization(void** state) {
  (void)state;

  setup_http_verbose_mock();
  assert_int_equal(ONVIF_SUCCESS, onvif_media_init());
  assert_int_equal(ONVIF_SUCCESS, onvif_media_init());
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_MEDIA_SERVICE_NAME));
}

void test_unit_media_callback_unregistration_success(void** state) {
  (void)state;

  setup_http_verbose_mock();
  assert_int_equal(ONVIF_SUCCESS, onvif_media_init());
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_MEDIA_SERVICE_NAME));

  onvif_media_cleanup();
  assert_int_equal(0, onvif_service_dispatcher_is_registered(TEST_MEDIA_SERVICE_NAME));
}

void test_unit_media_callback_unregistration_not_initialized(void** state) {
  (void)state;

  onvif_media_cleanup();
  assert_int_equal(0, onvif_service_dispatcher_is_registered(TEST_MEDIA_SERVICE_NAME));
}

void test_unit_media_callback_unregistration_failure(void** state) {
  (void)state;

  setup_http_verbose_mock();
  assert_int_equal(ONVIF_SUCCESS, onvif_media_init());
  assert_int_equal(1, onvif_service_dispatcher_is_registered(TEST_MEDIA_SERVICE_NAME));

  // Remove service manually to force unregister path to encounter NOT_FOUND
  assert_int_equal(ONVIF_SUCCESS, onvif_service_dispatcher_unregister_service(TEST_MEDIA_SERVICE_NAME));
  assert_int_equal(0, onvif_service_dispatcher_is_registered(TEST_MEDIA_SERVICE_NAME));

  onvif_media_cleanup();
  assert_int_equal(0, onvif_service_dispatcher_is_registered(TEST_MEDIA_SERVICE_NAME));
}
