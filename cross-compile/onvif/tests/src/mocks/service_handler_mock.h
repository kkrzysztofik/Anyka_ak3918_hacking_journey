/**
 * @file service_handler_mock.h
 * @brief Mock functions for ONVIF service handler
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef SERVICE_HANDLER_MOCK_H
#define SERVICE_HANDLER_MOCK_H

#include <stddef.h>

/* ============================================================================
 * Service Handler Mock Functions
 * ============================================================================ */

/**
 * @brief Mock service handler initialization
 * @return 0 on success, -1 on failure
 */
int onvif_service_handler_init(void);

/**
 * @brief Mock service handler cleanup
 */
void onvif_service_handler_cleanup(void);

/**
 * @brief Mock service handler request handling
 * @param request Request data
 * @param request_size Request size
 * @param response Response buffer
 * @param response_size Response buffer size
 * @return 0 on success, -1 on failure
 */
int onvif_service_handler_handle_request(const char* request, size_t request_size, char* response,
                                         size_t response_size);

#endif // SERVICE_HANDLER_MOCK_H
