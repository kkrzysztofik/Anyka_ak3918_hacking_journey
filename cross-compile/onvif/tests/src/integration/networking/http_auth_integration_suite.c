/**
 * @file http_auth_integration_suite.c
 * @brief HTTP authentication integration test suite wrapper
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

/* Forward declarations from test_http_auth_integration.c */
extern const struct CMUnitTest http_auth_integration_tests[];

/**
 * @brief Get HTTP auth integration tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_http_auth_integration_tests(size_t* count)
{
  *count = 10; /* Number of tests in http_auth_integration_tests[] (removed 2 legacy tests) */
  return http_auth_integration_tests;
}
