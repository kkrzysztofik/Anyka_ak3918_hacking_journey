/**
 * @file config_runtime.c
 * @brief Schema-driven runtime configuration manager implementation
 *
 * Part of the Unified Configuration System (Feature 001)
 *
 * @author Anyka ONVIF Development Team
 * @date 2025-10-11
 */

#include "core/config/config_runtime.h"

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "common/onvif_constants.h"
#include "core/config/config.h"
#include "core/config/config_storage.h"
#include "platform/platform.h"
#include "services/common/onvif_imaging_types.h"
#include "services/common/onvif_types.h"
#include "services/ptz/onvif_ptz.h"
#include "utils/error/error_handling.h"
#include "utils/security/hash_utils.h"
#include "utils/validation/common_validation.h"

/* Constants */
#define CONFIG_STRING_MAX_LEN_DEFAULT  256
#define CONFIG_STRING_MAX_LEN_STANDARD 64
#define CONFIG_STRING_MAX_LEN_SHORT    32
#define CONFIG_PORT_MIN                1
#define CONFIG_PORT_MAX                65535
#define CONFIG_PERSISTENCE_QUEUE_MAX   32

/* Global state variables */
static struct application_config* g_config_runtime_app_config = NULL;
static pthread_mutex_t g_config_runtime_mutex = PTHREAD_MUTEX_INITIALIZER;
static uint32_t g_config_runtime_generation = 0;
static int g_config_runtime_initialized = 0;

/* Persistence queue state */
static persistence_queue_entry_t g_persistence_queue[CONFIG_PERSISTENCE_QUEUE_MAX];
static size_t g_persistence_queue_count = 0;
static pthread_mutex_t g_persistence_queue_mutex = PTHREAD_MUTEX_INITIALIZER;

/* Schema definition for validation */
static const config_schema_entry_t g_config_schema[] = {
  /* ONVIF Section */
  {CONFIG_SECTION_ONVIF, "onvif", "enabled", CONFIG_TYPE_BOOL, 1, 0, 1, 0, "1"},
  {CONFIG_SECTION_ONVIF, "onvif", "http_port", CONFIG_TYPE_INT, 1, CONFIG_PORT_MIN, CONFIG_PORT_MAX,
   0, "8080"},
  {CONFIG_SECTION_ONVIF, "onvif", "auth_enabled", CONFIG_TYPE_BOOL, 1, 0, 1, 0, "0"},
  {CONFIG_SECTION_ONVIF, "onvif", "username", CONFIG_TYPE_STRING, 0, 0, 0,
   CONFIG_STRING_MAX_LEN_SHORT, "admin"},
  {CONFIG_SECTION_ONVIF, "onvif", "password", CONFIG_TYPE_STRING, 0, 0, 0,
   CONFIG_STRING_MAX_LEN_SHORT, "admin"},

  /* Network Section */
  {CONFIG_SECTION_NETWORK, "network", "rtsp_port", CONFIG_TYPE_INT, 1, CONFIG_PORT_MIN,
   CONFIG_PORT_MAX, 0, "554"},
  {CONFIG_SECTION_NETWORK, "network", "snapshot_port", CONFIG_TYPE_INT, 1, CONFIG_PORT_MIN,
   CONFIG_PORT_MAX, 0, "8080"},
  {CONFIG_SECTION_NETWORK, "network", "ws_discovery_port", CONFIG_TYPE_INT, 1, CONFIG_PORT_MIN,
   CONFIG_PORT_MAX, 0, "3702"},

  /* Device Section */
  {CONFIG_SECTION_DEVICE, "device", "manufacturer", CONFIG_TYPE_STRING, 1, 0, 0,
   CONFIG_STRING_MAX_LEN_STANDARD, "Anyka"},
  {CONFIG_SECTION_DEVICE, "device", "model", CONFIG_TYPE_STRING, 1, 0, 0,
   CONFIG_STRING_MAX_LEN_STANDARD, "AK3918"},
  {CONFIG_SECTION_DEVICE, "device", "firmware_version", CONFIG_TYPE_STRING, 1, 0, 0,
   CONFIG_STRING_MAX_LEN_STANDARD, "1.0"},
  {CONFIG_SECTION_DEVICE, "device", "serial_number", CONFIG_TYPE_STRING, 1, 0, 0,
   CONFIG_STRING_MAX_LEN_STANDARD, "000000"},
  {CONFIG_SECTION_DEVICE, "device", "hardware_id", CONFIG_TYPE_STRING, 1, 0, 0,
   CONFIG_STRING_MAX_LEN_STANDARD, "AK3918"},

  /* Logging Section */
  {CONFIG_SECTION_LOGGING, "logging", "enabled", CONFIG_TYPE_INT, 1, 0, 1, 0, "1"},
  {CONFIG_SECTION_LOGGING, "logging", "use_colors", CONFIG_TYPE_INT, 0, 0, 1, 0, "1"},
  {CONFIG_SECTION_LOGGING, "logging", "use_timestamps", CONFIG_TYPE_INT, 0, 0, 1, 0, "1"},
  {CONFIG_SECTION_LOGGING, "logging", "min_level", CONFIG_TYPE_INT, 1, 0, 5, 0, "2"},
  {CONFIG_SECTION_LOGGING, "logging", "tag", CONFIG_TYPE_STRING, 0, 0, 0,
   CONFIG_STRING_MAX_LEN_SHORT, "ONVIF"},
  {CONFIG_SECTION_LOGGING, "logging", "http_verbose", CONFIG_TYPE_INT, 0, 0, 1, 0, "0"},

  /* Server Section */
  {CONFIG_SECTION_SERVER, "server", "worker_threads", CONFIG_TYPE_INT, 1, 1, 32, 0, "4"},
  {CONFIG_SECTION_SERVER, "server", "max_connections", CONFIG_TYPE_INT, 1, 1, 1000, 0, "100"},
  {CONFIG_SECTION_SERVER, "server", "connection_timeout", CONFIG_TYPE_INT, 1, 1, 300, 0, "30"},
  {CONFIG_SECTION_SERVER, "server", "keepalive_timeout", CONFIG_TYPE_INT, 1, 1, 300, 0, "60"},
  {CONFIG_SECTION_SERVER, "server", "epoll_timeout", CONFIG_TYPE_INT, 1, 1, 10000, 0, "1000"},
  {CONFIG_SECTION_SERVER, "server", "cleanup_interval", CONFIG_TYPE_INT, 1, 1, 3600, 0, "300"},

  /* Stream Profile 1 (User Story 4) */
  {CONFIG_SECTION_STREAM_PROFILE_1, "stream_profile_1", "name", CONFIG_TYPE_STRING, 0, 0, 0,
   CONFIG_STRING_MAX_LEN_STANDARD, "High Definition"},
  {CONFIG_SECTION_STREAM_PROFILE_1, "stream_profile_1", "width", CONFIG_TYPE_INT, 0, 160, 1920, 0,
   "1920"},
  {CONFIG_SECTION_STREAM_PROFILE_1, "stream_profile_1", "height", CONFIG_TYPE_INT, 0, 120, 1080, 0,
   "1080"},
  {CONFIG_SECTION_STREAM_PROFILE_1, "stream_profile_1", "fps", CONFIG_TYPE_INT, 0, 1, 60, 0, "30"},
  {CONFIG_SECTION_STREAM_PROFILE_1, "stream_profile_1", "bitrate", CONFIG_TYPE_INT, 0, 64, 16384, 0,
   "4096"},
  {CONFIG_SECTION_STREAM_PROFILE_1, "stream_profile_1", "gop_size", CONFIG_TYPE_INT, 0, 1, 300, 0,
   "60"},
  {CONFIG_SECTION_STREAM_PROFILE_1, "stream_profile_1", "profile", CONFIG_TYPE_INT, 0, 0, 2, 0,
   "1"},
  {CONFIG_SECTION_STREAM_PROFILE_1, "stream_profile_1", "codec_type", CONFIG_TYPE_INT, 0, 0, 2, 0,
   "0"},
  {CONFIG_SECTION_STREAM_PROFILE_1, "stream_profile_1", "br_mode", CONFIG_TYPE_INT, 0, 0, 1, 0,
   "0"},

  /* Stream Profile 2 (User Story 4) */
  {CONFIG_SECTION_STREAM_PROFILE_2, "stream_profile_2", "name", CONFIG_TYPE_STRING, 0, 0, 0,
   CONFIG_STRING_MAX_LEN_STANDARD, "Standard Definition"},
  {CONFIG_SECTION_STREAM_PROFILE_2, "stream_profile_2", "width", CONFIG_TYPE_INT, 0, 160, 1920, 0,
   "1280"},
  {CONFIG_SECTION_STREAM_PROFILE_2, "stream_profile_2", "height", CONFIG_TYPE_INT, 0, 120, 1080, 0,
   "720"},
  {CONFIG_SECTION_STREAM_PROFILE_2, "stream_profile_2", "fps", CONFIG_TYPE_INT, 0, 1, 60, 0, "30"},
  {CONFIG_SECTION_STREAM_PROFILE_2, "stream_profile_2", "bitrate", CONFIG_TYPE_INT, 0, 64, 16384, 0,
   "2048"},
  {CONFIG_SECTION_STREAM_PROFILE_2, "stream_profile_2", "gop_size", CONFIG_TYPE_INT, 0, 1, 300, 0,
   "60"},
  {CONFIG_SECTION_STREAM_PROFILE_2, "stream_profile_2", "profile", CONFIG_TYPE_INT, 0, 0, 2, 0,
   "1"},
  {CONFIG_SECTION_STREAM_PROFILE_2, "stream_profile_2", "codec_type", CONFIG_TYPE_INT, 0, 0, 2, 0,
   "0"},
  {CONFIG_SECTION_STREAM_PROFILE_2, "stream_profile_2", "br_mode", CONFIG_TYPE_INT, 0, 0, 1, 0,
   "0"},

  /* Stream Profile 3 (User Story 4) */
  {CONFIG_SECTION_STREAM_PROFILE_3, "stream_profile_3", "name", CONFIG_TYPE_STRING, 0, 0, 0,
   CONFIG_STRING_MAX_LEN_STANDARD, "Mobile Stream"},
  {CONFIG_SECTION_STREAM_PROFILE_3, "stream_profile_3", "width", CONFIG_TYPE_INT, 0, 160, 1920, 0,
   "640"},
  {CONFIG_SECTION_STREAM_PROFILE_3, "stream_profile_3", "height", CONFIG_TYPE_INT, 0, 120, 1080, 0,
   "480"},
  {CONFIG_SECTION_STREAM_PROFILE_3, "stream_profile_3", "fps", CONFIG_TYPE_INT, 0, 1, 60, 0, "15"},
  {CONFIG_SECTION_STREAM_PROFILE_3, "stream_profile_3", "bitrate", CONFIG_TYPE_INT, 0, 64, 16384, 0,
   "512"},
  {CONFIG_SECTION_STREAM_PROFILE_3, "stream_profile_3", "gop_size", CONFIG_TYPE_INT, 0, 1, 300, 0,
   "30"},
  {CONFIG_SECTION_STREAM_PROFILE_3, "stream_profile_3", "profile", CONFIG_TYPE_INT, 0, 0, 2, 0,
   "0"},
  {CONFIG_SECTION_STREAM_PROFILE_3, "stream_profile_3", "codec_type", CONFIG_TYPE_INT, 0, 0, 2, 0,
   "0"},
  {CONFIG_SECTION_STREAM_PROFILE_3, "stream_profile_3", "br_mode", CONFIG_TYPE_INT, 0, 0, 1, 0,
   "0"},

  /* Stream Profile 4 (User Story 4) */
  {CONFIG_SECTION_STREAM_PROFILE_4, "stream_profile_4", "name", CONFIG_TYPE_STRING, 0, 0, 0,
   CONFIG_STRING_MAX_LEN_STANDARD, "Low Bandwidth"},
  {CONFIG_SECTION_STREAM_PROFILE_4, "stream_profile_4", "width", CONFIG_TYPE_INT, 0, 160, 1920, 0,
   "320"},
  {CONFIG_SECTION_STREAM_PROFILE_4, "stream_profile_4", "height", CONFIG_TYPE_INT, 0, 120, 1080, 0,
   "240"},
  {CONFIG_SECTION_STREAM_PROFILE_4, "stream_profile_4", "fps", CONFIG_TYPE_INT, 0, 1, 60, 0, "10"},
  {CONFIG_SECTION_STREAM_PROFILE_4, "stream_profile_4", "bitrate", CONFIG_TYPE_INT, 0, 64, 16384, 0,
   "256"},
  {CONFIG_SECTION_STREAM_PROFILE_4, "stream_profile_4", "gop_size", CONFIG_TYPE_INT, 0, 1, 300, 0,
   "20"},
  {CONFIG_SECTION_STREAM_PROFILE_4, "stream_profile_4", "profile", CONFIG_TYPE_INT, 0, 0, 2, 0,
   "0"},
  {CONFIG_SECTION_STREAM_PROFILE_4, "stream_profile_4", "codec_type", CONFIG_TYPE_INT, 0, 0, 2, 0,
   "0"},
  {CONFIG_SECTION_STREAM_PROFILE_4, "stream_profile_4", "br_mode", CONFIG_TYPE_INT, 0, 0, 1, 0,
   "0"},

  /* PTZ Preset Profile 1 - 4 presets max per profile */
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset_count", CONFIG_TYPE_INT, 0,
   0, 4, 0, "0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset1_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset1_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset1_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset1_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset1_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset2_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset2_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset2_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset2_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset2_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset3_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset3_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset3_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset3_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset3_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset4_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset4_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset4_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset4_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "ptz_preset_profile_1", "preset4_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},

  /* PTZ Preset Profile 2 - 4 presets max per profile */
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset_count", CONFIG_TYPE_INT, 0,
   0, 4, 0, "0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset1_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset1_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset1_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset1_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset1_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset2_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset2_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset2_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset2_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset2_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset3_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset3_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset3_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset3_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset3_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset4_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset4_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset4_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset4_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_2, "ptz_preset_profile_2", "preset4_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},

  /* PTZ Preset Profile 3 - 4 presets max per profile */
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset_count", CONFIG_TYPE_INT, 0,
   0, 4, 0, "0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset1_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset1_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset1_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset1_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset1_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset2_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset2_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset2_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset2_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset2_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset3_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset3_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset3_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset3_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset3_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset4_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset4_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset4_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset4_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_3, "ptz_preset_profile_3", "preset4_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},

  /* PTZ Preset Profile 4 - 4 presets max per profile */
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset_count", CONFIG_TYPE_INT, 0,
   0, 4, 0, "0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset1_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset1_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset1_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset1_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset1_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset2_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset2_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset2_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset2_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset2_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset3_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset3_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset3_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset3_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset3_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset4_token", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset4_name", CONFIG_TYPE_STRING,
   0, 0, 0, CONFIG_STRING_MAX_LEN_STANDARD, ""},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset4_pan", CONFIG_TYPE_FLOAT, 0,
   -180, 180, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset4_tilt", CONFIG_TYPE_FLOAT,
   0, -90, 90, 0, "0.0"},
  {CONFIG_SECTION_PTZ_PRESET_PROFILE_4, "ptz_preset_profile_4", "preset4_zoom", CONFIG_TYPE_FLOAT,
   0, 0, 1, 0, "0.0"},

  /* Imaging Section */
  {CONFIG_SECTION_IMAGING, "imaging", "brightness", CONFIG_TYPE_INT, 0, -100, 100, 0, "0"},
  {CONFIG_SECTION_IMAGING, "imaging", "contrast", CONFIG_TYPE_INT, 0, -100, 100, 0, "0"},
  {CONFIG_SECTION_IMAGING, "imaging", "saturation", CONFIG_TYPE_INT, 0, -100, 100, 0, "0"},
  {CONFIG_SECTION_IMAGING, "imaging", "sharpness", CONFIG_TYPE_INT, 0, -100, 100, 0, "0"},
  {CONFIG_SECTION_IMAGING, "imaging", "hue", CONFIG_TYPE_INT, 0, -180, 180, 0, "0"},

  /* Auto Day/Night Section */
  {CONFIG_SECTION_AUTO_DAYNIGHT, "imaging_auto", "mode", CONFIG_TYPE_INT, 0, 0, 2, 0, "0"},
  {CONFIG_SECTION_AUTO_DAYNIGHT, "imaging_auto", "day_to_night_threshold", CONFIG_TYPE_INT, 0, 0,
   100, 0, "30"},
  {CONFIG_SECTION_AUTO_DAYNIGHT, "imaging_auto", "night_to_day_threshold", CONFIG_TYPE_INT, 0, 0,
   100, 0, "70"},
  {CONFIG_SECTION_AUTO_DAYNIGHT, "imaging_auto", "lock_time_seconds", CONFIG_TYPE_INT, 0, 1, 600, 0,
   "10"},
  {CONFIG_SECTION_AUTO_DAYNIGHT, "imaging_auto", "ir_led_mode", CONFIG_TYPE_INT, 0, 0, 2, 0, "2"},
  {CONFIG_SECTION_AUTO_DAYNIGHT, "imaging_auto", "ir_led_level", CONFIG_TYPE_INT, 0, 0, 100, 0, "1"},
  {CONFIG_SECTION_AUTO_DAYNIGHT, "imaging_auto", "enable_auto_switching", CONFIG_TYPE_INT, 0, 0, 1,
   0, "1"},
};

static const size_t g_config_schema_count = sizeof(g_config_schema) / sizeof(g_config_schema[0]);

/* Forward declarations */
static int config_runtime_validate_section(config_section_t section);
static int config_runtime_validate_key(const char* key);
static void* config_runtime_get_section_ptr(config_section_t section);
static void* config_runtime_get_field_ptr(config_section_t section, const char* key,
                                          config_value_type_t* out_type);
static const config_schema_entry_t* config_runtime_find_schema_entry(config_section_t section,
                                                                     const char* key);
static int config_runtime_validate_int_value(const config_schema_entry_t* schema, int value);
static int config_runtime_validate_float_value(const config_schema_entry_t* schema, float value);
static int config_runtime_validate_string_value(const config_schema_entry_t* schema,
                                                const char* value);
static int config_runtime_find_queue_entry(config_section_t section, const char* key);
static int config_runtime_validate_username(const char* username);
static int config_runtime_find_user_index(const char* username);
static int config_runtime_find_free_user_slot(void);

/**
 * @brief Bootstrap the runtime configuration manager
 */
int config_runtime_init(struct application_config* cfg) {
  if (cfg == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (g_config_runtime_initialized) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_ALREADY_EXISTS;
  }

  g_config_runtime_app_config = cfg;
  g_config_runtime_generation = 0;
  g_config_runtime_initialized = 1;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return ONVIF_SUCCESS;
}

/**
 * @brief Shutdown the runtime configuration manager
 */
int config_runtime_cleanup(void) {
  int persist_count = 0;

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  pthread_mutex_unlock(&g_config_runtime_mutex);

  /* Flush any pending persistence updates before shutdown */
  pthread_mutex_lock(&g_persistence_queue_mutex);
  persist_count = (int)g_persistence_queue_count;
  pthread_mutex_unlock(&g_persistence_queue_mutex);

  if (persist_count > 0) {
    platform_log_info(
      "[CONFIG] Flushing %d pending configuration updates to disk before shutdown\n",
      persist_count);
    config_runtime_process_persistence_queue();
  }

  pthread_mutex_lock(&g_config_runtime_mutex);
  g_config_runtime_app_config = NULL;
  g_config_runtime_initialized = 0;
  pthread_mutex_unlock(&g_config_runtime_mutex);

  /* Clear persistence queue after flush */
  pthread_mutex_lock(&g_persistence_queue_mutex);
  g_persistence_queue_count = 0;
  pthread_mutex_unlock(&g_persistence_queue_mutex);

  return ONVIF_SUCCESS;
}

/**
 * @brief Apply default values for all configuration parameters
 */
int config_runtime_apply_defaults(void) {
  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Apply default values using existing config system */
  /* For now, we set reasonable defaults directly */
  /* TODO: In Phase 4 (Schema-Driven Validation), this will load from schema */

  if (g_config_runtime_app_config->network) {
    g_config_runtime_app_config->network->rtsp_port = 554;
    g_config_runtime_app_config->network->snapshot_port = 8080;
    g_config_runtime_app_config->network->ws_discovery_port = 3702;
  }

  if (g_config_runtime_app_config->logging) {
    g_config_runtime_app_config->logging->enabled = 1;
    g_config_runtime_app_config->logging->min_level = 2; /* NOTICE */
  }

  if (g_config_runtime_app_config->imaging) {
    g_config_runtime_app_config->imaging->brightness = 0;
    g_config_runtime_app_config->imaging->contrast = 0;
    g_config_runtime_app_config->imaging->saturation = 0;
    g_config_runtime_app_config->imaging->sharpness = 0;
    g_config_runtime_app_config->imaging->hue = 0;
  }

  if (g_config_runtime_app_config->auto_daynight) {
    g_config_runtime_app_config->auto_daynight->mode = DAY_NIGHT_AUTO;
    g_config_runtime_app_config->auto_daynight->day_to_night_threshold = 30;
    g_config_runtime_app_config->auto_daynight->night_to_day_threshold = 70;
    g_config_runtime_app_config->auto_daynight->lock_time_seconds = 10;
    g_config_runtime_app_config->auto_daynight->ir_led_mode = IR_LED_AUTO;
    g_config_runtime_app_config->auto_daynight->ir_led_level = 1;
    g_config_runtime_app_config->auto_daynight->enable_auto_switching = 1;
  }

  g_config_runtime_app_config->onvif.enabled = 1;
  g_config_runtime_app_config->onvif.http_port = 8080;
  g_config_runtime_app_config->onvif.auth_enabled = 0;

  g_config_runtime_generation++;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return ONVIF_SUCCESS;
}

/**
 * @brief Get integer configuration value with validation
 */
int config_runtime_get_int(config_section_t section, const char* key, int* out_value) {
  config_value_type_t field_type = CONFIG_TYPE_INT;
  void* field_ptr = NULL;

  if (key == NULL || out_value == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (config_runtime_validate_section(section) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (config_runtime_validate_key(key) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Get pointer to the field */
  field_ptr = config_runtime_get_field_ptr(section, key, &field_type);
  if (field_ptr == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  /* Validate type matches */
  if (field_type != CONFIG_TYPE_INT && field_type != CONFIG_TYPE_BOOL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Copy the value */
  *out_value = *(int*)field_ptr;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return ONVIF_SUCCESS;
}

/**
 * @brief Get string configuration value with validation
 */
int config_runtime_get_string(config_section_t section, const char* key, char* out_value,
                              size_t buffer_size) {
  config_value_type_t field_type = CONFIG_TYPE_STRING;
  void* field_ptr = NULL;
  const char* str_value = NULL;
  int result = 0;

  if (key == NULL || out_value == NULL || buffer_size == 0) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (config_runtime_validate_section(section) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (config_runtime_validate_key(key) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Get pointer to the field */
  field_ptr = config_runtime_get_field_ptr(section, key, &field_type);
  if (field_ptr == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  /* Validate type matches */
  if (field_type != CONFIG_TYPE_STRING) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Copy the string safely */
  str_value = (const char*)field_ptr;
  result = snprintf(out_value, buffer_size, "%s", str_value);

  /* Check for truncation */
  if (result < 0 || (size_t)result >= buffer_size) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_INVALID;
  }

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return ONVIF_SUCCESS;
}

/**
 * @brief Get boolean configuration value with validation
 */
int config_runtime_get_bool(config_section_t section, const char* key, int* out_value) {
  /* Booleans are stored as integers */
  return config_runtime_get_int(section, key, out_value);
}

/**
 * @brief Get float configuration value with validation
 */
int config_runtime_get_float(config_section_t section, const char* key, float* out_value) {
  config_value_type_t field_type = CONFIG_TYPE_FLOAT;

  if (key == NULL || out_value == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (config_runtime_validate_section(section) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (config_runtime_validate_key(key) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Get pointer to the field */
  void* field_ptr = config_runtime_get_field_ptr(section, key, &field_type);
  if (field_ptr == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  /* Type must be int or float */
  if (field_type == CONFIG_TYPE_INT) {
    *out_value = (float)(*(int*)field_ptr);
  } else if (field_type == CONFIG_TYPE_FLOAT) {
    *out_value = *(float*)field_ptr;
  } else {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_INVALID;
  }

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return ONVIF_SUCCESS;
}

/**
 * @brief Set integer configuration value with validation
 */
int config_runtime_set_int(config_section_t section, const char* key, int value) {
  config_value_type_t field_type = CONFIG_TYPE_INT;
  void* field_ptr = NULL;
  const config_schema_entry_t* schema = NULL;

  if (key == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (config_runtime_validate_section(section) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (config_runtime_validate_key(key) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Get pointer to the field */
  field_ptr = config_runtime_get_field_ptr(section, key, &field_type);
  if (field_ptr == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  /* Validate type matches */
  if (field_type != CONFIG_TYPE_INT && field_type != CONFIG_TYPE_BOOL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Find schema entry for bounds validation */
  schema = config_runtime_find_schema_entry(section, key);
  if (schema != NULL) {
    /* Validate value against schema bounds */
    if (config_runtime_validate_int_value(schema, value) != ONVIF_SUCCESS) {
      pthread_mutex_unlock(&g_config_runtime_mutex);
      return ONVIF_ERROR_INVALID_PARAMETER;
    }
  }

  /* Set the value */
  *(int*)field_ptr = value;

  /* Increment generation counter to signal change */
  g_config_runtime_generation++;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  /* Queue for async persistence */
  config_runtime_queue_persistence_update(section, key, &value, CONFIG_TYPE_INT);

  return ONVIF_SUCCESS;
}

/**
 * @brief Set string configuration value with validation
 */
int config_runtime_set_string(config_section_t section, const char* key, const char* value) {
  config_value_type_t field_type = CONFIG_TYPE_STRING;
  void* field_ptr = NULL;
  char* str_field = NULL;
  const config_schema_entry_t* schema = NULL;
  size_t max_len = CONFIG_STRING_MAX_LEN_DEFAULT;

  if (key == NULL || value == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (config_runtime_validate_section(section) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (config_runtime_validate_key(key) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Get pointer to the field */
  field_ptr = config_runtime_get_field_ptr(section, key, &field_type);
  if (field_ptr == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  /* Validate type matches */
  if (field_type != CONFIG_TYPE_STRING) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Find schema entry for length validation */
  schema = config_runtime_find_schema_entry(section, key);
  if (schema != NULL) {
    /* Validate value against schema */
    if (config_runtime_validate_string_value(schema, value) != ONVIF_SUCCESS) {
      pthread_mutex_unlock(&g_config_runtime_mutex);
      return ONVIF_ERROR_INVALID_PARAMETER;
    }
    max_len = schema->max_length;
  } else {
    /* Fallback: Determine max length based on section */
    if (section == CONFIG_SECTION_LOGGING) {
      max_len = CONFIG_STRING_MAX_LEN_SHORT;
    } else if (section == CONFIG_SECTION_ONVIF || section == CONFIG_SECTION_DEVICE) {
      max_len = CONFIG_STRING_MAX_LEN_STANDARD;
    }
  }

  /* Copy the string safely */
  str_field = (char*)field_ptr;
  strncpy(str_field, value, max_len - 1);
  str_field[max_len - 1] = '\0'; /* Ensure null termination */

  /* Increment generation counter to signal change */
  g_config_runtime_generation++;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  /* Queue for async persistence */
  config_runtime_queue_persistence_update(section, key, value, CONFIG_TYPE_STRING);

  return ONVIF_SUCCESS;
}

/**
 * @brief Set boolean configuration value with validation
 */
int config_runtime_set_bool(config_section_t section, const char* key, int value) {
  /* Booleans are stored as integers */
  return config_runtime_set_int(section, key, value);
}

/**
 * @brief Set float configuration value with validation
 */
int config_runtime_set_float(config_section_t section, const char* key, float value) {
  config_value_type_t field_type = CONFIG_TYPE_FLOAT;
  void* field_ptr = NULL;

  if (key == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (config_runtime_validate_section(section) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (config_runtime_validate_key(key) != ONVIF_SUCCESS) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Get pointer to the field */
  field_ptr = config_runtime_get_field_ptr(section, key, &field_type);
  if (field_ptr == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  /* Type must be float or int (for conversion) */
  if (field_type == CONFIG_TYPE_FLOAT) {
    *(float*)field_ptr = value;
  } else if (field_type == CONFIG_TYPE_INT) {
    *(int*)field_ptr = (int)value;
  } else {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_INVALID;
  }

  g_config_runtime_generation++;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  /* Queue for async persistence */
  config_runtime_queue_persistence_update(section, key, &value, CONFIG_TYPE_FLOAT);

  return ONVIF_SUCCESS;
}

/**
 * @brief Get current configuration snapshot
 */
const struct application_config* config_runtime_snapshot(void) {
  const struct application_config* snapshot = NULL;

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (g_config_runtime_initialized && g_config_runtime_app_config != NULL) {
    snapshot = g_config_runtime_app_config;
  }

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return snapshot;
}

/**
 * @brief Get current configuration generation counter
 */
uint32_t config_runtime_get_generation(void) {
  uint32_t generation;

  pthread_mutex_lock(&g_config_runtime_mutex);
  generation = g_config_runtime_generation;
  pthread_mutex_unlock(&g_config_runtime_mutex);

  return generation;
}

/* Persistence Queue Implementation (User Story 3) */

/**
 * @brief Find existing queue entry for coalescing
 *
 * @param[in] section Configuration section
 * @param[in] key Configuration key
 * @return Index of existing entry, or -1 if not found
 */
static int config_runtime_find_queue_entry(config_section_t section, const char* key) {
  size_t i;

  for (i = 0; i < g_persistence_queue_count; i++) {
    if (g_persistence_queue[i].section == section && strcmp(g_persistence_queue[i].key, key) == 0) {
      return (int)i;
    }
  }

  return -1;
}

/**
 * @brief Queue a configuration update for async persistence
 *
 * This function implements coalescing: multiple updates to the same key
 * will replace the previous queued value rather than adding a new entry.
 */
int config_runtime_queue_persistence_update(config_section_t section, const char* key,
                                            const void* value, config_value_type_t type) {
  int existing_index;
  persistence_queue_entry_t* entry;

  if (key == NULL || value == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_persistence_queue_mutex);

  /* Check for existing entry (coalescing) */
  existing_index = config_runtime_find_queue_entry(section, key);

  if (existing_index >= 0) {
    /* Update existing entry (coalescing) */
    entry = &g_persistence_queue[existing_index];
  } else {
    /* Add new entry */
    if (g_persistence_queue_count >= CONFIG_PERSISTENCE_QUEUE_MAX) {
      pthread_mutex_unlock(&g_persistence_queue_mutex);
      return ONVIF_ERROR_RESOURCE_LIMIT;
    }

    entry = &g_persistence_queue[g_persistence_queue_count];
    g_persistence_queue_count++;

    /* Initialize entry */
    entry->section = section;
    strncpy(entry->key, key, sizeof(entry->key) - 1);
    entry->key[sizeof(entry->key) - 1] = '\0';
    entry->type = type;
  }

  /* Copy the value based on type */
  switch (type) {
  case CONFIG_TYPE_INT:
  case CONFIG_TYPE_BOOL:
    entry->value.int_value = *(const int*)value;
    break;

  case CONFIG_TYPE_FLOAT:
    entry->value.float_value = *(const float*)value;
    break;

  case CONFIG_TYPE_STRING:
    strncpy(entry->value.string_value, (const char*)value, sizeof(entry->value.string_value) - 1);
    entry->value.string_value[sizeof(entry->value.string_value) - 1] = '\0';
    break;

  default:
    pthread_mutex_unlock(&g_persistence_queue_mutex);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Update timestamp */
  entry->timestamp = (uint64_t)time(NULL);

  pthread_mutex_unlock(&g_persistence_queue_mutex);

  return ONVIF_SUCCESS;
}

/**
 * @brief Process pending persistence queue entries
 *
 * Writes all queued configuration updates to persistent storage.
 * This function is called during shutdown or can be called periodically.
 */
int config_runtime_process_persistence_queue(void) {
  int result = ONVIF_SUCCESS;
  size_t queue_count = 0;

  pthread_mutex_lock(&g_persistence_queue_mutex);
  queue_count = g_persistence_queue_count;
  pthread_mutex_unlock(&g_persistence_queue_mutex);

  /* Early return if queue is empty */
  if (queue_count == 0) {
    return ONVIF_SUCCESS;
  }

  /* Save the current runtime configuration to disk */
  result = config_storage_save(ONVIF_CONFIG_FILE, NULL);

  if (result != ONVIF_SUCCESS) {
    platform_log_error("[CONFIG] Failed to persist %zu configuration updates to %s (error=%d)\n",
                       queue_count, ONVIF_CONFIG_FILE, result);
    /* Don't clear queue on failure - allow retry */
    return result;
  }

  /* Success - clear the queue */
  pthread_mutex_lock(&g_persistence_queue_mutex);
  g_persistence_queue_count = 0;
  pthread_mutex_unlock(&g_persistence_queue_mutex);

  platform_log_debug("[CONFIG] Successfully persisted %zu configuration updates to %s\n",
                     queue_count, ONVIF_CONFIG_FILE);

  return ONVIF_SUCCESS;
}

/**
 * @brief Get persistence queue status
 *
 * @return Number of pending operations, or -1 on error
 */
int config_runtime_get_persistence_status(void) {
  int count;

  pthread_mutex_lock(&g_persistence_queue_mutex);
  count = (int)g_persistence_queue_count;
  pthread_mutex_unlock(&g_persistence_queue_mutex);

  return count;
}

/**
 * @brief Get stream profile configuration
 */
int config_runtime_get_stream_profile(int profile_index, video_config_t* profile) {
  config_section_t section;
  int result;

  /* Validate parameters */
  if (profile_index < 0 || profile_index > 3) {
    platform_log_error("[CONFIG] Invalid profile index: %d (valid range: 0-3)\n", profile_index);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (profile == NULL) {
    platform_log_error("[CONFIG] NULL profile pointer\n");
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Check initialization */
  pthread_mutex_lock(&g_config_runtime_mutex);
  if (!g_config_runtime_initialized) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }
  pthread_mutex_unlock(&g_config_runtime_mutex);

  /* Map profile index to section */
  section = CONFIG_SECTION_STREAM_PROFILE_1 + profile_index;

  /* Retrieve all profile parameters */
  result = config_runtime_get_string(section, "name", profile->name, sizeof(profile->name));
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_get_int(section, "width", &profile->width);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_get_int(section, "height", &profile->height);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_get_int(section, "fps", &profile->fps);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_get_int(section, "bitrate", &profile->bitrate);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_get_int(section, "gop_size", &profile->gop_size);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_get_int(section, "profile", &profile->profile);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_get_int(section, "codec_type", &profile->codec_type);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_get_int(section, "br_mode", &profile->br_mode);
  if (result != ONVIF_SUCCESS)
    return result;

  platform_log_debug("[CONFIG] Retrieved stream profile %d: %dx%d@%dfps, %dkbps\n",
                     profile_index + 1, profile->width, profile->height, profile->fps,
                     profile->bitrate);

  return ONVIF_SUCCESS;
}

/**
 * @brief Set stream profile configuration
 */
int config_runtime_set_stream_profile(int profile_index, const video_config_t* profile) {
  config_section_t section;
  int result;

  /* Validate parameters */
  if (profile_index < 0 || profile_index > 3) {
    platform_log_error("[CONFIG] Invalid profile index: %d (valid range: 0-3)\n", profile_index);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (profile == NULL) {
    platform_log_error("[CONFIG] NULL profile pointer\n");
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate profile parameters first */
  result = config_runtime_validate_stream_profile(profile);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Check initialization */
  pthread_mutex_lock(&g_config_runtime_mutex);
  if (!g_config_runtime_initialized) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }
  pthread_mutex_unlock(&g_config_runtime_mutex);

  /* Map profile index to section */
  section = CONFIG_SECTION_STREAM_PROFILE_1 + profile_index;

  /* Set all profile parameters (schema validation happens in set_int/set_string) */
  result = config_runtime_set_string(section, "name", profile->name);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_set_int(section, "width", profile->width);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_set_int(section, "height", profile->height);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_set_int(section, "fps", profile->fps);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_set_int(section, "bitrate", profile->bitrate);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_set_int(section, "gop_size", profile->gop_size);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_set_int(section, "profile", profile->profile);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_set_int(section, "codec_type", profile->codec_type);
  if (result != ONVIF_SUCCESS)
    return result;

  result = config_runtime_set_int(section, "br_mode", profile->br_mode);
  if (result != ONVIF_SUCCESS)
    return result;

  platform_log_info("[CONFIG] Updated stream profile %d: %dx%d@%dfps, %dkbps\n", profile_index + 1,
                    profile->width, profile->height, profile->fps, profile->bitrate);

  return ONVIF_SUCCESS;
}

/**
 * @brief Validate stream profile parameters
 */
int config_runtime_validate_stream_profile(const video_config_t* profile) {
  const config_schema_entry_t* schema_entry;

  if (profile == NULL) {
    platform_log_error("[CONFIG] NULL profile pointer for validation\n");
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate width */
  schema_entry = config_runtime_find_schema_entry(CONFIG_SECTION_STREAM_PROFILE_1, "width");
  if (schema_entry &&
      config_runtime_validate_int_value(schema_entry, profile->width) != ONVIF_SUCCESS) {
    platform_log_error("[CONFIG] Invalid width: %d (valid range: %d-%d)\n", profile->width,
                       schema_entry->min_value, schema_entry->max_value);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate height */
  schema_entry = config_runtime_find_schema_entry(CONFIG_SECTION_STREAM_PROFILE_1, "height");
  if (schema_entry &&
      config_runtime_validate_int_value(schema_entry, profile->height) != ONVIF_SUCCESS) {
    platform_log_error("[CONFIG] Invalid height: %d (valid range: %d-%d)\n", profile->height,
                       schema_entry->min_value, schema_entry->max_value);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate FPS */
  schema_entry = config_runtime_find_schema_entry(CONFIG_SECTION_STREAM_PROFILE_1, "fps");
  if (schema_entry &&
      config_runtime_validate_int_value(schema_entry, profile->fps) != ONVIF_SUCCESS) {
    platform_log_error("[CONFIG] Invalid FPS: %d (valid range: %d-%d)\n", profile->fps,
                       schema_entry->min_value, schema_entry->max_value);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate bitrate */
  schema_entry = config_runtime_find_schema_entry(CONFIG_SECTION_STREAM_PROFILE_1, "bitrate");
  if (schema_entry &&
      config_runtime_validate_int_value(schema_entry, profile->bitrate) != ONVIF_SUCCESS) {
    platform_log_error("[CONFIG] Invalid bitrate: %d (valid range: %d-%d kbps)\n", profile->bitrate,
                       schema_entry->min_value, schema_entry->max_value);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate GOP size */
  schema_entry = config_runtime_find_schema_entry(CONFIG_SECTION_STREAM_PROFILE_1, "gop_size");
  if (schema_entry &&
      config_runtime_validate_int_value(schema_entry, profile->gop_size) != ONVIF_SUCCESS) {
    platform_log_error("[CONFIG] Invalid GOP size: %d (valid range: %d-%d)\n", profile->gop_size,
                       schema_entry->min_value, schema_entry->max_value);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate profile */
  schema_entry = config_runtime_find_schema_entry(CONFIG_SECTION_STREAM_PROFILE_1, "profile");
  if (schema_entry &&
      config_runtime_validate_int_value(schema_entry, profile->profile) != ONVIF_SUCCESS) {
    platform_log_error("[CONFIG] Invalid profile: %d (valid range: %d-%d)\n", profile->profile,
                       schema_entry->min_value, schema_entry->max_value);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate codec type */
  schema_entry = config_runtime_find_schema_entry(CONFIG_SECTION_STREAM_PROFILE_1, "codec_type");
  if (schema_entry &&
      config_runtime_validate_int_value(schema_entry, profile->codec_type) != ONVIF_SUCCESS) {
    platform_log_error("[CONFIG] Invalid codec type: %d (valid range: %d-%d)\n",
                       profile->codec_type, schema_entry->min_value, schema_entry->max_value);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate bitrate mode */
  schema_entry = config_runtime_find_schema_entry(CONFIG_SECTION_STREAM_PROFILE_1, "br_mode");
  if (schema_entry &&
      config_runtime_validate_int_value(schema_entry, profile->br_mode) != ONVIF_SUCCESS) {
    platform_log_error("[CONFIG] Invalid bitrate mode: %d (valid range: %d-%d)\n", profile->br_mode,
                       schema_entry->min_value, schema_entry->max_value);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  platform_log_debug("[CONFIG] Stream profile validation passed: %dx%d@%dfps, %dkbps\n",
                     profile->width, profile->height, profile->fps, profile->bitrate);

  return ONVIF_SUCCESS;
}

/**
 * @brief Get stream profile count
 */
int config_runtime_get_stream_profile_count(void) {
  return 4; /* Fixed at 4 profiles per FR-012, FR-013 */
}

/* PTZ Preset Profile Management */

/**
 * @brief Get PTZ presets for a specific profile
 */
int config_runtime_get_ptz_profile_presets(int profile_index, ptz_preset_list_t* presets) {
  config_section_t section;
  int result;
  int i;
  char key[64];

  /* Validate parameters */
  if (profile_index < 0 || profile_index > 3) {
    platform_log_error("[CONFIG] Invalid PTZ profile index: %d (valid range: 0-3)\n",
                       profile_index);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (presets == NULL) {
    platform_log_error("[CONFIG] NULL presets pointer\n");
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Check initialization - assume mutex is already held by caller */
  if (!g_config_runtime_initialized) {
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Map profile index to section */
  section = CONFIG_SECTION_PTZ_PRESET_PROFILE_1 + profile_index;

  /* Get preset count */
  result = config_runtime_get_int(section, "preset_count", &presets->preset_count);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Retrieve each preset */
  for (i = 0; i < presets->preset_count && i < 4; i++) {
    /* Get token */
    snprintf(key, sizeof(key), "preset%d_token", i + 1);
    result = config_runtime_get_string(section, key, presets->presets[i].token,
                                       sizeof(presets->presets[i].token));
    if (result != ONVIF_SUCCESS) {
      return result;
    }

    /* Get name */
    snprintf(key, sizeof(key), "preset%d_name", i + 1);
    result = config_runtime_get_string(section, key, presets->presets[i].name,
                                       sizeof(presets->presets[i].name));
    if (result != ONVIF_SUCCESS) {
      return result;
    }

    /* Get pan */
    snprintf(key, sizeof(key), "preset%d_pan", i + 1);
    result = config_runtime_get_float(section, key, &presets->presets[i].ptz_position.pan_tilt.x);
    if (result != ONVIF_SUCCESS) {
      return result;
    }

    /* Get tilt */
    snprintf(key, sizeof(key), "preset%d_tilt", i + 1);
    result = config_runtime_get_float(section, key, &presets->presets[i].ptz_position.pan_tilt.y);
    if (result != ONVIF_SUCCESS) {
      return result;
    }

    /* Get zoom */
    snprintf(key, sizeof(key), "preset%d_zoom", i + 1);
    result = config_runtime_get_float(section, key, &presets->presets[i].ptz_position.zoom);
    if (result != ONVIF_SUCCESS) {
      return result;
    }

    /* Set space URI (empty for now, will be set by PTZ service) */
    presets->presets[i].ptz_position.space[0] = '\0';
  }

  platform_log_debug("[CONFIG] Retrieved %d PTZ presets for profile %d\n", presets->preset_count,
                     profile_index + 1);

  return ONVIF_SUCCESS;
}

/**
 * @brief Set PTZ presets for a specific profile
 */
int config_runtime_set_ptz_profile_presets(int profile_index, const ptz_preset_list_t* presets) {
  config_section_t section;
  int result;
  int i;
  char key[64];

  /* Validate parameters */
  if (profile_index < 0 || profile_index > 3) {
    platform_log_error("[CONFIG] Invalid PTZ profile index: %d (valid range: 0-3)\n",
                       profile_index);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (presets == NULL) {
    platform_log_error("[CONFIG] NULL presets pointer\n");
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate presets first */
  result = config_runtime_validate_ptz_profile_presets(presets);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Check initialization - assume mutex is already held by caller */
  if (!g_config_runtime_initialized) {
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Map profile index to section */
  section = CONFIG_SECTION_PTZ_PRESET_PROFILE_1 + profile_index;

  /* Set preset count */
  result = config_runtime_set_int(section, "preset_count", presets->preset_count);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Set each preset */
  for (i = 0; i < presets->preset_count && i < 4; i++) {
    /* Set token */
    snprintf(key, sizeof(key), "preset%d_token", i + 1);
    result = config_runtime_set_string(section, key, presets->presets[i].token);
    if (result != ONVIF_SUCCESS)
      return result;

    /* Set name */
    snprintf(key, sizeof(key), "preset%d_name", i + 1);
    result = config_runtime_set_string(section, key, presets->presets[i].name);
    if (result != ONVIF_SUCCESS)
      return result;

    /* Set pan */
    snprintf(key, sizeof(key), "preset%d_pan", i + 1);
    result = config_runtime_set_float(section, key, presets->presets[i].ptz_position.pan_tilt.x);
    if (result != ONVIF_SUCCESS)
      return result;

    /* Set tilt */
    snprintf(key, sizeof(key), "preset%d_tilt", i + 1);
    result = config_runtime_set_float(section, key, presets->presets[i].ptz_position.pan_tilt.y);
    if (result != ONVIF_SUCCESS)
      return result;

    /* Set zoom */
    snprintf(key, sizeof(key), "preset%d_zoom", i + 1);
    result = config_runtime_set_float(section, key, presets->presets[i].ptz_position.zoom);
    if (result != ONVIF_SUCCESS)
      return result;
  }

  /* Clear remaining preset slots */
  for (i = presets->preset_count; i < 4; i++) {
    /* Clear token */
    snprintf(key, sizeof(key), "preset%d_token", i + 1);
    result = config_runtime_set_string(section, key, "");
    if (result != ONVIF_SUCCESS)
      return result;

    /* Clear name */
    snprintf(key, sizeof(key), "preset%d_name", i + 1);
    result = config_runtime_set_string(section, key, "");
    if (result != ONVIF_SUCCESS)
      return result;

    /* Reset positions to zero */
    snprintf(key, sizeof(key), "preset%d_pan", i + 1);
    result = config_runtime_set_float(section, key, 0.0f);
    if (result != ONVIF_SUCCESS)
      return result;

    snprintf(key, sizeof(key), "preset%d_tilt", i + 1);
    result = config_runtime_set_float(section, key, 0.0f);
    if (result != ONVIF_SUCCESS)
      return result;

    snprintf(key, sizeof(key), "preset%d_zoom", i + 1);
    result = config_runtime_set_float(section, key, 0.0f);
    if (result != ONVIF_SUCCESS)
      return result;
  }

  platform_log_info("[CONFIG] Updated %d PTZ presets for profile %d\n", presets->preset_count,
                    profile_index + 1);

  return ONVIF_SUCCESS;
}

/**
 * @brief Validate PTZ preset list parameters
 */
int config_runtime_validate_ptz_profile_presets(const ptz_preset_list_t* presets) {
  const config_schema_entry_t* schema_entry;
  int i;

  if (presets == NULL) {
    platform_log_error("[CONFIG] NULL presets pointer for validation\n");
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate preset count */
  schema_entry =
    config_runtime_find_schema_entry(CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "preset_count");
  if (schema_entry &&
      config_runtime_validate_int_value(schema_entry, presets->preset_count) != ONVIF_SUCCESS) {
    platform_log_error("[CONFIG] Invalid preset count: %d (valid range: %d-%d)\n",
                       presets->preset_count, schema_entry->min_value, schema_entry->max_value);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate each preset's position values */
  for (i = 0; i < presets->preset_count && i < 4; i++) {
    /* Validate pan (-180 to 180) */
    schema_entry =
      config_runtime_find_schema_entry(CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "preset1_pan");
    if (schema_entry &&
        config_runtime_validate_float_value(
          schema_entry, presets->presets[i].ptz_position.pan_tilt.x) != ONVIF_SUCCESS) {
      platform_log_error("[CONFIG] Invalid pan for preset %d: %.2f (valid range: %.2f-%.2f)\n",
                         i + 1, presets->presets[i].ptz_position.pan_tilt.x,
                         (float)schema_entry->min_value, (float)schema_entry->max_value);
      return ONVIF_ERROR_INVALID_PARAMETER;
    }

    /* Validate tilt (-90 to 90) */
    schema_entry =
      config_runtime_find_schema_entry(CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "preset1_tilt");
    if (schema_entry &&
        config_runtime_validate_float_value(
          schema_entry, presets->presets[i].ptz_position.pan_tilt.y) != ONVIF_SUCCESS) {
      platform_log_error("[CONFIG] Invalid tilt for preset %d: %.2f (valid range: %.2f-%.2f)\n",
                         i + 1, presets->presets[i].ptz_position.pan_tilt.y,
                         (float)schema_entry->min_value, (float)schema_entry->max_value);
      return ONVIF_ERROR_INVALID_PARAMETER;
    }

    /* Validate zoom (0 to 1) */
    schema_entry =
      config_runtime_find_schema_entry(CONFIG_SECTION_PTZ_PRESET_PROFILE_1, "preset1_zoom");
    if (schema_entry && config_runtime_validate_float_value(
                          schema_entry, presets->presets[i].ptz_position.zoom) != ONVIF_SUCCESS) {
      platform_log_error("[CONFIG] Invalid zoom for preset %d: %.2f (valid range: %.2f-%.2f)\n",
                         i + 1, presets->presets[i].ptz_position.zoom,
                         (float)schema_entry->min_value, (float)schema_entry->max_value);
      return ONVIF_ERROR_INVALID_PARAMETER;
    }

    /* Validate token and name are not empty */
    if (presets->presets[i].token[0] == '\0') {
      platform_log_error("[CONFIG] Empty token for preset %d\n", i + 1);
      return ONVIF_ERROR_INVALID_PARAMETER;
    }

    if (presets->presets[i].name[0] == '\0') {
      platform_log_error("[CONFIG] Empty name for preset %d\n", i + 1);
      return ONVIF_ERROR_INVALID_PARAMETER;
    }
  }

  platform_log_debug("[CONFIG] PTZ preset list validation passed: %d presets\n",
                     presets->preset_count);

  return ONVIF_SUCCESS;
}

/* User Credential Management Implementation (User Story 5) */

/**
 * @brief Hash password using salted SHA256 from hash_utils
 * Generates random salt and produces salt$hash format
 */
int config_runtime_hash_password(const char* password, char* hash_output, size_t output_size) {
  /* Validate parameters */
  if (password == NULL || hash_output == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Output buffer must hold salted hash: salt$hash format (128 bytes) */
  if (output_size < ONVIF_PASSWORD_HASH_SIZE) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Use hash_utils salted password hashing */
  return onvif_hash_password(password, hash_output, output_size);
}

/**
 * @brief Verify password against stored salted hash
 */
int config_runtime_verify_password(const char* password, const char* stored_hash) {
  int result;

  /* Validate parameters */
  if (password == NULL || stored_hash == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Use hash_utils salted password verification */
  result = onvif_verify_password(password, stored_hash);

  /* Convert hash_utils result to config_runtime result */
  if (result == ONVIF_SUCCESS) {
    return ONVIF_SUCCESS;
  }

  if (result == ONVIF_ERROR_AUTH_FAILED) {
    return ONVIF_ERROR_AUTHENTICATION_FAILED;
  }

  return result;
}

/**
 * @brief Add a new user account
 */
int config_runtime_add_user(const char* username, const char* password) {
  int user_index;
  int result;
  char password_hash[ONVIF_PASSWORD_HASH_SIZE] = {0};

  /* Validate parameters */
  if (username == NULL || password == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate username format */
  result = config_runtime_validate_username(username);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Check if user already exists */
  user_index = config_runtime_find_user_index(username);
  if (user_index >= 0) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_ALREADY_EXISTS;
  }

  /* Find free slot */
  user_index = config_runtime_find_free_user_slot();
  if (user_index < 0) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_OUT_OF_RESOURCES;
  }

  pthread_mutex_unlock(&g_config_runtime_mutex);

  /* Hash the password */
  result = config_runtime_hash_password(password, password_hash, sizeof(password_hash));
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Add the user */
  pthread_mutex_lock(&g_config_runtime_mutex);

  strncpy(g_config_runtime_app_config->users[user_index].username, username, MAX_USERNAME_LENGTH);
  g_config_runtime_app_config->users[user_index].username[MAX_USERNAME_LENGTH] = '\0';

  strncpy(g_config_runtime_app_config->users[user_index].password_hash, password_hash,
          MAX_PASSWORD_HASH_LENGTH);
  g_config_runtime_app_config->users[user_index].password_hash[MAX_PASSWORD_HASH_LENGTH] = '\0';

  g_config_runtime_app_config->users[user_index].active = 1;

  g_config_runtime_generation++;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  platform_log_info("[CONFIG] Added user: %s\n", username);

  return ONVIF_SUCCESS;
}

/**
 * @brief Remove a user account
 */
int config_runtime_remove_user(const char* username) {
  int user_index;

  /* Validate parameters */
  if (username == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Find user */
  user_index = config_runtime_find_user_index(username);
  if (user_index < 0) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  /* Mark as inactive and clear data */
  g_config_runtime_app_config->users[user_index].active = 0;
  memset(g_config_runtime_app_config->users[user_index].username, 0,
         sizeof(g_config_runtime_app_config->users[user_index].username));
  memset(g_config_runtime_app_config->users[user_index].password_hash, 0,
         sizeof(g_config_runtime_app_config->users[user_index].password_hash));

  g_config_runtime_generation++;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  platform_log_info("[CONFIG] Removed user: %s\n", username);

  return ONVIF_SUCCESS;
}

/**
 * @brief Update user password
 */
int config_runtime_update_user_password(const char* username, const char* new_password) {
  int user_index;
  int result;
  char password_hash[ONVIF_PASSWORD_HASH_SIZE] = {0};

  /* Validate parameters */
  if (username == NULL || new_password == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Find user */
  user_index = config_runtime_find_user_index(username);
  if (user_index < 0) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  pthread_mutex_unlock(&g_config_runtime_mutex);

  /* Hash the new password */
  result = config_runtime_hash_password(new_password, password_hash, sizeof(password_hash));
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Update the password hash */
  pthread_mutex_lock(&g_config_runtime_mutex);

  strncpy(g_config_runtime_app_config->users[user_index].password_hash, password_hash,
          MAX_PASSWORD_HASH_LENGTH);
  g_config_runtime_app_config->users[user_index].password_hash[MAX_PASSWORD_HASH_LENGTH] = '\0';

  g_config_runtime_generation++;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  platform_log_info("[CONFIG] Updated password for user: %s\n", username);

  return ONVIF_SUCCESS;
}

/**
 * @brief Authenticate user with username and password
 */
int config_runtime_authenticate_user(const char* username, const char* password) {
  int user_index = -1;
  int result = ONVIF_ERROR_INVALID_PARAMETER;

  /* Validate parameters */
  if (username == NULL || password == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Find user */
  user_index = config_runtime_find_user_index(username);
  if (user_index < 0) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_FOUND;
  }

  /* Verify password against stored hash */
  result = config_runtime_verify_password(
    password, g_config_runtime_app_config->users[user_index].password_hash);

  pthread_mutex_unlock(&g_config_runtime_mutex);

  return result;
}

/**
 * @brief Get list of all usernames (T081)
 */
int config_runtime_enumerate_users(char usernames[][MAX_USERNAME_LENGTH + 1], int max_users,
                                   int* user_count) {
  int i = 0;
  int count = 0;

  /* Validate parameters */
  if (usernames == NULL || user_count == NULL || max_users <= 0) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  pthread_mutex_lock(&g_config_runtime_mutex);

  if (!g_config_runtime_initialized || g_config_runtime_app_config == NULL) {
    pthread_mutex_unlock(&g_config_runtime_mutex);
    return ONVIF_ERROR_NOT_INITIALIZED;
  }

  /* Iterate through all user slots and copy active usernames */
  for (i = 0; i < MAX_USERS && count < max_users; i++) {
    if (g_config_runtime_app_config->users[i].active) {
      strncpy(usernames[count], g_config_runtime_app_config->users[i].username,
              MAX_USERNAME_LENGTH);
      usernames[count][MAX_USERNAME_LENGTH] = '\0';
      count++;
    }
  }

  *user_count = count;

  pthread_mutex_unlock(&g_config_runtime_mutex);

  platform_log_debug("[CONFIG] Enumerated %d active users\n", count);

  return ONVIF_SUCCESS;
}

/* Helper functions */

static int config_runtime_validate_section(config_section_t section) {
  /* Validate that section is within reasonable bounds */
  if (section < CONFIG_SECTION_ONVIF || section > CONFIG_SECTION_USER_8) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }
  return ONVIF_SUCCESS;
}

static int config_runtime_validate_key(const char* key) {
  if (key == NULL || key[0] == '\0') {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }
  return ONVIF_SUCCESS;
}

/**
 * @brief Get pointer to configuration section structure
 */
static void* config_runtime_get_section_ptr(config_section_t section) {
  if (g_config_runtime_app_config == NULL) {
    return NULL;
  }

  switch (section) {
  case CONFIG_SECTION_ONVIF:
    return &g_config_runtime_app_config->onvif;
  case CONFIG_SECTION_NETWORK:
    return g_config_runtime_app_config->network;
  case CONFIG_SECTION_DEVICE:
    return g_config_runtime_app_config->device;
  case CONFIG_SECTION_LOGGING:
    return g_config_runtime_app_config->logging;
  case CONFIG_SECTION_SERVER:
    return g_config_runtime_app_config->server;
  case CONFIG_SECTION_MAIN_STREAM:
    return g_config_runtime_app_config->main_stream;
  case CONFIG_SECTION_SUB_STREAM:
    return g_config_runtime_app_config->sub_stream;
  case CONFIG_SECTION_STREAM_PROFILE_1:
    return g_config_runtime_app_config->stream_profile_1;
  case CONFIG_SECTION_STREAM_PROFILE_2:
    return g_config_runtime_app_config->stream_profile_2;
  case CONFIG_SECTION_STREAM_PROFILE_3:
    return g_config_runtime_app_config->stream_profile_3;
  case CONFIG_SECTION_STREAM_PROFILE_4:
    return g_config_runtime_app_config->stream_profile_4;
  case CONFIG_SECTION_IMAGING:
    return g_config_runtime_app_config->imaging;
  case CONFIG_SECTION_AUTO_DAYNIGHT:
    return g_config_runtime_app_config->auto_daynight;
  case CONFIG_SECTION_PTZ_PRESET_PROFILE_1:
    return g_config_runtime_app_config->ptz_preset_profile_1;
  case CONFIG_SECTION_PTZ_PRESET_PROFILE_2:
    return g_config_runtime_app_config->ptz_preset_profile_2;
  case CONFIG_SECTION_PTZ_PRESET_PROFILE_3:
    return g_config_runtime_app_config->ptz_preset_profile_3;
  case CONFIG_SECTION_PTZ_PRESET_PROFILE_4:
    return g_config_runtime_app_config->ptz_preset_profile_4;
  default:
    return NULL;
  }
}

/**
 * @brief Get pointer to specific field within a configuration section
 *
 * This function maps section/key pairs to actual struct field pointers.
 * It also returns the type of the field for validation.
 */
static void* config_runtime_get_field_ptr(config_section_t section, const char* key,
                                          config_value_type_t* out_type) {
  void* section_ptr = config_runtime_get_section_ptr(section);
  if (section_ptr == NULL) {
    return NULL;
  }

  /* Map common ONVIF section keys */
  if (section == CONFIG_SECTION_ONVIF) {
    struct onvif_settings* onvif = (struct onvif_settings*)section_ptr;
    if (strcmp(key, "enabled") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &onvif->enabled;
    } else if (strcmp(key, "http_port") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &onvif->http_port;
    } else if (strcmp(key, "auth_enabled") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &onvif->auth_enabled;
    } else if (strcmp(key, "username") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return onvif->username;
    } else if (strcmp(key, "password") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return onvif->password;
    }
  }
  /* Map network section keys */
  else if (section == CONFIG_SECTION_NETWORK) {
    struct network_settings* network = (struct network_settings*)section_ptr;
    if (strcmp(key, "rtsp_port") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &network->rtsp_port;
    } else if (strcmp(key, "snapshot_port") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &network->snapshot_port;
    } else if (strcmp(key, "ws_discovery_port") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &network->ws_discovery_port;
    }
  }
  /* Map device section keys */
  else if (section == CONFIG_SECTION_DEVICE) {
    struct device_info* device = (struct device_info*)section_ptr;
    if (strcmp(key, "manufacturer") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return device->manufacturer;
    } else if (strcmp(key, "model") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return device->model;
    } else if (strcmp(key, "firmware_version") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return device->firmware_version;
    } else if (strcmp(key, "serial_number") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return device->serial_number;
    } else if (strcmp(key, "hardware_id") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return device->hardware_id;
    }
  }
  /* Map logging section keys */
  else if (section == CONFIG_SECTION_LOGGING) {
    struct logging_settings* logging = (struct logging_settings*)section_ptr;
    if (strcmp(key, "enabled") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &logging->enabled;
    } else if (strcmp(key, "use_colors") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &logging->use_colors;
    } else if (strcmp(key, "use_timestamps") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &logging->use_timestamps;
    } else if (strcmp(key, "min_level") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &logging->min_level;
    } else if (strcmp(key, "tag") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return logging->tag;
    } else if (strcmp(key, "http_verbose") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &logging->http_verbose;
    }
  }
  /* Map server section keys */
  else if (section == CONFIG_SECTION_SERVER) {
    struct server_settings* server = (struct server_settings*)section_ptr;
    if (strcmp(key, "worker_threads") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &server->worker_threads;
    } else if (strcmp(key, "max_connections") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &server->max_connections;
    } else if (strcmp(key, "connection_timeout") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &server->connection_timeout;
    } else if (strcmp(key, "keepalive_timeout") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &server->keepalive_timeout;
    } else if (strcmp(key, "epoll_timeout") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &server->epoll_timeout;
    } else if (strcmp(key, "cleanup_interval") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &server->cleanup_interval;
    }
  }
  /* Map stream profile keys (User Story 4) */
  else if (section == CONFIG_SECTION_STREAM_PROFILE_1 ||
           section == CONFIG_SECTION_STREAM_PROFILE_2 ||
           section == CONFIG_SECTION_STREAM_PROFILE_3 ||
           section == CONFIG_SECTION_STREAM_PROFILE_4) {
    video_config_t* profile = (video_config_t*)section_ptr;
    if (strcmp(key, "name") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return profile->name;
    } else if (strcmp(key, "width") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &profile->width;
    } else if (strcmp(key, "height") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &profile->height;
    } else if (strcmp(key, "fps") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &profile->fps;
    } else if (strcmp(key, "bitrate") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &profile->bitrate;
    } else if (strcmp(key, "gop_size") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &profile->gop_size;
    } else if (strcmp(key, "profile") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &profile->profile;
    } else if (strcmp(key, "codec_type") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &profile->codec_type;
    } else if (strcmp(key, "br_mode") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &profile->br_mode;
    }
  }
  /* Map PTZ preset profile section keys */
  else if (section == CONFIG_SECTION_PTZ_PRESET_PROFILE_1 ||
           section == CONFIG_SECTION_PTZ_PRESET_PROFILE_2 ||
           section == CONFIG_SECTION_PTZ_PRESET_PROFILE_3 ||
           section == CONFIG_SECTION_PTZ_PRESET_PROFILE_4) {
    struct ptz_preset_profile* profile = (struct ptz_preset_profile*)section_ptr;
    if (strcmp(key, "preset_count") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &profile->preset_count;
    } else if (strcmp(key, "preset1_token") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return profile->preset1_token;
    } else if (strcmp(key, "preset1_name") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return profile->preset1_name;
    } else if (strcmp(key, "preset1_pan") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_FLOAT;
      return &profile->preset1_pan;
    } else if (strcmp(key, "preset1_tilt") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_FLOAT;
      return &profile->preset1_tilt;
    } else if (strcmp(key, "preset1_zoom") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_FLOAT;
      return &profile->preset1_zoom;
    } else if (strcmp(key, "preset2_token") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return profile->preset2_token;
    } else if (strcmp(key, "preset2_name") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return profile->preset2_name;
    } else if (strcmp(key, "preset2_pan") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_FLOAT;
      return &profile->preset2_pan;
    } else if (strcmp(key, "preset2_tilt") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_FLOAT;
      return &profile->preset2_tilt;
    } else if (strcmp(key, "preset2_zoom") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_FLOAT;
      return &profile->preset2_zoom;
    } else if (strcmp(key, "preset3_token") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return profile->preset3_token;
    } else if (strcmp(key, "preset3_name") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return profile->preset3_name;
    } else if (strcmp(key, "preset3_pan") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_FLOAT;
      return &profile->preset3_pan;
    } else if (strcmp(key, "preset3_tilt") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_FLOAT;
      return &profile->preset3_tilt;
    } else if (strcmp(key, "preset3_zoom") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_FLOAT;
      return &profile->preset3_zoom;
    } else if (strcmp(key, "preset4_token") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return profile->preset4_token;
    } else if (strcmp(key, "preset4_name") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_STRING;
      return profile->preset4_name;
    } else if (strcmp(key, "preset4_pan") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_FLOAT;
      return &profile->preset4_pan;
    } else if (strcmp(key, "preset4_tilt") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_FLOAT;
      return &profile->preset4_tilt;
    } else if (strcmp(key, "preset4_zoom") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_FLOAT;
      return &profile->preset4_zoom;
    }
  }
  /* Map imaging section keys */
  else if (section == CONFIG_SECTION_IMAGING) {
    struct imaging_settings* imaging = (struct imaging_settings*)section_ptr;
    if (strcmp(key, "brightness") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &imaging->brightness;
    } else if (strcmp(key, "contrast") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &imaging->contrast;
    } else if (strcmp(key, "saturation") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &imaging->saturation;
    } else if (strcmp(key, "sharpness") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &imaging->sharpness;
    } else if (strcmp(key, "hue") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &imaging->hue;
    }
  }
  /* Map auto day/night section keys */
  else if (section == CONFIG_SECTION_AUTO_DAYNIGHT) {
    struct auto_daynight_config* auto_dn = (struct auto_daynight_config*)section_ptr;
    if (strcmp(key, "mode") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return (int*)&auto_dn->mode;
    } else if (strcmp(key, "day_to_night_threshold") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &auto_dn->day_to_night_threshold;
    } else if (strcmp(key, "night_to_day_threshold") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &auto_dn->night_to_day_threshold;
    } else if (strcmp(key, "lock_time_seconds") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &auto_dn->lock_time_seconds;
    } else if (strcmp(key, "ir_led_mode") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return (int*)&auto_dn->ir_led_mode;
    } else if (strcmp(key, "ir_led_level") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &auto_dn->ir_led_level;
    } else if (strcmp(key, "enable_auto_switching") == 0) {
      if (out_type)
        *out_type = CONFIG_TYPE_INT;
      return &auto_dn->enable_auto_switching;
    }
  }

  /* Key not found in this section */
  return NULL;
}

/**
 * @brief Find schema entry for a given section and key
 *
 * @param[in] section Configuration section
 * @param[in] key Configuration key
 * @return Pointer to schema entry, or NULL if not found
 */
static const config_schema_entry_t* config_runtime_find_schema_entry(config_section_t section,
                                                                     const char* key) {
  size_t i;

  if (key == NULL) {
    return NULL;
  }

  for (i = 0; i < g_config_schema_count; i++) {
    if (g_config_schema[i].section == section && strcmp(g_config_schema[i].key, key) == 0) {
      return &g_config_schema[i];
    }
  }

  return NULL;
}

/**
 * @brief Validate integer value against schema bounds
 *
 * @param[in] schema Schema entry with validation rules
 * @param[in] value Value to validate
 * @return ONVIF_SUCCESS if valid, error code otherwise
 */
static int config_runtime_validate_int_value(const config_schema_entry_t* schema, int value) {
  validation_result_t validation_result;

  if (schema == NULL) {
    platform_log_error("[CONFIG] Schema validation failed: NULL schema pointer\n");
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Check if this is an integer or boolean type */
  if (schema->type != CONFIG_TYPE_INT && schema->type != CONFIG_TYPE_BOOL) {
    platform_log_error("[CONFIG] Schema validation failed for '%s.%s': Expected integer or boolean "
                       "type, got type %d\n",
                       schema->section_name, schema->key, schema->type);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Use common validation utility for integer bounds checking */
  validation_result = validate_int(schema->key, value, schema->min_value, schema->max_value);

  if (!validation_is_valid(&validation_result)) {
    /* Log structured validation error */
    platform_log_error(
      "[CONFIG] Configuration validation failed for '%s.%s': %s (value=%d, min=%d, max=%d)\n",
      schema->section_name, schema->key, validation_get_error_message(&validation_result), value,
      schema->min_value, schema->max_value);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Validate float value against schema bounds
 */
static int config_runtime_validate_float_value(const config_schema_entry_t* schema, float value) {
  if (schema == NULL) {
    platform_log_error("[CONFIG] Schema validation failed: NULL schema pointer\n");
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Check if this is a float type */
  if (schema->type != CONFIG_TYPE_FLOAT) {
    platform_log_error(
      "[CONFIG] Schema validation failed for '%s.%s': Expected float type, got type %d\n",
      schema->section_name, schema->key, schema->type);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Validate bounds (min_value and max_value are stored as ints in schema) */
  float min = (float)schema->min_value;
  float max = (float)schema->max_value;

  if (value < min || value > max) {
    platform_log_error("[CONFIG] Configuration validation failed for '%s.%s': Value %.2f out of "
                       "range (min=%.2f, max=%.2f)\n",
                       schema->section_name, schema->key, value, min, max);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Validate string value against schema constraints
 *
 * @param[in] schema Schema entry with validation rules
 * @param[in] value String value to validate
 * @return ONVIF_SUCCESS if valid, error code otherwise
 */
static int config_runtime_validate_string_value(const config_schema_entry_t* schema,
                                                const char* value) {
  validation_result_t validation_result;

  if (schema == NULL) {
    platform_log_error("[CONFIG] String validation failed: NULL schema pointer\n");
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (value == NULL) {
    platform_log_error("[CONFIG] String validation failed for '%s.%s': NULL value provided\n",
                       schema->section_name, schema->key);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Check if this is a string type */
  if (schema->type != CONFIG_TYPE_STRING) {
    platform_log_error(
      "[CONFIG] Schema validation failed for '%s.%s': Expected string type, got type %d\n",
      schema->section_name, schema->key, schema->type);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Use common validation utility for string validation */
  /* Note: max_length includes null terminator, so we check against max_length-1 */
  validation_result = validate_string(schema->key, value, 0, schema->max_length - 1, 1);

  if (!validation_is_valid(&validation_result)) {
    /* Log structured validation error */
    platform_log_error(
      "[CONFIG] Configuration validation failed for '%s.%s': %s (length=%zu, max=%zu)\n",
      schema->section_name, schema->key, validation_get_error_message(&validation_result),
      strlen(value), schema->max_length - 1);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Validate username format
 * Username must be 3-32 alphanumeric characters
 */
static int config_runtime_validate_username(const char* username) {
  size_t len;
  size_t i;

  if (username == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  len = strlen(username);

  /* Check length */
  if (len < 3 || len > MAX_USERNAME_LENGTH) {
    platform_log_error("[CONFIG] Invalid username length: %zu (valid range: 3-%d)\n", len,
                       MAX_USERNAME_LENGTH);
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Check for alphanumeric characters only */
  for (i = 0; i < len; i++) {
    if (!((username[i] >= 'a' && username[i] <= 'z') ||
          (username[i] >= 'A' && username[i] <= 'Z') ||
          (username[i] >= '0' && username[i] <= '9'))) {
      platform_log_error("[CONFIG] Invalid character in username: '%c' at position %zu\n",
                         username[i], i);
      return ONVIF_ERROR_INVALID_PARAMETER;
    }
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Find user index by username
 * @return User index (0-7) if found, -1 if not found
 */
static int config_runtime_find_user_index(const char* username) {
  int i;

  if (username == NULL || g_config_runtime_app_config == NULL) {
    return -1;
  }

  for (i = 0; i < MAX_USERS; i++) {
    if (g_config_runtime_app_config->users[i].active &&
        strcmp(g_config_runtime_app_config->users[i].username, username) == 0) {
      return i;
    }
  }

  return -1;
}

/**
 * @brief Find first free user slot
 * @return Free slot index (0-7) if found, -1 if all slots full
 */
static int config_runtime_find_free_user_slot(void) {
  int i;

  if (g_config_runtime_app_config == NULL) {
    return -1;
  }

  for (i = 0; i < MAX_USERS; i++) {
    if (!g_config_runtime_app_config->users[i].active) {
      return i;
    }
  }

  return -1;
}
