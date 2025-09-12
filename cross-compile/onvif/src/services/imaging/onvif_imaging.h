/**
 * @file onvif_imaging.h
 * @brief ONVIF Imaging service API: brightness/contrast etc + day/night logic.
 */

#ifndef ONVIF_IMAGING_H
#define ONVIF_IMAGING_H

#include <stdint.h>
#include "../common/onvif_types.h"
#include "../common/onvif_request.h"
#include "../common/onvif_imaging_types.h"

int onvif_imaging_init(void *vi_handle); /**< Initialize imaging module with VI handle (nullable). */
void onvif_imaging_cleanup(void);        /**< Release imaging resources. */
int onvif_imaging_get_settings(struct imaging_settings *settings); /**< Fetch current settings. */
int onvif_imaging_set_settings(const struct imaging_settings *settings); /**< Apply settings. */
int onvif_imaging_set_day_night_mode(enum day_night_mode mode); /**< Force day/night mode. */
int onvif_imaging_get_day_night_mode(void); /**< Query current forced/auto mode. */
int onvif_imaging_set_irled_mode(enum ir_led_mode mode); /**< Set IR LED mode. */
int onvif_imaging_get_irled_status(void); /**< Get IR LED status/level. */
int onvif_imaging_set_flip_mirror(int flip, int mirror); /**< Configure image flip/mirror. */
int onvif_imaging_set_auto_config(const struct auto_daynight_config *config); /**< Persist auto config. */
int onvif_imaging_get_auto_config(struct auto_daynight_config *config); /**< Retrieve auto config. */
int onvif_imaging_get_imaging_settings(char *response, int response_size); /**< SOAP: GetImagingSettings. */
int onvif_imaging_set_imaging_settings(const char *request, char *response, int response_size); /**< SOAP: SetImagingSettings. */
int onvif_imaging_get_options(char *response, int response_size); /**< SOAP: GetOptions. */
int onvif_imaging_handle_request(onvif_action_type_t action, const onvif_request_t *request, onvif_response_t *response); /**< Handle ONVIF imaging service requests. */

#endif
