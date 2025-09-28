/**
 * @file test_onvif_media_callbacks.h
 * @brief Header for ONVIF media service callback unit tests
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_ONVIF_MEDIA_CALLBACKS_H
#define TEST_ONVIF_MEDIA_CALLBACKS_H

#include <stddef.h>

#include "cmocka_wrapper.h"

/* ============================================================================
 * Test Function Declarations
 * ============================================================================ */

/**
 * @brief Test media service callback registration success
 * @param state Test state (unused)
 */
void test_media_callback_registration_success(void** state);

/**
 * @brief Test media service callback registration with duplicate
 * @param state Test state (unused)
 */
void test_media_callback_registration_duplicate(void** state);

/**
 * @brief Test media service callback registration with null config
 * @param state Test state (unused)
 */
void test_media_callback_registration_null_config(void** state);

/**
 * @brief Test media service callback registration with service dispatcher failure
 * @param state Test state (unused)
 */
void test_media_callback_registration_dispatcher_failure(void** state);

/**
 * @brief Test media service callback double initialization
 * @param state Test state (unused)
 */
void test_media_callback_double_initialization(void** state);

/**
 * @brief Test media service callback unregistration success
 * @param state Test state (unused)
 */
void test_media_callback_unregistration_success(void** state);

/**
 * @brief Test media service callback unregistration when not initialized
 * @param state Test state (unused)
 */
void test_media_callback_unregistration_not_initialized(void** state);

/**
 * @brief Test media service callback unregistration failure
 * @param state Test state (unused)
 */
void test_media_callback_unregistration_failure(void** state);

/**
 * @brief Test media service callback dispatch success
 * @param state Test state (unused)
 */
void test_media_callback_dispatch_success(void** state);

/**
 * @brief Test media service callback dispatch when not initialized
 * @param state Test state (unused)
 */
void test_media_callback_dispatch_not_initialized(void** state);

/**
 * @brief Test media service callback dispatch with null parameters
 * @param state Test state (unused)
 */
void test_media_callback_dispatch_null_params(void** state);

/**
 * @brief Run media service callback tests
 * @return Number of test failures
 */
int run_media_callback_tests(void);

#endif /* TEST_ONVIF_MEDIA_CALLBACKS_H */
