/**
 * @file test_service_handler_suite.c
 * @brief Service handler test suite registration
 * @author kkrzysztofik
 * @date 2025
 */

#include <stddef.h>

#include "cmocka_wrapper.h"

/* Forward declaration of test getter function */
extern const struct CMUnitTest* get_service_handler_unit_tests(size_t* count);

/**
 * @brief Run service handler test suite
 * @return 0 on success, non-zero on failure
 */
int run_service_handler_tests(void) {
  size_t count = 0;
  const struct CMUnitTest* tests = get_service_handler_unit_tests(&count);

  return cmocka_run_group_tests_name("Service Handler Tests", tests, count, NULL, NULL);
}
