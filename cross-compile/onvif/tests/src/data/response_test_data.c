/**
 * @file response_test_data.c
 * @brief Implementation of shared test data for gSOAP response generation tests
 * @author kkrzysztofik
 * @date 2025
 */

#include "response_test_data.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "utils/error/error_handling.h"

// ============================================================================
// Device Service Test Data
// ============================================================================

const mock_device_info_t mock_device_info_valid = {.manufacturer = "Anyka",
                                                   .model = "AK3918",
                                                   .firmware_version = "1.0.0",
                                                   .serial_number = "AK3918-12345",
                                                   .hardware_id = "AK3918-HW-001"};

const mock_device_info_t mock_device_info_empty = {
  .manufacturer = "", .model = "", .firmware_version = "", .serial_number = "", .hardware_id = ""};

const mock_device_info_t mock_device_info_null_strings = {.manufacturer = NULL,
                                                          .model = NULL,
                                                          .firmware_version = NULL,
                                                          .serial_number = NULL,
                                                          .hardware_id = NULL};

// ============================================================================
// Media Service Test Data
// ============================================================================

const mock_media_profile_t mock_profiles[] = {
  {.token = "Profile_1",
   .name = "Main Profile",
   .fixed = 0,
   .video_source = {.source_token = "VideoSource_1",
                    .bounds = {.width = 1920, .height = 1080, .x = 0, .y = 0}},
   .video_encoder = {.token = "VideoEncoder_1",
                     .encoding = "H264",
                     .width = 1920,
                     .height = 1080,
                     .frame_rate = 30,
                     .bitrate = 2000000},
   .audio_source = {.source_token = "AudioSource_1"},
   .audio_encoder =
     {.token = "AudioEncoder_1", .encoding = "G711", .sample_rate = 8000, .bitrate = 64000},
   .ptz_configuration = {.token = "PTZConfig_1"},
   .metadata_configuration = {.token = "MetadataConfig_1"}},
  {.token = "Profile_2",
   .name = "Secondary Profile",
   .fixed = 0,
   .video_source = {.source_token = "VideoSource_2",
                    .bounds = {.width = 1280, .height = 720, .x = 0, .y = 0}},
   .video_encoder = {.token = "VideoEncoder_2",
                     .encoding = "H264",
                     .width = 1280,
                     .height = 720,
                     .frame_rate = 25,
                     .bitrate = 1000000},
   .audio_source = {.source_token = "AudioSource_2"},
   .audio_encoder =
     {.token = "AudioEncoder_2", .encoding = "G711", .sample_rate = 8000, .bitrate = 64000},
   .ptz_configuration = {.token = "PTZConfig_2"},
   .metadata_configuration = {.token = "MetadataConfig_2"}},
  {.token = "Profile_3",
   .name = "PTZ Profile",
   .fixed = 0,
   .video_source = {.source_token = "VideoSource_3",
                    .bounds = {.width = 1920, .height = 1080, .x = 0, .y = 0}},
   .video_encoder = {.token = "VideoEncoder_3",
                     .encoding = "H264",
                     .width = 1920,
                     .height = 1080,
                     .frame_rate = 30,
                     .bitrate = 2000000},
   .audio_source = {.source_token = ""},
   .audio_encoder = {.token = "", .encoding = "", .sample_rate = 0, .bitrate = 0},
   .ptz_configuration = {.token = "PTZConfig_3"},
   .metadata_configuration = {.token = "MetadataConfig_3"}}};

const size_t mock_profile_count = sizeof(mock_profiles) / sizeof(mock_profiles[0]);

const mock_stream_uri_t mock_stream_uri_valid = {.uri = "rtsp://192.168.1.100:554/stream1",
                                                 .invalid_after_connect = 0,
                                                 .invalid_after_reboot = 0,
                                                 .timeout = 60};

const mock_stream_uri_t mock_stream_uri_empty = {
  .uri = "", .invalid_after_connect = 0, .invalid_after_reboot = 0, .timeout = 0};

const mock_video_source_config_t mock_video_source_config_valid = {
  .token = "VideoSource_1", .width = 1920, .height = 1080, .encoding = "JPEG", .frame_rate = 30};

const mock_video_encoder_config_t mock_video_encoder_config_valid = {.token = "VideoEncoder_1",
                                                                     .width = 1920,
                                                                     .height = 1080,
                                                                     .encoding = "H264",
                                                                     .frame_rate = 30,
                                                                     .bitrate = 2000000,
                                                                     .gop_size = 30};

const mock_metadata_config_t mock_metadata_configs[] = {{.token = "MetadataConfig_1",
                                                         .name = "Main Metadata",
                                                         .use_count = 1,
                                                         .analytics = "true",
                                                         .multicast = "false",
                                                         .session_timeout = "PT60S",
                                                         .analytics_engine_configuration = "true",
                                                         .extension = NULL},
                                                        {.token = "MetadataConfig_2",
                                                         .name = "Secondary Metadata",
                                                         .use_count = 2,
                                                         .analytics = "false",
                                                         .multicast = "true",
                                                         .session_timeout = "PT120S",
                                                         .analytics_engine_configuration = "false",
                                                         .extension = NULL}};

const size_t mock_metadata_config_count =
  sizeof(mock_metadata_configs) / sizeof(mock_metadata_configs[0]);

// ============================================================================
// PTZ Service Test Data
// ============================================================================

const mock_ptz_node_t mock_ptz_nodes[] = {
  {.token = "PTZNode_1",
   .name = "Main PTZ Node",
   .supported_ptz_spaces = {.absolute_pan_tilt_position_space =
                              {.x_min = -180.0F, .x_max = 180.0F, .y_min = -90.0F, .y_max = 90.0F},
                            .absolute_zoom_position_space = {.min = 0.0F, .max = 1.0F},
                            .relative_pan_tilt_translation_space =
                              {.x_min = -180.0F, .x_max = 180.0F, .y_min = -90.0F, .y_max = 90.0F},
                            .relative_zoom_translation_space = {.min = -1.0F, .max = 1.0F},
                            .continuous_pan_tilt_velocity_space =
                              {.x_min = -1.0F, .x_max = 1.0F, .y_min = -1.0F, .y_max = 1.0F},
                            .continuous_zoom_velocity_space = {.min = -1.0F, .max = 1.0F}},
   .maximum_number_of_presets = 10,
   .home_supported = 1,
   .auxiliary_commands = "Home,SetPreset,GotoPreset"},
  {.token = "PTZNode_2",
   .name = "Secondary PTZ Node",
   .supported_ptz_spaces = {.absolute_pan_tilt_position_space =
                              {.x_min = -90.0F, .x_max = 90.0F, .y_min = -45.0F, .y_max = 45.0F},
                            .absolute_zoom_position_space = {.min = 0.0F, .max = 0.5F},
                            .relative_pan_tilt_translation_space =
                              {.x_min = -90.0F, .x_max = 90.0F, .y_min = -45.0F, .y_max = 45.0F},
                            .relative_zoom_translation_space = {.min = -0.5F, .max = 0.5F},
                            .continuous_pan_tilt_velocity_space =
                              {.x_min = -0.5F, .x_max = 0.5F, .y_min = -0.5F, .y_max = 0.5F},
                            .continuous_zoom_velocity_space = {.min = -0.5F, .max = 0.5F}},
   .maximum_number_of_presets = 5,
   .home_supported = 0,
   .auxiliary_commands = "SetPreset,GotoPreset"}};

const size_t mock_ptz_node_count = sizeof(mock_ptz_nodes) / sizeof(mock_ptz_nodes[0]);

const mock_ptz_preset_t mock_ptz_presets[] = {
  {.token = "Preset_1", .name = "Home Position", .ptz_position = "0,0,0"},
  {.token = "Preset_2", .name = "Left Corner", .ptz_position = "-90,0,0"},
  {.token = "Preset_3", .name = "Right Corner", .ptz_position = "90,0,0"},
  {.token = "Preset_4", .name = "Zoom In", .ptz_position = "0,0,1.0"}};

const size_t mock_ptz_preset_count = sizeof(mock_ptz_presets) / sizeof(mock_ptz_presets[0]);

const mock_ptz_position_t mock_ptz_position_valid = {
  .pan = 45.0F,
  .tilt = 30.0F,
  .zoom = 0.5F,
  .space = "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace"};

// ============================================================================
// Error Response Test Data
// ============================================================================

const mock_fault_response_t mock_fault_invalid_parameter = {
  .fault_code = "soap:Client",
  .fault_string = "Invalid parameter",
  .fault_detail = "The parameter 'token' is invalid or missing",
  .fault_subcode = "onvif:InvalidParameter"};

const mock_fault_response_t mock_fault_action_not_supported = {
  .fault_code = "soap:Client",
  .fault_string = "Action not supported",
  .fault_detail = "The requested action is not supported by this device",
  .fault_subcode = "onvif:ActionNotSupported"};

const mock_fault_response_t mock_fault_internal_error = {
  .fault_code = "soap:Server",
  .fault_string = "Internal error",
  .fault_detail = "An internal error occurred while processing the request",
  .fault_subcode = "onvif:InternalError"};

const mock_fault_response_t mock_fault_authentication_failed = {
  .fault_code = "soap:Client",
  .fault_string = "Authentication failed",
  .fault_detail = "Invalid credentials provided",
  .fault_subcode = "onvif:AuthenticationFailed"};

// ============================================================================
// Test Data Initialization
// ============================================================================

int response_test_data_init(void) {
  // Initialize any dynamic test data if needed
  // For now, all data is static, so just return success
  return ONVIF_SUCCESS;
}

void response_test_data_cleanup(void) {
  // Cleanup any dynamic test data if needed
  // For now, all data is static, so nothing to clean up
}

// ============================================================================
// Test Data Validation
// ============================================================================

int validate_device_info_data(const mock_device_info_t* device_info) {
  if (!device_info) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid manufacturer (not NULL and not empty)
  if (!device_info->manufacturer || strlen(device_info->manufacturer) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid model (not NULL and not empty)
  if (!device_info->model || strlen(device_info->model) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Validate against expected values from mock_device_info_valid
  if (strcmp(device_info->manufacturer, mock_device_info_valid.manufacturer) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(device_info->model, mock_device_info_valid.model) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(device_info->firmware_version, mock_device_info_valid.firmware_version) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(device_info->serial_number, mock_device_info_valid.serial_number) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(device_info->hardware_id, mock_device_info_valid.hardware_id) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

int validate_media_profile_data(const mock_media_profile_t* profile) {
  if (!profile) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid token (not empty)
  if (strlen(profile->token) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid name (not empty)
  if (strlen(profile->name) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid video source token
  if (strlen(profile->video_source.source_token) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid video encoder token
  if (strlen(profile->video_encoder.token) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Validate against expected values from mock_profiles[0] (Main Profile)
  if (strcmp(profile->token, mock_profiles[0].token) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(profile->name, mock_profiles[0].name) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (profile->fixed != mock_profiles[0].fixed) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(profile->video_source.source_token, mock_profiles[0].video_source.source_token) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (profile->video_source.bounds.width != mock_profiles[0].video_source.bounds.width) {
    return ONVIF_ERROR_INVALID;
  }

  if (profile->video_source.bounds.height != mock_profiles[0].video_source.bounds.height) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(profile->video_encoder.token, mock_profiles[0].video_encoder.token) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(profile->video_encoder.encoding, mock_profiles[0].video_encoder.encoding) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (profile->video_encoder.width != mock_profiles[0].video_encoder.width) {
    return ONVIF_ERROR_INVALID;
  }

  if (profile->video_encoder.height != mock_profiles[0].video_encoder.height) {
    return ONVIF_ERROR_INVALID;
  }

  if (profile->video_encoder.frame_rate != mock_profiles[0].video_encoder.frame_rate) {
    return ONVIF_ERROR_INVALID;
  }

  if (profile->video_encoder.bitrate != mock_profiles[0].video_encoder.bitrate) {
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

int validate_ptz_node_data(const mock_ptz_node_t* node) {
  if (!node) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid token (not empty)
  if (strlen(node->token) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid name (not empty)
  if (strlen(node->name) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for positive values
  if (node->maximum_number_of_presets < 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Validate against expected values from mock_ptz_nodes[0] (Main PTZ Node)
  if (strcmp(node->token, mock_ptz_nodes[0].token) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(node->name, mock_ptz_nodes[0].name) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (node->maximum_number_of_presets != mock_ptz_nodes[0].maximum_number_of_presets) {
    return ONVIF_ERROR_INVALID;
  }

  if (node->home_supported != mock_ptz_nodes[0].home_supported) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(node->auxiliary_commands, mock_ptz_nodes[0].auxiliary_commands) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Validate PTZ spaces with floating point comparison (using epsilon)
  const float epsilon = 0.001F;

  if (fabsf(node->supported_ptz_spaces.absolute_pan_tilt_position_space.x_min -
            mock_ptz_nodes[0].supported_ptz_spaces.absolute_pan_tilt_position_space.x_min) >
      epsilon) {
    return ONVIF_ERROR_INVALID;
  }

  if (fabsf(node->supported_ptz_spaces.absolute_pan_tilt_position_space.x_max -
            mock_ptz_nodes[0].supported_ptz_spaces.absolute_pan_tilt_position_space.x_max) >
      epsilon) {
    return ONVIF_ERROR_INVALID;
  }

  if (fabsf(node->supported_ptz_spaces.absolute_pan_tilt_position_space.y_min -
            mock_ptz_nodes[0].supported_ptz_spaces.absolute_pan_tilt_position_space.y_min) >
      epsilon) {
    return ONVIF_ERROR_INVALID;
  }

  if (fabsf(node->supported_ptz_spaces.absolute_pan_tilt_position_space.y_max -
            mock_ptz_nodes[0].supported_ptz_spaces.absolute_pan_tilt_position_space.y_max) >
      epsilon) {
    return ONVIF_ERROR_INVALID;
  }

  if (fabsf(node->supported_ptz_spaces.absolute_zoom_position_space.min -
            mock_ptz_nodes[0].supported_ptz_spaces.absolute_zoom_position_space.min) > epsilon) {
    return ONVIF_ERROR_INVALID;
  }

  if (fabsf(node->supported_ptz_spaces.absolute_zoom_position_space.max -
            mock_ptz_nodes[0].supported_ptz_spaces.absolute_zoom_position_space.max) > epsilon) {
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

int validate_fault_response_data(const mock_fault_response_t* fault) {
  if (!fault) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid fault code (not NULL and not empty)
  if (!fault->fault_code || strlen(fault->fault_code) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid fault string (not NULL and not empty)
  if (!fault->fault_string || strlen(fault->fault_string) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Validate against expected values from mock_fault_invalid_parameter
  if (strcmp(fault->fault_code, mock_fault_invalid_parameter.fault_code) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(fault->fault_string, mock_fault_invalid_parameter.fault_string) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(fault->fault_detail, mock_fault_invalid_parameter.fault_detail) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(fault->fault_subcode, mock_fault_invalid_parameter.fault_subcode) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

int validate_stream_uri_data(const mock_stream_uri_t* stream_uri) {
  if (!stream_uri) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid URI (not empty)
  if (strlen(stream_uri->uri) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Validate against expected values from mock_stream_uri_valid
  if (strcmp(stream_uri->uri, mock_stream_uri_valid.uri) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (stream_uri->invalid_after_connect != mock_stream_uri_valid.invalid_after_connect) {
    return ONVIF_ERROR_INVALID;
  }

  if (stream_uri->invalid_after_reboot != mock_stream_uri_valid.invalid_after_reboot) {
    return ONVIF_ERROR_INVALID;
  }

  if (stream_uri->timeout != mock_stream_uri_valid.timeout) {
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

int validate_ptz_preset_data(const mock_ptz_preset_t* preset) {
  if (!preset) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid token (not empty)
  if (strlen(preset->token) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Check for valid name (not empty)
  if (strlen(preset->name) == 0) {
    return ONVIF_ERROR_INVALID;
  }

  // Validate against expected values from mock_ptz_presets[0] (Home Position)
  if (strcmp(preset->token, mock_ptz_presets[0].token) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(preset->name, mock_ptz_presets[0].name) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(preset->ptz_position, mock_ptz_presets[0].ptz_position) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}

int validate_ptz_position_data(const mock_ptz_position_t* position) {
  if (!position) {
    return ONVIF_ERROR_INVALID;
  }

  // Validate against expected values from mock_ptz_position_valid
  const float epsilon = 0.001F;

  if (fabsf(position->pan - mock_ptz_position_valid.pan) > epsilon) {
    return ONVIF_ERROR_INVALID;
  }

  if (fabsf(position->tilt - mock_ptz_position_valid.tilt) > epsilon) {
    return ONVIF_ERROR_INVALID;
  }

  if (fabsf(position->zoom - mock_ptz_position_valid.zoom) > epsilon) {
    return ONVIF_ERROR_INVALID;
  }

  if (strcmp(position->space, mock_ptz_position_valid.space) != 0) {
    return ONVIF_ERROR_INVALID;
  }

  return ONVIF_SUCCESS;
}
