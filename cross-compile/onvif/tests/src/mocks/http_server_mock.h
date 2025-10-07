/**
 * @file http_server_mock.h
 * @brief Mock implementation of HTTP server functions for testing
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef HTTP_SERVER_MOCK_H
#define HTTP_SERVER_MOCK_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void http_server_mock_use_real_function(bool use_real);

#ifdef __cplusplus
}
#endif

#endif // HTTP_SERVER_MOCK_H
