/**
 * @file service_handler_mock.h
 * @brief Mock action handlers for service handler testing
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef SERVICE_HANDLER_MOCK_H
#define SERVICE_HANDLER_MOCK_H

#include "protocol/response/onvif_service_handler.h"
#include "networking/http/http_parser.h"
#include "protocol/gsoap/onvif_gsoap_core.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Mock Action Handler Functions
 * ============================================================================ */

/**
 * @brief Mock action handler for basic testing
 *
 * This handler increments the call count and returns the configured result.
 *
 * @param config Service handler configuration
 * @param request HTTP request
 * @param response HTTP response
 * @param gsoap_ctx gSOAP context
 * @return Configured mock result
 */
int mock_action_handler(const service_handler_config_t* config,
                        const http_request_t* request,
                        http_response_t* response,
                        onvif_gsoap_context_t* gsoap_ctx);

/**
 * @brief Mock action handler that sets specific error status code
 *
 * This handler simulates an error condition with a specific HTTP status code (400).
 *
 * @param config Service handler configuration
 * @param request HTTP request
 * @param response HTTP response (status code set to 400)
 * @param gsoap_ctx gSOAP context
 * @return ONVIF_ERROR
 */
int mock_action_handler_with_error_status(const service_handler_config_t* config,
                                          const http_request_t* request,
                                          http_response_t* response,
                                          onvif_gsoap_context_t* gsoap_ctx);

/**
 * @brief Mock action handler that sets specific success status code
 *
 * This handler simulates a successful operation with a specific HTTP status code (201).
 *
 * @param config Service handler configuration
 * @param request HTTP request
 * @param response HTTP response (status code set to 201)
 * @param gsoap_ctx gSOAP context
 * @return ONVIF_SUCCESS
 */
int mock_action_handler_with_success_status(const service_handler_config_t* config,
                                            const http_request_t* request,
                                            http_response_t* response,
                                            onvif_gsoap_context_t* gsoap_ctx);

/**
 * @brief Mock action handler that fails without setting status code
 *
 * This handler simulates an error without setting a status code
 * (should default to 500).
 *
 * @param config Service handler configuration
 * @param request HTTP request
 * @param response HTTP response (status code not set)
 * @param gsoap_ctx gSOAP context
 * @return ONVIF_ERROR
 */
int mock_action_handler_fail_no_status(const service_handler_config_t* config,
                                       const http_request_t* request,
                                       http_response_t* response,
                                       onvif_gsoap_context_t* gsoap_ctx);

/* ============================================================================
 * Mock State Management Functions
 * ============================================================================ */

/**
 * @brief Initialize the service handler mock
 *
 * Resets all mock state variables to their default values.
 */
void service_handler_mock_init(void);

/**
 * @brief Cleanup the service handler mock
 *
 * Cleans up any resources used by the mock (currently no-op).
 */
void service_handler_mock_cleanup(void);

/**
 * @brief Reset the service handler mock state
 *
 * Resets call count and result to default values. Should be called
 * between tests to ensure clean state.
 */
void service_handler_mock_reset(void);

/**
 * @brief Configure the mock handler return value
 *
 * @param result Result code to return from mock_action_handler
 */
void service_handler_mock_set_result(int result);

/**
 * @brief Get the number of times the mock handler was called
 *
 * @return Call count
 */
int service_handler_mock_get_call_count(void);

#ifdef __cplusplus
}
#endif

#endif // SERVICE_HANDLER_MOCK_H


