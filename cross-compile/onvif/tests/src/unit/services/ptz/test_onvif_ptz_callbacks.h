/**
 * @file test_onvif_ptz_callbacks.h
 * @brief Unit tests for ONVIF PTZ service callback registration and dispatch
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_ONVIF_PTZ_CALLBACKS_H
#define TEST_ONVIF_PTZ_CALLBACKS_H

/* ============================================================================
 * Test Function Declarations
 * ============================================================================ */

// Service Registration Tests
void test_ptz_service_registration_success(void** state);
void test_ptz_service_registration_duplicate(void** state);
void test_ptz_service_registration_invalid_params(void** state);
void test_ptz_service_unregistration_success(void** state);
void test_ptz_service_unregistration_not_found(void** state);

// Service Dispatch Tests
void test_ptz_service_dispatch_success(void** state);
void test_ptz_service_dispatch_unknown_operation(void** state);
void test_ptz_service_dispatch_null_service(void** state);
void test_ptz_service_dispatch_null_operation(void** state);
void test_ptz_service_dispatch_null_request(void** state);
void test_ptz_service_dispatch_null_response(void** state);

// Operation Handler Tests
void test_ptz_operation_handler_success(void** state);
void test_ptz_operation_handler_null_operation(void** state);
void test_ptz_operation_handler_null_request(void** state);
void test_ptz_operation_handler_null_response(void** state);
void test_ptz_operation_handler_unknown_operation(void** state);

// Error Handling Tests
void test_ptz_service_registration_failure_handling(void** state);
void test_ptz_service_dispatch_failure_handling(void** state);
void test_ptz_service_unregistration_failure_handling(void** state);

// Logging Tests
void test_ptz_service_callback_logging_success(void** state);
void test_ptz_service_callback_logging_failure(void** state);

// Test Suite Functions
int run_ptz_callback_tests(void);

#endif /* TEST_ONVIF_PTZ_CALLBACKS_H */
