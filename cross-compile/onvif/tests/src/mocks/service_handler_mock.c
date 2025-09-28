/**
 * @file service_handler_mock.c
 * @brief Mock implementations for ONVIF service handler
 * @author kkrzysztofik
 * @date 2025
 */

#include "service_handler_mock.h"

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ============================================================================
 * Mock State Management
 * ============================================================================ */

static struct {
  int initialized;
  int init_call_count;
  int cleanup_call_count;
  int handle_request_call_count;
  int error_simulation_enabled;
  int error_code;
  pthread_mutex_t mutex;
} g_service_handler_mock_state = {0};

/* ============================================================================
 * Mock State Management Functions
 * ============================================================================ */

void service_handler_mock_init(void) {
  pthread_mutex_init(&g_service_handler_mock_state.mutex, NULL);
  g_service_handler_mock_state.initialized = 1;
  g_service_handler_mock_state.init_call_count = 0;
  g_service_handler_mock_state.cleanup_call_count = 0;
  g_service_handler_mock_state.handle_request_call_count = 0;
  g_service_handler_mock_state.error_simulation_enabled = 0;
  g_service_handler_mock_state.error_code = 0;
}

void service_handler_mock_cleanup(void) {
  pthread_mutex_destroy(&g_service_handler_mock_state.mutex);
  memset(&g_service_handler_mock_state, 0, sizeof(g_service_handler_mock_state));
}

void service_handler_mock_reset(void) {
  pthread_mutex_lock(&g_service_handler_mock_state.mutex);
  g_service_handler_mock_state.init_call_count = 0;
  g_service_handler_mock_state.cleanup_call_count = 0;
  g_service_handler_mock_state.handle_request_call_count = 0;
  g_service_handler_mock_state.error_simulation_enabled = 0;
  g_service_handler_mock_state.error_code = 0;
  pthread_mutex_unlock(&g_service_handler_mock_state.mutex);
}

void service_handler_mock_enable_error_simulation(int error_code) {
  pthread_mutex_lock(&g_service_handler_mock_state.mutex);
  g_service_handler_mock_state.error_simulation_enabled = 1;
  g_service_handler_mock_state.error_code = error_code;
  pthread_mutex_unlock(&g_service_handler_mock_state.mutex);
}

void service_handler_mock_disable_error_simulation(void) {
  pthread_mutex_lock(&g_service_handler_mock_state.mutex);
  g_service_handler_mock_state.error_simulation_enabled = 0;
  pthread_mutex_unlock(&g_service_handler_mock_state.mutex);
}

int service_handler_mock_get_init_call_count(void) {
  pthread_mutex_lock(&g_service_handler_mock_state.mutex);
  int count = g_service_handler_mock_state.init_call_count;
  pthread_mutex_unlock(&g_service_handler_mock_state.mutex);
  return count;
}

int service_handler_mock_get_cleanup_call_count(void) {
  pthread_mutex_lock(&g_service_handler_mock_state.mutex);
  int count = g_service_handler_mock_state.cleanup_call_count;
  pthread_mutex_unlock(&g_service_handler_mock_state.mutex);
  return count;
}

int service_handler_mock_get_handle_request_call_count(void) {
  pthread_mutex_lock(&g_service_handler_mock_state.mutex);
  int count = g_service_handler_mock_state.handle_request_call_count;
  pthread_mutex_unlock(&g_service_handler_mock_state.mutex);
  return count;
}

/* ============================================================================
 * Service Handler Mock Functions
 * ============================================================================ */

int onvif_service_handler_init(void) {
  pthread_mutex_lock(&g_service_handler_mock_state.mutex);
  g_service_handler_mock_state.init_call_count++;

  if (g_service_handler_mock_state.error_simulation_enabled) {
    int error = g_service_handler_mock_state.error_code;
    pthread_mutex_unlock(&g_service_handler_mock_state.mutex);
    return error;
  }

  pthread_mutex_unlock(&g_service_handler_mock_state.mutex);
  return 0; // Success
}

void onvif_service_handler_cleanup(void) {
  pthread_mutex_lock(&g_service_handler_mock_state.mutex);
  g_service_handler_mock_state.cleanup_call_count++;
  pthread_mutex_unlock(&g_service_handler_mock_state.mutex);
}

int onvif_service_handler_handle_request(const char* request, size_t request_size, char* response,
                                         size_t response_size) {
  if (!request || !response || response_size == 0) {
    return -1;
  }

  pthread_mutex_lock(&g_service_handler_mock_state.mutex);
  g_service_handler_mock_state.handle_request_call_count++;

  if (g_service_handler_mock_state.error_simulation_enabled) {
    int error = g_service_handler_mock_state.error_code;
    pthread_mutex_unlock(&g_service_handler_mock_state.mutex);
    return error;
  }

  // Mock response
  const char* mock_response =
    "HTTP/1.1 200 OK\r\nContent-Type: "
    "text/xml\r\n\r\n<soap:Envelope><soap:Body><soap:Fault><faultstring>Mock "
    "Response</faultstring></soap:Fault></soap:Body></soap:Envelope>";
  size_t response_len = strlen(mock_response);
  size_t copy_size = (response_len < response_size - 1) ? response_len : response_size - 1;

  strncpy(response, mock_response, copy_size);
  response[copy_size] = '\0';

  pthread_mutex_unlock(&g_service_handler_mock_state.mutex);
  return 0; // Success
}
