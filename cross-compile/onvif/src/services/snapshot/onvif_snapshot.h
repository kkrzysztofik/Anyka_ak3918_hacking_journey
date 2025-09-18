/**
 * @file onvif_snapshot.h
 * @brief ONVIF Snapshot service implementation.
 * 
 * This file provides the interface for the ONVIF Snapshot service,
 * which handles snapshot capture and HTTP serving for Profile S compliance.
 */

#ifndef ONVIF_SNAPSHOT_H
#define ONVIF_SNAPSHOT_H

#include "platform.h"
#include "services/media/onvif_media.h"

#ifdef __cplusplus
extern "C" {
#endif

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
 * @param width Snapshot width
 * @param height Snapshot height
 * @param data Pointer to store JPEG data
 * @param size Pointer to store data size
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int onvif_snapshot_capture(int width, int height, uint8_t **data, size_t *size);

/**
 * @brief Release snapshot data
 * @param data Snapshot data to release
 */
void onvif_snapshot_release(uint8_t *data);

/**
 * @brief Get snapshot URI for a profile
 * @param profile_token Profile token
 * @param uri URI structure to populate
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int onvif_snapshot_get_uri(const char *profile_token, struct stream_uri *uri);

/**
 * @brief Initialize snapshot service handler
 * @param config Centralized configuration
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int onvif_snapshot_service_init(config_manager_t *config);

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
int onvif_snapshot_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response);

#ifdef __cplusplus
}
#endif

#endif /* ONVIF_SNAPSHOT_H */
