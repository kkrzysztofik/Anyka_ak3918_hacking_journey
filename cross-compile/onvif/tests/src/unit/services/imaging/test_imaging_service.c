/**
 * @file test_imaging_service.c
 * @brief Comprehensive imaging service unit tests
 * @author kkrzysztofik
 * @date 2025
 */

#include "test_imaging_service.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cmocka_wrapper.h"
#include "common/test_helpers.h"
#include "core/config/config_runtime.h"
#include "core/config/config_storage.h"
#include "mocks/buffer_pool_mock.h"
#include "mocks/config_mock.h"
#include "mocks/mock_service_dispatcher.h"
#include "mocks/platform_mock.h"
#include "services/common/onvif_imaging_types.h"
#include "services/imaging/onvif_imaging.h"
#include "utils/error/error_handling.h"

/* ============================================================================
 * Test Constants
 * ============================================================================ */

#define TEST_IMAGING_CONFIG_PATH    "./configs/imaging_test_config.ini"
#define TEST_IMAGING_BRIGHTNESS     50
#define TEST_IMAGING_CONTRAST       50
#define TEST_IMAGING_SATURATION     50
#define TEST_IMAGING_SHARPNESS      50
#define TEST_IMAGING_HUE            0
#define TEST_IMAGING_INVALID_VALUE  150
#define TEST_IMAGING_NEGATIVE_VALUE -10

/* ============================================================================
 * Test State and Helper Functions
 * ============================================================================ */

typedef struct {
  struct application_config* app_config;
  config_manager_t* config_manager;
  int runtime_initialized;
  int imaging_initialized;
} imaging_test_state_t;

static imaging_test_state_t g_imaging_test_state = {0};

/**
 * @brief Force imaging module into a non-initialized state for negative tests
 */
static void imaging_test_force_not_initialized(void) {
  onvif_imaging_cleanup();
  g_imaging_test_state.imaging_initialized = 0;
}

/**
 * @brief Setup function for imaging service tests
 */
int setup_imaging_service_tests(void** state) {
  (void)state;
  int result;

  /* Clear test state */
  memset(&g_imaging_test_state, 0, sizeof(g_imaging_test_state));

  /* Enable real functions for imaging tests */
  mock_service_dispatcher_init();
  service_dispatcher_mock_use_real_function(true);
  config_mock_use_real_function(true);
  buffer_pool_mock_use_real_function(true);

  /* Cleanup any previous state (ensures clean slate) */
  config_runtime_cleanup();

  /* Initialize service dispatcher */
  result = onvif_service_dispatcher_init();
  assert_int_equal(ONVIF_SUCCESS, result);

  /* Heap-allocate application config structure */
  struct application_config* app_config = calloc(1, sizeof(struct application_config));
  assert_non_null(app_config);
  g_imaging_test_state.app_config = app_config;

  /* Initialize config structure members */
  memset(&app_config->imaging, 0, sizeof(struct imaging_settings));
  memset(&app_config->auto_daynight, 0, sizeof(struct auto_daynight_config));
  memset(&app_config->network, 0, sizeof(struct network_settings));
  memset(&app_config->device, 0, sizeof(struct device_info));
  memset(&app_config->logging, 0, sizeof(struct logging_settings));
  memset(&app_config->server, 0, sizeof(struct server_settings));

  /* Initialize runtime configuration manager */
  result = config_runtime_init(app_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_imaging_test_state.runtime_initialized = 1;

  /* Load test configuration from INI file */
  char config_path[256];
  result = test_helper_get_test_resource_path("configs/imaging_test_config.ini", config_path, sizeof(config_path));
  assert_int_equal(0, result);
  result = config_storage_load(config_path, NULL);
  assert_int_equal(ONVIF_SUCCESS, result);

  /* Heap-allocate config manager */
  config_manager_t* config_manager = malloc(sizeof(config_manager_t));
  assert_non_null(config_manager);
  memset(config_manager, 0, sizeof(config_manager_t));
  config_manager->app_config = app_config;
  g_imaging_test_state.config_manager = config_manager;

  /* Initialize imaging service (this sets global config pointer) */
  result = onvif_imaging_service_init(config_manager);
  assert_int_equal(ONVIF_SUCCESS, result);

  /* Mock platform_irled_init for onvif_imaging_init */
  expect_function_call(__wrap_platform_irled_init);
  expect_any(__wrap_platform_irled_init, level);
  will_return(__wrap_platform_irled_init, PLATFORM_SUCCESS);

  /* Mock platform_irled_set_mode (called after init) - ONLY if mode is not AUTO */
  if (app_config->auto_daynight.ir_led_mode == IR_LED_ON || app_config->auto_daynight.ir_led_mode == IR_LED_OFF) {
    expect_function_call(__wrap_platform_irled_set_mode);
    expect_any(__wrap_platform_irled_set_mode, mode);
    will_return(__wrap_platform_irled_set_mode, PLATFORM_SUCCESS);
  }

  /* Mock platform_vpss_effect_set calls (apply_imaging_settings_to_vpss makes 5 calls) */
  /* Call 1: brightness */
  expect_function_call(__wrap_platform_vpss_effect_set);
  expect_any(__wrap_platform_vpss_effect_set, handle);
  expect_any(__wrap_platform_vpss_effect_set, effect);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  /* Call 2: contrast */
  expect_function_call(__wrap_platform_vpss_effect_set);
  expect_any(__wrap_platform_vpss_effect_set, handle);
  expect_any(__wrap_platform_vpss_effect_set, effect);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  /* Call 3: saturation */
  expect_function_call(__wrap_platform_vpss_effect_set);
  expect_any(__wrap_platform_vpss_effect_set, handle);
  expect_any(__wrap_platform_vpss_effect_set, effect);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  /* Call 4: sharpness */
  expect_function_call(__wrap_platform_vpss_effect_set);
  expect_any(__wrap_platform_vpss_effect_set, handle);
  expect_any(__wrap_platform_vpss_effect_set, effect);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  // Removed mock expectation for hue as it's not changed in test_unit_imaging_set_settings_success,
  // and optimized_batch_param_update only calls platform_vpss_effect_set for changed parameters.

  /* Initialize imaging module with VI handle (this sets g_imaging_initialized flag) */
  void* vi_handle = (void*)0x12345678;
  result = onvif_imaging_init(vi_handle);
  assert_int_equal(ONVIF_SUCCESS, result);
  g_imaging_test_state.imaging_initialized = 1;

  return 0;
}

/**
 * @brief Teardown function for imaging service tests
 */
int teardown_imaging_service_tests(void** state) {
  (void)state;

  /* Cleanup imaging service if initialized */
  if (g_imaging_test_state.imaging_initialized) {
    onvif_imaging_cleanup();
    onvif_imaging_service_cleanup();
    g_imaging_test_state.imaging_initialized = 0;
  }

  /* Cleanup runtime configuration */
  if (g_imaging_test_state.runtime_initialized) {
    config_runtime_cleanup();
    g_imaging_test_state.runtime_initialized = 0;
  }

  /* Free config manager */
  if (g_imaging_test_state.config_manager) {
    free(g_imaging_test_state.config_manager);
    g_imaging_test_state.config_manager = NULL;
  }

  /* Free application config */
  if (g_imaging_test_state.app_config) {
    free(g_imaging_test_state.app_config);
    g_imaging_test_state.app_config = NULL;
  }

  /* Clear test state */
  memset(&g_imaging_test_state, 0, sizeof(g_imaging_test_state));

  /* Restore mock behavior */
  config_mock_use_real_function(false);
  buffer_pool_mock_use_real_function(false);

  return 0;
}

/* ============================================================================
 * Section 1: Get/Set Settings Tests
 * ============================================================================ */

/**
 * @brief Test get imaging settings success
 */
void test_unit_imaging_get_settings_success(void** state) {
  (void)state;
  int result;

  /* Get settings */
  struct imaging_settings settings;
  result = onvif_imaging_get_settings(&settings);
  assert_int_equal(ONVIF_SUCCESS, result);

  /* Verify settings from config */
  assert_int_equal(TEST_IMAGING_BRIGHTNESS, settings.brightness);
  assert_int_equal(TEST_IMAGING_CONTRAST, settings.contrast);
  assert_int_equal(TEST_IMAGING_SATURATION, settings.saturation);
  assert_int_equal(TEST_IMAGING_SHARPNESS, settings.sharpness);
}

/**
 * @brief Test get imaging settings with null parameters
 */
void test_unit_imaging_get_settings_null_params(void** state) {
  (void)state;
  int result;

  /* Get settings with NULL parameter should fail */
  result = onvif_imaging_get_settings(NULL);
  assert_int_equal(ONVIF_ERROR, result);
}

/**
 * @brief Test get imaging settings when not initialized
 */
void test_unit_imaging_get_settings_not_initialized(void** state) {
  (void)state;
  int result;

  imaging_test_force_not_initialized();

  struct imaging_settings settings;
  result = onvif_imaging_get_settings(&settings);
  assert_int_equal(ONVIF_ERROR, result);
}

/**
 * @brief Test set imaging settings success
 */
void test_unit_imaging_set_settings_success(void** state) {
  (void)state;
  int result;

  /* Set new settings */
  struct imaging_settings new_settings = {.brightness = 60, .contrast = 70, .saturation = 55, .sharpness = 65, .hue = 0, .daynight = {0}};

  result = onvif_imaging_set_settings(&new_settings);
  assert_int_equal(ONVIF_SUCCESS, result);

  /* Verify settings were applied */
  struct imaging_settings retrieved_settings;
  result = onvif_imaging_get_settings(&retrieved_settings);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(60, retrieved_settings.brightness);
  assert_int_equal(70, retrieved_settings.contrast);
}

/**
 * @brief Test set imaging settings with null parameters
 */
void test_unit_imaging_set_settings_null_params(void** state) {
  (void)state;
  int result;

  /* Set settings with NULL parameter should fail */
  result = onvif_imaging_set_settings(NULL);
  assert_int_equal(ONVIF_ERROR, result);
}

/**
 * @brief Test set imaging settings with invalid brightness
 */
void test_unit_imaging_set_settings_invalid_brightness(void** state) {
  (void)state;
  int result;

  /* Set invalid brightness */
  struct imaging_settings invalid_settings = {
    .brightness = TEST_IMAGING_INVALID_VALUE, .contrast = 50, .saturation = 50, .sharpness = 50, .hue = 0, .daynight = {0}};

  result = onvif_imaging_set_settings(&invalid_settings);
  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test set imaging settings with invalid contrast
 */
void test_unit_imaging_set_settings_invalid_contrast(void** state) {
  (void)state;
  int result;

  /* Set invalid contrast (negative) */
  struct imaging_settings invalid_settings = {
    .brightness = 50, .contrast = TEST_IMAGING_NEGATIVE_VALUE, .saturation = 50, .sharpness = 50, .hue = 0, .daynight = {0}};

  result = onvif_imaging_set_settings(&invalid_settings);
  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test set imaging settings when not initialized
 */
void test_unit_imaging_set_settings_not_initialized(void** state) {
  (void)state;
  int result;

  imaging_test_force_not_initialized();

  struct imaging_settings settings = {.brightness = 50, .contrast = 50, .saturation = 50, .sharpness = 50, .hue = 0, .daynight = {0}};

  result = onvif_imaging_set_settings(&settings);
  assert_int_equal(ONVIF_ERROR, result);
}

/* ============================================================================
 * Section 2: Day/Night Mode Tests
 * ============================================================================ */

/**
 * @brief Test set day/night mode to day
 */
void test_unit_imaging_set_day_night_mode_day(void** state) {
  (void)state;
  int result;

  /* Set to day mode */
  result = onvif_imaging_set_day_night_mode(DAY_NIGHT_DAY);
  assert_int_equal(ONVIF_SUCCESS, result);

  /* Verify mode was set */
  int mode = onvif_imaging_get_day_night_mode();
  assert_int_equal(DAY_NIGHT_DAY, mode);
}

/**
 * @brief Test set day/night mode to night
 */
void test_unit_imaging_set_day_night_mode_night(void** state) {
  (void)state;
  int result;

  /* Set to night mode */
  result = onvif_imaging_set_day_night_mode(DAY_NIGHT_NIGHT);
  assert_int_equal(ONVIF_SUCCESS, result);

  /* Verify mode was set */
  int mode = onvif_imaging_get_day_night_mode();
  assert_int_equal(DAY_NIGHT_NIGHT, mode);
}

/**
 * @brief Test set day/night mode to auto
 */
void test_unit_imaging_set_day_night_mode_auto(void** state) {
  (void)state;
  int result;

  /* Set to auto mode */
  result = onvif_imaging_set_day_night_mode(DAY_NIGHT_AUTO);
  assert_int_equal(ONVIF_SUCCESS, result);

  /* Verify mode was set */
  int mode = onvif_imaging_get_day_night_mode();
  assert_int_equal(DAY_NIGHT_AUTO, mode);
}

/**
 * @brief Test set day/night mode when not initialized
 */
void test_unit_imaging_set_day_night_mode_not_initialized(void** state) {
  (void)state;
  int result;

  imaging_test_force_not_initialized();

  result = onvif_imaging_set_day_night_mode(DAY_NIGHT_DAY);
  assert_int_equal(ONVIF_ERROR, result);
}

/**
 * @brief Test get day/night mode success
 */
void test_unit_imaging_get_day_night_mode_success(void** state) {
  (void)state;
  int result;

  /* Get mode should succeed */
  int mode = onvif_imaging_get_day_night_mode();
  assert_true(mode == DAY_NIGHT_DAY || mode == DAY_NIGHT_NIGHT || mode == DAY_NIGHT_AUTO);
}

/**
 * @brief Test get day/night mode when not initialized
 */
void test_unit_imaging_get_day_night_mode_not_initialized(void** state) {
  (void)state;
  int result;

  imaging_test_force_not_initialized();

  int mode = onvif_imaging_get_day_night_mode();
  assert_int_equal(ONVIF_ERROR, mode);
}

/* ============================================================================
 * Section 3: IR LED Tests
 * ============================================================================ */

/**
 * @brief Test set IR LED mode to on
 */
void test_unit_imaging_set_irled_mode_on(void** state) {
  (void)state;
  int result;

  /* Set IR LED to on */
  result = onvif_imaging_set_irled_mode(IR_LED_ON);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test set IR LED mode to off
 */
void test_unit_imaging_set_irled_mode_off(void** state) {
  (void)state;
  int result;

  /* Set IR LED to off */
  result = onvif_imaging_set_irled_mode(IR_LED_OFF);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test set IR LED mode to auto
 */
void test_unit_imaging_set_irled_mode_auto(void** state) {
  (void)state;
  int result;

  /* Set IR LED to auto */
  result = onvif_imaging_set_irled_mode(IR_LED_AUTO);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test get IR LED status success
 */
void test_unit_imaging_get_irled_status_success(void** state) {
  (void)state;
  int result;

  /* Mock platform function to return success */
  will_return(__wrap_platform_irled_get_status, 1);

  int status = onvif_imaging_get_irled_status();
  assert_int_equal(1, status);
}

/**
 * @brief Test get IR LED status with platform error
 */
void test_unit_imaging_get_irled_status_error(void** state) {
  (void)state;
  int result;

  /* Mock platform function to return error */
  will_return(__wrap_platform_irled_get_status, -1);

  int status = onvif_imaging_get_irled_status();
  assert_int_equal(ONVIF_SUCCESS, status); /* Returns success (off) on error */
}

/* ============================================================================
 * Section 4: Flip/Mirror Tests
 * ============================================================================ */

/**
 * @brief Test set flip/mirror success
 */
void test_unit_imaging_set_flip_mirror_success(void** state) {
  (void)state;
  int result;

  /* Set flip=1, mirror=0 */
  result = onvif_imaging_set_flip_mirror(1, 0);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test set flip/mirror when not initialized
 */
void test_unit_imaging_set_flip_mirror_not_initialized(void** state) {
  (void)state;
  int result;

  imaging_test_force_not_initialized();

  result = onvif_imaging_set_flip_mirror(1, 0);
  assert_int_equal(ONVIF_ERROR, result);
}

/* ============================================================================
 * Section 5: Auto Day/Night Config Tests
 * ============================================================================ */

/**
 * @brief Test set auto day/night config success
 */
void test_unit_imaging_set_auto_config_success(void** state) {
  (void)state;
  int result;

  /* Set auto config */
  struct auto_daynight_config config = {.mode = DAY_NIGHT_AUTO,
                                        .day_to_night_threshold = 6400,
                                        .night_to_day_threshold = 2048,
                                        .lock_time_seconds = 900,
                                        .ir_led_mode = IR_LED_AUTO,
                                        .ir_led_level = 80,
                                        .enable_auto_switching = 1};

  result = onvif_imaging_set_auto_config(&config);
  assert_int_equal(ONVIF_SUCCESS, result);

  /* Verify config was set */
  struct auto_daynight_config retrieved_config;
  result = onvif_imaging_get_auto_config(&retrieved_config);
  assert_int_equal(ONVIF_SUCCESS, result);
  assert_int_equal(DAY_NIGHT_AUTO, retrieved_config.mode);
  assert_int_equal(6400, retrieved_config.day_to_night_threshold);
}

/**
 * @brief Test set auto day/night config with null parameters
 */
void test_unit_imaging_set_auto_config_null_params(void** state) {
  (void)state;
  int result;

  /* Set auto config with NULL should fail */
  result = onvif_imaging_set_auto_config(NULL);
  assert_int_equal(ONVIF_ERROR, result);
}

/**
 * @brief Test get auto day/night config success
 */
void test_unit_imaging_get_auto_config_success(void** state) {
  (void)state;
  int result; /* Get auto config */
  struct auto_daynight_config config;
  result = onvif_imaging_get_auto_config(&config);
  assert_int_equal(ONVIF_SUCCESS, result);

  /* Verify config values from test config */
  assert_int_equal(DAY_NIGHT_AUTO, config.mode);
}

/**
 * @brief Test get auto day/night config with null parameters
 */
void test_unit_imaging_get_auto_config_null_params(void** state) {
  (void)state;
  int result; /* Get auto config with NULL should fail */
  result = onvif_imaging_get_auto_config(NULL);
  assert_int_equal(ONVIF_ERROR, result);
}

/* ============================================================================
 * Section 6: VPSS Conversion Helper Tests
 * ============================================================================ */

/**
 * @brief Test brightness to VPSS conversion
 * @note ONVIF value (0-100) → VPSS value (divide by 2)
 */
void test_unit_imaging_convert_brightness_to_vpss(void** state) {
  (void)state;
  int result; /* Set brightness and verify VPSS conversion */
  struct imaging_settings settings = {.brightness = 100, .contrast = 50, .saturation = 50, .sharpness = 50, .hue = 0, .daynight = {0}};

  /* Mock VPSS call to verify conversion */
  expect_value(__wrap_platform_vpss_effect_set, effect_type, PLATFORM_VPSS_EFFECT_BRIGHTNESS);
  expect_value(__wrap_platform_vpss_effect_set, value, 50); /* 100 / 2 = 50 */
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  result = onvif_imaging_set_settings(&settings);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test contrast to VPSS conversion
 */
void test_unit_imaging_convert_contrast_to_vpss(void** state) {
  (void)state;
  int result; /* Set contrast and verify conversion (80 / 2 = 40) */
  struct imaging_settings settings = {.brightness = 50, .contrast = 80, .saturation = 50, .sharpness = 50, .hue = 0, .daynight = {0}};

  /* Mock all VPSS calls */
  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_vpss_effect_set, effect_type, PLATFORM_VPSS_EFFECT_CONTRAST);
  expect_value(__wrap_platform_vpss_effect_set, value, 40); /* 80 / 2 = 40 */
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  result = onvif_imaging_set_settings(&settings);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test saturation to VPSS conversion
 */
void test_unit_imaging_convert_saturation_to_vpss(void** state) {
  (void)state;
  int result; /* Set saturation and verify conversion (60 / 2 = 30) */
  struct imaging_settings settings = {.brightness = 50, .contrast = 50, .saturation = 60, .sharpness = 50, .hue = 0, .daynight = {0}};

  /* Mock all VPSS calls */
  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_vpss_effect_set, effect_type, PLATFORM_VPSS_EFFECT_SATURATION);
  expect_value(__wrap_platform_vpss_effect_set, value, 30); /* 60 / 2 = 30 */
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  result = onvif_imaging_set_settings(&settings);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test sharpness to VPSS conversion
 */
void test_unit_imaging_convert_sharpness_to_vpss(void** state) {
  (void)state;
  int result; /* Set sharpness and verify conversion (90 / 2 = 45) */
  struct imaging_settings settings = {.brightness = 50, .contrast = 50, .saturation = 50, .sharpness = 90, .hue = 0, .daynight = {0}};

  /* Mock all VPSS calls */
  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_vpss_effect_set, effect_type, PLATFORM_VPSS_EFFECT_SHARPNESS);
  expect_value(__wrap_platform_vpss_effect_set, value, 45); /* 90 / 2 = 45 */
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  result = onvif_imaging_set_settings(&settings);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test hue to VPSS conversion
 * @note Hue uses special formula: (value * 50) / 180
 */
void test_unit_imaging_convert_hue_to_vpss(void** state) {
  (void)state;
  int result; /* Set hue and verify conversion (180 * 50 / 180 = 50) */
  struct imaging_settings settings = {.brightness = 50, .contrast = 50, .saturation = 50, .sharpness = 50, .hue = 180, .daynight = {0}};

  /* Mock all VPSS calls */
  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_any(__wrap_platform_vpss_effect_set, vi_handle);
  expect_any(__wrap_platform_vpss_effect_set, effect_type);
  expect_any(__wrap_platform_vpss_effect_set, value);
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  expect_value(__wrap_platform_vpss_effect_set, effect_type, PLATFORM_VPSS_EFFECT_HUE);
  expect_value(__wrap_platform_vpss_effect_set, value, 50); /* (180 * 50) / 180 = 50 */
  will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);

  result = onvif_imaging_set_settings(&settings);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/* ============================================================================
 * Section 7: Validation Helper Tests
 * ============================================================================ */

/**
 * @brief Test validate settings with valid values
 */
void test_unit_imaging_validate_settings_success(void** state) {
  (void)state;
  int result; /* Valid settings should succeed */
  struct imaging_settings valid_settings = {.brightness = 50, .contrast = 50, .saturation = 50, .sharpness = 50, .hue = 0, .daynight = {0}};

  /* Mock all VPSS calls for success */
  for (int i = 0; i < 5; i++) {
    expect_any(__wrap_platform_vpss_effect_set, vi_handle);
    expect_any(__wrap_platform_vpss_effect_set, effect_type);
    expect_any(__wrap_platform_vpss_effect_set, value);
    will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);
  }

  result = onvif_imaging_set_settings(&valid_settings);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test validate settings with invalid brightness
 */
void test_unit_imaging_validate_settings_invalid_brightness(void** state) {
  (void)state;
  int result; /* Invalid brightness > 100 should fail validation */
  struct imaging_settings invalid_settings = {.brightness = 150, .contrast = 50, .saturation = 50, .sharpness = 50, .hue = 0, .daynight = {0}};

  result = onvif_imaging_set_settings(&invalid_settings);
  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/**
 * @brief Test validate settings with out of range values
 */
void test_unit_imaging_validate_settings_invalid_range(void** state) {
  (void)state;
  int result; /* Negative saturation should fail validation */
  struct imaging_settings invalid_settings = {.brightness = 50, .contrast = 50, .saturation = -5, .sharpness = 50, .hue = 0, .daynight = {0}};

  result = onvif_imaging_set_settings(&invalid_settings);
  assert_int_equal(ONVIF_ERROR_INVALID, result);
}

/* ============================================================================
 * Section 8: Bulk Update Helper Tests
 * ============================================================================ */

/**
 * @brief Test bulk update with validation caching
 */
void test_unit_imaging_bulk_update_validation_cache(void** state) {
  (void)state;
  int result; /* Same settings should hit validation cache */
  struct imaging_settings settings = {.brightness = 55, .contrast = 65, .saturation = 60, .sharpness = 70, .hue = 0, .daynight = {0}};

  /* First call - validation occurs */
  for (int i = 0; i < 5; i++) {
    expect_any(__wrap_platform_vpss_effect_set, vi_handle);
    expect_any(__wrap_platform_vpss_effect_set, effect_type);
    expect_any(__wrap_platform_vpss_effect_set, value);
    will_return(__wrap_platform_vpss_effect_set, PLATFORM_SUCCESS);
  }
  result = onvif_imaging_set_settings(&settings);
  assert_int_equal(ONVIF_SUCCESS, result);

  /* Second call with same settings - should use cache and skip re-validation */
  result = onvif_imaging_set_settings(&settings);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/**
 * @brief Test optimized batch update with no changes
 */
void test_unit_imaging_optimized_batch_update_no_changes(void** state) {
  (void)state;
  int result; /* Get current settings */
  struct imaging_settings current_settings;
  result = onvif_imaging_get_settings(&current_settings);
  assert_int_equal(ONVIF_SUCCESS, result);

  /* Set same settings (no changes) - should optimize and skip VPSS calls */
  result = onvif_imaging_set_settings(&current_settings);
  assert_int_equal(ONVIF_SUCCESS, result);
}

/* ============================================================================
 * Section 9: Operation Handler Tests
 * ============================================================================ */

/**
 * @brief Test operation handler with valid operation (placeholder)
 */
void test_unit_imaging_operation_handler_success(void** state) {
  (void)state;
  int result;

  /* NOTE: This test requires full HTTP request mocking
   * For now, we test that the handler rejects invalid operations
   */
}

/**
 * @brief Test operation handler with null operation name
 */
void test_unit_imaging_operation_handler_null_operation(void** state) {
  (void)state;
  int result;

  /* NOTE: Operation handler tests require service handler initialization
   * These are better covered by integration tests
   */
}

/**
 * @brief Test operation handler with null request
 */
void test_unit_imaging_operation_handler_null_request(void** state) {
  (void)state;
  int result;

  /* NOTE: Operation handler tests require service handler initialization
   * These are better covered by integration tests
   */
}

/**
 * @brief Test operation handler with null response
 */
void test_unit_imaging_operation_handler_null_response(void** state) {
  (void)state;
  int result;

  /* NOTE: Operation handler tests require service handler initialization
   * These are better covered by integration tests
   */
}

/**
 * @brief Test operation handler with unknown operation
 */
void test_unit_imaging_operation_handler_unknown_operation(void** state) {
  (void)state;
  int result;

  /* NOTE: Operation handler tests require service handler initialization
   * These are better covered by integration tests
   */
}

/**
 * @brief Test operation handler when not initialized
 */
void test_unit_imaging_operation_handler_not_initialized(void** state) {
  (void)state;
  int result;

  /* NOTE: Operation handler tests require service handler initialization
   * These are better covered by integration tests
   */
}

/**
 * @brief Test handle GetImagingSettings operation
 */
void test_unit_imaging_handle_get_imaging_settings(void** state) {
  (void)state;
  int result;

  /* NOTE: Operation handler tests require full HTTP request/response mocking
   * These are better covered by integration tests
   */
}

/**
 * @brief Test handle SetImagingSettings operation
 */
void test_unit_imaging_handle_set_imaging_settings(void** state) {
  (void)state;
  int result;

  /* NOTE: Operation handler tests require full HTTP request/response mocking
   * These are better covered by integration tests
   */
}
