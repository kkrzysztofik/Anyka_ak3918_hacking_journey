/**
 * @file service_handler_mock.c
 * @brief Implementation of mock action handlers for service handler testing
 * @author kkrzysztofik
 * @date 2025
 */

#include "service_handler_mock.h"

#include <string.h>

#include "utils/error/error_handling.h"

/* ============================================================================
 * Mock State Variables
 * ============================================================================ */

static int g_mock_action_call_count = 0;
static int g_mock_action_result = ONVIF_SUCCESS;

/* ============================================================================
 * Mock Action Handler Functions
 * ============================================================================ */

/**
 * @brief Mock action handler for basic testing
 */
int mock_action_handler(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                        onvif_gsoap_context_t* gsoap_ctx) {
  (void)config;
  (void)request;
  (void)response;
  (void)gsoap_ctx;

  g_mock_action_call_count++;
  return g_mock_action_result;
}

/**
 * @brief Mock action handler that sets specific error status code
 */
int mock_action_handler_with_error_status(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                          onvif_gsoap_context_t* gsoap_ctx) {
  (void)config;
  (void)request;
  (void)gsoap_ctx;

  g_mock_action_call_count++;

  // Set specific error status code (e.g., 400 Bad Request)
  response->status_code = 400;
  response->body = (char*)"Bad Request";
  response->body_length = strlen("Bad Request");
  response->content_type = "application/soap+xml";

  return ONVIF_ERROR; // Handler fails but sets specific error code
}

/**
 * @brief Mock action handler that sets specific success status code
 */
int mock_action_handler_with_success_status(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                            onvif_gsoap_context_t* gsoap_ctx) {
  (void)config;
  (void)request;
  (void)gsoap_ctx;

  g_mock_action_call_count++;

  // Set specific success status code (e.g., 201 Created)
  response->status_code = 201;
  response->body = (char*)"Created";
  response->body_length = strlen("Created");
  response->content_type = "application/soap+xml";

  return ONVIF_SUCCESS;
}

/**
 * @brief Mock action handler that fails without setting status code
 */
int mock_action_handler_fail_no_status(const service_handler_config_t* config, const http_request_t* request, http_response_t* response,
                                       onvif_gsoap_context_t* gsoap_ctx) {
  (void)config;
  (void)request;
  (void)response;
  (void)gsoap_ctx;

  g_mock_action_call_count++;

  // Handler fails but doesn't set status code - should default to 500
  return ONVIF_ERROR;
}

/* ============================================================================
 * Mock State Management Functions
 * ============================================================================ */

/**
 * @brief Initialize the service handler mock
 */
void service_handler_mock_init(void) {
  g_mock_action_call_count = 0;
  g_mock_action_result = ONVIF_SUCCESS;
}

/**
 * @brief Cleanup the service handler mock
 */
void service_handler_mock_cleanup(void) {
  // Currently no resources to clean up
  g_mock_action_call_count = 0;
  g_mock_action_result = ONVIF_SUCCESS;
}

/**
 * @brief Reset the service handler mock state
 */
void service_handler_mock_reset(void) {
  g_mock_action_call_count = 0;
  g_mock_action_result = ONVIF_SUCCESS;
}

/**
 * @brief Configure the mock handler return value
 */
void service_handler_mock_set_result(int result) {
  g_mock_action_result = result;
}

/**
 * @brief Get the number of times the mock handler was called
 */
int service_handler_mock_get_call_count(void) {
  return g_mock_action_call_count;
}
