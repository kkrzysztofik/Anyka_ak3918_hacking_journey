/**
 * @file platform_mock.h
 * @brief CMocka-based platform mock header using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef PLATFORM_MOCK_H
#define PLATFORM_MOCK_H

#include "cmocka_wrapper.h"
#include "platform/platform.h"
#include "platform/platform_common.h"

/* ============================================================================
 * CMocka Wrapped Platform Functions
 * ============================================================================
 * All platform functions are wrapped using CMocka's --wrap linker mechanism.
 * Tests use will_return() and expect_*() to configure mock behavior.
 */

/**
 * @brief CMocka wrapped platform initialization
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_init(void);

/**
 * @brief CMocka wrapped platform cleanup
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_cleanup(void);

/**
 * @brief CMocka wrapped platform error logging
 * @param format Format string
 * @param ... Variable arguments
 * @return Number of characters written (configured via will_return)
 */
int __wrap_platform_log_error(const char* format, ...);

/**
 * @brief CMocka wrapped platform warning logging
 * @param format Format string
 * @param ... Variable arguments
 * @return Number of characters written (configured via will_return)
 */
int __wrap_platform_log_warning(const char* format, ...);

/**
 * @brief CMocka wrapped platform notice logging
 * @param format Format string
 * @param ... Variable arguments
 * @return Number of characters written (configured via will_return)
 */
int __wrap_platform_log_notice(const char* format, ...);

/**
 * @brief CMocka wrapped platform info logging
 * @param format Format string
 * @param ... Variable arguments
 * @return Number of characters written (configured via will_return)
 */
int __wrap_platform_log_info(const char* format, ...);

/**
 * @brief CMocka wrapped platform debug logging
 * @param format Format string
 * @param ... Variable arguments
 * @return Number of characters written (configured via will_return)
 */
int __wrap_platform_log_debug(const char* format, ...);

/**
 * @brief CMocka wrapped video input sensor matching
 * @param sensor Sensor information
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vi_match_sensor(void* sensor);

/**
 * @brief CMocka wrapped video input open
 * @param vi_handle Output VI handle pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vi_open(void** vi_handle);

/**
 * @brief CMocka wrapped video input close
 * @param vi_handle VI handle to close
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vi_close(void* vi_handle);

/**
 * @brief CMocka wrapped get sensor resolution
 * @param resolution Output resolution pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vi_get_sensor_resolution(platform_video_resolution_t* resolution);

/**
 * @brief CMocka wrapped day/night mode switch
 * @param mode Day/night mode
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vi_switch_day_night(platform_daynight_mode_t mode);

/**
 * @brief CMocka wrapped flip/mirror setting
 * @param flip Flip enable flag
 * @param mirror Mirror enable flag
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vi_set_flip_mirror(int flip, int mirror);

/**
 * @brief CMocka wrapped capture on
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vi_capture_on(void);

/**
 * @brief CMocka wrapped start global capture
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vi_start_global_capture(void);

/**
 * @brief CMocka wrapped capture off
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vi_capture_off(void);

/**
 * @brief CMocka wrapped set channel attributes
 * @param channel Channel number
 * @param attr Channel attributes
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vi_set_channel_attr(int channel, void* attr);

/**
 * @brief CMocka wrapped get FPS
 * @param fps Output FPS pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vi_get_fps(int* fps);

/**
 * @brief CMocka wrapped VPSS effect set
 * @param effect_type Effect type
 * @param value Effect value
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vpss_effect_set(int effect_type, int value);

/**
 * @brief CMocka wrapped VPSS effect get
 * @param effect_type Effect type
 * @param value Output value pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_vpss_effect_get(int effect_type, int* value);

/**
 * @brief CMocka wrapped video encoder initialization
 * @param venc_handle Output VENC handle pointer
 * @param config Encoder configuration
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_venc_init(void** venc_handle, void* config);

/**
 * @brief CMocka wrapped VENC config validation
 * @param config Configuration to validate
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_validate_venc_config(void* config);

/**
 * @brief CMocka wrapped video encoder cleanup
 * @param venc_handle VENC handle to cleanup
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_venc_cleanup(void* venc_handle);

/**
 * @brief CMocka wrapped VENC get frame
 * @param venc_handle VENC handle
 * @param frame Output frame pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_venc_get_frame(void* venc_handle, void* frame);

/**
 * @brief CMocka wrapped VENC release frame
 * @param venc_handle VENC handle
 * @param frame Frame to release
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_venc_release_frame(void* venc_handle, void* frame);

/**
 * @brief CMocka wrapped VENC get stream
 * @param venc_handle VENC handle
 * @param stream Output stream pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_venc_get_stream(void* venc_handle,
                                                  platform_venc_stream_t* stream);

/**
 * @brief CMocka wrapped VENC release stream
 * @param venc_handle VENC handle
 * @param stream Stream to release
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_venc_release_stream(void* venc_handle,
                                                      platform_venc_stream_t* stream);

/**
 * @brief CMocka wrapped VENC request stream
 * @param venc_handle VENC handle
 * @param stream_handle Output stream handle pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_venc_request_stream(void* venc_handle, void** stream_handle);

/**
 * @brief CMocka wrapped VENC cancel stream
 * @param stream_handle Stream handle to cancel
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_venc_cancel_stream(void* stream_handle);

/**
 * @brief CMocka wrapped VENC get stream by handle
 * @param stream_handle Stream handle
 * @param stream Output stream pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_venc_get_stream_by_handle(void* stream_handle,
                                                            platform_venc_stream_t* stream);

/**
 * @brief CMocka wrapped VENC release stream by handle
 * @param stream_handle Stream handle
 * @param stream Stream to release
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_venc_release_stream_by_handle(void* stream_handle,
                                                                platform_venc_stream_t* stream);

/**
 * @brief CMocka wrapped VENC get buffer status
 * @param venc_handle VENC handle
 * @param buffer_count Output buffer count pointer
 * @param max_buffers Output max buffers pointer
 * @param overflow_count Output overflow count pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_venc_get_buffer_status(void* venc_handle, int* buffer_count,
                                                         int* max_buffers, int* overflow_count);

/**
 * @brief CMocka wrapped audio input open
 * @param ai_handle Output AI handle pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_ai_open(void** ai_handle);

/**
 * @brief CMocka wrapped audio input close
 * @param ai_handle AI handle to close
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_ai_close(void* ai_handle);

/**
 * @brief CMocka wrapped audio encoder initialization
 * @param aenc_handle Output AENC handle pointer
 * @param config Encoder configuration
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_aenc_init(void** aenc_handle, void* config);

/**
 * @brief CMocka wrapped audio encoder cleanup
 * @param aenc_handle AENC handle to cleanup
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_aenc_cleanup(void* aenc_handle);

/**
 * @brief CMocka wrapped AENC get frame
 * @param aenc_handle AENC handle
 * @param frame Output frame pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_aenc_get_frame(void* aenc_handle, void* frame);

/**
 * @brief CMocka wrapped AENC release frame
 * @param aenc_handle AENC handle
 * @param frame Frame to release
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_aenc_release_frame(void* aenc_handle, void* frame);

/**
 * @brief CMocka wrapped AENC get stream
 * @param aenc_handle AENC handle
 * @param stream Output stream pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_aenc_get_stream(void* aenc_handle,
                                                  platform_aenc_stream_t* stream);

/**
 * @brief CMocka wrapped AENC release stream
 * @param aenc_handle AENC handle
 * @param stream Stream to release
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_aenc_release_stream(void* aenc_handle,
                                                      platform_aenc_stream_t* stream);

/**
 * @brief CMocka wrapped PTZ initialization
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_ptz_init(void);

/**
 * @brief CMocka wrapped PTZ cleanup
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_ptz_cleanup(void);

/**
 * @brief CMocka wrapped PTZ set degree
 * @param pan_range_deg Pan range degree
 * @param tilt_range_deg Tilt range degree
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_ptz_set_degree(int pan_range_deg, int tilt_range_deg);

/**
 * @brief CMocka wrapped PTZ self check
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_ptz_check_self(void);

/**
 * @brief CMocka wrapped PTZ move to position
 * @param pan_deg Pan degree value
 * @param tilt_deg Tilt degree value
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_ptz_move_to_position(int pan_deg, int tilt_deg);

/**
 * @brief CMocka wrapped PTZ get step position
 * @param axis PTZ axis type
 * @return Step position value
 */
int __wrap_platform_ptz_get_step_position(platform_ptz_axis_t axis);

/**
 * @brief CMocka wrapped PTZ get status
 * @param axis PTZ axis type
 * @param status Output status pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_ptz_get_status(platform_ptz_axis_t axis,
                                                 platform_ptz_status_t* status);

/**
 * @brief CMocka wrapped PTZ set speed
 * @param axis_type PTZ axis type
 * @param speed_value Speed value
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_ptz_set_speed(platform_ptz_axis_t axis_type, int speed_value);

/**
 * @brief CMocka wrapped PTZ turn
 * @param direction Turn direction (platform_ptz_direction_t)
 * @param steps Number of steps to turn
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_ptz_turn(platform_ptz_direction_t direction, int steps);

/**
 * @brief CMocka wrapped PTZ turn stop
 * @param direction Direction to stop (platform_ptz_direction_t)
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_ptz_turn_stop(platform_ptz_direction_t direction);

/**
 * @brief CMocka wrapped IR LED initialization
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_irled_init(void);

/**
 * @brief CMocka wrapped IR LED cleanup
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_irled_cleanup(void);

/**
 * @brief CMocka wrapped IR LED set mode
 * @param mode IR LED mode
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_irled_set_mode(int mode);

/**
 * @brief CMocka wrapped IR LED get status
 * @param status Output status pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_irled_get_status(int* status);

/**
 * @brief CMocka wrapped snapshot initialization
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_snapshot_init(void);

/**
 * @brief CMocka wrapped snapshot cleanup
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_snapshot_cleanup(void);

/**
 * @brief CMocka wrapped snapshot capture
 * @param handle Snapshot handle
 * @param snapshot Output snapshot pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_snapshot_capture(platform_snapshot_handle_t handle,
                                                   platform_snapshot_t* snapshot);

/**
 * @brief CMocka wrapped snapshot release
 * @param handle Snapshot handle
 * @param snapshot Snapshot to release
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_snapshot_release(platform_snapshot_handle_t handle,
                                                   platform_snapshot_t* snapshot);

/**
 * @brief CMocka wrapped config load
 * @param filename Configuration filename
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_config_load(const char* filename);

/**
 * @brief CMocka wrapped config save
 * @param filename Configuration filename
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_config_save(const char* filename);

/**
 * @brief CMocka wrapped config get string
 * @param key Configuration key
 * @param value Output value buffer
 * @param size Buffer size
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_config_get_string(const char* key, char* value, size_t size);

/**
 * @brief CMocka wrapped config get int
 * @param key Configuration key
 * @param value Output value pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_config_get_int(const char* key, int* value);

/**
 * @brief CMocka wrapped get system info
 * @param info Output system info pointer
 * @return Platform result code (configured via will_return)
 */
platform_result_t __wrap_platform_get_system_info(platform_system_info_t* info);

/**
 * @brief CMocka wrapped platform sleep milliseconds
 * @param ms Milliseconds to sleep
 */
void __wrap_platform_sleep_ms(unsigned int ms);

/* ============================================================================
 * CMocka Test Helper Macros
 * ============================================================================ */

/**
 * @brief Set up expectations for successful platform initialization
 */
#define EXPECT_PLATFORM_INIT_SUCCESS() will_return(__wrap_platform_init, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for platform cleanup
 */
#define EXPECT_PLATFORM_CLEANUP() will_return(__wrap_platform_cleanup, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for platform info log
 * @param msg Expected log message
 */
#define EXPECT_PLATFORM_LOG_INFO(msg)                                                              \
  expect_string(__wrap_platform_log_info, format, msg);                                            \
  will_return(__wrap_platform_log_info, 0)

/**
 * @brief Set up expectations for platform error log
 * @param msg Expected error message
 */
#define EXPECT_PLATFORM_LOG_ERROR(msg)                                                             \
  expect_string(__wrap_platform_log_error, format, msg);                                           \
  will_return(__wrap_platform_log_error, 0)

/**
 * @brief Set up expectations for platform warning log
 * @param msg Expected warning message
 */
#define EXPECT_PLATFORM_LOG_WARNING(msg)                                                           \
  expect_string(__wrap_platform_log_warning, format, msg);                                         \
  will_return(__wrap_platform_log_warning, 0)

/**
 * @brief Set up expectations for platform debug log
 * @param msg Expected debug message
 */
#define EXPECT_PLATFORM_LOG_DEBUG(msg)                                                             \
  expect_string(__wrap_platform_log_debug, format, msg);                                           \
  will_return(__wrap_platform_log_debug, 0)

/**
 * @brief Set up expectations for PTZ initialization success
 */
#define EXPECT_PTZ_INIT_SUCCESS() will_return(__wrap_platform_ptz_init, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for PTZ cleanup
 */
#define EXPECT_PTZ_CLEANUP() will_return(__wrap_platform_ptz_cleanup, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for VI open success
 */
#define EXPECT_VI_OPEN_SUCCESS() will_return(__wrap_platform_vi_open, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for VI close
 */
#define EXPECT_VI_CLOSE() will_return(__wrap_platform_vi_close, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for VENC initialization success
 */
#define EXPECT_VENC_INIT_SUCCESS() will_return(__wrap_platform_venc_init, PLATFORM_SUCCESS)

/**
 * @brief Set up expectations for VENC cleanup
 */
#define EXPECT_VENC_CLEANUP() will_return(__wrap_platform_venc_cleanup, PLATFORM_SUCCESS)

#endif // PLATFORM_MOCK_H
