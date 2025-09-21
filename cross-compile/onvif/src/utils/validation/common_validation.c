/**
 * @file common_validation.c
 * @brief Common validation utilities implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "common_validation.h"

#include <math.h>
#include <string.h>

#include "services/common/onvif_imaging_types.h"

// Note: clang-tidy warns about easily swappable parameters, but this signature
// is used throughout the codebase. Defensive programming below catches common
// mistakes.
validation_result_t validate_string(const char *field_name, const char *value,
                                    size_t min_length, size_t max_length,
                                    int allow_empty) {
  validation_result_t result = {0};

  // Defensive programming: validate parameters
  if (!field_name) {
    result.is_valid = 0;
    result.error_message = "Field name is NULL";
    result.field_name = "field_name";
    return result;
  }

  if (!value) {
    result.is_valid = 0;
    result.error_message = "Value is NULL";
    result.field_name = field_name;
    return result;
  }

  // Validate parameter ranges
  if (min_length > max_length && max_length > 0) {
    result.is_valid = 0;
    result.error_message = "Invalid parameter: min_length > max_length";
    result.field_name = field_name;
    return result;
  }

  size_t length = strlen(value);

  if (!allow_empty && length == 0) {
    result.is_valid = 0;
    result.error_message = "Value cannot be empty";
    result.field_name = field_name;
    return result;
  }

  if (min_length > 0 && length < min_length) {
    result.is_valid = 0;
    result.error_message = "Value too short";
    result.field_name = field_name;
    return result;
  }

  if (max_length > 0 && length > max_length) {
    result.is_valid = 0;
    result.error_message = "Value too long";
    result.field_name = field_name;
    return result;
  }

  result.is_valid = 1;
  return result;
}

validation_result_t validate_int(const char *field_name, int value,
                                 int min_value, int max_value) {
  validation_result_t result = {0};

  // Defensive programming: validate parameters
  if (!field_name) {
    result.is_valid = 0;
    result.error_message = "Field name is NULL";
    result.field_name = "field_name";
    return result;
  }

  // Validate parameter ranges
  if (min_value > max_value) {
    result.is_valid = 0;
    result.error_message = "Invalid parameter: min_value > max_value";
    result.field_name = field_name;
    return result;
  }

  if (value < min_value) {
    result.is_valid = 0;
    result.error_message = "Value below minimum";
    result.field_name = field_name;
    return result;
  }

  if (value > max_value) {
    result.is_valid = 0;
    result.error_message = "Value above maximum";
    result.field_name = field_name;
    return result;
  }

  result.is_valid = 1;
  return result;
}

validation_result_t validate_float(const char *field_name, float value,
                                   float min_value, float max_value) {
  validation_result_t result = {0};

  // Defensive programming: validate parameters
  if (!field_name) {
    result.is_valid = 0;
    result.error_message = "Field name is NULL";
    result.field_name = "field_name";
    return result;
  }

  // Validate parameter ranges
  if (min_value > max_value) {
    result.is_valid = 0;
    result.error_message = "Invalid parameter: min_value > max_value";
    result.field_name = field_name;
    return result;
  }

  if (isnan(value)) {
    result.is_valid = 0;
    result.error_message = "Value is NaN";
    result.field_name = field_name;
    return result;
  }

  if (isinf(value)) {
    result.is_valid = 0;
    result.error_message = "Value is infinite";
    result.field_name = field_name;
    return result;
  }

  if (value < min_value) {
    result.is_valid = 0;
    result.error_message = "Value below minimum";
    result.field_name = field_name;
    return result;
  }

  if (value > max_value) {
    result.is_valid = 0;
    result.error_message = "Value above maximum";
    result.field_name = field_name;
    return result;
  }

  result.is_valid = 1;
  return result;
}

validation_result_t validate_onvif_token(const char *token,
                                         const char *field_name) {
  return validate_string(field_name, token, 1, 64, 0);
}

validation_result_t validate_profile_token(const char *token,
                                           const char *field_name) {
  validation_result_t result = validate_onvif_token(token, field_name);

  if (!result.is_valid) {
    return result;
  }

  // Check for known profile tokens
  if (strcmp(token, "MainProfile") == 0 || strcmp(token, "SubProfile") == 0) {
    result.is_valid = 1;
  } else {
    result.is_valid = 0;
    result.error_message = "Unknown profile token";
    result.field_name = field_name;
  }

  return result;
}

validation_result_t validate_protocol(const char *protocol,
                                      const char *field_name) {
  validation_result_t result = validate_string(field_name, protocol, 1, 32, 0);

  if (!result.is_valid) {
    return result;
  }

  // Check for supported protocols
  if (strcmp(protocol, "RTSP") == 0 || strcmp(protocol, "RTP-Unicast") == 0) {
    result.is_valid = 1;
  } else {
    result.is_valid = 0;
    result.error_message = "Unsupported protocol";
    result.field_name = field_name;
  }

  return result;
}

validation_result_t validate_ptz_position(float pan, float tilt, float zoom) {
  validation_result_t result = {0};

  // Validate pan (-1.0 to 1.0)
  validation_result_t pan_result = validate_float("pan", pan, -1.0f, 1.0f);
  if (!pan_result.is_valid) {
    return pan_result;
  }

  // Validate tilt (-1.0 to 1.0)
  validation_result_t tilt_result = validate_float("tilt", tilt, -1.0f, 1.0f);
  if (!tilt_result.is_valid) {
    return tilt_result;
  }

  // Validate zoom (0.0 to 1.0)
  validation_result_t zoom_result = validate_float("zoom", zoom, 0.0f, 1.0f);
  if (!zoom_result.is_valid) {
    return zoom_result;
  }

  result.is_valid = 1;
  return result;
}

validation_result_t validate_ptz_speed(float pan_speed, float tilt_speed,
                                       float zoom_speed) {
  validation_result_t result = {0};

  // Validate pan speed (-1.0 to 1.0)
  validation_result_t pan_result =
      validate_float("pan_speed", pan_speed, -1.0f, 1.0f);
  if (!pan_result.is_valid) {
    return pan_result;
  }

  // Validate tilt speed (-1.0 to 1.0)
  validation_result_t tilt_result =
      validate_float("tilt_speed", tilt_speed, -1.0f, 1.0f);
  if (!tilt_result.is_valid) {
    return tilt_result;
  }

  // Validate zoom speed (-1.0 to 1.0)
  validation_result_t zoom_result =
      validate_float("zoom_speed", zoom_speed, -1.0f, 1.0f);
  if (!zoom_result.is_valid) {
    return zoom_result;
  }

  result.is_valid = 1;
  return result;
}

validation_result_t validate_imaging_settings(
    const struct imaging_settings *settings) {
  validation_result_t result = {0};

  if (!settings) {
    result.is_valid = 0;
    result.error_message = "Settings is NULL";
    return result;
  }

  // Validate brightness (-100 to 100)
  validation_result_t brightness_result =
      validate_int("brightness", settings->brightness, -100, 100);
  if (!brightness_result.is_valid) {
    return brightness_result;
  }

  // Validate contrast (-100 to 100)
  validation_result_t contrast_result =
      validate_int("contrast", settings->contrast, -100, 100);
  if (!contrast_result.is_valid) {
    return contrast_result;
  }

  // Validate saturation (-100 to 100)
  validation_result_t saturation_result =
      validate_int("saturation", settings->saturation, -100, 100);
  if (!saturation_result.is_valid) {
    return saturation_result;
  }

  // Validate sharpness (-100 to 100)
  validation_result_t sharpness_result =
      validate_int("sharpness", settings->sharpness, -100, 100);
  if (!sharpness_result.is_valid) {
    return sharpness_result;
  }

  // Validate hue (-180 to 180)
  validation_result_t hue_result =
      validate_int("hue", settings->hue, -180, 180);
  if (!hue_result.is_valid) {
    return hue_result;
  }

  result.is_valid = 1;
  return result;
}

validation_result_t validate_video_resolution(int width, int height) {
  validation_result_t result = {0};

  // Validate width (1 to 4096)
  validation_result_t width_result = validate_int("width", width, 1, 4096);
  if (!width_result.is_valid) {
    return width_result;
  }

  // Validate height (1 to 4096)
  validation_result_t height_result = validate_int("height", height, 1, 4096);
  if (!height_result.is_valid) {
    return height_result;
  }

  // Check for common aspect ratios
  if (width == 0 || height == 0) {
    result.is_valid = 0;
    result.error_message = "Invalid resolution";
    return result;
  }

  result.is_valid = 1;
  return result;
}

validation_result_t validate_video_quality(float quality) {
  return validate_float("quality", quality, 0.0f, 10.0f);
}

validation_result_t validate_bitrate(int bitrate) {
  return validate_int("bitrate", bitrate, 1, 100000);  // 1 bps to 100 Mbps
}

validation_result_t validate_framerate(int framerate) {
  return validate_int("framerate", framerate, 1, 120);  // 1 to 120 fps
}

int validation_is_valid(const validation_result_t *result) {
  return result ? result->is_valid : 0;
}

const char *validation_get_error_message(const validation_result_t *result) {
  return result ? result->error_message : NULL;
}

const char *validation_get_field_name(const validation_result_t *result) {
  return result ? result->field_name : NULL;
}
