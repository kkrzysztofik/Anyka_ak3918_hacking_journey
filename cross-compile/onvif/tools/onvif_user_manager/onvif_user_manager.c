/**
 * @file onvif_user_manager.c
 * @brief Command-line utility to manage ONVIF user credentials
 * @author kkrzysztofik
 * @date 2025
 */

#define _GNU_SOURCE
#include <getopt.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "core/config/config.h"
#include "core/config/config_runtime.h"
#include "core/config/config_storage.h"
#include "platform/platform.h"
#include "platform/platform_common.h"
#include "utils/error/error_handling.h"
#include "ini_merge.h"

// Constants
static const char* const g_program_name = "onvif_user_manager";

/**
 * @brief Display usage information
 */
static void print_usage(void) {
  printf("Usage: %s [OPTIONS]\n", g_program_name);
  printf("\n");
  printf("Manage ONVIF user credentials in configuration files.\n");
  printf("\n");
  printf("Required options:\n");
  printf("  -u, --user USERNAME     Username to add/update\n");
  printf("  -p, --password PASS     Password for the user\n");
  printf("  -f, --file CONFIG_FILE  Path to configuration file\n");
  printf("\n");
  printf("Other options:\n");
  printf("  -h, --help              Display this help message\n");
  printf("\n");
  printf("Examples:\n");
  printf("  %s --user admin --password secret123 --file /etc/onvif/config.ini\n", g_program_name);
  printf("  %s -u admin -p secret123 -f ./config.ini\n", g_program_name);
  printf("\n");
}

/**
 * @brief Validate input parameters
 * @param username Username to validate
 * @param password Password to validate
 * @param config_file Config file path to validate
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int validate_parameters(const char* username, const char* password, const char* config_file) {
  if (!username || strlen(username) == 0) {
    (void)fprintf(stderr, "Error: Username cannot be empty\n");
    return ONVIF_ERROR_INVALID;
  }

  if (!password || strlen(password) == 0) {
    (void)fprintf(stderr, "Error: Password cannot be empty\n");
    return ONVIF_ERROR_INVALID;
  }

  if (!config_file || strlen(config_file) == 0) {
    (void)fprintf(stderr, "Error: Config file path cannot be empty\n");
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Main function
 * @param argc Argument count
 * @param argv Argument vector
 * @return Exit code (0 for success, 1 for error)
 */
int main(int argc, char* argv[]) {
  struct application_config app_config = {0};
  int result = 0;
  char* username = NULL;
  char* password = NULL;
  char* config_file = NULL;
  int opt = 0;

  // Long options for getopt_long
  static struct option long_options[] = {{"user", required_argument, 0, 'u'},
                                         {"password", required_argument, 0, 'p'},
                                         {"file", required_argument, 0, 'f'},
                                         {"help", no_argument, 0, 'h'},
                                         {0, 0, 0, 0}};

  // Parse command line arguments
  while ((opt = getopt_long(argc, argv, "u:p:f:h", long_options, NULL)) != -1) {
    switch (opt) {
    case 'u':
      username = optarg;
      break;
    case 'p':
      password = optarg;
      break;
    case 'f':
      config_file = optarg;
      break;
    case 'h':
      print_usage();
      return 0;
    case '?':
      (void)fprintf(stderr, "Try '%s --help' for more information.\n", g_program_name);
      return 1;
    default:
      (void)fprintf(stderr, "Unknown option. Try '%s --help' for more information.\n", g_program_name);
      return 1;
    }
  }

  // Check for required parameters
  if (!username || !password || !config_file) {
    (void)fprintf(stderr, "Error: Missing required parameters.\n");
    (void)fprintf(stderr, "Required: --user, --password, --file\n");
    (void)fprintf(stderr, "Try '%s --help' for more information.\n", g_program_name);
    return 1;
  }

  // Validate parameters
  result = validate_parameters(username, password, config_file);
  if (result != ONVIF_SUCCESS) {
    return 1;
  }

  // Initialize platform
  platform_result_t platform_result = platform_init();
  if (platform_result != PLATFORM_SUCCESS) {
    (void)fprintf(stderr, "Failed to initialize platform: %d\n", platform_result);
    return 1;
  }

  // Initialize the runtime configuration manager
  result = config_runtime_init(&app_config);
  if (result != ONVIF_SUCCESS) {
    (void)fprintf(stderr, "Failed to initialize config runtime: %d\n", result);
    (void)platform_cleanup();
    return 1;
  }

  // Load existing configuration file (if it exists)
  result = config_storage_load(config_file, NULL);
  if (result != ONVIF_SUCCESS) {
    // If file doesn't exist or load fails, apply defaults and continue
    (void)fprintf(stderr, "Warning: Failed to load existing config file (will create new one): %d\n", result);
    result = config_runtime_apply_defaults();
    if (result != ONVIF_SUCCESS) {
      (void)fprintf(stderr, "Failed to apply defaults: %d\n", result);
      config_runtime_cleanup();
      (void)platform_cleanup();
      return 1;
    }
  } else {
    // File loaded successfully - defaults already applied during load
  }

  // Add or update user (add_user will update if username exists)
  result = config_runtime_add_user(username, password);
  if (result != ONVIF_SUCCESS) {
    (void)fprintf(stderr, "Failed to add/update user '%s': %d\n", username, result);
    config_runtime_cleanup();
    (void)platform_cleanup();
    return 1;
  }

  // Check if user already existed by trying to authenticate with a dummy password first
  // (Actually, simpler: just say added/updated - the function handles both)
  printf("Successfully added/updated user '%s'\n", username);

  // Get current config snapshot for merging
  const struct application_config* snapshot = config_runtime_snapshot();
  if (snapshot == NULL) {
    (void)fprintf(stderr, "Failed to get configuration snapshot\n");
    config_runtime_cleanup();
    (void)platform_cleanup();
    return 1;
  }

  // Merge user sections into existing file (preserves all other sections)
  result = ini_merge_user_sections(config_file, snapshot);
  if (result != ONVIF_SUCCESS) {
    (void)fprintf(stderr, "Failed to save configuration: %d\n", result);
    config_runtime_cleanup();
    (void)platform_cleanup();
    return 1;
  }

  printf("Configuration saved successfully\n");

  // Verify the user was added correctly
  result = config_runtime_authenticate_user(username, password);
  if (result != ONVIF_SUCCESS) {
    (void)fprintf(stderr, "Failed to authenticate user '%s': %d\n", username, result);
    config_runtime_cleanup();
    (void)platform_cleanup();
    return 1;
  }

  printf("User authentication test passed\n");

  // Cleanup
  config_runtime_cleanup();
  (void)platform_cleanup();

  return 0;
}
