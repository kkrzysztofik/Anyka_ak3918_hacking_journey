/**
 * @file test_runner.c
 * @brief Dynamic test runner with flexible filtering
 * @author kkrzysztofik
 * @date 2025
 */

#define _DEFAULT_SOURCE  // For strdup()
#include <getopt.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "cmocka_wrapper.h"
#include "common/test_suites.h"

#define MAX_SUITE_FILTERS 32

/**
 * @brief Test runner options
 */
typedef struct {
  test_category_t category;                 /**< Filter by category (unit/integration) */
  char* suite_filters[MAX_SUITE_FILTERS]; /**< Array of suite name filters */
  int suite_filter_count;                   /**< Number of suite filters */
  int list_only;                            /**< Just list suites, don't run */
  int verbose;                              /**< Verbose output */
  int help;                                 /**< Show help */
} test_runner_options_t;

/**
 * @brief Print usage information
 * @param program_name Name of the program
 */
static void print_help(const char* program_name) {
  printf("ONVIF Dynamic Test Runner\n");
  printf("=========================\n\n");
  printf("Usage: %s [OPTIONS]\n\n", program_name);
  printf("Options:\n");
  printf("  --type=TYPE          Filter by test type: unit, integration, all (default: all)\n");
  printf("  --suite=SUITE        Filter by suite name (can specify multiple with commas)\n");
  printf("                       Examples: --suite=ptz-service\n");
  printf("                                 --suite=ptz-service,media-utils\n");
  printf("  --list               List available test suites without running them\n");
  printf("  --verbose            Enable verbose output\n");
  printf("  --help, -h           Show this help message\n");
  printf("\nExamples:\n");
  printf("  %s                                # Run all tests\n", program_name);
  printf("  %s --type=unit                    # Run only unit tests\n", program_name);
  printf("  %s --type=integration             # Run only integration tests\n", program_name);
  printf("  %s --suite=ptz-service            # Run only PTZ service tests\n",
         program_name);
  printf("  %s --type=unit --suite=ptz-service,media-utils  # PTZ + Media unit tests\n",
         program_name);
  printf("  %s --list                         # List all available suites\n", program_name);
  printf("\nAvailable Suites:\n");
  for (size_t i = 0; i < g_test_suite_count; i++) {
    printf("  %-20s - %s [%s]\n", g_test_suites[i].name, g_test_suites[i].full_name,
           g_test_suites[i].category == TEST_CATEGORY_UNIT ? "unit" : "integration");
  }
}

/**
 * @brief Parse command-line arguments
 * @param argc Argument count
 * @param argv Argument vector
 * @param options Output options structure
 */
static void parse_arguments(int argc, char* argv[], test_runner_options_t* options) {
  // Initialize with defaults
  memset(options, 0, sizeof(test_runner_options_t));
  options->category = (test_category_t)-1; // -1 means "all"

  static struct option long_options[] = {
    {"type", required_argument, 0, 't'},   {"suite", required_argument, 0, 's'},
    {"list", no_argument, 0, 'l'},         {"verbose", no_argument, 0, 'v'},
    {"help", no_argument, 0, 'h'},         {0, 0, 0, 0}};

  int option_index = 0;
  int c;

  while ((c = getopt_long(argc, argv, "hvl", long_options, &option_index)) != -1) {
    switch (c) {
    case 't':
      if (strcmp(optarg, "unit") == 0) {
        options->category = TEST_CATEGORY_UNIT;
      } else if (strcmp(optarg, "integration") == 0) {
        options->category = TEST_CATEGORY_INTEGRATION;
      } else if (strcmp(optarg, "all") == 0) {
        options->category = (test_category_t)-1;
      } else {
        fprintf(stderr, "Invalid type: %s\n", optarg);
        exit(1);
      }
      break;

    case 's': {
      // Parse comma-separated suite names
      // Use system malloc/free, not CMocka's test versions
      #ifdef strdup
      #undef strdup
      #endif
      #ifdef free
      #undef free
      #endif
      char* suite_list = strdup(optarg);
      if (suite_list == NULL) {
        fprintf(stderr, "Memory allocation failed\n");
        exit(1);
      }
      char* token = strtok(suite_list, ",");
      while (token != NULL && options->suite_filter_count < MAX_SUITE_FILTERS) {
        char* filter = strdup(token);
        if (filter == NULL) {
          fprintf(stderr, "Memory allocation failed\n");
          free(suite_list);
          exit(1);
        }
        options->suite_filters[options->suite_filter_count++] = filter;
        token = strtok(NULL, ",");
      }
      free(suite_list);
      break;
    }

    case 'l':
      options->list_only = 1;
      break;

    case 'v':
      options->verbose = 1;
      break;

    case 'h':
      options->help = 1;
      break;

    case '?':
      exit(1);

    default:
      break;
    }
  }
}

/**
 * @brief Check if a suite matches the filter criteria
 * @param suite Test suite to check
 * @param options Filter options
 * @return 1 if suite matches, 0 otherwise
 */
static int suite_matches_filter(const test_suite_t* suite, const test_runner_options_t* options) {
  // Check category filter
  if (options->category != (test_category_t)-1 && suite->category != options->category) {
    return 0;
  }

  // Check suite name filter
  if (options->suite_filter_count > 0) {
    int found = 0;
    for (int i = 0; i < options->suite_filter_count; i++) {
      if (strcmp(suite->name, options->suite_filters[i]) == 0) {
        found = 1;
        break;
      }
    }
    if (!found) {
      return 0;
    }
  }

  return 1;
}

/**
 * @brief List all available test suites
 * @param options Options (for filtering)
 */
static void list_test_suites(const test_runner_options_t* options) {
  printf("Available Test Suites:\n");
  printf("=====================\n\n");

  int unit_count = 0;
  int integration_count = 0;

  for (size_t i = 0; i < g_test_suite_count; i++) {
    const test_suite_t* suite = &g_test_suites[i];

    if (!suite_matches_filter(suite, options)) {
      continue;
    }

    size_t test_count = 0;
    suite->get_tests(&test_count);

    const char* category_str =
      suite->category == TEST_CATEGORY_UNIT ? "unit" : "integration";
    printf("  %-20s (%2zu tests) - %s [%s]\n", suite->name, test_count, suite->full_name,
           category_str);

    if (suite->category == TEST_CATEGORY_UNIT) {
      unit_count += (int)test_count;
    } else {
      integration_count += (int)test_count;
    }
  }

  printf("\nSummary:\n");
  printf("  Unit tests:        %d\n", unit_count);
  printf("  Integration tests: %d\n", integration_count);
  printf("  Total tests:       %d\n", unit_count + integration_count);
}

/**
 * @brief Main test runner entry point
 * @param argc Argument count
 * @param argv Argument vector
 * @return Number of test failures
 */
int main(int argc, char* argv[]) {
  test_runner_options_t options;
  parse_arguments(argc, argv, &options);

  // Show help if requested
  if (options.help) {
    print_help(argv[0]);
    return 0;
  }

  // List suites if requested
  if (options.list_only) {
    list_test_suites(&options);
    return 0;
  }

  // Display header
  printf("ONVIF Dynamic Test Runner\n");
  printf("=========================\n\n");

  if (options.verbose) {
    if (options.category == TEST_CATEGORY_UNIT) {
      printf("Running unit tests only\n");
    } else if (options.category == TEST_CATEGORY_INTEGRATION) {
      printf("Running integration tests only\n");
    } else {
      printf("Running all tests\n");
    }

    if (options.suite_filter_count > 0) {
      printf("Filtering suites: ");
      for (int i = 0; i < options.suite_filter_count; i++) {
        printf("%s%s", options.suite_filters[i],
               i < options.suite_filter_count - 1 ? ", " : "\n");
      }
    }
    printf("\n");
  }

  clock_t start_time = clock();
  int total_failures = 0;
  int total_tests_run = 0;
  int suites_run = 0;

  // Run matching test suites
  for (size_t i = 0; i < g_test_suite_count; i++) {
    const test_suite_t* suite = &g_test_suites[i];

    if (!suite_matches_filter(suite, &options)) {
      continue;
    }

    size_t test_count = 0;
    const struct CMUnitTest* tests = suite->get_tests(&test_count);

    if (options.verbose) {
      printf("Running suite: %s (%zu tests)\n", suite->full_name, test_count);
    } else {
      printf("Running %s...\n", suite->full_name);
    }

    // Use _cmocka_run_group_tests directly with explicit count to avoid sizeof() macro issue
    int failures = _cmocka_run_group_tests(suite->name, tests, test_count, suite->setup, suite->teardown);
    total_failures += failures;
    total_tests_run += (int)test_count;
    suites_run++;

    if (options.verbose) {
      printf("Suite %s: %zu passed, %d failed\n\n", suite->name, test_count - (size_t)failures,
             failures);
    }
  }

  clock_t end_time = clock();
  double test_duration = ((double)(end_time - start_time)) / CLOCKS_PER_SEC;

  // Display summary
  printf("\nTest Summary\n");
  printf("============\n");
  printf("Suites run:    %d\n", suites_run);
  printf("Tests run:     %d\n", total_tests_run);
  printf("Test duration: %.2f seconds\n", test_duration);

  if (total_failures == 0) {
    printf("✅ All %d test(s) passed!\n", total_tests_run);
  } else {
    printf("❌ %d test(s) failed!\n", total_failures);
  }

  // Cleanup
  for (int i = 0; i < options.suite_filter_count; i++) {
    free(options.suite_filters[i]);
  }

  return total_failures;
}
