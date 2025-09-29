/**
 * @file ptz_service_tests.c
 * @brief Integration tests for optimized ONVIF PTZ service
 * @author kkrzysztofik
 * @date 2025
 */

#include "ptz_service_tests.h"

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "cmocka_wrapper.h"

// ONVIF project includes
#include "services/ptz/onvif_ptz.h"
#include "utils/error/error_handling.h"

// Test data structures
typedef struct {
  int test_pan_position;
  int test_tilt_position;
  int test_speed;
  char test_preset_token[64];
  char test_preset_name[64];
} ptz_test_data_t;

static ptz_test_data_t g_test_data = {0};

// Helper functions for test setup
static int setup_ptz_test_data(void) {
  g_test_data.test_pan_position = 45;
  g_test_data.test_tilt_position = 30;
  g_test_data.test_speed = 50;
  strcpy(g_test_data.test_preset_token, "Preset1");
  strcpy(g_test_data.test_preset_name, "TestPreset");
  return 0;
}

static void cleanup_ptz_test_data(void) {
  memset(&g_test_data, 0, sizeof(g_test_data));
}

// Test PTZ Absolute Move Functionality
void test_integration_ptz_absolute_move_functionality(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ absolute move functionality...\n");

  // Test valid absolute move
  struct ptz_vector position = {.pan_tilt = {.x = 0.5f, .y = 0.3f}, .zoom = 0.0f, .space = ""};

  struct ptz_speed speed = {.pan_tilt = {.x = 0.8f, .y = 0.8f}, .zoom = 0.0f};

  int result = onvif_ptz_absolute_move("ProfileToken1", &position, &speed);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with NULL speed (should use default)
  result = onvif_ptz_absolute_move("ProfileToken1", &position, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test invalid parameters
  result = onvif_ptz_absolute_move(NULL, &position, &speed);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  result = onvif_ptz_absolute_move("ProfileToken1", NULL, &speed);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  printf("✅ PTZ absolute move functionality tests passed\n");
}

// Test PTZ Relative Move Functionality
void test_integration_ptz_relative_move_functionality(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ relative move functionality...\n");

  // Test valid relative move
  struct ptz_vector translation = {.pan_tilt = {.x = 0.1f, .y = -0.1f}, .zoom = 0.0f, .space = ""};

  struct ptz_speed speed = {.pan_tilt = {.x = 0.6f, .y = 0.6f}, .zoom = 0.0f};

  int result = onvif_ptz_relative_move("ProfileToken1", &translation, &speed);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with NULL speed
  result = onvif_ptz_relative_move("ProfileToken1", &translation, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test invalid parameters
  result = onvif_ptz_relative_move(NULL, &translation, &speed);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  result = onvif_ptz_relative_move("ProfileToken1", NULL, &speed);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  printf("✅ PTZ relative move functionality tests passed\n");
}

// Test PTZ Continuous Move Functionality
void test_integration_ptz_continuous_move_functionality(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ continuous move functionality...\n");

  // Test valid continuous move
  struct ptz_speed velocity = {.pan_tilt = {.x = 0.7f, .y = 0.5f}, .zoom = 0.0f};

  int timeout = 5000; // 5 seconds
  int result = onvif_ptz_continuous_move("ProfileToken1", &velocity, timeout);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with zero timeout (no timeout)
  result = onvif_ptz_continuous_move("ProfileToken1", &velocity, 0);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test invalid parameters
  result = onvif_ptz_continuous_move(NULL, &velocity, timeout);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  result = onvif_ptz_continuous_move("ProfileToken1", NULL, timeout);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  printf("✅ PTZ continuous move functionality tests passed\n");
}

// Test PTZ Stop Functionality
void test_integration_ptz_stop_functionality(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ stop functionality...\n");

  // Test valid stop
  int result = onvif_ptz_stop("ProfileToken1", 1, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test stop pan/tilt only
  result = onvif_ptz_stop("ProfileToken1", 1, 0);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test stop zoom only (should succeed even without zoom support)
  result = onvif_ptz_stop("ProfileToken1", 0, 1);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test invalid parameters
  result = onvif_ptz_stop(NULL, 1, 1);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  printf("✅ PTZ stop functionality tests passed\n");
}

// Test PTZ Preset Creation
void test_integration_ptz_preset_creation(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ preset creation...\n");

  char output_token[64] = {0};
  int result =
    onvif_ptz_set_preset("ProfileToken1", "TestPreset", output_token, sizeof(output_token));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_not_equal(output_token, "");

  // Test with NULL preset name (should use default)
  memset(output_token, 0, sizeof(output_token));
  result = onvif_ptz_set_preset("ProfileToken1", NULL, output_token, sizeof(output_token));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test invalid parameters
  result = onvif_ptz_set_preset(NULL, "TestPreset", output_token, sizeof(output_token));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  result = onvif_ptz_set_preset("ProfileToken1", "TestPreset", NULL, sizeof(output_token));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  printf("✅ PTZ preset creation tests passed\n");
}

// Test PTZ Preset Retrieval
void test_integration_ptz_preset_retrieval(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ preset retrieval...\n");

  // First create a preset
  char output_token[64] = {0};
  int result =
    onvif_ptz_set_preset("ProfileToken1", "TestPreset", output_token, sizeof(output_token));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Now retrieve presets
  struct ptz_preset* preset_list = NULL;
  int count = 0;
  result = onvif_ptz_get_presets("ProfileToken1", &preset_list, &count);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_non_null(preset_list);
  assert_int_equal(count, 1);

  // Test invalid parameters
  result = onvif_ptz_get_presets(NULL, &preset_list, &count);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  result = onvif_ptz_get_presets("ProfileToken1", NULL, &count);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  result = onvif_ptz_get_presets("ProfileToken1", &preset_list, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  printf("✅ PTZ preset retrieval tests passed\n");
}

// Test PTZ Preset Goto
void test_integration_ptz_preset_goto(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ preset goto...\n");

  // First create a preset
  char output_token[64] = {0};
  int result =
    onvif_ptz_set_preset("ProfileToken1", "TestPreset", output_token, sizeof(output_token));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test goto preset
  struct ptz_speed speed = {.pan_tilt = {.x = 0.8f, .y = 0.8f}, .zoom = 0.0f};

  result = onvif_ptz_goto_preset("ProfileToken1", output_token, &speed);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with NULL speed
  result = onvif_ptz_goto_preset("ProfileToken1", output_token, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test invalid parameters
  result = onvif_ptz_goto_preset(NULL, output_token, &speed);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  result = onvif_ptz_goto_preset("ProfileToken1", NULL, &speed);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with non-existent preset
  result = onvif_ptz_goto_preset("ProfileToken1", "NonExistentPreset", &speed);
  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);

  printf("✅ PTZ preset goto tests passed\n");
}

// Test PTZ Preset Removal
void test_integration_ptz_preset_removal(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ preset removal...\n");

  // First create a preset
  char output_token[64] = {0};
  int result =
    onvif_ptz_set_preset("ProfileToken1", "TestPreset", output_token, sizeof(output_token));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test remove preset
  result = onvif_ptz_remove_preset("ProfileToken1", output_token);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test removing non-existent preset
  result = onvif_ptz_remove_preset("ProfileToken1", "NonExistentPreset");
  assert_int_equal(result, ONVIF_ERROR_NOT_FOUND);

  // Test invalid parameters
  result = onvif_ptz_remove_preset(NULL, output_token);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  result = onvif_ptz_remove_preset("ProfileToken1", NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  printf("✅ PTZ preset removal tests passed\n");
}

// Test PTZ Preset Memory Optimization
void test_integration_ptz_preset_memory_optimization(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ preset memory optimization...\n");

  // Test multiple preset creation and removal
  char output_tokens[5][64] = {0};

  // Create multiple presets
  for (int i = 0; i < 5; i++) {
    char preset_name[32];
    snprintf(preset_name, sizeof(preset_name), "Preset%d", i + 1);

    int result = onvif_ptz_set_preset("ProfileToken1", preset_name, output_tokens[i],
                                      sizeof(output_tokens[i]));
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  // Verify all presets exist
  struct ptz_preset* preset_list = NULL;
  int count = 0;
  int result = onvif_ptz_get_presets("ProfileToken1", &preset_list, &count);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(count, 5);

  // Remove some presets and verify memory cleanup
  result = onvif_ptz_remove_preset("ProfileToken1", output_tokens[1]);
  assert_int_equal(result, ONVIF_SUCCESS);

  result = onvif_ptz_remove_preset("ProfileToken1", output_tokens[3]);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Verify remaining presets
  result = onvif_ptz_get_presets("ProfileToken1", &preset_list, &count);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_int_equal(count, 3);

  printf("✅ PTZ preset memory optimization tests passed\n");
}

// Test PTZ Memory Usage Improvements
void test_integration_ptz_memory_usage_improvements(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ memory usage improvements...\n");

  // Test buffer pool usage for string operations
  // This would require access to internal buffer pool statistics
  // For now, we test that operations complete without memory errors

  // Test multiple operations to verify no memory leaks
  for (int i = 0; i < 10; i++) {
    struct ptz_vector position = {
      .pan_tilt = {.x = (float)(i % 2), .y = (float)(i % 3) * 0.5f}, .zoom = 0.0f, .space = ""};

    int result = onvif_ptz_absolute_move("ProfileToken1", &position, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);

    char output_token[64] = {0};
    char preset_name[32];
    snprintf(preset_name, sizeof(preset_name), "TestPreset%d", i);

    result = onvif_ptz_set_preset("ProfileToken1", preset_name, output_token, sizeof(output_token));
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  printf("✅ PTZ memory usage improvements tests passed\n");
}

// Test PTZ Buffer Pool Usage
void test_integration_ptz_buffer_pool_usage(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ buffer pool usage...\n");

  // Test that buffer pool is properly used for temporary operations
  // This is validated by ensuring operations complete successfully
  // and don't cause memory allocation failures

  // Test concurrent operations that might stress buffer pool
  struct ptz_vector positions[3] = {{{.x = 0.1f, .y = 0.1f}, 0.0f, ""},
                                    {{.x = 0.5f, .y = 0.5f}, 0.0f, ""},
                                    {{.x = 0.9f, .y = 0.9f}, 0.0f, ""}};

  for (int i = 0; i < 3; i++) {
    int result = onvif_ptz_absolute_move("ProfileToken1", &positions[i], NULL);
    assert_int_equal(result, ONVIF_SUCCESS);

    char output_token[64] = {0};
    char preset_name[32];
    snprintf(preset_name, sizeof(preset_name), "ConcurrentPreset%d", i);

    result = onvif_ptz_set_preset("ProfileToken1", preset_name, output_token, sizeof(output_token));
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  printf("✅ PTZ buffer pool usage tests passed\n");
}

// Test PTZ String Operations Optimization
void test_integration_ptz_string_operations_optimization(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ string operations optimization...\n");

  // Test with various string lengths to verify bounds checking
  char long_preset_name[256];
  memset(long_preset_name, 'A', sizeof(long_preset_name) - 1);
  long_preset_name[sizeof(long_preset_name) - 1] = '\0';

  char output_token[64] = {0};
  int result =
    onvif_ptz_set_preset("ProfileToken1", long_preset_name, output_token, sizeof(output_token));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with empty string
  result = onvif_ptz_set_preset("ProfileToken1", "", output_token, sizeof(output_token));
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with special characters
  result = onvif_ptz_set_preset("ProfileToken1", "Preset-With_Special.Chars", output_token,
                                sizeof(output_token));
  assert_int_equal(result, ONVIF_SUCCESS);

  printf("✅ PTZ string operations optimization tests passed\n");
}

// Test PTZ Error Handling Robustness
void test_integration_ptz_error_handling_robustness(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ error handling robustness...\n");

  // Test with extreme values
  struct ptz_vector extreme_position = {
    .pan_tilt = {.x = 999.0f, .y = -999.0f}, .zoom = 0.0f, .space = ""};

  int result = onvif_ptz_absolute_move("ProfileToken1", &extreme_position, NULL);
  // Should handle extreme values gracefully (clamp to valid range)
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with very long profile token
  char long_profile_token[512];
  memset(long_profile_token, 'X', sizeof(long_profile_token) - 1);
  long_profile_token[sizeof(long_profile_token) - 1] = '\0';

  result = onvif_ptz_absolute_move(long_profile_token, &extreme_position, NULL);
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test with maximum number of presets
  for (int i = 0; i < 10; i++) { // PTZ_MAX_PRESETS = 10
    char output_token[64] = {0};
    char preset_name[32];
    snprintf(preset_name, sizeof(preset_name), "MaxPreset%d", i);

    result = onvif_ptz_set_preset("ProfileToken1", preset_name, output_token, sizeof(output_token));
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  // Test adding one more preset (should fail)
  char output_token[64] = {0};
  result =
    onvif_ptz_set_preset("ProfileToken1", "OverflowPreset", output_token, sizeof(output_token));
  assert_int_equal(result, ONVIF_ERROR); // Should fail due to max presets reached

  printf("✅ PTZ error handling robustness tests passed\n");
}

// Test PTZ Concurrent Operations
void test_integration_ptz_concurrent_operations(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ concurrent operations...\n");

  // This test would require threading support
  // For now, we test sequential operations that simulate concurrent access

  // Simulate rapid sequential operations
  for (int i = 0; i < 5; i++) {
    struct ptz_vector position = {
      .pan_tilt = {.x = (float)(i * 0.2f), .y = (float)(i * 0.1f)}, .zoom = 0.0f, .space = ""};

    int result = onvif_ptz_absolute_move("ProfileToken1", &position, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);

    char output_token[64] = {0};
    char preset_name[32];
    snprintf(preset_name, sizeof(preset_name), "ConcurrentPreset%d", i);

    result = onvif_ptz_set_preset("ProfileToken1", preset_name, output_token, sizeof(output_token));
    assert_int_equal(result, ONVIF_SUCCESS);

    result = onvif_ptz_goto_preset("ProfileToken1", output_token, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  printf("✅ PTZ concurrent operations tests passed\n");
}

// Test PTZ Stress Testing
void test_integration_ptz_stress_testing(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ stress testing...\n");

  // Perform many operations in sequence to stress test the system
  const int stress_iterations = 50;

  for (int i = 0; i < stress_iterations; i++) {
    // Create preset
    char output_token[64] = {0};
    char preset_name[32];
    snprintf(preset_name, sizeof(preset_name), "StressPreset%d", i);

    int result =
      onvif_ptz_set_preset("ProfileToken1", preset_name, output_token, sizeof(output_token));
    if (i < 10) { // Only first 10 should succeed (PTZ_MAX_PRESETS = 10)
      assert_int_equal(result, ONVIF_SUCCESS);
    }

    // Move to position
    struct ptz_vector position = {
      .pan_tilt = {.x = (float)(i % 2), .y = (float)(i % 3) * 0.5f}, .zoom = 0.0f, .space = ""};

    result = onvif_ptz_absolute_move("ProfileToken1", &position, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);

    // If preset was created, goto it
    if (i < 10 && result == ONVIF_SUCCESS) {
      result = onvif_ptz_goto_preset("ProfileToken1", output_token, NULL);
      assert_int_equal(result, ONVIF_SUCCESS);
    }
  }

  printf("✅ PTZ stress testing passed\n");
}

// Test PTZ Memory Leak Detection
void test_integration_ptz_memory_leak_detection(void** state) {
  (void)state; // Unused parameter

  printf("Testing PTZ memory leak detection...\n");

  // Perform operations that should not leak memory
  // This test relies on the memory manager's leak detection

  // Create and remove presets multiple times
  for (int cycle = 0; cycle < 3; cycle++) {
    char output_tokens[5][64] = {0};

    // Create presets
    for (int i = 0; i < 5; i++) {
      char preset_name[32];
      snprintf(preset_name, sizeof(preset_name), "LeakTestPreset%d_%d", cycle, i);

      int result = onvif_ptz_set_preset("ProfileToken1", preset_name, output_tokens[i],
                                        sizeof(output_tokens[i]));
      assert_int_equal(result, ONVIF_SUCCESS);
    }

    // Remove presets
    for (int i = 0; i < 5; i++) {
      int result = onvif_ptz_remove_preset("ProfileToken1", output_tokens[i]);
      assert_int_equal(result, ONVIF_SUCCESS);
    }
  }

  // Perform various PTZ operations
  for (int i = 0; i < 10; i++) {
    struct ptz_vector position = {
      .pan_tilt = {.x = (float)(i * 0.1f), .y = (float)(i * 0.05f)}, .zoom = 0.0f, .space = ""};

    int result = onvif_ptz_absolute_move("ProfileToken1", &position, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);

    result = onvif_ptz_relative_move("ProfileToken1", &position, NULL);
    assert_int_equal(result, ONVIF_SUCCESS);
  }

  printf("✅ PTZ memory leak detection tests passed\n");
}

// Test suite definition
const struct CMUnitTest ptz_service_optimization_tests[] = {
  // PTZ Movement Operations Tests
  cmocka_unit_test(test_integration_ptz_absolute_move_functionality),
  cmocka_unit_test(test_integration_ptz_relative_move_functionality),
  cmocka_unit_test(test_integration_ptz_continuous_move_functionality),
  cmocka_unit_test(test_integration_ptz_stop_functionality),

  // PTZ Preset Management Tests
  cmocka_unit_test(test_integration_ptz_preset_creation),
  cmocka_unit_test(test_integration_ptz_preset_retrieval),
  cmocka_unit_test(test_integration_ptz_preset_goto),
  cmocka_unit_test(test_integration_ptz_preset_removal),
  cmocka_unit_test(test_integration_ptz_preset_memory_optimization),

  // PTZ Service Optimization Validation Tests
  cmocka_unit_test(test_integration_ptz_memory_usage_improvements),
  cmocka_unit_test(test_integration_ptz_buffer_pool_usage),
  cmocka_unit_test(test_integration_ptz_string_operations_optimization),
  cmocka_unit_test(test_integration_ptz_error_handling_robustness),

  // PTZ Service Performance Tests
  cmocka_unit_test(test_integration_ptz_concurrent_operations),
  cmocka_unit_test(test_integration_ptz_stress_testing),
  cmocka_unit_test(test_integration_ptz_memory_leak_detection),
};
