/**
 * @file common_validation.h
 * @brief Common validation utilities for ONVIF services
 * 
 * This module provides consolidated validation functions that eliminate
 * code duplication across all ONVIF service implementations.
 */

#ifndef ONVIF_COMMON_VALIDATION_H
#define ONVIF_COMMON_VALIDATION_H

#include <stddef.h>
#include "services/common/onvif_types.h"

// Forward declarations
struct imaging_settings;

/**
 * @brief Validation result
 */
typedef struct {
    int is_valid;
    const char *error_message;
    const char *field_name;
} validation_result_t;

/**
 * @brief Validate string parameter
 * @param value String value to validate
 * @param field_name Name of the field being validated
 * @param min_length Minimum length (0 for no minimum)
 * @param max_length Maximum length (0 for no maximum)
 * @param allow_empty Whether empty string is allowed
 * @return Validation result
 */
validation_result_t validate_string(const char *value, 
                                   const char *field_name,
                                   size_t min_length,
                                   size_t max_length,
                                   int allow_empty);

/**
 * @brief Validate integer parameter
 * @param value Integer value to validate
 * @param field_name Name of the field being validated
 * @param min_value Minimum value
 * @param max_value Maximum value
 * @return Validation result
 */
validation_result_t validate_int(int value,
                                const char *field_name,
                                int min_value,
                                int max_value);

/**
 * @brief Validate float parameter
 * @param value Float value to validate
 * @param field_name Name of the field being validated
 * @param min_value Minimum value
 * @param max_value Maximum value
 * @return Validation result
 */
validation_result_t validate_float(float value,
                                  const char *field_name,
                                  float min_value,
                                  float max_value);

/**
 * @brief Validate ONVIF token
 * @param token Token to validate
 * @param field_name Name of the field being validated
 * @return Validation result
 */
validation_result_t validate_onvif_token(const char *token, const char *field_name);

/**
 * @brief Validate ONVIF profile token
 * @param token Profile token to validate
 * @param field_name Name of the field being validated
 * @return Validation result
 */
validation_result_t validate_profile_token(const char *token, const char *field_name);

/**
 * @brief Validate ONVIF protocol
 * @param protocol Protocol to validate
 * @param field_name Name of the field being validated
 * @return Validation result
 */
validation_result_t validate_protocol(const char *protocol, const char *field_name);

/**
 * @brief Validate PTZ position
 * @param pan Pan value
 * @param tilt Tilt value
 * @param zoom Zoom value
 * @return Validation result
 */
validation_result_t validate_ptz_position(float pan, float tilt, float zoom);

/**
 * @brief Validate PTZ speed
 * @param pan_speed Pan speed value
 * @param tilt_speed Tilt speed value
 * @param zoom_speed Zoom speed value
 * @return Validation result
 */
validation_result_t validate_ptz_speed(float pan_speed, float tilt_speed, float zoom_speed);

/**
 * @brief Validate imaging settings
 * @param settings Imaging settings to validate
 * @return Validation result
 */
validation_result_t validate_imaging_settings(const struct imaging_settings *settings);

/**
 * @brief Validate video resolution
 * @param width Video width
 * @param height Video height
 * @return Validation result
 */
validation_result_t validate_video_resolution(int width, int height);

/**
 * @brief Validate video quality
 * @param quality Video quality value
 * @return Validation result
 */
validation_result_t validate_video_quality(float quality);

/**
 * @brief Validate bitrate
 * @param bitrate Bitrate value
 * @return Validation result
 */
validation_result_t validate_bitrate(int bitrate);

/**
 * @brief Validate framerate
 * @param framerate Framerate value
 * @return Validation result
 */
validation_result_t validate_framerate(int framerate);

/**
 * @brief Check if validation result is valid
 * @param result Validation result to check
 * @return 1 if valid, 0 if invalid
 */
int validation_is_valid(const validation_result_t *result);

/**
 * @brief Get validation error message
 * @param result Validation result
 * @return Error message or NULL if valid
 */
const char* validation_get_error_message(const validation_result_t *result);

/**
 * @brief Get validation field name
 * @param result Validation result
 * @return Field name or NULL if not applicable
 */
const char* validation_get_field_name(const validation_result_t *result);

#endif /* ONVIF_COMMON_VALIDATION_H */
