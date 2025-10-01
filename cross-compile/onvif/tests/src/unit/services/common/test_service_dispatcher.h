/**
 * @file test_service_dispatcher.h
 * @brief Unit tests for ONVIF service dispatcher
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_SERVICE_DISPATCHER_H
#define TEST_SERVICE_DISPATCHER_H

void test_unit_service_dispatcher_init(void** state);
void test_unit_service_dispatcher_cleanup(void** state);
void test_unit_service_dispatcher_register_service(void** state);
void test_unit_service_dispatcher_register_service_null_params(void** state);
void test_unit_service_dispatcher_register_service_invalid_params(void** state);
void test_unit_service_dispatcher_register_service_duplicate(void** state);
void test_unit_service_dispatcher_register_service_registry_full(void** state);
void test_unit_service_dispatcher_unregister_service(void** state);
void test_unit_service_dispatcher_unregister_service_not_found(void** state);
void test_unit_service_dispatcher_dispatch(void** state);
void test_unit_service_dispatcher_dispatch_invalid_params(void** state);
void test_unit_service_dispatcher_dispatch_service_not_found(void** state);
void test_unit_service_dispatcher_is_registered(void** state);
void test_unit_service_dispatcher_get_services(void** state);
void test_unit_service_dispatcher_init_cleanup_handlers(void** state);

// Service Dispatch with Multiple Services Tests (moved from service callbacks)
void test_unit_service_dispatcher_dispatch_with_registered_service(void** state);
void test_unit_service_dispatcher_dispatch_unknown_operation(void** state);
void test_unit_service_dispatcher_dispatch_null_service_name(void** state);
void test_unit_service_dispatcher_dispatch_null_operation_name(void** state);
void test_unit_service_dispatcher_dispatch_null_request_param(void** state);
void test_unit_service_dispatcher_dispatch_null_response_param(void** state);

#endif /* TEST_SERVICE_DISPATCHER_H */
