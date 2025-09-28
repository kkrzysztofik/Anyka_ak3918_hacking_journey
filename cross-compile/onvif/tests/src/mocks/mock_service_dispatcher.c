/**
 * @file mock_service_dispatcher.c
 * @brief Mock implementation for service dispatcher testing
 * @author kkrzysztofik
 * @date 2025
 */

#include "mock_service_dispatcher.h"

#include <string.h>

#include "networking/http/http_parser.h"
#include "services/common/service_dispatcher.h"
#include "utils/error/error_handling.h"

/* ============================================================================
 * Mock Service Dispatcher State
 * ============================================================================ */

// Mock state variables
int g_mock_register_result = ONVIF_SUCCESS;   // NOLINT
int g_mock_unregister_result = ONVIF_SUCCESS; // NOLINT
int g_mock_dispatch_result = ONVIF_SUCCESS;   // NOLINT
int g_mock_register_call_count = 0;           // NOLINT
int g_mock_unregister_call_count = 0;         // NOLINT
int g_mock_dispatch_call_count = 0;           // NOLINT
int g_mock_is_registered_result = 0;          // NOLINT
int g_mock_get_services_count = 0;            // NOLINT
int g_mock_init_result = ONVIF_SUCCESS;       // NOLINT
int g_mock_init_call_count = 0;               // NOLINT
int g_mock_cleanup_call_count = 0;            // NOLINT

// Mock service registration data
onvif_service_registration_t g_mock_last_registration = {0}; // NOLINT
char g_mock_last_unregister_service[64] = {0};               // NOLINT
char g_mock_last_dispatch_service[64] = {0};                 // NOLINT
char g_mock_last_dispatch_operation[64] = {0};               // NOLINT

/* ============================================================================
 * Mock Control Functions
 * ============================================================================ */

void mock_service_dispatcher_init(void) {
  g_mock_register_result = ONVIF_SUCCESS;
  g_mock_unregister_result = ONVIF_SUCCESS;
  g_mock_dispatch_result = ONVIF_SUCCESS;
  g_mock_register_call_count = 0;
  g_mock_unregister_call_count = 0;
  g_mock_dispatch_call_count = 0;
  g_mock_is_registered_result = 0;
  g_mock_get_services_count = 0;
  g_mock_init_result = ONVIF_SUCCESS;
  g_mock_init_call_count = 0;
  g_mock_cleanup_call_count = 0;

  memset(&g_mock_last_registration, 0, sizeof(g_mock_last_registration));
  memset(g_mock_last_unregister_service, 0, sizeof(g_mock_last_unregister_service));
  memset(g_mock_last_dispatch_service, 0, sizeof(g_mock_last_dispatch_service));
  memset(g_mock_last_dispatch_operation, 0, sizeof(g_mock_last_dispatch_operation));
}

void mock_service_dispatcher_cleanup(void) {
  mock_service_dispatcher_init();
}

void mock_service_dispatcher_set_register_result(int result) {
  g_mock_register_result = result;
}

void mock_service_dispatcher_set_unregister_result(int result) {
  g_mock_unregister_result = result;
}

void mock_service_dispatcher_set_dispatch_result(int result) {
  g_mock_dispatch_result = result;
}

void mock_service_dispatcher_set_is_registered_result(int result) {
  g_mock_is_registered_result = result;
}

void mock_service_dispatcher_set_get_services_count(int count) {
  g_mock_get_services_count = count;
}

void mock_service_dispatcher_set_init_result(int result) {
  g_mock_init_result = result;
}

/* ============================================================================
 * Mock Query Functions
 * ============================================================================ */

int mock_service_dispatcher_get_register_call_count(void) {
  return g_mock_register_call_count;
}

int mock_service_dispatcher_get_unregister_call_count(void) {
  return g_mock_unregister_call_count;
}

int mock_service_dispatcher_get_dispatch_call_count(void) {
  return g_mock_dispatch_call_count;
}

int mock_service_dispatcher_get_init_call_count(void) {
  return g_mock_init_call_count;
}

int mock_service_dispatcher_get_cleanup_call_count(void) {
  return g_mock_cleanup_call_count;
}

const onvif_service_registration_t* mock_service_dispatcher_get_last_registration(void) {
  return &g_mock_last_registration;
}

const char* mock_service_dispatcher_get_last_unregister_service(void) {
  return g_mock_last_unregister_service;
}

const char* mock_service_dispatcher_get_last_dispatch_service(void) {
  return g_mock_last_dispatch_service;
}

const char* mock_service_dispatcher_get_last_dispatch_operation(void) {
  return g_mock_last_dispatch_operation;
}

/* ============================================================================
 * Mock Implementation Functions
 * ============================================================================ */

int mock_onvif_service_dispatcher_register_service(
  const onvif_service_registration_t* registration) {
  g_mock_register_call_count++;

  if (registration) {
    g_mock_last_registration = *registration;
  }

  return g_mock_register_result;
}

int mock_onvif_service_dispatcher_unregister_service(const char* service_name) {
  g_mock_unregister_call_count++;

  if (service_name) {
    strncpy(g_mock_last_unregister_service, service_name,
            sizeof(g_mock_last_unregister_service) - 1);
    g_mock_last_unregister_service[sizeof(g_mock_last_unregister_service) - 1] = '\0';
  }

  return g_mock_unregister_result;
}

int mock_onvif_service_dispatcher_dispatch(const char* service_name, const char* operation_name,
                                           const http_request_t* request,
                                           http_response_t* response) {
  (void)request;
  (void)response;

  g_mock_dispatch_call_count++;

  if (service_name) {
    strncpy(g_mock_last_dispatch_service, service_name, sizeof(g_mock_last_dispatch_service) - 1);
    g_mock_last_dispatch_service[sizeof(g_mock_last_dispatch_service) - 1] = '\0';
  }

  if (operation_name) {
    strncpy(g_mock_last_dispatch_operation, operation_name,
            sizeof(g_mock_last_dispatch_operation) - 1);
    g_mock_last_dispatch_operation[sizeof(g_mock_last_dispatch_operation) - 1] = '\0';
  }

  return g_mock_dispatch_result;
}

int mock_onvif_service_dispatcher_is_registered(const char* service_name) {
  (void)service_name;
  return g_mock_is_registered_result;
}

int mock_onvif_service_dispatcher_get_services(const char** services, size_t max_services) {
  (void)services;
  (void)max_services;
  return g_mock_get_services_count;
}

int mock_onvif_service_dispatcher_init(void) {
  g_mock_init_call_count++;
  return g_mock_init_result;
}

void mock_onvif_service_dispatcher_cleanup(void) {
  g_mock_cleanup_call_count++;
}
