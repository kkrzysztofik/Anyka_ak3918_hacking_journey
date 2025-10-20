/**
 * @file onvif_imaging.h
 * @brief ONVIF Imaging service API: brightness/contrast etc + day/night logic
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_IMAGING_H
#define ONVIF_IMAGING_H

#include "core/config/config.h"
#include "networking/http/http_parser.h"
#include "services/common/onvif_imaging_types.h"

int onvif_imaging_init(void* vi_handle);                                      /**< Initialize imaging module with VI
                                                                                 handle (nullable). */
void onvif_imaging_cleanup(void);                                             /**< Release imaging resources. */
int onvif_imaging_get_settings(struct imaging_settings* settings);            /**< Fetch current settings. */
int onvif_imaging_set_settings(const struct imaging_settings* settings);      /**< Apply settings. */
int onvif_imaging_set_day_night_mode(enum day_night_mode mode);               /**< Force day/night mode. */
int onvif_imaging_get_day_night_mode(void);                                   /**< Query current forced/auto mode. */
int onvif_imaging_set_irled_mode(enum ir_led_mode mode);                      /**< Set IR LED mode. */
int onvif_imaging_get_irled_status(void);                                     /**< Get IR LED status/level. */
int onvif_imaging_set_flip_mirror(int flip, int mirror);                      /**< Configure image flip/mirror. */
int onvif_imaging_set_auto_config(const struct auto_daynight_config* config); /**< Persist auto config. */
int onvif_imaging_get_auto_config(struct auto_daynight_config* config);       /**< Retrieve auto config. */
int onvif_imaging_handle_operation(const char* operation_name, const http_request_t* request,
                                   http_response_t* response); /**< Handle ONVIF imaging service operations (standardized interface). */

/* Imaging service functions */

/**
 * @brief Initialize imaging service
 * @param config Centralized configuration
 * @return 0 on success, negative error code on failure
 */
int onvif_imaging_service_init(config_manager_t* config);

/**
 * @brief Clean up imaging service
 */
void onvif_imaging_service_cleanup(void);

#endif
