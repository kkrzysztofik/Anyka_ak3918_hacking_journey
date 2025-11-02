/**
 * @file platform_anyka.c
 * @brief Anyka-specific implementation of unified platform abstraction layer
 * @author kkrzysztofik
 * @date 2025
 *
 * This file consolidates the HAL and platform abstraction functionality
 * into a single implementation for the Anyka AK3918 platform.
 */

#define _GNU_SOURCE
#ifndef _POSIX_C_SOURCE
#define _POSIX_C_SOURCE 200809L
#endif

#include <bits/pthreadtypes.h>
#include <bits/types.h>
#include <errno.h>
#include <pthread.h>
#include <signal.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#include "ak_aenc.h"
#include "ak_ai.h"
#include "ak_drv_irled.h"
#include "ak_drv_ptz.h"
#include "ak_error.h"
#include "ak_global.h"
#include "ak_venc.h"
#include "ak_vi.h"
#include "ak_vpss.h"
#include "list.h"
#include "platform/platform.h"
#include "platform/platform_common.h"
#include "utils/common/time_utils.h"
#include "utils/logging/platform_logging.h"

/* Error type constants for enhanced error logging */
#define ERROR_TYPE_POINTER_NULL  1
#define ERROR_TYPE_MALLOC_FAILED 2
#define ERROR_TYPE_NO_DATA       3
#define ERROR_TYPE_INVALID_USER  4

/* Platform timing and delay constants (milliseconds) */
#define PLATFORM_DELAY_MS_SHORT             10    /* Short delay */
#define PLATFORM_DELAY_MS_MEDIUM            100   /* Medium delay for encoder startup */
#define PLATFORM_DELAY_MS_RETRY             200   /* Retry delay */
#define PLATFORM_DELAY_MS_LONG              300   /* Long delay before retry */
#define PLATFORM_DELAY_MS_VI_INIT           500   /* VI initialization delay */
#define PLATFORM_DELAY_MS_STATS_INTERVAL    2000  /* Statistics calculation interval */
#define PLATFORM_DELAY_MS_MAX_BITRATE_VALID 20000 /* Maximum bitrate validation timeout */

/* Platform retry and timeout constants */
#define PLATFORM_RETRY_COUNT_MAX          10  /* Maximum retry attempts */
#define PLATFORM_RETRY_DELAY_BASE_MS      10  /* Base retry delay */
#define PLATFORM_RETRY_DELAY_INCREMENT_MS 5   /* Retry delay increment per attempt */
#define PLATFORM_TIMEOUT_ITERATIONS_MAX   100 /* Maximum timeout loop iterations */

/* Video resolution constants (pixels) */
#define PLATFORM_VIDEO_WIDTH_HD      1280 /* HD video width */
#define PLATFORM_VIDEO_HEIGHT_HD     720  /* HD video height */
#define PLATFORM_VIDEO_WIDTH_VGA     640  /* VGA video width */
#define PLATFORM_VIDEO_HEIGHT_VGA    480  /* VGA video height */
#define PLATFORM_VIDEO_DIMENSION_MAX 4096 /* Maximum video dimension */

/* Video encoder FPS constants */
#define PLATFORM_VIDEO_FPS_MIN      1  /* Minimum FPS */
#define PLATFORM_VIDEO_FPS_MAX      60 /* Maximum FPS */
#define PLATFORM_VIDEO_FPS_SNAPSHOT 10 /* Snapshot FPS */

/* Video encoder bitrate constants (kbps) */
#define PLATFORM_VIDEO_BITRATE_MIN      100   /* Minimum bitrate */
#define PLATFORM_VIDEO_BITRATE_MAX      10000 /* Maximum bitrate */
#define PLATFORM_VIDEO_BITRATE_SNAPSHOT 1000  /* Snapshot bitrate */

/* Video encoder QP (Quantization Parameter) constants */
#define PLATFORM_VIDEO_QP_MIN_DEFAULT 20 /* Minimum QP for video */
#define PLATFORM_VIDEO_QP_MAX_DEFAULT 45 /* Maximum QP for video */
#define PLATFORM_VIDEO_QP_MAX_JPEG    51 /* Maximum QP for JPEG/snapshot */

/* Video encoder GOP constants */
#define PLATFORM_VIDEO_GOP_DEFAULT 50 /* Default GOP length */

/* Buffer size constants (bytes) */
#define PLATFORM_BUFFER_SIZE_SMALL  32   /* Small buffer size */
#define PLATFORM_BUFFER_SIZE_MEDIUM 256  /* Medium buffer size */
#define PLATFORM_BUFFER_SIZE_PAGE   4096 /* Page-aligned buffer size */

/* Stream buffer constants */
#define PLATFORM_STREAM_BUFFER_MAX 50 /* Maximum stream buffers (ONE_STREAM_MAX) */

/* Buffer usage thresholds (floating point ratios) */
#define PLATFORM_BUFFER_USAGE_HIGH_THRESHOLD 0.8F /* 80% buffer usage warning threshold */

/* Time conversion constants */
#define PLATFORM_TIME_MS_PER_SECOND      1000  /* Milliseconds per second */
#define PLATFORM_TIME_US_PER_MS          1000  /* Microseconds per millisecond */
#define PLATFORM_TIME_US_CONVERSION      10000 /* Microsecond conversion factor */
#define PLATFORM_TIME_SECONDS_PER_MINUTE 60    /* Seconds per minute / degrees per unit */

/* Memory conversion constants */
#define PLATFORM_MEMORY_KB_TO_BYTES 1024 /* Kilobytes to bytes */

/* Temperature conversion constants */
#define PLATFORM_TEMP_MILLI_TO_UNIT 1000.0F /* Millidegrees to degrees */

/* Bitrate conversion constants */
#define PLATFORM_BITRATE_BITS_PER_BYTE 8ULL /* Bits per byte */
#define PLATFORM_BITRATE_CONVERSION    1000 /* Bitrate conversion factor */

/* String parsing constants */
#define PLATFORM_PARSE_BASE_DECIMAL   10 /* Decimal base for parsing */
#define PLATFORM_MEMINFO_TOTAL_OFFSET 9  /* "MemTotal:" string length */
#define PLATFORM_MEMINFO_AVAIL_OFFSET 13 /* "MemAvailable:" string length */

/* Memory address validation constants */
#define PLATFORM_MEMORY_ADDR_MIN 0x1000     /* Minimum valid memory address */
#define PLATFORM_MEMORY_ADDR_MAX 0xFFFFFFFF /* Maximum valid memory address */

/* Percentage constants */
#define PLATFORM_PERCENTAGE_MULTIPLIER 100 /* Percentage multiplier */
#define PLATFORM_PERCENTAGE_THRESHOLD  50  /* Common percentage threshold */

#include "utils/memory/memory_manager.h"

/* Forward declarations */
static int get_stream_with_retry(void* stream_handle, struct video_stream* anyka_stream, uint32_t timeout_ms);
static bool lock_platform_mutex(void);
static void unlock_platform_mutex(void);

/* Stream utility functions - consolidate common stream operations */
static platform_result_t get_video_stream_internal(void* stream_handle, platform_venc_stream_t* stream, uint32_t timeout_ms, bool is_stream_handle);
static void release_video_stream_internal(void* stream_handle, platform_venc_stream_t* stream);

/* Platform state with mutex protection for thread safety */
static bool g_platform_initialized = false;         // NOLINT
static platform_vi_handle_t g_vi_handle = NULL;     // NOLINT
static platform_venc_handle_t g_venc_handle = NULL; // NOLINT
static platform_ai_handle_t g_ai_handle = NULL;     // NOLINT
static platform_aenc_handle_t g_aenc_handle = NULL; // NOLINT

/* Thread-safe counters and flags with mutex protection */
static uint32_t g_encoder_active_count = 0;     // NOLINT
static uint32_t g_audio_active_count = 0;       // NOLINT
static bool g_cleanup_in_progress = false;      // NOLINT
static pthread_mutex_t g_platform_state_mutex = // NOLINT
  PTHREAD_MUTEX_INITIALIZER;                    // NOLINT(cppcoreguidelines-avoid-non-const-global-variables)LIT
/* Enhanced video encoder statistics and monitoring - following ak_venc.c
 * reference */
typedef struct {
  uint32_t total_bytes;           /* Total encoded bytes */
  uint32_t bitrate_kbps;          /* Current bitrate in kbps */
  uint32_t frame_count;           /* Total encoded frame count */
  float fps;                      /* Current frames per second */
  uint32_t gop_length;            /* GOP length */
  uint32_t gop_factor;            /* GOP factor for statistics */
  uint64_t start_timestamp;       /* Statistics start timestamp */
  uint64_t last_calc_time;        /* Last calculation time */
  uint32_t stream_overflow_count; /* Stream buffer overflow count */
  uint32_t dropped_frames;        /* Number of dropped frames */
  uint32_t i_frame_count;         /* I-frame count */
  uint32_t p_frame_count;         /* P-frame count */
  uint32_t b_frame_count;         /* B-frame count */
  uint32_t max_frame_size;        /* Maximum frame size encountered */
  uint32_t min_frame_size;        /* Minimum frame size encountered */
  uint64_t last_frame_timestamp;  /* Last frame timestamp */
  uint32_t consecutive_errors;    /* Consecutive error count */
  bool statistics_active;         /* Whether statistics collection is active */
} platform_venc_statistics_t;

/* Video encoder performance monitoring */
typedef struct {
  uint64_t capture_start_time;   /* Capture thread start time */
  uint64_t encode_start_time;    /* Encode thread start time */
  uint32_t capture_frame_count;  /* Frames captured */
  uint32_t encode_frame_count;   /* Frames encoded */
  uint32_t capture_errors;       /* Capture errors */
  uint32_t encode_errors;        /* Encode errors */
  uint32_t sensor_fps;           /* Current sensor FPS */
  uint32_t previous_sensor_fps;  /* Previous sensor FPS */
  bool fps_switch_detected;      /* FPS switch detected flag */
  uint64_t last_fps_switch_time; /* Last FPS switch time */
} platform_venc_performance_t;

/* Global video encoder statistics and performance monitoring */
static platform_venc_statistics_t g_venc_stats = {0}; // NOLINT
static platform_venc_performance_t g_venc_perf = {0}; // NOLINT
static pthread_mutex_t g_venc_stats_mutex =           // NOLINT(cppcoreguidelines-avoid-non-const-global-variables)
  PTHREAD_MUTEX_INITIALIZER;

/* Audio stream context structure for proper lifecycle management */
struct audio_stream_context {
  platform_ai_handle_t ai_handle;              /* Audio input handle */
  platform_aenc_handle_t aenc_handle;          /* Audio encoder handle */
  platform_aenc_stream_handle_t stream_handle; /* Stream handle from binding */
  bool initialized;                            /* Whether the context is initialized */
};

/* Snapshot context structure */
struct snapshot_context {
  platform_vi_handle_t vi_handle;
  int width;
  int height;
  void* jpeg_encoder;
};

/* Internal helper functions - forward declarations */

/* Enhanced video encoder logging and statistics functions */
static void platform_venc_log_statistics(const platform_venc_statistics_t* stats, const char* context);
static void platform_venc_log_performance(const platform_venc_performance_t* perf, const char* context);
static void platform_venc_update_statistics(platform_venc_statistics_t* statistics, uint32_t frame_size_bytes, uint32_t frame_type_code,
                                            uint64_t timestamp_ms);
static void platform_venc_log_encoder_parameters(const struct encode_param* param, const char* context);
static void platform_venc_log_stream_info(const struct video_stream* stream, const char* context);
static void platform_venc_log_error_context(int error_code, const char* operation, void* handle);
static void platform_venc_log_buffer_status(uint32_t buffer_count, uint32_t max_buffers, uint32_t overflow_count, const char* context);

static int map_video_codec(platform_video_codec_t codec) {
  switch (codec) {
  case PLATFORM_VIDEO_CODEC_H264:
    return H264_ENC_TYPE;
  case PLATFORM_VIDEO_CODEC_H265:
    return HEVC_ENC_TYPE;
  case PLATFORM_VIDEO_CODEC_MJPEG:
    return MJPEG_ENC_TYPE;
  default:
    return -1;
  }
}

static int map_platform_profile(int platform_profile) {
  switch (platform_profile) {
  case PLATFORM_PROFILE_MAIN:
  case PLATFORM_PROFILE_BASELINE:
  case PLATFORM_PROFILE_HIGH:
    return PROFILE_MAIN; // Use MAIN as fallback for all profiles
  default:
    return -1;
  }
}

static int map_platform_br_mode(int platform_br_mode) {
  switch (platform_br_mode) {
  case PLATFORM_BR_MODE_CBR:
    return BR_MODE_CBR;
  case PLATFORM_BR_MODE_VBR:
    return BR_MODE_VBR;
  default:
    return -1;
  }
}

static int map_vpss_effect(platform_vpss_effect_t effect) {
  switch (effect) {
  case PLATFORM_VPSS_EFFECT_BRIGHTNESS:
    return VPSS_EFFECT_BRIGHTNESS;
  case PLATFORM_VPSS_EFFECT_CONTRAST:
    return VPSS_EFFECT_CONTRAST;
  case PLATFORM_VPSS_EFFECT_SATURATION:
    return VPSS_EFFECT_SATURATION;
  case PLATFORM_VPSS_EFFECT_SHARPNESS:
    return VPSS_EFFECT_SHARP;
  case PLATFORM_VPSS_EFFECT_HUE:
    return VPSS_EFFECT_HUE;
  default:
    return -1;
  }
}

static int map_daynight_mode(platform_daynight_mode_t mode) {
  switch (mode) {
  case PLATFORM_DAYNIGHT_DAY:
    return VI_MODE_DAY;
  case PLATFORM_DAYNIGHT_NIGHT:
    return VI_MODE_NIGHT;
  case PLATFORM_DAYNIGHT_AUTO:
    return VI_MODE_DAY; // Use DAY as default for AUTO
  default:
    return -1;
  }
}

/* Platform initialization */
platform_result_t platform_init(void) {
  // Use mutex to prevent double initialization
  if (!lock_platform_mutex()) {
    return PLATFORM_ERROR;
  }

  if (g_platform_initialized) {
    unlock_platform_mutex();
    platform_log_debug("Platform already initialized\n");
    return PLATFORM_SUCCESS;
  }

  // Initialize memory manager
  if (memory_manager_init() != 0) {
    platform_log_error("Failed to initialize memory manager\n");
    unlock_platform_mutex();
    return PLATFORM_ERROR;
  }

  // Initialize counters
  g_encoder_active_count = 0;
  g_audio_active_count = 0;
  g_cleanup_in_progress = false;
  g_platform_initialized = true;

  unlock_platform_mutex();

  platform_log_info("Unified platform abstraction initialized for Anyka\n");
  return PLATFORM_SUCCESS;
}

void platform_cleanup(void) {
  if (!lock_platform_mutex()) {
    return;
  }

  if (!g_platform_initialized) {
    unlock_platform_mutex();
    platform_log_debug("Platform not initialized, nothing to cleanup\n");
    return;
  }

  // Set cleanup flag to prevent new operations
  g_cleanup_in_progress = true;

  unlock_platform_mutex();

  // Wait for active operations to complete (with timeout)
  uint32_t timeout_count = 0;
  while (timeout_count < PLATFORM_TIMEOUT_ITERATIONS_MAX) {
    if (lock_platform_mutex()) {
      bool active_ops = (g_encoder_active_count > 0 || g_audio_active_count > 0);
      unlock_platform_mutex();

      if (!active_ops) {
        break;
      }
    }

    sleep_ms(PLATFORM_DELAY_MS_SHORT);
    timeout_count++;
  }

  if (timeout_count >= PLATFORM_TIMEOUT_ITERATIONS_MAX) {
    platform_log_warning("Platform cleanup: Timeout waiting for active operations to "
                         "complete\n");
  }

  platform_log_debug("Platform cleanup: Cleaning up resources\n");

  // CRITICAL: Stop video capture before closing video input to prevent hang
  if (g_vi_handle) {
    platform_log_info("Stopping video capture before cleanup...\n");

    // Try to stop video capture with retry mechanism
    int capture_result = -1;
    int retry_count = 0;
    const int max_retries = 3;

    while (retry_count < max_retries && capture_result != 0) {
      capture_result = ak_vi_capture_off(g_vi_handle);
      if (capture_result != 0) {
        platform_log_warning("platform_cleanup: ak_vi_capture_off attempt %d failed "
                             "(result=%d)\n",
                             retry_count + 1, capture_result);
        if (retry_count < max_retries - 1) {
          sleep_ms(PLATFORM_DELAY_MS_RETRY); // Wait before retry
        }
      } else {
        platform_log_debug("platform_cleanup: Video capture stopped successfully\n");
      }
      retry_count++;
    }

    if (capture_result != 0) {
      platform_log_warning("platform_cleanup: ak_vi_capture_off failed after %d attempts "
                           "(result=%d), continuing anyway\n",
                           max_retries, capture_result);
    }

    // Give the system more time to process the capture stop and clean up
    // internal threads
    platform_log_debug("platform_cleanup: Waiting for VI system to stabilize...\n");
    sleep_ms(PLATFORM_DELAY_MS_VI_INIT); // Increased delay for thread cleanup
  }

  platform_venc_cleanup(g_venc_handle);
  platform_aenc_cleanup(g_aenc_handle);
  platform_vi_close(g_vi_handle);
  platform_ai_close(g_ai_handle);
  platform_ptz_cleanup();
  platform_irled_cleanup();

  // Cleanup memory manager
  memory_manager_cleanup();

  // Reset platform state
  if (lock_platform_mutex()) {
    g_vi_handle = NULL;
    g_venc_handle = NULL;
    g_ai_handle = NULL;
    g_aenc_handle = NULL;
    g_platform_initialized = false;
    g_cleanup_in_progress = false;
    unlock_platform_mutex();
  }

  platform_log_info("Platform cleanup completed\n");
}

/* Video Input (VI) functions */
platform_result_t platform_vi_match_sensor(const char* isp_cfg_path) {
  if (!isp_cfg_path) {
    return PLATFORM_ERROR_NULL;
  }

  int ret = ak_vi_match_sensor(isp_cfg_path);
  if (ret != 0) {
    return PLATFORM_ERROR;
  }

  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_open(platform_vi_handle_t* handle) {
  if (!handle) {
    return PLATFORM_ERROR_NULL;
  }

  void* vi_handle = ak_vi_open(VIDEO_DEV0);
  if (!vi_handle) {
    return PLATFORM_ERROR;
  }

  *handle = vi_handle;
  return PLATFORM_SUCCESS;
}

// Global flag for timeout handling
static volatile int g_vi_close_timeout = 0; // NOLINT

// Signal handler for VI close timeout
static void vi_close_timeout_handler(int sig) {
  (void)sig; // Suppress unused parameter warning
  g_vi_close_timeout = 1;
  platform_log_warning("platform_vi_close: Timeout occurred during ak_vi_close\n");
}

void platform_vi_close(platform_vi_handle_t handle) {
  if (handle) {
    platform_log_debug("platform_vi_close: Closing video input handle (0x%p)\n", handle);

    // CRITICAL: Don't use fork() - ak_vi_close must be called from the same
    // process context The Anyka platform library manages internal threads that
    // expect the same process context Use a signal-based timeout instead

    // Reset timeout flag
    g_vi_close_timeout = 0;

    // Set up a timeout alarm with proper handler
    struct sigaction old_action;
    struct sigaction timeout_action;

    // Set up timeout handler that actually handles the signal
    timeout_action.sa_handler = vi_close_timeout_handler;
    sigemptyset(&timeout_action.sa_mask);
    timeout_action.sa_flags = 0; // Don't restart interrupted system calls
    sigaction(SIGALRM, &timeout_action, &old_action);

    // Set 3-second timeout
    alarm(3);

    // Call ak_vi_close directly - this is the only safe way
    platform_log_debug("platform_vi_close: Calling ak_vi_close directly (with 3s timeout)\n");

    // Try to close with timeout protection
    int close_result = ak_vi_close(handle);

    // Cancel the alarm
    alarm(0);

    // Restore original signal handler
    sigaction(SIGALRM, &old_action, NULL);

    if (g_vi_close_timeout) {
      platform_log_error("platform_vi_close: ak_vi_close timed out after 3 seconds, "
                         "continuing with cleanup\n");
    } else if (close_result != 0) {
      platform_log_warning("platform_vi_close: ak_vi_close returned error %d, continuing with "
                           "cleanup\n",
                           close_result);
    } else {
      platform_log_debug("platform_vi_close: Video input closed successfully\n");
    }
  }
}

platform_result_t platform_vi_get_sensor_resolution(platform_vi_handle_t handle, platform_video_resolution_t* resolution) {
  if (!handle || !resolution) {
    return PLATFORM_ERROR_NULL;
  }

  struct video_resolution resolution_data;
  if (ak_vi_get_sensor_resolution(handle, &resolution_data) != 0) {
    return PLATFORM_ERROR;
  }

  resolution->width = resolution_data.width;
  resolution->height = resolution_data.height;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_switch_day_night(platform_vi_handle_t handle, platform_daynight_mode_t mode) {
  if (!handle) {
    return PLATFORM_ERROR_NULL;
  }

  int vi_mode = map_daynight_mode(mode);
  if (vi_mode < 0) {
    return PLATFORM_ERROR_INVALID;
  }

  if (ak_vi_switch_mode(handle, vi_mode) != 0) {
    return PLATFORM_ERROR;
  }

  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_set_flip_mirror(platform_vi_handle_t handle, bool flip, bool mirror) {
  if (!handle) {
    return PLATFORM_ERROR_NULL;
  }

  if (ak_vi_set_flip_mirror(handle, flip ? 1 : 0, mirror ? 1 : 0) != 0) {
    return PLATFORM_ERROR;
  }

  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_capture_on(platform_vi_handle_t handle) {
  if (!handle) {
    return PLATFORM_ERROR_NULL;
  }

  int result = ak_vi_capture_on(handle);
  if (result != 0) {
    platform_log_error("platform_vi_capture_on: ak_vi_capture_on failed (result=%d)\n", result);
    return PLATFORM_ERROR;
  }

  platform_log_debug("platform_vi_capture_on: Video capture started successfully\n");
  return PLATFORM_SUCCESS;
}

/* Global video capture start - called once during platform initialization */
platform_result_t platform_vi_start_global_capture(platform_vi_handle_t handle) {
  if (!handle) {
    return PLATFORM_ERROR_NULL;
  }

  platform_log_debug("platform_vi_start_global_capture: Starting global video capture...\n");

  // Add delay to allow VI system to fully initialize
  sleep_ms(PLATFORM_DELAY_MS_VI_INIT); // VI initialization delay

  // Try to start video capture with retry mechanism
  int capture_result = -1;
  int retry_count = 0;
  const int max_retries = 3;

  while (retry_count < max_retries && capture_result != 0) {
    capture_result = ak_vi_capture_on(handle);
    if (capture_result != 0) {
      platform_log_warning("platform_vi_start_global_capture: ak_vi_capture_on attempt %d "
                           "failed (result=%d)\n",
                           retry_count + 1, capture_result);
      if (retry_count < max_retries - 1) {
        sleep_ms(PLATFORM_DELAY_MS_LONG); // Wait before retry
      }
    }
    retry_count++;
  }

  if (capture_result != 0) {
    platform_log_error("platform_vi_start_global_capture: ak_vi_capture_on failed after %d "
                       "attempts (result=%d)\n",
                       max_retries, capture_result);
    return PLATFORM_ERROR;
  }

  // Additional delay after starting capture to ensure it's ready
  platform_log_debug("platform_vi_start_global_capture: Video capture started, waiting for "
                     "stabilization...\n");
  sleep_ms(PLATFORM_DELAY_MS_RETRY); // Delay for capture stabilization

  platform_log_info("platform_vi_start_global_capture: Global video capture started "
                    "successfully\n");
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_capture_off(platform_vi_handle_t handle) {
  if (!handle) {
    return PLATFORM_ERROR_NULL;
  }

  int result = ak_vi_capture_off(handle);
  if (result != 0) {
    platform_log_error("platform_vi_capture_off: ak_vi_capture_off failed (result=%d)\n", result);
    return PLATFORM_ERROR;
  }

  platform_log_debug("platform_vi_capture_off: Video capture stopped successfully\n");
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_set_channel_attr(platform_vi_handle_t handle, const platform_video_channel_attr_t* attr) {
  if (!handle || !attr) {
    return PLATFORM_ERROR_NULL;
  }

  /* Convert platform channel attributes to Anyka format */
  struct video_channel_attr vi_attr;

  /* Set crop information */
  vi_attr.crop.left = attr->crop.left;
  vi_attr.crop.top = attr->crop.top;
  vi_attr.crop.width = attr->crop.width;
  vi_attr.crop.height = attr->crop.height;

  /* Set resolutions for both channels */
  vi_attr.res[VIDEO_CHN_MAIN].width = attr->res[VIDEO_CHN_MAIN].width;
  vi_attr.res[VIDEO_CHN_MAIN].height = attr->res[VIDEO_CHN_MAIN].height;

  vi_attr.res[VIDEO_CHN_SUB].width = attr->res[VIDEO_CHN_SUB].width;
  vi_attr.res[VIDEO_CHN_SUB].height = attr->res[VIDEO_CHN_SUB].height;

  // HACK inverted for compatiblity with older version of precompiled library
  vi_attr.res[VIDEO_CHN_SUB].max_width = PLATFORM_VIDEO_WIDTH_HD;
  vi_attr.res[VIDEO_CHN_SUB].max_height = PLATFORM_VIDEO_HEIGHT_HD;
  vi_attr.res[VIDEO_CHN_MAIN].max_width = PLATFORM_VIDEO_WIDTH_VGA;
  vi_attr.res[VIDEO_CHN_MAIN].max_height = PLATFORM_VIDEO_HEIGHT_VGA;

  int result = ak_vi_set_channel_attr(handle, &vi_attr);
  if (result != 0) {
    platform_log_error("platform_vi_set_channel_attr: ak_vi_set_channel_attr failed "
                       "(result=%d)\n",
                       result);
    return PLATFORM_ERROR;
  }

  platform_log_debug("platform_vi_set_channel_attr: Channel attributes set successfully\n");
  return PLATFORM_SUCCESS;
}

platform_result_t platform_vi_get_fps(platform_vi_handle_t handle, int* fps) {
  if (!handle || !fps) {
    platform_log_error("platform_vi_get_fps: Invalid parameters (handle=%p, fps=%p)\n", handle, fps);
    return PLATFORM_ERROR_NULL;
  }

  // Get current frame rate from the sensor
  int current_fps = ak_vi_get_fps(handle);
  if (current_fps <= 0) {
    platform_log_error("platform_vi_get_fps: ak_vi_get_fps failed or returned invalid value "
                       "(%d)\n",
                       current_fps);
    return PLATFORM_ERROR;
  }

  *fps = current_fps;
  platform_log_debug("platform_vi_get_fps: Current sensor frame rate: %d fps\n", current_fps);
  return PLATFORM_SUCCESS;
}

/* VPSS (Video Processing Subsystem) functions */
platform_result_t platform_vpss_effect_set( // NOLINT
  platform_vi_handle_t vi_handle,
  platform_vpss_effect_t effect_type, // NOLINT
  int effect_value) {                 // NOLINT
  if (!vi_handle) {
    return PLATFORM_ERROR_NULL;
  }

  int vpss_effect = map_vpss_effect(effect_type);
  if (vpss_effect < 0) {
    return PLATFORM_ERROR_INVALID;
  }

  if (ak_vpss_effect_set(vi_handle, (enum vpss_effect_type)vpss_effect, effect_value) != 0) {
    return PLATFORM_ERROR;
  }

  return PLATFORM_SUCCESS;
}

platform_result_t platform_vpss_effect_get(platform_vi_handle_t handle, platform_vpss_effect_t effect, int* value) {
  if (!handle || !value) {
    return PLATFORM_ERROR_NULL;
  }

  int vpss_effect = map_vpss_effect(effect);
  if (vpss_effect < 0) {
    return PLATFORM_ERROR_INVALID;
  }

  if (ak_vpss_effect_get(handle, vpss_effect, value) != 0) {
    return PLATFORM_ERROR;
  }

  return PLATFORM_SUCCESS;
}

/* Video Encoder functions */
platform_result_t platform_venc_init(platform_venc_handle_t* handle, const platform_video_config_t* config) {
  if (!handle || !config) {
    platform_log_error("platform_venc_init: Invalid parameters (handle=%p, config=%p)\n", handle, config);
    return PLATFORM_ERROR_NULL;
  }

  // Validate configuration parameters before proceeding
  platform_result_t validation_result = platform_validate_venc_config(config);
  if (validation_result != PLATFORM_SUCCESS) {
    platform_log_error("platform_venc_init: Configuration validation failed\n");
    return validation_result;
  }

  // Validate and clamp video configuration parameters
  int width = config->width;
  int height = config->height;
  int fps = config->fps;
  int bitrate = config->bitrate;

  // Enforce 4-byte alignment for width and height
  width = (width + 3) & ~3;   // Round up to nearest 4-byte boundary
  height = (height + 3) & ~3; // Round up to nearest 4-byte boundary

  // Clamp fps to valid range
  if (fps < PLATFORM_VIDEO_FPS_MIN) {
    fps = PLATFORM_VIDEO_FPS_MIN;
  }
  if (fps > PLATFORM_VIDEO_FPS_MAX) {
    fps = PLATFORM_VIDEO_FPS_MAX;
  }

  // Clamp bitrate to valid range (kbps)
  if (bitrate < PLATFORM_VIDEO_BITRATE_MIN) {
    bitrate = PLATFORM_VIDEO_BITRATE_MIN;
  }
  if (bitrate > PLATFORM_VIDEO_BITRATE_MAX) {
    bitrate = PLATFORM_VIDEO_BITRATE_MAX;
  }

  if (width <= 0 || height <= 0) {
    platform_log_error("platform_venc_init: Invalid video dimensions after alignment (w=%d, "
                       "h=%d)\n",
                       width, height);
    return PLATFORM_ERROR_INVALID;
  }

  platform_log_debug("platform_venc_init: Video config validated and clamped (w=%d->%d, "
                     "h=%d->%d, fps=%d->%d, bitrate=%d->%d)\n",
                     config->width, width, config->height, height, config->fps, fps, config->bitrate, bitrate);

  // Map codec type
  int enc_out_type = map_video_codec(config->codec);
  if (enc_out_type < 0) {
    return PLATFORM_ERROR_INVALID;
  }

  // Map bitrate mode from platform config
  int br_mode = map_platform_br_mode(PLATFORM_BR_MODE_CBR); // Default to CBR
  if (config->br_mode >= 0) {
    br_mode = map_platform_br_mode(config->br_mode);
    if (br_mode < 0) {
      platform_log_warning("platform_venc_init: Invalid bitrate mode %d, defaulting to CBR\n", config->br_mode);
      br_mode = BR_MODE_CBR;
    }
  }

  // Map profile from platform config
  int profile = PROFILE_MAIN; // Default to main profile
  if (config->profile >= 0) {
    profile = map_platform_profile(config->profile);
    if (profile < 0) {
      platform_log_warning("platform_venc_init: Invalid profile %d, defaulting to MAIN\n", config->profile);
      profile = PROFILE_MAIN;
    }
  }

  // Setup encoder parameters using clamped values - following reference
  // implementation pattern
  struct encode_param param = {0};

  param.width = width;
  param.height = height;
  param.fps = fps;
  param.bps = bitrate;
  param.enc_out_type = enc_out_type;
  param.br_mode = br_mode;
  param.profile = profile;

  // Set GOP length based on FPS - following reference implementation
  // Calculate GOP length: 2 seconds worth of frames for reasonable GOP size
  param.goplen = (fps > 0) ? fps * 2 : PLATFORM_VIDEO_GOP_DEFAULT;

  // Set default QP values - following reference implementation
  param.minqp = PLATFORM_VIDEO_QP_MIN_DEFAULT;
  param.maxqp = PLATFORM_VIDEO_QP_MAX_DEFAULT;

  // Set channel and encoder group based on stream type
  // Note: This should be configurable based on whether this is main or sub
  // stream
  param.use_chn = ENCODE_MAIN_CHN;
  param.enc_grp = ENCODE_MAINCHN_NET;

  // Set profile based on codec type - following reference implementation
  switch (enc_out_type) {
  case H264_ENC_TYPE:
  case MJPEG_ENC_TYPE:
  default:
    param.profile = PROFILE_MAIN; // MJPEG doesn't use profiles in the same way
    break;
  case HEVC_ENC_TYPE:
    param.profile = PROFILE_HEVC_MAIN;
    break;
  }

  // Log detailed encoder parameters following ak_venc.c reference
  platform_venc_log_encoder_parameters(&param, "init");

  // Open encoder
  void* encoder_handle = ak_venc_open(&param);
  if (!encoder_handle) {
    int error_code = ak_get_error_no();
    platform_venc_log_error_context(error_code, "ak_venc_open", NULL);
    return PLATFORM_ERROR;
  }

  *handle = encoder_handle;

  // Update active encoder count
  if (lock_platform_mutex()) {
    g_encoder_active_count++;
    unlock_platform_mutex();

    platform_log_notice("platform_venc_init: Video encoder initialized successfully "
                        "(handle=0x%p, %dx%d@%dfps, %dkbps)\n",
                        encoder_handle, width, height, fps, bitrate);
  } else {
    platform_log_warning("platform_venc_init: Failed to update encoder count\n");
  }

  return PLATFORM_SUCCESS;
}

void platform_venc_cleanup(platform_venc_handle_t handle) {
  if (!handle) {
    platform_log_debug("platform_venc_cleanup: Handle is NULL, nothing to cleanup\n");
    return;
  }

  // Validate handle before cleanup
  if ((uintptr_t)handle < PLATFORM_MEMORY_ADDR_MIN || (uintptr_t)handle > PLATFORM_MEMORY_ADDR_MAX) {
    platform_log_error("platform_venc_cleanup: Invalid handle (0x%p), skipping cleanup\n", handle);
    return;
  }

  platform_log_debug("platform_venc_cleanup: Cleaning up video encoder (handle=0x%p)\n", handle);

  // Log final statistics before cleanup
  if (pthread_mutex_lock(&g_venc_stats_mutex) == 0) {
    if (g_venc_stats.statistics_active) {
      platform_venc_log_statistics(&g_venc_stats, "cleanup_final");
      platform_venc_log_performance(&g_venc_perf, "cleanup_final");
    }
    pthread_mutex_unlock(&g_venc_stats_mutex);
  }

  // Close the encoder with error handling
  int result = ak_venc_close(handle);
  if (result != 0) {
    int error_code = ak_get_error_no();
    platform_venc_log_error_context(error_code, "ak_venc_close", handle);
  } else {
    platform_log_debug("platform_venc_cleanup: Video encoder closed successfully\n");
  }

  // Decrement active encoder count with mutex protection
  if (lock_platform_mutex()) {
    if (g_encoder_active_count > 0) {
      g_encoder_active_count--;
    }
    uint32_t count = g_encoder_active_count;
    unlock_platform_mutex();

    platform_log_debug("platform_venc_cleanup: Active encoder count decremented to %u\n", count);
  } else {
    platform_log_warning("platform_venc_cleanup: Failed to update encoder count\n");
  }
}

platform_result_t platform_venc_get_frame(platform_venc_handle_t handle, uint8_t** data, uint32_t* size) {
  if (!handle || !data || !size) {
    platform_log_error("platform_venc_get_frame: Invalid parameters (handle=%p, data=%p, "
                       "size=%p)\n",
                       handle, data, size);
    return PLATFORM_ERROR_NULL;
  }

  struct video_stream stream;
  memset(&stream, 0, sizeof(stream));

  /* Enhanced debugging: Log detailed information before calling
   * ak_venc_get_stream */
  uint32_t buffer_count = 0;
  uint32_t max_buffers = 0;
  uint32_t overflow_count = 0;
  platform_result_t status = platform_venc_get_buffer_status(handle, &buffer_count, &max_buffers, &overflow_count);
  if (status == PLATFORM_SUCCESS) {
    platform_venc_log_buffer_status(buffer_count, max_buffers, overflow_count, "get_frame_pre");
  } else {
    platform_log_debug("platform_venc_get_frame: Could not get buffer status (status=%d)", status);
  }

  platform_log_debug("platform_venc_get_frame: About to call ak_venc_get_stream with "
                     "handle=%p",
                     handle);
  platform_log_debug("platform_venc_get_frame: Preparing video stream structure at %p", &stream);

  int result = ak_venc_get_stream(handle, &stream);
  if (result != 0) {
    /* Enhanced error logging with detailed error information */
    int error_code = ak_get_error_no();
    platform_venc_log_error_context(error_code, "ak_venc_get_stream", handle);
    return PLATFORM_ERROR;
  }

  if (!stream.data || stream.len == 0) {
    platform_log_warning("platform_venc_get_frame: Empty or invalid stream data (data=%p, "
                         "len=%u)\n",
                         stream.data, stream.len);
    return PLATFORM_ERROR;
  }

  *data = stream.data;
  *size = stream.len;

  // Log detailed stream information following ak_venc.c reference
  platform_venc_log_stream_info(&stream, "get_frame_success");

  // Update statistics with thread-safe access
  if (pthread_mutex_lock(&g_venc_stats_mutex) == 0) {
    platform_venc_update_statistics(&g_venc_stats, stream.len, stream.frame_type, stream.ts);
    pthread_mutex_unlock(&g_venc_stats_mutex);
  }

  platform_log_debug("platform_venc_get_frame: Success (len=%u, timestamp=%llu)\n", stream.len, stream.ts);
  return PLATFORM_SUCCESS;
}

void platform_venc_release_frame(platform_venc_handle_t handle, uint8_t* data) {
  if (!handle || !data) {
    platform_log_warning("platform_venc_release_frame: Invalid parameters (handle=%p, "
                         "data=%p)\n",
                         handle, data);
    return;
  }

  // Validate handle before use
  if (!handle) {
    platform_log_error("platform_venc_release_frame: Invalid handle (0x%p)\n", handle);
    return;
  }

  /* Log release attempt with context */
  platform_log_debug("platform_venc_release_frame: Releasing frame data=%p, handle=%p", data, handle);

  /* Get buffer status before release for monitoring */
  uint32_t buffer_count = 0;
  uint32_t max_buffers = 0;
  uint32_t overflow_count = 0;
  platform_result_t status = platform_venc_get_buffer_status(handle, &buffer_count, &max_buffers, &overflow_count);
  if (status == PLATFORM_SUCCESS) {
    platform_venc_log_buffer_status(buffer_count, max_buffers, overflow_count, "release_frame_pre");
  }

  struct video_stream stream;
  memset(&stream, 0, sizeof(stream));
  stream.data = data;

  int result = ak_venc_release_stream(handle, &stream);
  if (result != 0) {
    int error_code = ak_get_error_no();
    platform_venc_log_error_context(error_code, "ak_venc_release_stream", handle);
  } else {
    platform_log_debug("platform_venc_release_frame: Successfully released frame data=%p", data);
  }

  /* Log buffer status after release for monitoring */
  status = platform_venc_get_buffer_status(handle, &buffer_count, &max_buffers, &overflow_count);
  if (status == PLATFORM_SUCCESS) {
    platform_venc_log_buffer_status(buffer_count, max_buffers, overflow_count, "release_frame_post");
  }
}

/* Audio Input (AI) functions */
platform_result_t platform_ai_open(platform_ai_handle_t* handle) {
  // Audio input completely disabled to prevent segmentation fault
  platform_log_debug("platform_ai_open: Audio input disabled\n");
  if (handle) {
    *handle = NULL;
  }
  return PLATFORM_ERROR_NOT_SUPPORTED;

  /* Original implementation commented out to prevent segmentation fault
  if (!handle) {
    return PLATFORM_ERROR_NULL;
  }

  // Follow reference implementation pattern from akipc and ai_demo
  // Use simple parameter structure like reference implementations
  struct pcm_param ai_param = {0};
  ai_param.sample_rate = 16000;  // Use standard rate like reference
  ai_param.sample_bits = 16;     // Always 16-bit for Anyka platform
  ai_param.channel_num = AUDIO_CHANNEL_MONO;  // Always mono for Anyka platform

  platform_log_debug("Opening audio input with reference params: rate=%d,
  bits=%d, channels=%d\n", ai_param.sample_rate, ai_param.sample_bits,
  ai_param.channel_num);

  // Open audio input first (like reference implementations)
  void *h = ak_ai_open(&ai_param);
  if (!h) {
      platform_log_error("Failed to open audio input - ak_ai_open returned
  NULL\n"); return PLATFORM_ERROR_FAILED;
  }

  platform_log_debug("Audio input opened successfully (handle=%p)\n", h);

  // Store handle
  *handle = h;

  platform_log_info("Audio input opened and configured successfully (rate=%d,
  bits=%d, channels=%d)\n", ai_param.sample_rate, ai_param.sample_bits,
  ai_param.channel_num);

  return PLATFORM_SUCCESS;
  */
}

void platform_ai_close(platform_ai_handle_t handle) {
  if (handle) {
    ak_ai_close(handle);
  }
}

/* Audio Encoder functions */
platform_result_t platform_aenc_init(platform_aenc_stream_handle_t* handle, const platform_audio_config_t* config) {
  (void)config; // Not used - audio encoder disabled
  // Audio encoder completely disabled to prevent segmentation fault
  platform_log_debug("platform_aenc_init: Audio encoder disabled\n");
  if (handle) {
    *handle = NULL;
  }
  return PLATFORM_ERROR_NOT_SUPPORTED;
  /* Original implementation commented out to prevent segmentation fault
  if (!handle || !config) {
      platform_log_error("platform_aenc_init: Invalid parameters (handle=%p,
  config=%p)\n", handle, config); return PLATFORM_ERROR_NULL;
  }

  // Follow reference implementation pattern from akipc and ai_demo
  // Use simple parameter structure like reference implementations
  struct aenc_param aenc_param = {0};
  aenc_param.sample_rate = config->sample_rate;
  aenc_param.sample_bits = config->bits_per_sample;
  aenc_param.channel_num = config->channels;
  aenc_param.bitrate = config->bitrate;

  platform_log_debug("Initializing audio encoder with reference params: rate=%d,
  bits=%d, channels=%d, bitrate=%d\n", aenc_param.sample_rate,
  aenc_param.sample_bits, aenc_param.channel_num, aenc_param.bitrate);

  // Open audio encoder first (like reference implementations)
  void *h = ak_aenc_open(&aenc_param);
  if (!h) {
      platform_log_error("Failed to open audio encoder - ak_aenc_open returned
  NULL\n"); return PLATFORM_ERROR_FAILED;
  }

  platform_log_debug("Audio encoder opened successfully (handle=%p)\n", h);

  // Store handle
  *handle = h;

  platform_log_info("Audio encoder initialized successfully (rate=%d, bits=%d,
  channels=%d, bitrate=%d)\n", config->sample_rate, config->bits_per_sample,
  config->channels, config->bitrate);

  return PLATFORM_SUCCESS;
  */

  // Validate audio configuration parameters
  // if (config->sample_rate <= 0 || config->channels <= 0 ||
  //     config->bits_per_sample <= 0 || config->bitrate <= 0) {
  //     platform_log_error("platform_aenc_init: Invalid audio config (rate=%d,
  //     channels=%d, bits=%d, bitrate=%d)\n",
  //                       config->sample_rate, config->channels,
  //                       config->bits_per_sample, config->bitrate);
  //     return PLATFORM_ERROR_INVALID;
  // }

  // // Allocate audio stream context
  // struct audio_stream_context *ctx = calloc(1, sizeof(struct
  // audio_stream_context)); if (!ctx) {
  //     platform_log_error("platform_aenc_init: Failed to allocate audio stream
  //     context\n"); return PLATFORM_ERROR_MEMORY;
  // }

  // // Initialize audio input first (following reference pattern from
  // akipc/aenc_demo) platform_result_t ai_result =
  // platform_ai_open(&ctx->ai_handle); if (ai_result != PLATFORM_SUCCESS) {
  //     platform_log_error("platform_aenc_init: Failed to open audio input
  //     (error: %d)\n", ai_result); free(ctx); return ai_result;
  // }

  // // Start audio capture (following reference pattern)
  // int ret = ak_ai_start_capture(ctx->ai_handle);
  // if (ret != 0) {
  //     platform_log_error("platform_aenc_init: Failed to start audio capture:
  //     %d\n", ret); platform_ai_close(ctx->ai_handle); free(ctx); return
  //     PLATFORM_ERROR;
  // }

  // // Initialize audio encoder
  // struct audio_param aenc_param;
  // memset(&aenc_param, 0, sizeof(aenc_param));

  // aenc_param.sample_rate = config->sample_rate;
  // aenc_param.channel_num = config->channels;
  // aenc_param.sample_bits = config->bits_per_sample;
  // aenc_param.type = map_audio_codec_with_validation(config->codec);

  // if (aenc_param.type < 0) {
  //     platform_log_error("platform_aenc_init: Unsupported audio codec
  //     (%d)\n", config->codec); ak_ai_stop_capture(ctx->ai_handle);
  //     platform_ai_close(ctx->ai_handle);
  //     free(ctx);
  //     return PLATFORM_ERROR_INVALID;
  // }

  // platform_log_debug("platform_aenc_init: Opening audio encoder (rate=%d,
  // channels=%d, bits=%d, codec=%d)\n",
  //                   aenc_param.sample_rate, aenc_param.channel_num,
  //                   aenc_param.sample_bits, aenc_param.type);

  // ctx->aenc_handle = ak_aenc_open(&aenc_param);
  // if (!ctx->aenc_handle) {
  //     platform_log_error("platform_aenc_init: ak_aenc_open failed\n");
  //     ak_ai_stop_capture(ctx->ai_handle);
  //     platform_ai_close(ctx->ai_handle);
  //     free(ctx);
  //     return PLATFORM_ERROR;
  // }

  // // CRITICAL: Bind audio input and encoder to create stream handle
  // ctx->stream_handle = ak_aenc_request_stream(ctx->ai_handle,
  // ctx->aenc_handle); if (!ctx->stream_handle) {
  //     platform_log_error("platform_aenc_init: ak_aenc_request_stream failed -
  //     this is the critical binding step\n"); ak_aenc_close(ctx->aenc_handle);
  //     ak_ai_stop_capture(ctx->ai_handle);
  //     platform_ai_close(ctx->ai_handle);
  //     free(ctx);
  //     return PLATFORM_ERROR;
  // }

  // // Set AAC attributes if needed
  // if (config->codec == PLATFORM_AUDIO_CODEC_AAC) {
  //     struct aenc_attr attr;
  //     attr.aac_head = AENC_AAC_SAVE_FRAME_HEAD;
  //     ak_aenc_set_attr(ctx->aenc_handle, &attr);
  // }

  // ctx->initialized = true;
  // *handle = (platform_aenc_stream_handle_t)ctx;
  // g_audio_config = *config;

  // platform_log_notice("platform_aenc_init: Audio stream initialized
  // successfully (ai=0x%p, aenc=0x%p, stream=0x%p, codec=%d)\n",
  //                    ctx->ai_handle, ctx->aenc_handle, ctx->stream_handle,
  //                    config->codec);
  // return PLATFORM_SUCCESS;
}

void platform_aenc_cleanup(platform_aenc_stream_handle_t handle) {
  if (!handle) {
    platform_log_debug("platform_aenc_cleanup: Handle is NULL, nothing to cleanup\n");
    return;
  }

  struct audio_stream_context* ctx = (struct audio_stream_context*)handle;

  if (!ctx->initialized) {
    platform_log_debug("platform_aenc_cleanup: Context not initialized, skipping cleanup\n");
    free(ctx);
    return;
  }

  platform_log_debug("platform_aenc_cleanup: Cleaning up audio stream context (ai=0x%p, "
                     "aenc=0x%p, stream=0x%p)\n",
                     ctx->ai_handle, ctx->aenc_handle, ctx->stream_handle);

  // CRITICAL: Cancel the stream binding first with error checking
  if (ctx->stream_handle) {
    int cancel_result = ak_aenc_cancel_stream(ctx->stream_handle);
    if (cancel_result != 0) {
      platform_log_error("platform_aenc_cleanup: ak_aenc_cancel_stream failed (result=%d)\n", cancel_result);
    } else {
      platform_log_debug("platform_aenc_cleanup: Audio stream cancelled successfully\n");
    }
    ctx->stream_handle = NULL;
  }

  // Close the audio encoder
  if (ctx->aenc_handle) {
    int result = ak_aenc_close(ctx->aenc_handle);
    if (result != 0) {
      platform_log_error("platform_aenc_cleanup: ak_aenc_close failed (result=%d)\n", result);
    } else {
      platform_log_debug("platform_aenc_cleanup: Audio encoder closed successfully\n");
    }
    ctx->aenc_handle = NULL;
  }

  // Stop and close audio input
  if (ctx->ai_handle) {
    ak_ai_stop_capture(ctx->ai_handle);
    platform_ai_close(ctx->ai_handle);
    ctx->ai_handle = NULL;
  }

  // Free the context
  ctx->initialized = false;
  free(ctx);

  platform_log_debug("platform_aenc_cleanup: Audio stream context cleanup completed\n");
}

platform_result_t platform_aenc_get_frame(platform_aenc_stream_handle_t handle, uint8_t** data, uint32_t* size) {
  if (!handle || !data || !size) {
    return PLATFORM_ERROR_NULL;
  }

  struct audio_stream_context* ctx = (struct audio_stream_context*)handle;

  if (!ctx->initialized || !ctx->stream_handle) {
    platform_log_error("platform_aenc_get_frame: Invalid or uninitialized context\n");
    return PLATFORM_ERROR_INVALID;
  }

  struct list_head stream_head;
  INIT_LIST_HEAD(&stream_head);

  // Use the stream handle (not the encoder handle)
  if (ak_aenc_get_stream(ctx->stream_handle, &stream_head) != 0) {
    return PLATFORM_ERROR;
  }

  if (list_empty(&stream_head)) {
    return PLATFORM_ERROR;
  }

  struct aenc_entry* entry = list_first_entry(&stream_head, struct aenc_entry, list);
  *data = entry->stream.data;
  *size = entry->stream.len;
  return PLATFORM_SUCCESS;
}

void platform_aenc_release_frame(platform_aenc_stream_handle_t handle, const uint8_t* data) {
  if (handle && data) {
    // Note: Audio encoder doesn't have a separate release function
    // The stream is managed internally by the Anyka SDK
  }
}

/* PTZ functions */
platform_result_t platform_ptz_init(void) {
  // Initialize PTZ driver with proper error handling - based on
  // IOT-ANYKA-PTZdaemon pattern
  if (ak_drv_ptz_open() != 0) {
    platform_log_error("Failed to open PTZ driver\n");
    return PLATFORM_ERROR;
  }

  // Use proper error handling for check_self (avoid ak_print() calls that may
  // affect shared resources)
  int check_result = ak_drv_ptz_check_self(0);
  if (check_result != 0) {
    platform_log_warning("PTZ self-check failed, continuing anyway\n");
    // Don't return error - PTZ may still work without self-check
  }

  // Note: ak_drv_ptz_setup_step_param function is not available in the current
  // library PTZ will work with basic functions only (no degree/angle rate
  // setup)
  platform_log_debug("PTZ initialized with basic functions (no degree/angle rate setup)\n");

  platform_log_info("PTZ driver initialized successfully\n");
  return PLATFORM_SUCCESS;
}

void platform_ptz_cleanup(void) {
  // Note: ak_drv_ptz_close() doesn't exist in the Anyka driver
  // PTZ cleanup is handled by the driver internally
  platform_log_info("PTZ driver cleanup completed\n");
}

platform_result_t platform_ptz_set_degree(int pan_range_deg, int tilt_range_deg) {
  // Note: ak_drv_ptz_setup_step_param function is not available in the current
  // library PTZ degree setting is not supported with current library
  platform_log_warning("PTZ degree setting not supported with current library (pan=%d, "
                       "tilt=%d)\n",
                       pan_range_deg, tilt_range_deg);
  return PLATFORM_SUCCESS; // Return success to avoid breaking the API
}

platform_result_t platform_ptz_check_self(void) {
  // Based on IOT-ANYKA-PTZdaemon, use 0 for no feedback pin
  int check_result = ak_drv_ptz_check_self(0);
  if (check_result != 0) {
    platform_log_warning("PTZ self-check returned error: %d\n", check_result);
  }
  return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_move_to_position(int pan_deg, int tilt_deg) {
  // Based on IOT-ANYKA-PTZdaemon, ak_drv_ptz_turn_to_pos doesn't return error
  // codes
  ak_drv_ptz_turn_to_pos(pan_deg, tilt_deg);
  return PLATFORM_SUCCESS;
}

int platform_ptz_get_step_position(platform_ptz_axis_t axis) {
  // Note: ak_drv_ptz_get_step_pos doesn't exist in the Anyka driver
  // Return current position based on axis type
  switch (axis) {
  case PLATFORM_PTZ_AXIS_PAN:
  case PLATFORM_PTZ_AXIS_TILT:
    return 0; // Default center position
  default:
    return PLATFORM_ERROR_INVALID;
  }
}

platform_result_t platform_ptz_get_status(platform_ptz_axis_t axis, platform_ptz_status_t* status) {
  (void)axis; // Not used - global status returned
  if (!status) {
    return PLATFORM_ERROR_NULL;
  }

  // Note: ak_drv_ptz_get_status doesn't exist in the Anyka driver
  // Return OK status as default - PTZ is assumed to be working
  *status = PLATFORM_PTZ_STATUS_OK;
  return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_set_speed(platform_ptz_axis_t axis_type, // NOLINT
                                         int speed_value) {             // NOLINT
  // Note: ak_drv_ptz_set_speed doesn't exist in the Anyka driver
  // Speed is controlled via ak_drv_ptz_set_angle_rate during initialization
  // This function is kept for API compatibility but doesn't change speed
  (void)axis_type;   // Suppress unused parameter warning
  (void)speed_value; // Suppress unused parameter warning
  return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_turn(platform_ptz_direction_t direction, // NOLINT
                                    int steps) {                        // NOLINT
  // Based on IOT-ANYKA-PTZdaemon, ak_drv_ptz_turn doesn't return error codes
  // Map direction to Anyka format and call the function
  int anyka_direction = 0; // Initialize to default value
  switch (direction) {
  case PLATFORM_PTZ_DIRECTION_LEFT:
    anyka_direction = 0;
    break; // Left
  case PLATFORM_PTZ_DIRECTION_RIGHT:
    anyka_direction = 1;
    break; // Right
  case PLATFORM_PTZ_DIRECTION_UP:
    anyka_direction = 2;
    break; // Up
  case PLATFORM_PTZ_DIRECTION_DOWN:
    anyka_direction = 3;
    break; // Down
  default:
    return PLATFORM_ERROR_INVALID;
  }

  ak_drv_ptz_turn(anyka_direction,
                  steps); // Only two parameters based on header
  return PLATFORM_SUCCESS;
}

platform_result_t platform_ptz_turn_stop(platform_ptz_direction_t direction) {
  // Note: ak_drv_ptz_turn_stop doesn't exist in the Anyka driver
  // Movement stops automatically when ak_drv_ptz_turn completes
  // This function is kept for API compatibility
  (void)direction; // Suppress unused parameter warning
  return PLATFORM_SUCCESS;
}

/* IR LED functions */
platform_result_t platform_irled_init(int level) {
  struct ak_drv_irled_hw_param param;
  param.irled_working_level = level;

  if (ak_drv_irled_init(&param) != 0) {
    return PLATFORM_ERROR;
  }
  return PLATFORM_SUCCESS;
}

void platform_irled_cleanup(void) {
  // No cleanup function available in Anyka SDK
}

platform_result_t platform_irled_set_mode(platform_irled_mode_t mode) {
  int anyka_mode = 0; // Initialize to default value
  switch (mode) {
  case PLATFORM_IRLED_OFF:
    anyka_mode = 0;
    break;
  case PLATFORM_IRLED_ON:
  case PLATFORM_IRLED_AUTO:
    anyka_mode = 1;
    break; // Use ON for AUTO
  default:
    return PLATFORM_ERROR_INVALID;
  }

  if (ak_drv_irled_set_working_stat(anyka_mode) != 0) {
    return PLATFORM_ERROR;
  }
  return PLATFORM_SUCCESS;
}

platform_result_t platform_irled_get_status(void) {
  int status = ak_drv_irled_get_working_stat();
  if (status < 0) {
    return PLATFORM_ERROR;
  }
  return (platform_result_t)status;
}

/* ============================================================================
 * Logging Functions
 * ============================================================================ */
int platform_log_error(const char* format, ...) {
  va_list args;
  va_start(args, format);
  int result = platform_log_printf(PLATFORM_LOG_ERROR, __FILE__, __FUNCTION__, __LINE__, format, args);
  va_end(args);
  return result;
}

int platform_log_warning(const char* format, ...) {
  va_list args;
  va_start(args, format);
  int result = platform_log_printf(PLATFORM_LOG_WARNING, __FILE__, __FUNCTION__, __LINE__, format, args);
  va_end(args);
  return result;
}

int platform_log_notice(const char* format, ...) {
  va_list args;
  va_start(args, format);
  int result = platform_log_printf(PLATFORM_LOG_NOTICE, __FILE__, __FUNCTION__, __LINE__, format, args);
  va_end(args);
  return result;
}

int platform_log_info(const char* format, ...) {
  va_list args;
  va_start(args, format);
  int result = platform_log_printf(PLATFORM_LOG_INFO, __FILE__, __FUNCTION__, __LINE__, format, args);
  va_end(args);
  return result;
}

int platform_log_debug(const char* format, ...) {
  va_list args;
  va_start(args, format);
  int result = platform_log_printf(PLATFORM_LOG_DEBUG, __FILE__, __FUNCTION__, __LINE__, format, args);
  va_end(args);
  return result;
}

/**
 * @brief Internal function to retrieve video stream with consolidated logic
 * @param stream_handle Handle to get stream from (encoder or stream handle)
 * @param stream Output stream structure to populate
 * @param timeout_ms Timeout in milliseconds
 * @param is_stream_handle True if stream_handle is a stream handle, false if
 * encoder handle
 * @return PLATFORM_SUCCESS on success, error code on failure
 * @note Consolidates common logic between platform_venc_get_stream and
 * platform_venc_get_stream_by_handle
 */
static platform_result_t get_video_stream_internal(void* stream_handle, platform_venc_stream_t* stream, uint32_t timeout_ms, bool is_stream_handle) {
  if (!stream_handle || !stream) {
    platform_log_error("get_video_stream_internal: Invalid parameters (handle=%p, "
                       "stream=%p)\n",
                       stream_handle, stream);
    return PLATFORM_ERROR_NULL;
  }

  /* Log stream retrieval attempt with context */
  platform_log_debug("get_video_stream_internal: Getting stream (handle=%p, timeout=%ums, "
                     "is_stream_handle=%s)",
                     stream_handle, timeout_ms, is_stream_handle ? "true" : "false");

  // Initialize stream structure
  memset(stream, 0, sizeof(platform_venc_stream_t));

  struct video_stream anyka_stream;
  memset(&anyka_stream, 0, sizeof(anyka_stream));

  // Get stream with retry mechanism
  int result = get_stream_with_retry(stream_handle, &anyka_stream, timeout_ms);

  if (result != 0) {
    if (is_stream_handle) {
      platform_log_error("get_video_stream_internal: Failed to get stream (result=%d, "
                         "handle=0x%p)\n",
                         result, stream_handle);

      // Provide more context about the failure for stream handles
      if (result == -1) {
        int error_code = ak_get_error_no();
        platform_log_error("get_video_stream_internal: No stream data available "
                           "(error_code=%d) - "
                           "check video capture status and frame rate synchronization\n",
                           error_code);

        // Additional diagnostics for ERROR_TYPE_NO_DATA
        if (error_code == ERROR_TYPE_NO_DATA) {
          platform_log_error("get_video_stream_internal: ERROR_TYPE_NO_DATA - This usually "
                             "means:\n");
          platform_log_error("  1. Video capture is not started or failed to start\n");
          platform_log_error("  2. No frames have been captured yet (try waiting longer)\n");
          platform_log_error("  3. Encoder threads are not running properly\n");
          platform_log_error("  4. Stream queue is empty (no encoded frames available)\n");
        }
      } else if (result == -2) {
        platform_log_error("get_video_stream_internal: Encoder resource busy - check system "
                           "load\n");
      } else {
        platform_log_error("get_video_stream_internal: Unknown error (result=%d) - check "
                           "encoder "
                           "initialization\n",
                           result);
      }
    } else {
      platform_log_error("get_video_stream_internal: Failed to get stream (result=%d)\n", result);
    }
    return PLATFORM_ERROR;
  }

  // Populate platform stream structure
  stream->data = anyka_stream.data;
  stream->len = anyka_stream.len;
  stream->timestamp = anyka_stream.ts;
  stream->is_keyframe = (anyka_stream.frame_type == FRAME_TYPE_I);

  platform_log_debug("get_video_stream_internal: Success (len=%u, keyframe=%s)\n", stream->len, stream->is_keyframe ? "true" : "false");
  return PLATFORM_SUCCESS;
}

/**
 * @brief Internal function to release video stream with consolidated logic
 * @param stream_handle Handle to release stream to (encoder or stream handle)
 * @param stream Stream structure to release
 * @note Consolidates common logic between platform_venc_release_stream and
 * platform_venc_release_stream_by_handle
 */
static void release_video_stream_internal(void* stream_handle, platform_venc_stream_t* stream) {
  if (!stream_handle || !stream) {
    platform_log_warning("release_video_stream_internal: Invalid parameters (handle=%p, "
                         "stream=%p)\n",
                         stream_handle, stream);
    return;
  }

  struct video_stream anyka_stream;
  anyka_stream.data = stream->data;
  anyka_stream.len = stream->len;
  anyka_stream.ts = stream->timestamp;

  int result = ak_venc_release_stream(stream_handle, &anyka_stream);
  if (result != 0) {
    platform_log_error("release_video_stream_internal: ak_venc_release_stream failed "
                       "(result=%d)\n",
                       result);
  } else {
    platform_log_debug("release_video_stream_internal: Stream released successfully\n");
  }
}

/* Video Encoder Stream functions (for RTSP) */
platform_result_t platform_venc_get_stream(platform_venc_handle_t handle, platform_venc_stream_t* stream, uint32_t timeout_ms) {
  return get_video_stream_internal(handle, stream, timeout_ms, false);
}

void platform_venc_release_stream(platform_venc_handle_t handle, platform_venc_stream_t* stream) {
  release_video_stream_internal(handle, stream);
}

/* Video Encoder Stream Request functions (for RTSP) */
platform_result_t platform_venc_request_stream(platform_vi_handle_t vi_handle, platform_venc_handle_t venc_handle,
                                               platform_venc_stream_handle_t* stream_handle) {
  if (!vi_handle || !venc_handle || !stream_handle) {
    platform_log_error("platform_venc_request_stream: Invalid parameters (vi=%p, venc=%p, "
                       "stream_handle=%p)\n",
                       vi_handle, venc_handle, stream_handle);
    return PLATFORM_ERROR_NULL;
  }

  // Validate that VI handle is properly initialized
  if (vi_handle != g_vi_handle) {
    platform_log_error("platform_venc_request_stream: VI handle mismatch - expected global VI "
                       "handle (vi=%p, global=%p)\n",
                       vi_handle, g_vi_handle);
    return PLATFORM_ERROR_INVALID;
  }

  /* Log stream request attempt with context */
  platform_log_debug("platform_venc_request_stream: Requesting stream binding (vi=%p, "
                     "venc=%p)",
                     vi_handle, venc_handle);

  // Note: Video capture should already be started globally, not per-stream
  // This function only requests a stream binding between VI and VENC

  /* Get buffer status before request for monitoring */
  uint32_t buffer_count = 0;
  uint32_t max_buffers = 0;
  uint32_t overflow_count = 0;
  platform_result_t status = platform_venc_get_buffer_status(venc_handle, &buffer_count, &max_buffers, &overflow_count);
  if (status == PLATFORM_SUCCESS) {
    platform_venc_log_buffer_status(buffer_count, max_buffers, overflow_count, "request_stream_pre");
  }

  // Request stream binding between VI and VENC
  void* stream = ak_venc_request_stream(vi_handle, venc_handle);
  if (!stream) {
    int error_code = ak_get_error_no();
    platform_venc_log_error_context(error_code, "ak_venc_request_stream", venc_handle);
    // Stop capture on failure
    ak_vi_capture_off(vi_handle);
    return PLATFORM_ERROR;
  }

  *stream_handle = stream;
  platform_log_debug("platform_venc_request_stream: Stream requested successfully "
                     "(handle=0x%p)\n",
                     stream);

  /* Log buffer status after request for monitoring */
  status = platform_venc_get_buffer_status(venc_handle, &buffer_count, &max_buffers, &overflow_count);
  if (status == PLATFORM_SUCCESS) {
    platform_venc_log_buffer_status(buffer_count, max_buffers, overflow_count, "request_stream_post");
  }

  return PLATFORM_SUCCESS;
}

void platform_venc_cancel_stream(platform_venc_stream_handle_t stream_handle) {
  if (!stream_handle) {
    platform_log_debug("platform_venc_cancel_stream: Stream handle is NULL, nothing to "
                       "cancel\n");
    return;
  }

  /* Log stream cancellation attempt with context */
  platform_log_debug("platform_venc_cancel_stream: Cancelling stream handle=%p", stream_handle);

  /* Get buffer status before cancellation for monitoring */
  uint32_t buffer_count = 0;
  uint32_t max_buffers = 0;
  uint32_t overflow_count = 0;
  platform_result_t status = platform_venc_get_buffer_status(stream_handle, &buffer_count, &max_buffers, &overflow_count);
  if (status == PLATFORM_SUCCESS) {
    platform_venc_log_buffer_status(buffer_count, max_buffers, overflow_count, "cancel_stream_pre");
  }

  int result = ak_venc_cancel_stream(stream_handle);
  if (result != 0) {
    int error_code = ak_get_error_no();
    platform_venc_log_error_context(error_code, "ak_venc_cancel_stream", stream_handle);
  } else {
    platform_log_debug("platform_venc_cancel_stream: Stream cancelled successfully "
                       "(handle=%p)",
                       stream_handle);
  }

  /* Log buffer status after cancellation for monitoring */
  status = platform_venc_get_buffer_status(stream_handle, &buffer_count, &max_buffers, &overflow_count);
  if (status == PLATFORM_SUCCESS) {
    platform_venc_log_buffer_status(buffer_count, max_buffers, overflow_count, "cancel_stream_post");
  }
}

platform_result_t platform_venc_get_stream_by_handle(platform_venc_stream_handle_t stream_handle, platform_venc_stream_t* stream,
                                                     uint32_t timeout_ms) {
  return get_video_stream_internal(stream_handle, stream, timeout_ms, true);
}

void platform_venc_release_stream_by_handle(platform_venc_stream_handle_t stream_handle, platform_venc_stream_t* stream) {
  /* Log stream release attempt with context */
  platform_log_debug("platform_venc_release_stream_by_handle: Releasing stream (handle=%p, "
                     "stream=%p)",
                     stream_handle, stream);

  release_video_stream_internal(stream_handle, stream);

  platform_log_debug("platform_venc_release_stream_by_handle: Stream released successfully");
}

platform_result_t platform_venc_get_buffer_status(platform_venc_stream_handle_t stream_handle, uint32_t* buffer_count, uint32_t* max_buffers,
                                                  uint32_t* overflow_count) {
  if (!stream_handle) {
    platform_log_error("platform_venc_get_buffer_status: Invalid stream handle\n");
    return PLATFORM_ERROR_NULL;
  }

  if (!buffer_count || !max_buffers || !overflow_count) {
    platform_log_error("platform_venc_get_buffer_status: Invalid output parameters\n");
    return PLATFORM_ERROR_NULL;
  }

  /* Log buffer status request with context */
  platform_log_debug("platform_venc_get_buffer_status: Getting buffer status for stream "
                     "handle=%p",
                     stream_handle);

  /* Note: Direct access to internal structures is not available in ONVIF
   * project */
  /* This is a stub implementation that returns reasonable default values */
  /* In a real implementation, this would query the actual encoder state */

  *buffer_count = 0;                         /* Would be retrieved from encoder state */
  *max_buffers = PLATFORM_STREAM_BUFFER_MAX; /* ONE_STREAM_MAX from reference implementation */
  *overflow_count = 0;                       /* Would be retrieved from encoder state */

  /* Log buffer status with detailed information */
  platform_log_debug("platform_venc_get_buffer_status: Stream handle=%p, buffer_count=%u, "
                     "max_buffers=%u, overflow_count=%u",
                     stream_handle, *buffer_count, *max_buffers, *overflow_count);

  /* Log warnings for high buffer usage or overflows */
  if (*buffer_count > (uint32_t)((float)(*max_buffers) * PLATFORM_BUFFER_USAGE_HIGH_THRESHOLD)) {
    platform_log_warning("platform_venc_get_buffer_status: High buffer usage detected "
                         "(count=%u, max=%u, usage=%.1f%%)",
                         *buffer_count, *max_buffers, ((float)(*buffer_count) * 100.0F) / (float)(*max_buffers));
  }

  if (*overflow_count > 0) {
    platform_log_warning("platform_venc_get_buffer_status: Buffer overflow detected (count=%u)", *overflow_count);
  }

  return PLATFORM_SUCCESS;
}

/* Snapshot functions */
platform_result_t platform_snapshot_init(platform_snapshot_handle_t* snapshot_handle, platform_vi_handle_t vi_handle,
                                         int image_width, // NOLINT(bugprone-easily-swappable-parameters) - width/height is universal convention
                                         int image_height) {
  if (!snapshot_handle || !vi_handle) {
    return PLATFORM_ERROR_NULL;
  }

  // Create snapshot context
  struct snapshot_context* ctx = malloc(sizeof(struct snapshot_context));
  if (!ctx) {
    return PLATFORM_ERROR;
  }

  ctx->vi_handle = vi_handle;
  ctx->width = image_width;
  ctx->height = image_height;
  ctx->jpeg_encoder = NULL;

  // Initialize JPEG encoder for snapshots
  struct encode_param param = {0};
  param.width = image_width;
  param.height = image_height;
  param.minqp = PLATFORM_VIDEO_QP_MIN_DEFAULT;
  param.maxqp = PLATFORM_VIDEO_QP_MAX_JPEG;
  param.fps = PLATFORM_VIDEO_FPS_SNAPSHOT;
  param.goplen = 1;
  param.bps = PLATFORM_VIDEO_BITRATE_SNAPSHOT; // kbps
  param.profile = PROFILE_MAIN;
  param.use_chn = ENCODE_SUB_CHN;
  param.enc_grp = ENCODE_PICTURE;
  param.br_mode = BR_MODE_CBR;
  param.enc_out_type = MJPEG_ENC_TYPE;

  ctx->jpeg_encoder = ak_venc_open(&param);
  if (!ctx->jpeg_encoder) {
    free(ctx);
    return PLATFORM_ERROR;
  }

  *snapshot_handle = ctx;
  return PLATFORM_SUCCESS;
}

void platform_snapshot_cleanup(platform_snapshot_handle_t handle) {
  if (!handle) {
    return;
  }

  struct snapshot_context* ctx = (struct snapshot_context*)handle;
  if (ctx->jpeg_encoder) {
    ak_venc_close(ctx->jpeg_encoder);
  }
  free(ctx);
}

platform_result_t platform_snapshot_capture(platform_snapshot_handle_t handle, platform_snapshot_t* snapshot, uint32_t timeout_ms) {
  (void)timeout_ms; // Not used - blocking operation
  if (!handle || !snapshot) {
    return PLATFORM_ERROR_NULL;
  }

  struct snapshot_context* ctx = (struct snapshot_context*)handle;
  struct video_input_frame frame;
  struct video_stream jpeg_stream;

  // Get a frame from video input
  if (ak_vi_get_frame(ctx->vi_handle, &frame) != 0) {
    return PLATFORM_ERROR;
  }

  // Encode frame to JPEG
  int result = ak_venc_send_frame(ctx->jpeg_encoder, frame.vi_frame[VIDEO_CHN_SUB].data, frame.vi_frame[VIDEO_CHN_SUB].len, &jpeg_stream);

  // Release the input frame
  ak_vi_release_frame(ctx->vi_handle, &frame);

  if (result != 0) {
    return PLATFORM_ERROR;
  }

  // Populate snapshot structure
  snapshot->data = jpeg_stream.data;
  snapshot->len = jpeg_stream.len;
  snapshot->timestamp = jpeg_stream.ts;

  return PLATFORM_SUCCESS;
}

void platform_snapshot_release(platform_snapshot_handle_t handle, platform_snapshot_t* snapshot) {
  if (!handle || !snapshot) {
    return;
  }

  struct snapshot_context* ctx = (struct snapshot_context*)handle;
  struct video_stream jpeg_stream;
  jpeg_stream.data = snapshot->data;
  jpeg_stream.len = snapshot->len;
  jpeg_stream.ts = snapshot->timestamp;

  ak_venc_release_stream(ctx->jpeg_encoder, &jpeg_stream);
}

/* Audio Encoder Stream functions (for RTSP) */
platform_result_t platform_aenc_get_stream(platform_aenc_stream_handle_t handle, platform_aenc_stream_t* stream, uint32_t timeout_ms) {
  (void)handle;     // Not used - audio encoder disabled
  (void)stream;     // Not used - audio encoder disabled
  (void)timeout_ms; // Not used - audio encoder disabled
  // Audio encoder completely disabled to prevent segmentation fault
  platform_log_debug("platform_aenc_get_stream: Audio encoder disabled\n");
  return PLATFORM_ERROR_NOT_SUPPORTED;

  /* Original implementation commented out to prevent segmentation fault
  if (!handle || !stream) {
      platform_log_error("platform_aenc_get_stream: Invalid parameters
  (handle=%p, stream=%p)\n", handle, stream); return PLATFORM_ERROR_NULL;
  }

  // Use safe mutex locking with timeout
  if (!platform_safe_mutex_lock(&aenc_mutex, timeout_ms)) {
      platform_log_error("platform_aenc_get_stream: Failed to acquire mutex
  after %d ms\n", timeout_ms); return PLATFORM_ERROR_TIMEOUT;
  }

  // Check if encoder is initialized
  if (!aenc_initialized) {
      platform_log_error("platform_aenc_get_stream: Audio encoder not
  initialized\n"); platform_safe_mutex_unlock(&aenc_mutex); return
  PLATFORM_ERROR_NOT_INITIALIZED;
  }

  // Get audio stream from encoder
  int result = ak_aenc_get_stream(handle, stream, timeout_ms);
  if (result != 0) {
      platform_log_error("platform_aenc_get_stream: ak_aenc_get_stream failed
  with result=%d\n", result); platform_safe_mutex_unlock(&aenc_mutex); return
  PLATFORM_ERROR_FAILED;
  }

  platform_safe_mutex_unlock(&aenc_mutex);
  return PLATFORM_SUCCESS;
  */

  // struct audio_stream_context *ctx = (struct audio_stream_context*)handle;

  // if (!ctx->initialized || !ctx->stream_handle) {
  //     platform_log_error("platform_aenc_get_stream: Invalid or uninitialized
  //     context (initialized=%d, stream_handle=%p)\n",
  //                       ctx->initialized, ctx->stream_handle);
  //     return PLATFORM_ERROR_INVALID;
  // }

  // // Check if audio input is still active
  // if (!ctx->ai_handle) {
  //     platform_log_error("platform_aenc_get_stream: Audio input handle is
  //     NULL\n"); return PLATFORM_ERROR_INVALID;
  // }

  // // Check if audio encoder is still active
  // if (!ctx->aenc_handle) {
  //     platform_log_error("platform_aenc_get_stream: Audio encoder handle is
  //     NULL\n"); return PLATFORM_ERROR_INVALID;
  // }

  // // Initialize stream structure to prevent uninitialized data
  // memset(stream, 0, sizeof(platform_aenc_stream_t));

  // struct list_head stream_head;
  // INIT_LIST_HEAD(&stream_head);

  // // Simplified retry mechanism for audio stream retrieval (following
  // reference implementation) uint32_t retry_count = 0; uint32_t max_retries =
  // 3; // Reduced retries to avoid blocking int result = -1;

  // while (retry_count < max_retries) {
  //     // Use the stream handle (not the encoder handle)
  //     result = ak_aenc_get_stream(ctx->stream_handle, &stream_head);

  //     if (result == 0) {
  //         // Success - check if we have data
  //         if (!list_empty(&stream_head)) {
  //             struct aenc_entry *entry = list_first_entry(&stream_head,
  //             struct aenc_entry, list); stream->data = entry->stream.data;
  //             stream->len = entry->stream.len;
  //             stream->timestamp = entry->stream.ts;

  //             platform_log_debug("platform_aenc_get_stream: Success (len=%u,
  //             timestamp=%llu)\n",
  //                               stream->len, stream->timestamp);
  //             return PLATFORM_SUCCESS;
  //         } else {
  //             // No data available, retry with longer delay
  //             platform_log_debug("platform_aenc_get_stream: No audio data
  //             available (retry %u/%u)\n",
  //                               retry_count + 1, max_retries);
  //             if (retry_count < max_retries - 1) {
  //                 sleep_ms(50); // 50ms delay for audio data
  //             }
  //         }
  //     } else {
  //         // Error occurred, retry with longer delay
  //         platform_log_debug("platform_aenc_get_stream: ak_aenc_get_stream
  //         failed (retry %u/%u, result=%d)\n",
  //                           retry_count + 1, max_retries, result);
  //         if (retry_count < max_retries - 1) {
  //             sleep_ms(100); // 100ms delay for errors
  //         }
  //     }

  //     retry_count++;
  // }

  // platform_log_error("platform_aenc_get_stream: Failed after %u retries
  // (result=%d)\n",
  //                   retry_count, result);
  // return PLATFORM_ERROR;
}

void platform_aenc_release_stream(platform_aenc_stream_handle_t handle, platform_aenc_stream_t* stream) {
  if (!handle || !stream) {
    return;
  }

  // Note: Audio encoder stream is managed internally by Anyka SDK
  // The stream data is released automatically when the next frame is retrieved
}

/**
 * @brief Parse CPU statistics from /proc/stat line
 * @param line Input line from /proc/stat
 * @param idle Output parameter for idle CPU time
 * @param total Output parameter for total CPU time
 * @return 1 on success, 0 on failure
 */
static int parse_cpu_stat_line(const char* line,
                               unsigned long long* idle, // NOLINT
                               unsigned long long* total) {
  char* endptr = NULL;
  char* token = strtok((char*)line, " ");

  if (!token || strcmp(token, "cpu") != 0) {
    return 0;
  }

  char* token2 = strtok(NULL, " ");
  if (!token2) {
    return 0;
  }

  *total = strtoull(token2, &endptr, PLATFORM_PARSE_BASE_DECIMAL);
  if (*endptr != '\0') {
    return 0;
  }

  char* token3 = strtok(NULL, " ");
  if (!token3) {
    return 0;
  }

  *idle = strtoull(token3, &endptr, PLATFORM_PARSE_BASE_DECIMAL);
  if (*endptr != '\0') {
    return 0;
  }

  // Skip remaining tokens (we only need first two values)
  (void)strtok(NULL, " "); // Skip third value
  (void)strtok(NULL, " "); // Skip fourth value

  return 1;
}

/**
 * @brief Calculate CPU percentage from current and previous values
 * @param prev_idle Previous idle time
 * @param prev_total Previous total time
 * @param idle Current idle time
 * @param total Current total time
 * @return CPU usage percentage
 */
static float calculate_cpu_percentage(unsigned long long prev_idle, // NOLINT
                                      unsigned long long prev_total,
                                      unsigned long long idle, // NOLINT
                                      unsigned long long total) {
  unsigned long long diff_idle = idle - prev_idle;
  unsigned long long diff_total = total - prev_total;

  if (diff_total > 0) {
    return 100.0F * (1.0F - (float)diff_idle / (float)diff_total);
  }

  return 0.0F;
}

/* System monitoring functions */
static float get_cpu_usage(void) {
  static unsigned long long prev_idle = 0;
  static unsigned long long prev_total = 0;
  unsigned long long idle = 0;
  unsigned long long total = 0;
  float cpu_percent = 0.0F;

  FILE* stat_file = fopen("/proc/stat", "r");
  if (!stat_file) {
    return 0.0F;
  }

  char line[PLATFORM_BUFFER_SIZE_MEDIUM];
  if (fgets(line, sizeof(line), stat_file)) {
    if (parse_cpu_stat_line(line, &idle, &total)) {
      cpu_percent = calculate_cpu_percentage(prev_idle, prev_total, idle, total);
    }
  }

  prev_idle = idle;
  prev_total = total;

  int close_result = fclose(stat_file);
  if (close_result != 0) {
    platform_log_warning("Failed to close /proc/stat file\n");
  }

  return cpu_percent;
}

static float get_cpu_temperature(void) {
  float temp = 0.0F;
  FILE* temp_file = fopen("/sys/class/thermal/thermal_zone0/temp", "r");
  if (!temp_file) {
    // Fallback to alternative temperature sources
    temp_file = fopen("/proc/thermal_zone0/temp", "r");
    if (!temp_file) {
      return 0.0F;
    }
  }

  char line[PLATFORM_BUFFER_SIZE_SMALL];
  if (fgets(line, sizeof(line), temp_file)) {
    char* endptr = NULL;
    temp = strtof(line, &endptr);
    if (endptr && (*endptr == '\0' || *endptr == '\n')) {
      temp /= PLATFORM_TEMP_MILLI_TO_UNIT; // Convert from millidegrees to degrees
    }
  }

  int close_result = fclose(temp_file);
  if (close_result != 0) {
    platform_log_warning("Failed to close temperature file\n");
  }
  return temp;
}

static void get_memory_info(uint64_t* total_memory_bytes,  // NOLINT
                            uint64_t* free_memory_bytes) { // NOLINT
  *total_memory_bytes = 0;
  *free_memory_bytes = 0;

  FILE* mem_file = fopen("/proc/meminfo", "r");
  if (!mem_file) {
    return;
  }

  char line[PLATFORM_BUFFER_SIZE_MEDIUM];
  while (fgets(line, sizeof(line), mem_file)) {
    unsigned long long temp_total = 0;
    unsigned long long temp_free = 0;
    if (strncmp(line, "MemTotal:", PLATFORM_MEMINFO_TOTAL_OFFSET) == 0) {
      char* endptr = NULL;
      temp_total = strtoull(line + PLATFORM_MEMINFO_TOTAL_OFFSET, &endptr, PLATFORM_PARSE_BASE_DECIMAL);
      if (endptr && *endptr == ' ' && strncmp(endptr + 1, "kB", 2) == 0) {
        *total_memory_bytes = (uint64_t)temp_total * PLATFORM_MEMORY_KB_TO_BYTES; // Convert from kB to bytes
      }
    } else if (strncmp(line, "MemAvailable:", PLATFORM_MEMINFO_AVAIL_OFFSET) == 0) {
      char* endptr = NULL;
      temp_free = strtoull(line + PLATFORM_MEMINFO_AVAIL_OFFSET, &endptr, PLATFORM_PARSE_BASE_DECIMAL);
      if (endptr && *endptr == ' ' && strncmp(endptr + 1, "kB", 2) == 0) {
        *free_memory_bytes = (uint64_t)temp_free * PLATFORM_MEMORY_KB_TO_BYTES; // Convert from kB to bytes
      }
    }
  }

  int close_result = fclose(mem_file);
  if (close_result != 0) {
    platform_log_warning("Failed to close /proc/meminfo file\n");
  }
}

static uint64_t get_system_uptime(void) {
  uint64_t uptime_seconds = 0;
  FILE* uptime_file = fopen("/proc/uptime", "r");
  if (!uptime_file) {
    return 0;
  }

  char line[PLATFORM_BUFFER_SIZE_SMALL];
  if (fgets(line, sizeof(line), uptime_file)) {
    char* endptr = NULL;
    uptime_seconds = strtoull(line, &endptr, PLATFORM_PARSE_BASE_DECIMAL);
    if (endptr && (*endptr == '\0' || *endptr == '\n')) {
      uptime_seconds *= PLATFORM_TIME_MS_PER_SECOND; // Convert to milliseconds
    }
  }

  int close_result = fclose(uptime_file);
  if (close_result != 0) {
    platform_log_warning("Failed to close /proc/uptime file\n");
  }
  return uptime_seconds;
}

platform_result_t platform_get_system_info(platform_system_info_t* info) {
  if (!info) {
    return PLATFORM_ERROR_NULL;
  }

  memset(info, 0, sizeof(platform_system_info_t));

  // Get CPU usage
  info->cpu_usage = get_cpu_usage();

  // Get CPU temperature
  info->cpu_temperature = get_cpu_temperature();

  // Get memory information
  get_memory_info(&info->total_memory, &info->free_memory);

  // Get system uptime
  info->uptime_ms = get_system_uptime();

  return PLATFORM_SUCCESS;
}

/**
 * @brief Execute a system command
 * @param command Command string to execute
 * @return Command exit status on success, -1 on failure
 * @note This is a thin wrapper around system() for testability
 * @warning Use with caution - validates command is not NULL but does not sanitize input
 */
int platform_system(const char* command) {
  if (!command) {
    platform_log_error("platform_system: NULL command\n");
    return -1;
  }

  // Execute the command using standard C library system()
  // Note: system() return value is platform-specific
  // NOLINTNEXTLINE(cert-env33-c) - Acceptable: Only used with hardcoded commands (e.g., "reboot"), no user input
  int result = system(command);
  return result;
}

/**
 * @brief Get the absolute path to the currently running executable
 */
platform_result_t platform_get_executable_path(char* path_buffer, size_t buffer_size) {
  ssize_t len = 0;

  if (path_buffer == NULL || buffer_size == 0) {
    platform_log_error("[PLATFORM] Invalid parameters for get_executable_path\n");
    return PLATFORM_ERROR_INVALID_PARAM;
  }

  /* Use /proc/self/exe on Linux */
  len = readlink("/proc/self/exe", path_buffer, buffer_size - 1);

  if (len == -1) {
    platform_log_warning("[PLATFORM] Failed to read /proc/self/exe: %s\n", strerror(errno));
    return PLATFORM_ERROR;
  }

  if ((size_t)len >= buffer_size) {
    platform_log_error("[PLATFORM] Executable path buffer too small\n");
    return PLATFORM_ERROR_INVALID;
  }

  path_buffer[len] = '\0';
  return PLATFORM_SUCCESS;
}

/* Enhanced video encoder logging and statistics functions implementation */

/**
 * @brief Log comprehensive video encoder statistics
 * @param stats Statistics structure to log
 * @param context Context string for logging
 * @note Follows ak_venc.c reference implementation logging patterns
 */
static void platform_venc_log_statistics(const platform_venc_statistics_t* stats, const char* context) {
  if (!stats || !stats->statistics_active) {
    return;
  }

  platform_log_info("VENC_STATS[%s]: bytes=%u, bitrate=%ukbps, frames=%u, fps=%.1f, gop=%u\n", context ? context : "unknown", stats->total_bytes,
                    stats->bitrate_kbps, stats->frame_count, stats->fps, stats->gop_length);

  platform_log_debug("VENC_STATS[%s]: I=%u, P=%u, B=%u, dropped=%u, overflow=%u\n", context ? context : "unknown", stats->i_frame_count,
                     stats->p_frame_count, stats->b_frame_count, stats->dropped_frames, stats->stream_overflow_count);

  platform_log_debug("VENC_STATS[%s]: frame_size_range=[%u-%u], errors=%u\n", context ? context : "unknown", stats->min_frame_size,
                     stats->max_frame_size, stats->consecutive_errors);
}

/**
 * @brief Log video encoder performance metrics
 * @param perf Performance structure to log
 * @param context Context string for logging
 * @note Follows ak_venc.c reference implementation performance logging patterns
 */
static void platform_venc_log_performance(const platform_venc_performance_t* perf, const char* context) {
  if (!perf) {
    return;
  }

  platform_log_info("VENC_PERF[%s]: capture_frames=%u, encode_frames=%u, sensor_fps=%u\n", context ? context : "unknown", perf->capture_frame_count,
                    perf->encode_frame_count, perf->sensor_fps);

  if (perf->capture_errors > 0 || perf->encode_errors > 0) {
    platform_log_warning("VENC_PERF[%s]: capture_errors=%u, encode_errors=%u\n", context ? context : "unknown", perf->capture_errors,
                         perf->encode_errors);
  }

  if (perf->fps_switch_detected) {
    platform_log_notice("VENC_PERF[%s]: FPS switch detected: %u->%u at %llu\n", context ? context : "unknown", perf->previous_sensor_fps,
                        perf->sensor_fps, (unsigned long long)perf->last_fps_switch_time);
  }
}

/**
 * @brief Update video encoder statistics with new frame data
 * @param stats Statistics structure to update
 * @param frame_size Size of the encoded frame
 * @param frame_type Type of frame (I/P/B)
 * @param timestamp Frame timestamp
 * @note Follows ak_venc.c reference implementation statistics calculation
 */
static void platform_venc_update_statistics( // NOLINT
  platform_venc_statistics_t* statistics,
  const uint32_t frame_size_bytes,                               // NOLINT
  const uint32_t frame_type_code, const uint64_t timestamp_ms) { // NOLINT
  if (!statistics) {
    return;
  }

  // Initialize statistics if not active
  if (!statistics->statistics_active) {
    statistics->statistics_active = true;
    statistics->start_timestamp = timestamp_ms;
    statistics->last_calc_time = get_time_ms();
    statistics->min_frame_size = UINT32_MAX;
    statistics->max_frame_size = 0;
    platform_log_debug("VENC_STATS: Statistics collection started at timestamp %llu\n", timestamp_ms);
  }

  // Update frame counts and sizes
  statistics->total_bytes += frame_size_bytes;
  statistics->frame_count++;
  statistics->last_frame_timestamp = timestamp_ms;

  // Update frame type counts
  switch (frame_type_code) {
  case FRAME_TYPE_I:
    statistics->i_frame_count++;
    break;
  case FRAME_TYPE_P:
    statistics->p_frame_count++;
    break;
  case FRAME_TYPE_B:
    statistics->b_frame_count++;
    break;
  default:
    break;
  }

  // Update frame size statistics
  if (frame_size_bytes > statistics->max_frame_size) {
    statistics->max_frame_size = frame_size_bytes;
  }
  if (frame_size_bytes < statistics->min_frame_size) {
    statistics->min_frame_size = frame_size_bytes;
  }

  // Calculate bitrate and FPS periodically (every 2 seconds like reference)
  uint64_t current_time = get_time_ms();
  uint64_t time_diff = current_time - statistics->last_calc_time;

  if (time_diff >= PLATFORM_DELAY_MS_STATS_INTERVAL) { // Statistics interval
    float time_factor = (float)time_diff / (float)PLATFORM_TIME_MS_PER_SECOND;
    statistics->fps = (float)statistics->frame_count / time_factor;
    statistics->bitrate_kbps =
      (uint32_t)((statistics->total_bytes * PLATFORM_BITRATE_BITS_PER_BYTE) / (time_diff * PLATFORM_BITRATE_CONVERSION)); // Convert to kbps

    // Log statistics periodically
    platform_venc_log_statistics(statistics, "periodic_update");

    // Reset for next period
    statistics->last_calc_time = current_time;
  }
}

/**
 * @brief Log video encoder parameters in detail
 * @param param Encoder parameters to log
 * @param context Context string for logging
 * @note Follows ak_venc.c reference implementation parameter logging
 */
static void platform_venc_log_encoder_parameters(const struct encode_param* param, const char* context) {
  if (!param) {
    return;
  }

  platform_log_info("VENC_PARAMS[%s]: w=%d, h=%d, fps=%d, bitrate=%d, codec=%d\n", context ? context : "unknown", param->width, param->height,
                    param->fps, param->bps, param->enc_out_type);

  platform_log_debug("VENC_PARAMS[%s]: profile=%d, br_mode=%d, gop=%d, qp=[%d-%d], chn=%d, "
                     "grp=%d\n",
                     context ? context : "unknown", param->profile, param->br_mode, param->goplen, param->minqp, param->maxqp, param->use_chn,
                     param->enc_grp);
}

/**
 * @brief Log video stream information
 * @param stream Video stream to log
 * @param context Context string for logging
 * @note Follows ak_venc.c reference implementation stream logging
 */
static void platform_venc_log_stream_info(const struct video_stream* stream, const char* context) {
  if (!stream) {
    return;
  }

  const char* frame_type_str = "Unknown";
  switch (stream->frame_type) {
  case FRAME_TYPE_I:
    frame_type_str = "I";
    break;
  case FRAME_TYPE_P:
    frame_type_str = "P";
    break;
  case FRAME_TYPE_B:
    frame_type_str = "B";
    break;
  case FRAME_TYPE_PI:
    frame_type_str = "PI";
    break;
  default:
    break;
  }

  platform_log_debug("VENC_STREAM[%s]: type=%s, len=%u, ts=%llu, seq=%lu\n", context ? context : "unknown", frame_type_str, stream->len, stream->ts,
                     stream->seq_no);
}

/**
 * @brief Log detailed error context for video operations
 * @param error_code Error code from Anyka SDK
 * @param operation Operation that failed
 * @param handle Handle involved in the operation
 * @note Provides comprehensive error context for debugging
 */
static void platform_venc_log_error_context(int error_code, const char* operation, void* handle) {
  const char* error_msg = ak_get_error_str(error_code);

  platform_log_error("VENC_ERROR[%s]: code=%d, msg='%s', handle=0x%p\n", operation ? operation : "unknown", error_code,
                     error_msg ? error_msg : "unknown", handle);

  // Log additional context based on error type
  switch (error_code) {
  case ERROR_TYPE_POINTER_NULL:
    platform_log_error("VENC_ERROR[%s]: NULL pointer detected - check handle "
                       "initialization\n",
                       operation ? operation : "unknown");
    break;
  case ERROR_TYPE_MALLOC_FAILED:
    platform_log_error("VENC_ERROR[%s]: Memory allocation failed - check available memory\n", operation ? operation : "unknown");
    break;
  case ERROR_TYPE_NO_DATA:
    platform_log_error("VENC_ERROR[%s]: No data available - check video capture and frame "
                       "rate sync\n",
                       operation ? operation : "unknown");
    break;
  case ERROR_TYPE_INVALID_USER:
    platform_log_error("VENC_ERROR[%s]: Invalid user/handle - check handle validity\n", operation ? operation : "unknown");
    break;
  default:
    platform_log_error("VENC_ERROR[%s]: Unknown error code %d\n", operation ? operation : "unknown", error_code);
    break;
  }
}

/**
 * @brief Log video encoder buffer status
 * @param buffer_count Current buffer count
 * @param max_buffers Maximum buffer count
 * @param overflow_count Overflow count
 * @param context Context string for logging
 * @note Follows ak_venc.c reference implementation buffer monitoring
 */
static void platform_venc_log_buffer_status(uint32_t buffer_count, uint32_t max_buffers, uint32_t overflow_count, const char* context) {
  platform_log_debug("VENC_BUFFER[%s]: count=%u/%u, overflow=%u\n", context ? context : "unknown", buffer_count, max_buffers, overflow_count);

  if (overflow_count > 0) {
    platform_log_warning("VENC_BUFFER[%s]: Buffer overflow detected (count=%u)\n", context ? context : "unknown", overflow_count);
  }

  if (buffer_count > (uint32_t)((float)max_buffers * PLATFORM_BUFFER_USAGE_HIGH_THRESHOLD)) {
    platform_log_warning("VENC_BUFFER[%s]: Buffer usage high (%.1f%%)\n", context ? context : "unknown",
                         ((float)buffer_count / (float)max_buffers) * 100.0F);
  }
}

/* Utility function for range validation to reduce code duplication */
static bool validate_range(int value, int min, int max, const char* name) {
  if (value < min || value > max) {
    platform_log_error("Invalid %s %d (must be %d-%d)\n", name, value, min, max);
    return false;
  }
  return true;
}

/* Mutex utility functions to reduce code duplication */
static bool lock_platform_mutex(void) {
  if (pthread_mutex_lock(&g_platform_state_mutex) != 0) {
    platform_log_error("Failed to acquire platform state mutex\n");
    return false;
  }
  return true;
}

static void unlock_platform_mutex(void) {
  pthread_mutex_unlock(&g_platform_state_mutex);
}

/* Logging utility functions for common patterns */
/**
 * @brief Validate video encoder configuration parameters
 * @param config Video encoder configuration to validate
 * @return PLATFORM_SUCCESS if valid, PLATFORM_ERROR_INVALID if invalid
 * @note Performs comprehensive validation of all encoder parameters
 */
platform_result_t platform_validate_venc_config(const platform_video_config_t* config) {
  if (!config) {
    platform_log_error("platform_validate_venc_config: Invalid config parameter (NULL)\n");
    return PLATFORM_ERROR_NULL;
  }

  // Validate dimensions
  if (!validate_range(config->width, PLATFORM_VIDEO_FPS_MIN, PLATFORM_VIDEO_DIMENSION_MAX, "width")) {
    return PLATFORM_ERROR_INVALID;
  }

  if (!validate_range(config->height, PLATFORM_VIDEO_FPS_MIN, PLATFORM_VIDEO_DIMENSION_MAX, "height")) {
    return PLATFORM_ERROR_INVALID;
  }

  // Validate FPS
  if (!validate_range(config->fps, PLATFORM_VIDEO_FPS_MIN, PLATFORM_VIDEO_FPS_MAX, "FPS")) {
    return PLATFORM_ERROR_INVALID;
  }

  // Validate bitrate
  if (!validate_range(config->bitrate, PLATFORM_VIDEO_BITRATE_MIN, PLATFORM_DELAY_MS_MAX_BITRATE_VALID, "bitrate")) {
    return PLATFORM_ERROR_INVALID;
  }

  // Validate codec
  if (config->codec < 0 || config->codec >= PLATFORM_VIDEO_CODEC_MAX) {
    platform_log_error("Invalid codec %d\n", config->codec);
    return PLATFORM_ERROR_INVALID;
  }

  // Validate bitrate mode
  if (config->br_mode < 0 || config->br_mode >= PLATFORM_BR_MODE_MAX) {
    platform_log_error("Invalid bitrate mode %d\n", config->br_mode);
    return PLATFORM_ERROR_INVALID;
  }

  // Validate profile
  if (config->profile < 0 || config->profile >= PLATFORM_PROFILE_MAX) {
    platform_log_error("Invalid profile %d\n", config->profile);
    return PLATFORM_ERROR_INVALID;
  }

  // Validate width/height alignment (should be even for most codecs)
  if (config->width % 4 != 0) {
    platform_log_warning("Width %d not 4-byte aligned, may cause issues\n", config->width);
  }

  if (config->height % 4 != 0) {
    platform_log_warning("Height %d not 4-byte aligned, may cause issues\n", config->height);
  }

  platform_log_debug("Configuration validated successfully (%dx%d@%dfps, %dkbps, codec=%d)\n", config->width, config->height, config->fps,
                     config->bitrate, config->codec);

  return PLATFORM_SUCCESS;
}

/**
 * @brief Log buffer status for debugging
 * @param stream_handle Stream handle for context
 * @param context Context string for logging
 */
static void log_buffer_status_debug(void* stream_handle, const char* context) {
  uint32_t buffer_count = 0;
  uint32_t max_buffers = 0;
  uint32_t overflow_count = 0;
  platform_result_t status = platform_venc_get_buffer_status(stream_handle, &buffer_count, &max_buffers, &overflow_count);

  if (status == PLATFORM_SUCCESS) {
    platform_venc_log_buffer_status(buffer_count, max_buffers, overflow_count, context);
  } else {
    platform_log_debug("get_stream_with_retry: %s handle=%p (buffer status unavailable)", context, stream_handle);
  }
}

/**
 * @brief Handle stream retrieval error and determine retry strategy
 * @param stream_handle Stream handle for logging
 * @param result Error result from ak_venc_get_stream
 * @param retry_count Current retry count
 * @return 1 if should retry, 0 if should stop
 */
static int handle_stream_error(void* stream_handle, int result, // NOLINT
                               uint32_t retry_count) {
  if (result == 0) {
    return 0; // Success, no retry needed
  }

  if (result == -1) {
    // No data available - retry with short delay
    if (retry_count == 0) {
      int error_code = ak_get_error_no();
      platform_venc_log_error_context(error_code, "no_data_retry", stream_handle);
    }
    return 1; // Retry
  }

  if (result == -2) {
    // Resource busy - retry with longer delay
    if (retry_count == 0) {
      int error_code = ak_get_error_no();
      platform_venc_log_error_context(error_code, "resource_busy_retry", stream_handle);
    }
    return 1; // Retry
  }

  // Other errors - don't retry
  int error_code = ak_get_error_no();
  platform_venc_log_error_context(error_code, "fatal_error_no_retry", stream_handle);
  return 0; // Don't retry
}

/**
 * @brief Calculate delay for retry with exponential backoff
 * @param result Error result to determine delay multiplier
 * @param retry_count Current retry count
 * @return Delay in milliseconds
 */
static uint32_t calculate_retry_delay(int result, // NOLINT
                                      uint32_t retry_count) {
  uint32_t delay_ms = PLATFORM_RETRY_DELAY_BASE_MS + (retry_count * PLATFORM_RETRY_DELAY_INCREMENT_MS);
  if (result == -2) {
    delay_ms *= 2; // Double delay for resource busy
  }
  return delay_ms;
}

/**
 * @brief Get video stream with simplified retry mechanism
 * @param stream_handle The stream handle to get data from
 * @param anyka_stream Output stream structure to populate
 * @param timeout_ms Timeout in milliseconds (0 for default retries)
 * @return 0 on success, error code on failure
 * @note Simplified retry logic with exponential backoff for better performance
 */
static int get_stream_with_retry(void* stream_handle, struct video_stream* anyka_stream, uint32_t timeout_ms) {
  uint32_t retry_count = 0;
  uint32_t max_retries = (timeout_ms > 0) ? (timeout_ms / PLATFORM_RETRY_DELAY_BASE_MS) : PLATFORM_RETRY_COUNT_MAX;
  int result = -1;

  // For the first attempt, add a longer initial delay to allow for
  // encoder thread startup and first frame capture
  if (max_retries > 0) {
    platform_log_debug("get_stream_with_retry: Initial delay for encoder startup\n");
    sleep_ms(PLATFORM_DELAY_MS_MEDIUM); // Initial delay for encoder startup
  }

  while (retry_count < max_retries) {
    log_buffer_status_debug(stream_handle, "retry_attempt");

    result = ak_venc_get_stream(stream_handle, anyka_stream);

    if (result == 0) {
      // Success - log only on first attempt or after retries
      if (retry_count > 0) {
        platform_log_debug("get_stream_with_retry: Success after %u retries\n", retry_count);
      }
      return 0;
    }

    if (!handle_stream_error(stream_handle, result, retry_count)) {
      break; // Don't retry
    }

    // Calculate and apply delay before next retry
    if (retry_count < max_retries - 1) {
      uint32_t delay_ms = calculate_retry_delay(result, retry_count);
      sleep_ms(delay_ms);
    }

    retry_count++;
  }

  // Enhanced final error logging with detailed information
  int error_code = ak_get_error_no();
  platform_venc_log_error_context(error_code, "final_failure", stream_handle);
  log_buffer_status_debug(stream_handle, "final_failure");

  return result;
}
