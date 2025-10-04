/**
 * @file response_test_data.h
 * @brief Shared test data for gSOAP response generation tests
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef RESPONSE_TEST_DATA_H
#define RESPONSE_TEST_DATA_H

#include <stddef.h>

// ============================================================================
// Device Service Test Data
// ============================================================================

/**
 * @brief Mock device information for testing
 */
typedef struct {
  const char* manufacturer;
  const char* model;
  const char* firmware_version;
  const char* serial_number;
  const char* hardware_id;
} mock_device_info_t;

extern const mock_device_info_t mock_device_info_valid;
extern const mock_device_info_t mock_device_info_empty;
extern const mock_device_info_t mock_device_info_null_strings;

// ============================================================================
// Media Service Test Data
// ============================================================================

/**
 * @brief Mock media profile for testing
 */
typedef struct {
  char token[64];
  char name[64];
  int fixed;
  struct {
    char source_token[64];
    struct {
      int width;
      int height;
      int x;
      int y;
    } bounds;
  } video_source;
  struct {
    char token[64];
    char encoding[32];
    int width;
    int height;
    int frame_rate;
    int bitrate;
  } video_encoder;
  struct {
    char source_token[64];
  } audio_source;
  struct {
    char token[64];
    char encoding[32];
    int sample_rate;
    int bitrate;
  } audio_encoder;
  struct {
    char token[64];
  } ptz_configuration;
  struct {
    char token[64];
  } metadata_configuration;
} mock_media_profile_t;

/**
 * @brief Mock stream URI for testing
 */
typedef struct {
  char uri[256];
  int invalid_after_connect;
  int invalid_after_reboot;
  int timeout;
} mock_stream_uri_t;

/**
 * @brief Mock video source configuration for testing
 */
typedef struct {
  const char* token;
  int width;
  int height;
  const char* encoding;
  int frame_rate;
} mock_video_source_config_t;

/**
 * @brief Mock video encoder configuration for testing
 */
typedef struct {
  const char* token;
  int width;
  int height;
  const char* encoding;
  int frame_rate;
  int bitrate;
  int gop_size;
} mock_video_encoder_config_t;

/**
 * @brief Mock metadata configuration for testing
 */
typedef struct {
  const char* token;
  const char* name;
  int use_count;
  const char* analytics;
  const char* multicast;
  const char* session_timeout;
  const char* analytics_engine_configuration;
  const char* extension;
} mock_metadata_config_t;

extern const mock_media_profile_t mock_profiles[];
extern const size_t mock_profile_count;
extern const mock_stream_uri_t mock_stream_uri_valid;
extern const mock_stream_uri_t mock_stream_uri_empty;
extern const mock_video_source_config_t mock_video_source_config_valid;
extern const mock_video_encoder_config_t mock_video_encoder_config_valid;
extern const mock_metadata_config_t mock_metadata_configs[];
extern const size_t mock_metadata_config_count;

// ============================================================================
// PTZ Service Test Data
// ============================================================================

/**
 * @brief Mock PTZ node for testing
 */
typedef struct {
  char token[64];
  char name[64];
  struct {
    struct {
      float x_min, x_max, y_min, y_max;
    } absolute_pan_tilt_position_space;
    struct {
      float min, max;
    } absolute_zoom_position_space;
    struct {
      float x_min, x_max, y_min, y_max;
    } relative_pan_tilt_translation_space;
    struct {
      float min, max;
    } relative_zoom_translation_space;
    struct {
      float x_min, x_max, y_min, y_max;
    } continuous_pan_tilt_velocity_space;
    struct {
      float min, max;
    } continuous_zoom_velocity_space;
  } supported_ptz_spaces;
  int maximum_number_of_presets;
  int home_supported;
  char auxiliary_commands[256];
} mock_ptz_node_t;

/**
 * @brief Mock PTZ preset for testing
 */
typedef struct {
  const char* token;
  const char* name;
  const char* ptz_position;
} mock_ptz_preset_t;

/**
 * @brief Mock PTZ position for testing
 */
typedef struct {
  float pan;
  float tilt;
  float zoom;
  const char* space;
} mock_ptz_position_t;

extern const mock_ptz_node_t mock_ptz_nodes[];
extern const size_t mock_ptz_node_count;
extern const mock_ptz_preset_t mock_ptz_presets[];
extern const size_t mock_ptz_preset_count;
extern const mock_ptz_position_t mock_ptz_position_valid;

// ============================================================================
// Error Response Test Data
// ============================================================================

/**
 * @brief Mock fault response for testing
 */
typedef struct {
  const char* fault_code;
  const char* fault_string;
  const char* fault_detail;
  const char* fault_subcode;
} mock_fault_response_t;

extern const mock_fault_response_t mock_fault_invalid_parameter;
extern const mock_fault_response_t mock_fault_action_not_supported;
extern const mock_fault_response_t mock_fault_internal_error;
extern const mock_fault_response_t mock_fault_authentication_failed;

// ============================================================================
// Test Data Initialization
// ============================================================================

/**
 * @brief Initialize all test data
 * @return 0 on success, -1 on failure
 */
int response_test_data_init(void);

/**
 * @brief Cleanup all test data
 */
void response_test_data_cleanup(void);

// ============================================================================
// Test Data Validation
// ============================================================================

/**
 * @brief Validate device info test data
 * @param device_info Device info to validate
 * @return 1 if valid, 0 if invalid
 */
int validate_device_info_data(const mock_device_info_t* device_info);

/**
 * @brief Validate media profile test data
 * @param profile Profile to validate
 * @return 1 if valid, 0 if invalid
 */
int validate_media_profile_data(const mock_media_profile_t* profile);

/**
 * @brief Validate PTZ node test data
 * @param node PTZ node to validate
 * @return 1 if valid, 0 if invalid
 */
int validate_ptz_node_data(const mock_ptz_node_t* node);

/**
 * @brief Validate fault response test data
 * @param fault Fault response to validate
 * @return 1 if valid, 0 if invalid
 */
int validate_fault_response_data(const mock_fault_response_t* fault);

/**
 * @brief Validate stream URI test data
 * @param stream_uri Stream URI to validate
 * @return 1 if valid, 0 if invalid
 */
int validate_stream_uri_data(const mock_stream_uri_t* stream_uri);

/**
 * @brief Validate PTZ preset test data
 * @param preset PTZ preset to validate
 * @return 1 if valid, 0 if invalid
 */
int validate_ptz_preset_data(const mock_ptz_preset_t* preset);

/**
 * @brief Validate PTZ position test data
 * @param position PTZ position to validate
 * @return 1 if valid, 0 if invalid
 */
int validate_ptz_position_data(const mock_ptz_position_t* position);

#endif // RESPONSE_TEST_DATA_H
