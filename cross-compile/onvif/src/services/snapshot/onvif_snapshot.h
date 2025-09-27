/**
 * @file onvif_snapshot.h
 * @brief ONVIF Snapshot service implementation
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_SNAPSHOT_H
#define ONVIF_SNAPSHOT_H

#include "core/config/config.h"
#include "networking/http/http_parser.h"
#include "services/common/onvif_types.h"
#include "services/media/onvif_media.h"

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Snapshot dimensions structure
 */
typedef struct {
  int width;  /**< Snapshot width in pixels */
  int height; /**< Snapshot height in pixels */
} snapshot_dimensions_t;

/**
 * @brief Initialize the snapshot service
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int onvif_snapshot_init(void);

/**
 * @brief Cleanup snapshot service resources
 */
void onvif_snapshot_cleanup(void);

/**
 * @brief Capture a snapshot
 * @param dimensions Snapshot dimensions (width and height)
 * @param data Pointer to store JPEG data
 * @param size Pointer to store data size
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int onvif_snapshot_capture(const snapshot_dimensions_t* dimensions, uint8_t** data, size_t* size);

/**
 * @brief Release snapshot data
 * @param data Snapshot data to release
 */
void onvif_snapshot_release(uint8_t* data);

/**
 * @brief Get snapshot URI for a profile
 * @param profile_token Profile token
 * @param uri URI structure to populate
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int onvif_snapshot_get_uri(const char* profile_token, struct stream_uri* uri);

/**
 * @brief Initialize snapshot service handler
 * @param config Centralized configuration
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int onvif_snapshot_service_init(config_manager_t* config);

/**
 * @brief Cleanup snapshot service handler
 */
void onvif_snapshot_service_cleanup(void);

/**
 * @brief Handle ONVIF snapshot service requests
 * @param action Action type
 * @param request Request structure
 * @param response Response structure
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int onvif_snapshot_handle_request(const char* action_name, const http_request_t* request,
                                  http_response_t* response);

#ifdef __cplusplus
}
#endif

#endif /* ONVIF_SNAPSHOT_H */
