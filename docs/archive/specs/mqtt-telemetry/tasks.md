# Tasks Document

## Phase 0: Prerequisites - Implement Missing Telemetry Infrastructure

These tasks implement telemetry collection infrastructure in the existing ONVIF codebase that the MQTT telemetry system will depend on.

- [ ] 0.1 Implement platform_get_system_info() for system metrics
  - Files: `cross-compile/onvif/src/platform/platform_anyka.c`
  - Implement platform_get_system_info() function to collect system metrics
  - Read CPU usage from /proc/stat (calculate idle vs total time)
  - Read memory usage from /proc/meminfo (MemTotal, MemAvailable)
  - Read uptime from /proc/uptime
  - Read CPU temperature from /sys/class/thermal/thermal_zone0/temp (if available)
  - Purpose: Provide system metrics for telemetry collection
  - _Leverage: Existing platform abstraction patterns_
  - _Requirements: 5 (Telemetry Data Collection)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded Linux Systems Developer with /proc filesystem expertise | Task: Implement platform_get_system_info() in cross-compile/onvif/src/platform/platform_anyka.c collecting CPU usage, memory usage, uptime, temperature from /proc filesystem. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Read /proc/stat for CPU idle/total time and calculate percentage, read /proc/meminfo for MemTotal/MemAvailable and calculate percentage, read /proc/uptime for uptime in seconds, read temperature from /sys/class/thermal/thermal_zone0/temp (handle file not existing gracefully), fill platform_system_info_t structure, handle all file I/O errors gracefully with logging, mandatory Doxygen documentation, use return code constants (PLATFORM_SUCCESS, PLATFORM_ERROR_*) | _Leverage: POSIX file I/O (fopen, fgets, sscanf), existing platform error handling patterns, utils/logging/ for error messages | Success: Function reads all /proc files correctly, calculates CPU/memory percentages accurately, handles missing temperature sensor gracefully, fills platform_system_info_t completely, compiles without warnings. Test on actual device. Mark task as completed [x] in tasks.md when done._

- [ ] 0.2 Add RTSP metrics API (optional, can stub initially)
  - Files: `cross-compile/onvif/src/networking/rtsp/rtsp_metrics.h`, `cross-compile/onvif/src/networking/rtsp/rtsp_metrics.c`
  - Create RTSP metrics header defining rtsp_metrics_t structure
  - Implement rtsp_metrics_get_active_sessions() to count active RTSP sessions
  - Implement rtsp_metrics_get_bitrate() to get current video bitrate
  - Implement rtsp_metrics_get_fps() to get current frame rate
  - Purpose: Provide video streaming metrics for telemetry
  - _Leverage: Existing RTSP server implementation_
  - _Requirements: 5 (Telemetry Data Collection)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: RTSP Server Developer with metrics collection expertise | Task: Create rtsp_metrics.h/c in cross-compile/onvif/src/networking/rtsp/ providing RTSP metrics API. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Define rtsp_metrics_t structure with active_sessions, bitrate_kbps, frame_rate fields, implement rtsp_metrics_get_current() returning metrics, count active sessions from RTSP server state, get bitrate/fps from video encoder configuration or stub with configured values if not available, thread-safe access to metrics, mandatory Doxygen documentation, use return code constants | _Leverage: Existing RTSP server implementation in networking/rtsp/, platform video encoder APIs | Success: Metrics API compiles and links, returns accurate session count, returns bitrate/fps (even if from config), thread-safe. Can stub initially with zeros if complex. Mark task as completed [x] in tasks.md when done._

- [ ] 0.3 Add telemetry and MQTT configuration sections to anyka_cfg.ini
  - Files: `cross-compile/onvif/configs/anyka_cfg.ini`
  - Add [telemetry] section with telemetry_enabled flag (default: 0)
  - Add [mqtt] section with broker settings, enable flag, credentials
  - Add [telemetry_intervals] section with publishing interval settings
  - Purpose: Centralized configuration for telemetry system
  - _Leverage: Existing INI file structure_
  - _Requirements: 2, 3, 10 (Enable/Disable Controls, Configuration Management)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Configuration Management Specialist | Task: Add telemetry and MQTT configuration sections to cross-compile/onvif/configs/anyka_cfg.ini. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Add [telemetry] section with: telemetry_enabled=0 (opt-in), device_id= (empty, will auto-generate). Add [mqtt] section with: mqtt_enabled=0, broker_host=, broker_port=1883, client_id=, username=, password=, topic_prefix=camera, keep_alive_interval=60, qos_telemetry=1, qos_events=1. Add [telemetry_intervals] section with: system_metrics_interval=30, http_metrics_interval=60, ptz_status_interval=10, video_metrics_interval=30. Include comments explaining each setting. | _Leverage: Existing INI section patterns from anyka_cfg.ini | Success: Configuration sections added, all settings documented with comments, defaults are opt-in (disabled), settings match requirements. Mark task as completed [x] in tasks.md when done._

## Phase 1: Foundation and Library Integration

- [ ] 1. Download and integrate Paho Embedded C library
  - Files: `cross-compile/paho/` (new directory)
  - Clone Eclipse paho.mqtt.embedded-c repository
  - Copy MQTTClient-C source files to cross-compile/paho/
  - Copy MQTTPacket source files to cross-compile/paho/
  - Purpose: Obtain MQTT client library source code (external dependency, separate from onvif code)
  - _Leverage: Eclipse paho.mqtt.embedded-c repository_
  - _Requirements: 1 (MQTT Client Library Integration)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Build Engineer with embedded systems library integration expertise | Task: Download Eclipse paho.mqtt.embedded-c from GitHub and integrate into cross-compile/paho/ directory (separate from onvif to avoid bloating). Copy MQTTClient-C and MQTTPacket source files. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Use specific commit/tag (not latest), verify license compatibility (EPL/EDL), do NOT modify library source files, preserve directory structure, keep SEPARATE from cross-compile/onvif/ | _Leverage: GitHub repository https://github.com/eclipse-paho/paho.mqtt.embedded-c | Success: Library source files present in cross-compile/paho/, directory structure preserved, license files included. Verify with: ls -la cross-compile/paho/. Mark task as completed [x] in tasks.md when done._

- [ ] 2. Implement Linux platform layer for Paho
  - Files: `cross-compile/paho/MQTTLinux.c`, `cross-compile/paho/MQTTLinux.h`
  - Implement Linux-specific networking functions (socket, select, timing)
  - Adapt Paho's platform abstraction for ARM Linux with uClibc
  - Purpose: Provide platform-specific implementations for Paho client
  - _Leverage: Paho's MQTTClient-C/src/linux/ reference implementation_
  - _Requirements: 1 (MQTT Client Library Integration)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded Linux Developer with socket programming expertise | Task: Implement Linux platform layer for Paho MQTT in cross-compile/paho/MQTTLinux.c/h. Adapt reference implementation for ARM Linux uClibc environment with socket operations, select-based I/O, and timing functions. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Follow Paho platform abstraction API exactly, use POSIX sockets only, NO OpenSSL/TLS code, compatible with uClibc, mandatory Doxygen documentation | _Leverage: Paho's MQTTClient-C/src/linux/MQTTLinux.c as reference, existing socket utilities if available | Success: MQTTLinux.c implements all platform functions (networking, timing), compiles with ARM toolchain, compatible with uClibc, full Doxygen docs. Test compilation. Mark task as completed [x] in tasks.md when done._

- [ ] 3. Cross-compile Paho library to static library
  - Files: `cross-compile/paho/Makefile`
  - Create Makefile for cross-compiling Paho to libpaho-embed-mqtt3c.a
  - Configure ARM GCC toolchain, include paths, compiler flags
  - Purpose: Build Paho library for ARM target platform
  - _Leverage: Project's main Makefile cross-compilation patterns_
  - _Requirements: 1 (MQTT Client Library Integration)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Build System Engineer with ARM cross-compilation expertise | Task: Create Makefile in cross-compile/paho/ to cross-compile Paho MQTT library to static library libpaho-embed-mqtt3c.a using arm-anykav200-linux-uclibcgnueabi-gcc toolchain. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Use same toolchain as main project (arm-anykav200-linux-uclibcgnueabi-gcc), same compiler flags (-std=c99, -fPIC, etc.), output libpaho-embed-mqtt3c.a static library in cross-compile/paho/, clean target included | _Leverage: cross-compile/onvif/Makefile for toolchain and flags patterns | Success: Makefile builds libpaho-embed-mqtt3c.a successfully in cross-compile/paho/, library links without errors, size <100KB. Verify with: make -C cross-compile/paho && ls -lh cross-compile/paho/libpaho-embed-mqtt3c.a. Mark task as completed [x] in tasks.md when done._

- [ ] 4. Create MQTT configuration header and types
  - Files: `cross-compile/onvif/src/telemetry/mqtt_config.h`
  - Define mqtt_config_t structure with all configuration fields
  - Define return code constants (MQTT_SUCCESS, MQTT_ERROR_*, etc.)
  - Add function declarations for config loading/validation
  - Purpose: Define configuration data structures and API
  - _Leverage: Existing config header patterns in src/core/config/_
  - _Requirements: 2, 3, 10 (Enable/Disable Controls, Configuration Management)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer specializing in configuration management | Task: Create mqtt_config.h header file in cross-compile/onvif/src/telemetry/ defining mqtt_config_t structure, return code constants (NO magic numbers), and function declarations for config loading/validation. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Structure must include telemetry_enabled, mqtt_enabled flags, all broker settings, intervals, QoS settings from requirements, constants for all return codes (MQTT_SUCCESS=0, MQTT_ERROR_CONFIG=-1, etc.), NO magic numbers, mandatory Doxygen documentation for all fields/functions, include guards | _Leverage: src/core/config/ header patterns, utils/error/error_handling.h for return code patterns | Success: Header defines complete mqtt_config_t with all required fields, return code constants defined, function declarations documented, compiles without warnings, follows project naming conventions. Mark task as completed [x] in tasks.md when done._

- [ ] 5. Implement MQTT configuration loader
  - Files: `cross-compile/onvif/src/telemetry/mqtt_config.c`
  - Implement mqtt_config_load() to parse INI file
  - Implement mqtt_config_validate() for validation
  - Handle defaults (telemetry_enabled=false, mqtt_enabled=false, etc.)
  - Purpose: Load MQTT configuration from INI file
  - _Leverage: Existing INI parsing utilities_
  - _Requirements: 2, 3, 10 (Enable/Disable Controls, Configuration Management)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with INI parsing and validation expertise | Task: Implement mqtt_config.c with mqtt_config_load() parsing INI file, mqtt_config_validate() for validation, mqtt_config_free() for cleanup. Parse all configuration sections ([telemetry], [mqtt], [qos], [intervals], [device]) with sensible defaults. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Use existing INI parsing utilities, defaults MUST be telemetry_enabled=false, mqtt_enabled=false (opt-in), validate broker_host required if mqtt_enabled, validate port range (1-65535), validate QoS (0-2), NO magic numbers for return codes, mandatory Doxygen documentation, proper error logging | _Leverage: Existing INI parsing utilities from src/core/config/, utils/string/ for parsing, utils/logging/ for errors | Success: Loads valid INI files correctly, applies defaults when fields missing, validates all fields with clear error messages, handles invalid config gracefully. Test with sample configs. Mark task as completed [x] in tasks.md when done._

## Phase 2: MQTT Client Wrapper

- [ ] 6. Create MQTT client wrapper header
  - Files: `cross-compile/onvif/src/telemetry/mqtt_client.h`
  - Define MQTT client state structure
  - Define connection status enum
  - Add function declarations for init/connect/disconnect/publish
  - Purpose: Define MQTT client wrapper API
  - _Leverage: Paho MQTTClient.h types_
  - _Requirements: 4 (Broker Connection Management)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with MQTT and API design expertise | Task: Create mqtt_client.h header in cross-compile/onvif/src/telemetry/ defining client state structure (wraps Paho Network, MQTTClient), connection status enum, and function declarations for connection management and publishing. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Wrap Paho's MQTTClient structure, define connection states (DISCONNECTED, CONNECTING, CONNECTED, ERROR), return code constants (NO magic numbers), function signatures must match design doc, mandatory Doxygen documentation, include guards | _Leverage: Paho's MQTTClient.h for types, design doc for API signatures | Success: Header defines mqtt_client_state_t structure, connection status enum, all client API functions declared, Doxygen documented, compiles without warnings. Mark task as completed [x] in tasks.md when done._

- [ ] 7. Implement MQTT client initialization
  - Files: `cross-compile/onvif/src/telemetry/mqtt_client.c` (start)
  - Implement mqtt_client_init() function
  - Initialize Paho Network and MQTTClient structures
  - Allocate buffers using buffer pool
  - Purpose: Initialize MQTT client with configuration
  - _Leverage: Paho MQTTClient API, buffer pool_
  - _Requirements: 4 (Broker Connection Management)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with Paho MQTT client expertise | Task: Implement mqtt_client_init() in mqtt_client.c initializing Paho Network and MQTTClient structures, allocating send/receive buffers using buffer pool, storing configuration. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Use buffer pool for all buffers (NO malloc directly), initialize Paho Network with TCP socket functions from MQTTLinux, set client_id/username/password from config, NO connection yet (just init), proper error handling with cleanup on failure, mandatory Doxygen documentation, use return code constants | _Leverage: networking/common/buffer_pool.h for buffers, Paho MQTTClient.h API, design doc for structure | Success: Initializes Paho structures correctly, allocates buffers from pool, stores config, handles init failures gracefully, returns MQTT_SUCCESS or error codes. Mark task as completed [x] in tasks.md when done._

- [ ] 8. Implement MQTT client connection logic
  - Files: `cross-compile/onvif/src/telemetry/mqtt_client.c` (continue)
  - Implement mqtt_client_connect() function
  - Connect to broker with TCP socket
  - Send MQTT CONNECT packet with credentials
  - Handle connection response and errors
  - Purpose: Establish connection to MQTT broker
  - _Leverage: Paho MQTTConnect, Network API_
  - _Requirements: 4 (Broker Connection Management)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Network Programming Expert with MQTT protocol knowledge | Task: Implement mqtt_client_connect() in mqtt_client.c establishing TCP connection to broker, sending MQTT CONNECT packet with username/password authentication, handling CONNACK response. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Use Paho NetworkConnect() for TCP socket, MQTTConnect() for MQTT handshake, set keep-alive from config, clean session flag, timeout 5 seconds, handle CONNACK return codes (0=success, 1-5=errors), update connection state, log connection status, mandatory Doxygen documentation, use return code constants | _Leverage: Paho Network and MQTTClient APIs, utils/logging/ for status messages, MQTTLinux socket functions | Success: Connects to broker successfully, authenticates with username/password, handles connection errors gracefully, updates state, logs connection status. Test with local Mosquitto broker. Mark task as completed [x] in tasks.md when done._

- [ ] 9. Implement MQTT client publish function
  - Files: `cross-compile/onvif/src/telemetry/mqtt_client.c` (continue)
  - Implement mqtt_client_publish() function
  - Publish message with topic, payload, QoS, retained flag
  - Handle publish acknowledgment for QoS > 0
  - Purpose: Publish telemetry messages to MQTT broker
  - _Leverage: Paho MQTTPublish API_
  - _Requirements: 4, 5, 6 (Connection, QoS, Publishing)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: MQTT Protocol Developer with QoS expertise | Task: Implement mqtt_client_publish() in mqtt_client.c publishing messages using Paho MQTTPublish(), supporting QoS 0/1/2, retained flag, handling PUBACK for QoS 1. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Check connection state before publishing, validate topic not NULL/empty, validate payload length, use Paho MQTTPublish() with QoS/retained parameters, wait for PUBACK if QoS>0 (with timeout), handle publish errors, log publish failures, mandatory Doxygen documentation, use return code constants | _Leverage: Paho MQTTPublish API, design doc for signature, utils/logging/ for errors | Success: Publishes messages successfully with QoS 0/1, handles PUBACK correctly, validates parameters, handles errors gracefully, logs failures. Test publishing to Mosquitto. Mark task as completed [x] in tasks.md when done._

- [ ] 10. Implement MQTT client disconnect and cleanup
  - Files: `cross-compile/onvif/src/telemetry/mqtt_client.c` (continue)
  - Implement mqtt_client_disconnect() function
  - Implement mqtt_client_cleanup() function
  - Send DISCONNECT packet, close socket, free buffers
  - Purpose: Clean shutdown of MQTT connection
  - _Leverage: Paho MQTTDisconnect, Network API_
  - _Requirements: 4 (Broker Connection Management)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with resource management expertise | Task: Implement mqtt_client_disconnect() and mqtt_client_cleanup() in mqtt_client.c sending MQTT DISCONNECT packet, closing socket, freeing buffers to buffer pool. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: mqtt_client_disconnect() sends DISCONNECT packet using Paho MQTTDisconnect(), closes network connection, updates state; mqtt_client_cleanup() frees all buffers to pool, clears structures, idempotent (safe to call multiple times), mandatory Doxygen documentation, use return code constants | _Leverage: Paho MQTTDisconnect and NetworkDisconnect APIs, buffer_pool_release() for buffer cleanup | Success: Disconnect sends DISCONNECT packet cleanly, cleanup frees all resources properly, safe to call multiple times, no memory leaks. Test disconnect/cleanup sequence. Mark task as completed [x] in tasks.md when done._

## Phase 3: Telemetry Data Collection

- [ ] 11. Create telemetry collector header with data structures
  - Files: `cross-compile/onvif/src/telemetry/telemetry_collector.h`
  - Define telemetry_data_t structure with all metric fields
  - Define collector function declarations
  - Purpose: Define telemetry data structure and collection API
  - _Leverage: HTTP metrics, platform types_
  - _Requirements: 5 (Telemetry Data Collection)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with systems monitoring expertise | Task: Create telemetry_collector.h header defining telemetry_data_t structure with all metric fields (HTTP, system, PTZ, video, camera status) and collector function declarations. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Structure must match design doc exactly with device_id, timestamp, HTTP metrics (requests, latency, etc.), system metrics (CPU, memory, uptime), PTZ (pan/tilt/zoom/moving), video (sessions, bitrate, fps), camera (IR LED, temp), function declarations for init and per-metric-type collectors, mandatory Doxygen documentation, include guards | _Leverage: networking/http/http_server.h for http_performance_metrics_t types, design doc for structure definition | Success: Header defines complete telemetry_data_t structure, all collector functions declared, Doxygen documented, compiles without warnings. Mark task as completed [x] in tasks.md when done._

- [ ] 12. Implement telemetry collector initialization - **CENTRAL COORDINATOR**
  - Files: `cross-compile/onvif/src/telemetry/telemetry_collector.c` (start)
  - Implement telemetry_collector_init() function - **CENTRAL TELEMETRY COORDINATOR**
  - Get device_id from config or generate from MAC/serial
  - Initialize timing/state for collection
  - Store configuration reference for enable/disable checks
  - Purpose: Initialize CENTRAL telemetry collection subsystem (single source of truth)
  - _Leverage: Platform API for MAC/serial, mqtt_config_t for settings_
  - _Requirements: 2, 3, 5, 6 (Enable/Disable Controls, Telemetry Collection, Publishing)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with device management expertise | Task: Implement telemetry_collector_init() in telemetry_collector.c initializing the CENTRAL telemetry coordinator, getting device_id from config or auto-generating from MAC address/serial number as fallback, storing mqtt_config_t pointer for enable/disable checks. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Store mqtt_config_t pointer (NOT copy) to check telemetry_enabled flag, if device_id in config use it else get MAC address from platform API and format as device_id, initialize timing for collection intervals, store device_id globally (g_telemetry_device_id), store config pointer globally (g_telemetry_config), proper error handling, mandatory Doxygen documentation emphasizing this is CENTRAL COORDINATOR, use return code constants | _Leverage: platform/platform.h for MAC/serial retrieval, mqtt_config_t for device_id and enable flags, utils/string/ for formatting | Success: Initializes successfully as central coordinator, gets/generates device_id correctly, stores config reference for enable checks, handles errors. Mark task as completed [x] in tasks.md when done._

- [ ] 13. Implement HTTP metrics collector
  - Files: `cross-compile/onvif/src/telemetry/telemetry_collector.c` (continue)
  - Implement telemetry_collector_get_http_metrics() function
  - Query HTTP server performance metrics
  - Fill telemetry_data_t HTTP fields
  - Purpose: Collect HTTP server telemetry data
  - _Leverage: http_metrics_get_current() API_
  - _Requirements: 5 (Telemetry Data Collection)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer familiar with HTTP server metrics | Task: Implement telemetry_collector_get_http_metrics() in telemetry_collector.c calling http_metrics_get_current() and filling telemetry_data_t HTTP fields (total_requests, successful_requests, errors, latency, connections). Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Call http_metrics_get_current() from networking/http/http_server.h, copy all http_performance_metrics_t fields to telemetry_data_t, handle case where HTTP server not initialized (return error), set timestamp_ms to current time, mandatory Doxygen documentation, use return code constants | _Leverage: networking/http/http_server.h http_metrics_get_current() function | Success: Collects HTTP metrics correctly, fills all HTTP fields in telemetry_data_t, handles HTTP server not running case, sets timestamp. Test with HTTP server running. Mark task as completed [x] in tasks.md when done._

- [ ] 14. Implement system metrics collector
  - Files: `cross-compile/onvif/src/telemetry/telemetry_collector.c` (continue)
  - Implement telemetry_collector_get_system_metrics() function
  - Call platform_get_system_info() to get system metrics
  - Fill telemetry_data_t system fields from platform_system_info_t
  - Purpose: Collect system resource telemetry
  - _Leverage: platform_get_system_info() implemented in Phase 0_
  - _Requirements: 5 (Telemetry Data Collection)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with platform integration expertise | Task: Implement telemetry_collector_get_system_metrics() in telemetry_collector.c calling platform_get_system_info() and copying system metrics to telemetry_data_t. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Call platform_get_system_info() from platform/platform.h to get platform_system_info_t structure, copy cpu_usage, cpu_temperature, total_memory, free_memory, uptime_ms fields to telemetry_data_t, calculate memory_usage_percent from total/free memory, handle platform_get_system_info() returning error (fill with zeros or return error), set timestamp_ms to current time, mandatory Doxygen documentation, use return code constants | _Leverage: platform/platform.h platform_get_system_info() function implemented in task 0.1, time() for timestamp | Success: Calls platform_get_system_info() correctly, copies all system metrics to telemetry_data_t, calculates derived metrics (memory %), handles platform errors gracefully, sets timestamp. Test with platform_get_system_info() working. Mark task as completed [x] in tasks.md when done._

- [ ] 15. Implement PTZ status collector
  - Files: `cross-compile/onvif/src/telemetry/telemetry_collector.c` (continue)
  - Implement telemetry_collector_get_ptz_status() function
  - Query PTZ position and movement status
  - Fill telemetry_data_t PTZ fields
  - Purpose: Collect PTZ telemetry data
  - _Leverage: onvif_ptz_get_status() API_
  - _Requirements: 5 (Telemetry Data Collection)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer familiar with PTZ APIs | Task: Implement telemetry_collector_get_ptz_status() in telemetry_collector.c calling onvif_ptz_get_status() and filling telemetry_data_t PTZ fields (pan, tilt, zoom, moving state). Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Call onvif_ptz_get_status() from services/ptz/onvif_ptz.h with valid profile token (use default profile), copy position fields (pan, tilt, zoom) and movement status to telemetry_data_t, handle PTZ not available (return error or zeros), set timestamp_ms, mandatory Doxygen documentation, use return code constants | _Leverage: services/ptz/onvif_ptz.h onvif_ptz_get_status() function | Success: Collects PTZ status correctly, fills pan/tilt/zoom/moving fields, handles PTZ unavailable gracefully, sets timestamp. Test with PTZ service running. Mark task as completed [x] in tasks.md when done._

- [ ] 16. Implement video and camera status collectors
  - Files: `cross-compile/onvif/src/telemetry/telemetry_collector.c` (continue)
  - Implement telemetry_collector_get_video_metrics() function
  - Implement telemetry_collector_get_camera_status() function
  - Query RTSP sessions, IR LED status, temperature
  - Purpose: Collect video streaming and camera hardware telemetry
  - _Leverage: rtsp_metrics_get_current() from Phase 0, platform API_
  - _Requirements: 5 (Telemetry Data Collection)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with video streaming and hardware APIs knowledge | Task: Implement telemetry_collector_get_video_metrics() calling rtsp_metrics_get_current() for video metrics; and telemetry_collector_get_camera_status() collecting IR LED status and temperature. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: For video metrics, call rtsp_metrics_get_current() from networking/rtsp/rtsp_metrics.h (implemented in task 0.2) to get active_sessions, bitrate_kbps, frame_rate, copy to telemetry_data_t, handle rtsp_metrics not available (stub with zeros); for camera status, call platform_irled_get_status() for IR LED status, get temperature from platform_system_info_t (already collected in system metrics), handle unavailable data gracefully, set timestamp_ms for both, mandatory Doxygen documentation, use return code constants | _Leverage: networking/rtsp/rtsp_metrics.h rtsp_metrics_get_current() from task 0.2, platform/platform.h for platform_irled_get_status(), platform_system_info_t for temperature | Success: Collects video metrics from RTSP metrics API (or stubs with zeros), collects camera status (IR LED/temp), handles missing data gracefully, sets timestamp. Mark task as completed [x] in tasks.md when done._

## Phase 4: JSON Formatting

- [ ] 17. Create JSON formatter header
  - Files: `cross-compile/onvif/src/telemetry/json_formatter.h`
  - Define JSON formatting function declarations
  - Define buffer size constants
  - Purpose: Define JSON serialization API
  - _Requirements: 6 (Telemetry Publishing)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with JSON serialization expertise | Task: Create json_formatter.h header defining JSON formatting functions for each telemetry type (HTTP, system, PTZ, video, events) and buffer size constants. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Function signatures must match design doc (take telemetry_data_t* and output buffer), define buffer size constants (JSON_BUFFER_SIZE_HTTP=1024, etc.), all functions return int (bytes written or error), mandatory Doxygen documentation, include guards | _Leverage: Design doc for function signatures | Success: Header defines all JSON formatter functions, buffer size constants, Doxygen documented, compiles without warnings. Mark task as completed [x] in tasks.md when done._

- [ ] 18. Implement HTTP metrics JSON formatter
  - Files: `cross-compile/onvif/src/telemetry/json_formatter.c` (start)
  - Implement json_format_http_metrics() function
  - Generate JSON manually with snprintf (no JSON library)
  - Include device_id, timestamp, all HTTP metrics
  - Purpose: Format HTTP metrics as JSON
  - _Requirements: 6 (Telemetry Publishing)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with string formatting and JSON expertise | Task: Implement json_format_http_metrics() in json_formatter.c generating JSON payload manually using snprintf, including device_id, timestamp, all HTTP metric fields per design doc example. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Use snprintf() for JSON generation (NO JSON library), escape special characters if needed, validate buffer size, return bytes written or error if buffer too small, JSON must be valid per RFC 8259, match design doc payload example exactly, mandatory Doxygen documentation, use return code constants | _Leverage: Design doc JSON payload examples, utils/string/ for safe formatting | Success: Generates valid JSON matching design example, fits in buffer, handles buffer overflow gracefully, all HTTP fields included. Test JSON validity with JSON parser. Mark task as completed [x] in tasks.md when done._

- [ ] 19. Implement system, PTZ, video JSON formatters
  - Files: `cross-compile/onvif/src/telemetry/json_formatter.c` (continue)
  - Implement json_format_system_metrics() function
  - Implement json_format_ptz_status() function
  - Implement json_format_video_metrics() function
  - Purpose: Format all telemetry types as JSON
  - _Requirements: 6 (Telemetry Publishing)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with JSON formatting expertise | Task: Implement json_format_system_metrics(), json_format_ptz_status(), json_format_video_metrics() in json_formatter.c generating JSON payloads for each telemetry type matching design doc examples. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Use snprintf() for all JSON generation, follow design doc payload examples exactly, include device_id and timestamp in all, validate buffer sizes, handle buffer overflow, return bytes written or error, mandatory Doxygen documentation, use return code constants | _Leverage: Design doc JSON examples, json_format_http_metrics() as pattern | Success: All three formatters generate valid JSON matching design examples, fit in buffers, handle overflow gracefully. Test JSON validity. Mark task as completed [x] in tasks.md when done._

- [ ] 20. Implement event JSON formatter
  - Files: `cross-compile/onvif/src/telemetry/json_formatter.c` (continue)
  - Implement json_format_event() function
  - Accept event_type and event_data parameters
  - Generate event JSON with device_id, timestamp, event fields
  - Purpose: Format events as JSON for immediate publishing
  - _Requirements: 7 (Event-Driven Notifications)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with event formatting expertise | Task: Implement json_format_event() in json_formatter.c taking event_type and event_data strings, generating JSON payload with device_id, timestamp, event_type, event_data per design doc event example. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Use snprintf() for JSON, escape event_type and event_data strings (handle quotes, backslashes), validate buffer size, match design doc event payload example, mandatory Doxygen documentation, use return code constants | _Leverage: Design doc event JSON example, string escaping utilities if available | Success: Generates valid event JSON matching design example, escapes strings properly, handles buffer overflow. Test with various event types. Mark task as completed [x] in tasks.md when done._

## Phase 5: Telemetry Publishing

- [ ] 21. Create telemetry publisher header
  - Files: `cross-compile/onvif/src/telemetry/telemetry_publisher.h`
  - Define publisher state structure
  - Define function declarations for init/start/stop/publish
  - Purpose: Define telemetry publishing orchestration API
  - _Requirements: 6, 8 (Publishing, Intervals)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with multi-threaded publishing expertise | Task: Create telemetry_publisher.h defining publisher state structure (config, thread, running flag, last_publish_times), and function declarations for init/start/stop/cleanup and event publishing. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Structure must store mqtt_config_t*, pthread_t for background thread, running flag, last publish timestamps per metric type, function signatures match design doc, mandatory Doxygen documentation, include guards | _Leverage: Design doc for API signatures, pthread types | Success: Header defines complete publisher state, all functions declared, Doxygen documented, compiles without warnings. Mark task as completed [x] in tasks.md when done._

- [ ] 22. Implement telemetry publisher initialization
  - Files: `cross-compile/onvif/src/telemetry/telemetry_publisher.c` (start)
  - Implement telemetry_publisher_init() function
  - Store configuration, initialize state
  - Allocate JSON buffers using buffer pool
  - Purpose: Initialize telemetry publisher
  - _Leverage: Buffer pool for JSON buffers_
  - _Requirements: 6 (Publishing)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with resource initialization expertise | Task: Implement telemetry_publisher_init() in telemetry_publisher.c storing mqtt_config_t pointer, initializing state (running=false, last_publish_times=0), allocating JSON buffers from buffer pool. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Store config pointer (NOT copy), allocate JSON buffers (4KB each) using buffer_pool_acquire(), initialize pthread mutex for state protection, set all last_publish_times to 0, proper error handling with cleanup on failure, mandatory Doxygen documentation, use return code constants | _Leverage: networking/common/buffer_pool.h for buffers, pthread mutex for thread safety | Success: Initializes publisher state correctly, allocates buffers from pool, handles init failures gracefully. Mark task as completed [x] in tasks.md when done._

- [ ] 23. Implement periodic telemetry publishing thread
  - Files: `cross-compile/onvif/src/telemetry/telemetry_publisher.c` (continue)
  - Implement telemetry_publisher_start() function
  - Create background pthread for periodic publishing
  - Implement publisher thread function with interval-based publishing
  - Purpose: Orchestrate periodic telemetry collection and publishing
  - _Leverage: pthread API, telemetry collector, JSON formatter, MQTT client_
  - _Requirements: 6, 8 (Publishing, Intervals)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Multi-threaded C Developer with timer-based task expertise | Task: Implement telemetry_publisher_start() creating background pthread and publisher thread function that periodically (every second loop) checks if each metric type interval elapsed, collects telemetry, formats JSON, publishes to MQTT, updates last_publish_time. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Create pthread with pthread_create(), set lowest priority, thread function loops while running=true, check each metric type interval (system_metrics_interval, http_metrics_interval, ptz_status_interval, video_metrics_interval) against time elapsed since last_publish_time, call appropriate collector function, json formatter, mqtt_client_publish() with topic "camera/{device_id}/telemetry/{type}", update last_publish_time, sleep 1 second between loops, proper thread synchronization, mandatory Doxygen documentation, use return code constants | _Leverage: pthread API, telemetry_collector functions, json_formatter functions, mqtt_client_publish(), time() for intervals | Success: Background thread starts successfully, publishes each telemetry type at configured intervals, formats JSON correctly, publishes to correct topics, runs continuously. Test with Mosquitto broker and mosquitto_sub. Mark task as completed [x] in tasks.md when done._

- [ ] 24. Implement event publishing functions
  - Files: `cross-compile/onvif/src/telemetry/telemetry_publisher.c` (continue)
  - Implement mqtt_event_motion_detected(), mqtt_event_auth_failure(), etc.
  - Format event JSON and publish immediately to event topics
  - Purpose: Publish critical events immediately when triggered
  - _Leverage: JSON formatter, MQTT client_
  - _Requirements: 7 (Event-Driven Notifications)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with event-driven systems expertise | Task: Implement event publishing functions (mqtt_event_motion_detected, mqtt_event_auth_failure, mqtt_event_ptz_complete, mqtt_event_stream_started, mqtt_event_stream_stopped, mqtt_event_system_error) in telemetry_publisher.c formatting event JSON and publishing immediately to "camera/{device_id}/events/{type}" topics. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Each function takes relevant event data parameters, calls json_format_event() with appropriate event_type and event_data, publishes to correct event topic using mqtt_client_publish() with QoS from config (default 1), returns immediately (non-blocking), check MQTT enabled before publishing, mandatory Doxygen documentation, use return code constants | _Leverage: json_format_event(), mqtt_client_publish(), design doc event topics | Success: All event functions format JSON correctly and publish to correct event topics immediately, handle MQTT disabled gracefully, non-blocking. Test triggering events manually. Mark task as completed [x] in tasks.md when done._

- [ ] 25. Implement telemetry publisher stop and cleanup
  - Files: `cross-compile/onvif/src/telemetry/telemetry_publisher.c` (continue)
  - Implement telemetry_publisher_stop() function
  - Implement telemetry_publisher_cleanup() function
  - Stop background thread, free resources
  - Purpose: Clean shutdown of telemetry publisher
  - _Leverage: pthread join, buffer pool_
  - _Requirements: 6 (Publishing)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer with thread lifecycle management expertise | Task: Implement telemetry_publisher_stop() stopping background thread (set running=false, pthread_join), and telemetry_publisher_cleanup() freeing buffers to buffer pool, destroying mutex. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: telemetry_publisher_stop() sets running=false and joins thread with pthread_join() (wait for thread to exit cleanly), telemetry_publisher_cleanup() releases all buffers to buffer pool, destroys pthread mutex, idempotent (safe to call multiple times), mandatory Doxygen documentation, use return code constants | _Leverage: pthread_join(), buffer_pool_release(), pthread_mutex_destroy() | Success: Stop signals thread and waits for clean exit, cleanup frees all resources properly, safe to call multiple times, no memory leaks. Test stop/cleanup sequence. Mark task as completed [x] in tasks.md when done._

## Phase 5.5: HTTP Telemetry API

- [ ] 25.1 Create HTTP telemetry handler header
  - Files: `cross-compile/onvif/src/telemetry/telemetry_http_handler.h`
  - Define HTTP request handler function declarations
  - Define telemetry endpoint paths as constants
  - Purpose: Define HTTP API for telemetry retrieval
  - _Leverage: HTTP server request/response types_
  - _Requirements: 5 (Telemetry Data Collection) - HTTP access requirement_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: HTTP API Developer with RESTful endpoint design expertise | Task: Create telemetry_http_handler.h defining HTTP telemetry API with endpoint path constants and request handler function declarations per design doc Component 7. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Define endpoint path constants (TELEMETRY_ENDPOINT_ALL="/api/telemetry/all", TELEMETRY_ENDPOINT_SYSTEM="/api/telemetry/system", etc.), declare telemetry_http_handler_init(), telemetry_http_handle_request(const http_request_t*, http_response_t**), telemetry_http_handler_cleanup(), mandatory Doxygen documentation with endpoint descriptions, include guards | _Leverage: networking/http/http_parser.h for http_request_t/http_response_t types, design doc Component 7 for API specification | Success: Header defines all endpoint paths, handler functions declared, Doxygen documented with API examples, compiles without warnings. Mark task as completed [x] in tasks.md when done._

- [ ] 25.2 Implement HTTP telemetry request router
  - Files: `cross-compile/onvif/src/telemetry/telemetry_http_handler.c` (start)
  - Implement telemetry_http_handle_request() function
  - Route requests to appropriate telemetry endpoint handlers
  - Purpose: Route HTTP requests to telemetry data collectors
  - _Leverage: HTTP request path parsing_
  - _Requirements: 5 (Telemetry Data Collection)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: HTTP API Developer with request routing expertise | Task: Implement telemetry_http_handle_request() in telemetry_http_handler.c routing GET requests to /api/telemetry/* endpoints, checking request method is GET, parsing path, calling appropriate handler function. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Check http_request->method is GET (return 405 Method Not Allowed otherwise), parse http_request->path and match against endpoint constants, route to internal handler functions (handle_telemetry_all, handle_telemetry_system, etc.), return 404 if path doesn't match any endpoint, return 500 on errors, mandatory Doxygen documentation, use return code constants | _Leverage: http_request_t structure, HTTP status code constants, string comparison for path matching | Success: Routes all defined endpoints correctly, returns 404 for unknown paths, returns 405 for non-GET methods, handles errors gracefully. Test with curl. Mark task as completed [x] in tasks.md when done._

- [ ] 25.3 Implement individual telemetry endpoint handlers
  - Files: `cross-compile/onvif/src/telemetry/telemetry_http_handler.c` (continue)
  - Implement handle_telemetry_all() - returns all telemetry as JSON
  - Implement handle_telemetry_system(), handle_telemetry_http(), handle_telemetry_ptz(), handle_telemetry_video(), handle_telemetry_camera()
  - Purpose: Generate JSON responses for each telemetry endpoint
  - _Leverage: Telemetry collector, JSON formatter_
  - _Requirements: 5 (Telemetry Data Collection)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: HTTP API Developer with JSON response generation expertise | Task: Implement internal handler functions (handle_telemetry_all, handle_telemetry_system, etc.) in telemetry_http_handler.c collecting telemetry data, formatting as JSON, creating HTTP 200 response. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Each handler calls appropriate telemetry_collector_get_*() function, formats with json_format_*() function, creates http_response_t with status 200, Content-Type: application/json header, JSON body, handle_telemetry_all combines all telemetry types into single JSON object per design doc, allocate response buffer from buffer pool, handle errors with 500 Internal Server Error response, mandatory Doxygen documentation, use return code constants | _Leverage: telemetry_collector.h APIs, json_formatter.h functions, http_response_t from http_parser.h, buffer pool for response buffers, design doc Component 7 for JSON response format | Success: All endpoint handlers generate correct JSON responses, handle errors gracefully with 500, set proper Content-Type headers, allocate/free buffers correctly. Test each endpoint with curl. Mark task as completed [x] in tasks.md when done._

- [ ] 25.4 Register HTTP telemetry endpoints with HTTP server
  - Files: `cross-compile/onvif/src/networking/http/http_server.c` (modify)
  - Register /api/telemetry/* path pattern with telemetry handler
  - Add routing to telemetry_http_handle_request() for matching requests
  - Purpose: Integrate telemetry API into HTTP server routing
  - _Leverage: Existing HTTP server routing infrastructure_
  - _Requirements: 5 (Telemetry Data Collection)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: HTTP Server Developer with endpoint registration expertise | Task: Modify http_server.c to add routing for /api/telemetry/* paths to telemetry_http_handle_request() handler. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: In HTTP request processing (http_server_process_request or equivalent), add path prefix matching for /api/telemetry/, if matches call telemetry_http_handle_request(), check telemetry_enabled config flag before routing (return 503 Service Unavailable if telemetry disabled), maintain existing routing for ONVIF/other endpoints, proper error handling, mandatory code comments | _Leverage: Existing HTTP server routing logic, telemetry_http_handler.h API, mqtt_config for telemetry_enabled check | Success: HTTP server routes /api/telemetry/* requests correctly to telemetry handler, returns 503 if telemetry disabled, existing endpoints still work. Test with curl http://camera-ip/api/telemetry/all. Mark task as completed [x] in tasks.md when done._

## Phase 6: Main Integration

- [ ] 26. Add MQTT initialization to main daemon
  - Files: `cross-compile/onvif/src/core/main/main.c` (modify)
  - Load MQTT configuration on startup
  - Initialize MQTT client, telemetry collector, publisher
  - Start telemetry publishing if enabled
  - Purpose: Integrate MQTT telemetry into main daemon lifecycle
  - _Leverage: Existing main initialization sequence_
  - _Requirements: 2, 3 (Enable/Disable Controls)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: System Integration Engineer with daemon lifecycle expertise | Task: Modify main.c to load MQTT config from configs/mqtt_telemetry.ini, initialize MQTT components (client, collector, publisher) if enabled, start publisher thread. Add to main() initialization sequence after existing services. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Load config with mqtt_config_load(), check telemetry_enabled flag, if false skip all initialization, if true check mqtt_enabled flag, initialize telemetry_collector_init() always if telemetry_enabled, initialize mqtt_client_init() and connect only if mqtt_enabled, initialize and start telemetry_publisher_start() if mqtt_enabled, proper error handling (log and continue if MQTT fails), NO blocking on MQTT initialization (async connection OK), mandatory code comments | _Leverage: Existing main.c initialization patterns, mqtt_config/mqtt_client/telemetry_collector/telemetry_publisher APIs | Success: MQTT initializes correctly when enabled, skips when disabled, handles config errors gracefully, does not block daemon startup. Test with config enabled/disabled. Mark task as completed [x] in tasks.md when done._

- [ ] 27. Add MQTT cleanup to main daemon shutdown
  - Files: `cross-compile/onvif/src/core/main/main.c` (modify)
  - Stop telemetry publisher
  - Disconnect MQTT client
  - Cleanup all MQTT resources
  - Purpose: Clean shutdown of MQTT telemetry
  - _Leverage: Existing main cleanup sequence_
  - _Requirements: All_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: System Integration Engineer with resource cleanup expertise | Task: Modify main.c cleanup/shutdown sequence to stop telemetry publisher, disconnect MQTT client, cleanup all MQTT resources in reverse order of initialization. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Add to main cleanup (signal handler or exit path), call telemetry_publisher_stop(), telemetry_publisher_cleanup(), mqtt_client_disconnect(), mqtt_client_cleanup(), telemetry_collector_cleanup(), mqtt_config_free() in reverse initialization order, check if initialized before cleaning up, mandatory code comments | _Leverage: Existing main.c cleanup patterns, MQTT cleanup APIs | Success: MQTT shuts down cleanly on daemon exit, all resources freed, no memory leaks, no crashes on shutdown. Test daemon start/stop cycles. Mark task as completed [x] in tasks.md when done._

- [ ] 28. Update main Makefile to link MQTT components
  - Files: `cross-compile/onvif/Makefile` (modify)
  - Add src/telemetry/ source files to build
  - Add Paho library include paths and link flags
  - Ensure MQTT components compile and link
  - Purpose: Build system integration for MQTT telemetry
  - _Leverage: Existing Makefile structure_
  - _Requirements: All_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Build System Engineer with Makefile expertise | Task: Update cross-compile/onvif/Makefile to add src/telemetry/ source files, Paho library include paths pointing to ../paho/, and link libpaho-embed-mqtt3c.a from ../paho/. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Add TELEMETRY_SRCS variable with mqtt_config.c mqtt_client.c telemetry_collector.c json_formatter.c telemetry_publisher.c telemetry_http_handler.c, add to SRCS, add Paho include paths to INCLUDES (-I../paho/MQTTPacket/src -I../paho/MQTTClient-C/src), add -L../paho -lpaho-embed-mqtt3c to LIBS, ensure pthread already linked, maintain existing Makefile structure and patterns | _Leverage: Existing Makefile SRC and LIB patterns | Success: Makefile builds all MQTT components successfully including HTTP handler, links Paho library from ../paho/ without errors, onvifd binary includes MQTT code. Test with: make clean && make. Mark task as completed [x] in tasks.md when done._

## Phase 7: Configuration and Documentation

- [ ] 29. Create sample MQTT configuration file
  - Files: `cross-compile/onvif/configs/mqtt_telemetry.ini`
  - Create INI file with all configuration sections
  - Provide sensible defaults and comments
  - Purpose: Provide template configuration for users
  - _Requirements: 10 (Configuration Management)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Technical Writer with configuration management expertise | Task: Create mqtt_telemetry.ini sample configuration file in cross-compile/onvif/configs/ with all sections ([telemetry], [mqtt], [qos], [intervals], [device]) per design doc, sensible defaults, and inline comments explaining each setting. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Must match design doc configuration format exactly, defaults: telemetry enabled=false, mqtt enabled=false, broker_host=localhost, port=1883, keep_alive=60, qos values=1, intervals per design, device_id empty (auto-generate), include comments explaining each setting and valid values | _Leverage: Design doc configuration file format section | Success: INI file is complete with all sections, defaults match requirements, comments explain settings clearly. Validate with mqtt_config_load(). Mark task as completed [x] in tasks.md when done._

- [ ] 30. Add Doxygen documentation for MQTT modules
  - Files: All `src/telemetry/*.h` and `src/telemetry/*.c` files
  - Ensure all functions, structures, enums have Doxygen comments
  - Add file headers with @file, @brief, @author, @date
  - Purpose: Complete API documentation for MQTT telemetry
  - _Requirements: All (documentation)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Technical Documentation Specialist with Doxygen expertise | Task: Review all src/telemetry/ source and header files ensuring complete Doxygen documentation: file headers (@file, @brief, @author kkrzysztofik, @date 2025), function documentation (@brief, @param, @return, @note), structure/enum documentation. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Follow project Doxygen style per AGENTS.md, all public functions must have @brief @param @return, all structures must document each field, all enums document each value, file headers mandatory, examples for complex APIs helpful | _Leverage: Existing Doxygen patterns in src/, AGENTS.md documentation requirements | Success: All MQTT source files have complete Doxygen documentation, generates HTML docs without warnings. Run: make docs, check docs/html/. Mark task as completed [x] in tasks.md when done._

- [ ] 31. Create MQTT integration README documentation
  - Files: `cross-compile/onvif/src/telemetry/README.md`
  - Document MQTT integration architecture
  - Provide configuration guide
  - Include usage examples and testing instructions
  - Purpose: User documentation for MQTT telemetry feature
  - _Requirements: All (documentation)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Technical Writer with IoT documentation expertise | Task: Create README.md in src/telemetry/ documenting MQTT integration including: overview/architecture, configuration instructions, topic structure, payload examples, testing with Mosquitto, troubleshooting. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Must explain what telemetry_enabled and mqtt_enabled do, document all config options from INI, show topic structure with examples, provide JSON payload examples, include mosquitto_sub commands for testing, troubleshooting common issues (connection refused, auth failures), markdown format | _Leverage: Design doc for architecture and examples, requirements doc for features | Success: README is comprehensive and clear, covers all configuration options, examples are accurate, testing instructions work. Test documentation accuracy. Mark task as completed [x] in tasks.md when done._

## Phase 8: Testing and Validation

- [ ] 32. Create unit tests for MQTT configuration
  - Files: `cross-compile/onvif/tests/src/unit/telemetry/test_mqtt_config.c`
  - Test INI parsing, validation, defaults
  - Test error handling for invalid configs
  - Purpose: Ensure configuration loading works correctly
  - _Leverage: CMocka test framework_
  - _Requirements: 10 (Configuration Management)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: QA Engineer with unit testing expertise | Task: Create test_mqtt_config.c unit tests for MQTT configuration loading, testing valid INI parsing, defaults application, validation, invalid config error handling. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Use CMocka framework, test naming pattern test_unit_mqtt_config_<scenario>, test valid config loads correctly, defaults applied when fields missing, validation catches invalid broker_host/port/qos, error handling for missing file, mock file I/O if needed, mandatory Doxygen comments | _Leverage: Existing unit test patterns from tests/src/unit/, CMocka assertion macros | Success: All config scenarios tested (valid, defaults, invalid, missing), tests pass consistently. Run: make test. Mark task as completed [x] in tasks.md when done._

- [ ] 33. Create unit tests for JSON formatter
  - Files: `cross-compile/onvif/tests/src/unit/telemetry/test_json_formatter.c`
  - Test JSON generation for all telemetry types
  - Test buffer overflow handling
  - Validate JSON syntax
  - Purpose: Ensure JSON formatting is correct
  - _Leverage: CMocka test framework_
  - _Requirements: 6 (Telemetry Publishing)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: QA Engineer with JSON validation expertise | Task: Create test_json_formatter.c unit tests for all JSON formatters (HTTP, system, PTZ, video, events), testing valid JSON generation, field presence, buffer overflow handling. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Use CMocka framework, test naming pattern test_unit_json_formatter_<type>_<scenario>, populate sample telemetry_data_t structures, call formatters, validate JSON syntax (parse with simple parser or manually), check all required fields present, test buffer too small scenarios, mandatory Doxygen comments | _Leverage: Existing unit test patterns, CMocka assertions | Success: All JSON formatters tested, generate valid JSON with all required fields, handle buffer overflow. Validate JSON with parser. Run: make test. Mark task as completed [x] in tasks.md when done._

- [ ] 34. Create unit tests for telemetry collector
  - Files: `cross-compile/onvif/tests/src/unit/telemetry/test_telemetry_collector.c`
  - Test data collection with mocked sources
  - Test error handling when sources unavailable
  - Purpose: Ensure telemetry collection works correctly
  - _Leverage: CMocka test framework, mocks_
  - _Requirements: 5 (Telemetry Data Collection)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: QA Engineer with mocking expertise | Task: Create test_telemetry_collector.c unit tests for telemetry collectors, testing HTTP/system/PTZ/video/camera collection with mocked data sources, error handling when unavailable. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Use CMocka framework, test naming pattern test_unit_telemetry_collector_<type>_<scenario>, mock http_metrics_get_current(), onvif_ptz_get_status(), /proc reads, verify telemetry_data_t fields populated correctly, test unavailable sources return errors gracefully, mandatory Doxygen comments | _Leverage: Existing mock patterns from tests/src/mocks/, CMocka mocking features | Success: All collector functions tested with mocked sources, populate data correctly, handle unavailable sources. Run: make test. Mark task as completed [x] in tasks.md when done._

- [ ] 35. Create integration test for MQTT publishing
  - Files: `cross-compile/onvif/tests/src/integration/telemetry/test_mqtt_publishing.c`
  - Test end-to-end MQTT publishing to local Mosquitto broker
  - Verify messages published to correct topics
  - Validate payload JSON
  - Purpose: Validate complete MQTT integration
  - _Leverage: Local Mosquitto broker for testing_
  - _Requirements: All_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Integration Test Engineer with MQTT testing expertise | Task: Create test_mqtt_publishing.c integration test connecting to local Mosquitto broker (localhost:1883), publishing telemetry messages, subscribing to verify receipt, validating topics and payloads. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Test naming pattern test_integration_mqtt_publishing_<scenario>, requires Mosquitto running locally, connect MQTT client to broker, publish test telemetry, subscribe to verify (use separate Paho client instance), validate topic format "camera/{device_id}/telemetry/{type}", parse and validate JSON payloads, test QoS 0 and QoS 1, mandatory Doxygen comments | _Leverage: Paho client for subscriber, mqtt_client/telemetry_publisher APIs, local Mosquitto broker | Success: Integration test publishes and receives messages successfully, topics correct, payloads valid JSON with correct data. Requires Mosquitto: sudo apt install mosquitto mosquitto-clients. Run test. Mark task as completed [x] in tasks.md when done._

- [ ] 36. Create resource usage validation test
  - Files: `cross-compile/onvif/tests/src/integration/telemetry/test_mqtt_resources.c`
  - Test memory usage stays under 1MB
  - Test CPU usage stays under 5% average
  - Test network bandwidth under 10KB/sec
  - Purpose: Validate resource requirements met
  - _Leverage: /proc for resource monitoring_
  - _Requirements: 11 (Resource Management)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Performance Test Engineer with profiling expertise | Task: Create test_mqtt_resources.c integration test measuring MQTT memory usage (<1MB), CPU usage (<5% avg), network bandwidth (<10KB/sec) during operation. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Test naming pattern test_integration_mqtt_resources_<metric>, initialize MQTT, run publisher for 60 seconds, measure memory from /proc/self/status VmRSS, measure CPU from /proc/self/stat, estimate network from bytes published, assert within requirements, mandatory Doxygen comments | _Leverage: /proc filesystem for metrics, telemetry_publisher APIs | Success: Test confirms memory <1MB, CPU <5%, network <10KB/sec during typical operation. Run and verify metrics. Mark task as completed [x] in tasks.md when done._

- [ ] 37. Run code quality validation (linting and formatting)
  - Run linting script on all MQTT source files
  - Run formatting script on all MQTT source files
  - Fix any issues found
  - Purpose: Ensure code quality standards met
  - _Leverage: Project linting/formatting scripts_
  - _Requirements: All (code quality)_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: QA Engineer with code quality validation expertise | Task: Run linting and formatting validation on all src/telemetry/ files, fix any issues found. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Run ./cross-compile/onvif/scripts/lint_code.sh --check on src/telemetry/ files, run ./cross-compile/onvif/scripts/format_code.sh --check on src/telemetry/ files, fix ALL linting errors and formatting issues, re-run until zero errors, verify function ordering (definitions at top, execution logic at bottom), verify global variables at top after includes, NO magic numbers in code | _Leverage: cross-compile/onvif/scripts/lint_code.sh and format_code.sh | Success: All src/telemetry/ files pass linting with zero errors, pass formatting with zero issues, function ordering correct, no magic numbers. Verify: ./scripts/lint_code.sh --check src/telemetry/* && ./scripts/format_code.sh --check src/telemetry/*. Mark task as completed [x] in tasks.md when done._

- [ ] 38. Perform end-to-end manual testing
  - Test MQTT telemetry on actual camera hardware or emulator
  - Subscribe to all topics and verify payloads
  - Test enable/disable flags
  - Test connection resilience
  - Purpose: Validate complete MQTT integration in real environment
  - _Requirements: All_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: QA Test Engineer with manual testing expertise | Task: Perform comprehensive manual E2E testing of MQTT telemetry: configure mqtt_telemetry.ini, start daemon, subscribe to topics with mosquitto_sub, verify all telemetry types published at correct intervals, test enable/disable flags, trigger events, test broker disconnect/reconnect. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Test scenarios: 1) telemetry_enabled=false (no collection), 2) mqtt_enabled=false telemetry_enabled=true (collect but no publish), 3) both enabled (full operation), 4) broker disconnect during operation (auto-reconnect), 5) trigger events manually, subscribe with: mosquitto_sub -h localhost -t 'camera/#' -v, verify JSON payloads valid and contain expected data, verify intervals match config, document any issues found | _Leverage: mosquitto_sub for subscription, camera hardware or dev environment, mqtt_telemetry.ini config | Success: All telemetry types publish at configured intervals with valid data, enable/disable flags work correctly, connection resilience verified, events publish immediately. Document test results. Mark task as completed [x] in tasks.md when done._

- [ ] 39. Generate final documentation and coverage report
  - Generate Doxygen HTML documentation
  - Generate test coverage report
  - Verify coverage >80% for MQTT code
  - Purpose: Complete documentation and validate test coverage
  - _Leverage: Doxygen, gcov/lcov_
  - _Requirements: All_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Documentation and QA Specialist | Task: Generate final Doxygen documentation and test coverage report for MQTT telemetry, verify coverage >80% of src/telemetry/ code. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Run make docs to generate Doxygen HTML, verify no Doxygen warnings, run make test-coverage to generate coverage report, check coverage for src/telemetry/ files >80%, if below 80% add more tests, generate HTML coverage report with make test-coverage-html | _Leverage: make docs, make test-coverage, make test-coverage-html targets | Success: Doxygen generates docs without warnings, coverage >80% for all src/telemetry/ files, coverage report available. View docs/html/index.html and coverage/html/index.html. Mark task as completed [x] in tasks.md when done._

- [ ] 40. Final integration validation and spec completion
  - Verify all requirements met
  - Verify all tests pass
  - Verify resource usage within limits
  - Document any deviations or known issues
  - Purpose: Final validation before marking spec complete
  - _Requirements: All_
  - _Prompt: Implement the task for spec mqtt-telemetry, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Senior QA Engineer with spec validation expertise | Task: Perform final comprehensive validation of MQTT telemetry integration: verify all 12 requirements met with evidence, all unit/integration tests pass, resource usage within limits (<1MB memory, <5% CPU, <10KB/sec network), code quality validated (linting/formatting pass), documentation complete. Set this task to in_progress in .spec-workflow/specs/mqtt-telemetry/tasks.md, then implement. | Restrictions: Review requirements.md and check each requirement with test evidence or manual verification, run full test suite (make test), verify resource limits with test_mqtt_resources or manual monitoring, verify linting/formatting clean, verify Doxygen docs complete, document any deviations or known issues in README, create requirements traceability matrix showing each requirement tested | _Leverage: All previous tasks, requirements.md, tests | Success: All requirements verified as met, all tests pass, resource limits satisfied, code quality validated, documentation complete, spec ready for deployment. Generate final validation report. Mark task as completed [x] in tasks.md when done._
