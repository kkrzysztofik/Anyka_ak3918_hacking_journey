# ONVIF Daemon for Anyka AK3918

Comprehensive ONVIF implementation (Device, Media, PTZ, Imaging, RTSP integration, WS-Discovery) for the Anyka AK3918 platform. Features a complete platform abstraction layer, live RTSP streaming, PTZ control, imaging (day/night + IR LED), and robust SOAP/HTTP handling.

## Contents

- Overview & Features
- Platform Abstraction Layer
- Build & Deployment
- Configuration (`/etc/jffs2/ankya_cfg.ini`)
- Runtime Services & Endpoints
- RTSP Streaming Integration
- Imaging (Day/Night & IR LED)
- Implementation Status Summary
- Usage & Testing
- Troubleshooting
- Next Steps / Roadmap

---

## 1. Overview & Features

Core services:

- Device Service: GetDeviceInformation, GetCapabilities, date/time, network info
- Media Service: GetProfiles, GetStreamUri, GetSnapshotUri, encoder/source configs
- PTZ Service: Absolute/Relative/Continuous Move, Stop, Presets, Home
- Imaging Service: Brightness/Contrast/Saturation/Sharpness (basic), Day/Night + IR control
- WS-Discovery: Probe/ProbeMatch responses
- RTSP server: H.264 main/sub streams on port 554 (`/vs0`, `/vs1`)

Supporting components:

- **Platform Abstraction Layer**: Complete hardware abstraction for Anyka AK3918
- PTZ adapter (thread-safe mapping between ONVIF normalized coords and driver degrees)
- HTTP/SOAP server (simple request routing per service endpoint)
- Config loader for unified device INI (`ankya_cfg.ini`)
- Auto day/night + IR LED logic reading `[autoir]` section

Key characteristics:

- Cross-compilable with Anyka toolchain
- Complete platform abstraction layer with proper Anyka SDK integration
- Minimal dependencies (plain C + platform SDK headers)
- Graceful degradation if PTZ or RTSP init fails
- Robust error handling and resource management

---

## 2. Platform Abstraction Layer

The implementation features a comprehensive platform abstraction layer (`src/platform/platform_anyka.c`) that provides unified hardware access for the Anyka AK3918 platform.

### Core Components

**Video Input (VI)**
- Video device management and sensor resolution detection
- Day/night mode switching with automatic IR LED control
- Flip/mirror settings for camera orientation

**Video Processing Subsystem (VPSS)**
- Real-time video effects: brightness, contrast, saturation, sharpness, hue
- Hardware-accelerated video processing using Anyka VPSS

**Video Encoder**
- H.264/H.265/MJPEG encoding with configurable parameters
- Real-time stream generation for RTSP transmission
- Proper resource management and frame handling

**Audio Input & Encoder**
- Audio capture and encoding (AAC, G.711, PCM)
- Synchronized audio/video streaming support
- List-based stream management using Anyka audio APIs

**PTZ Control**
- Complete pan-tilt-zoom functionality
- Position control, speed settings, and status monitoring
- Thread-safe coordinate mapping between ONVIF and hardware

**IR LED Management**
- Automatic night vision control
- Manual and automatic mode switching
- Integration with day/night detection logic

**Configuration Management**
- INI-style configuration file parsing
- Runtime configuration loading and saving
- Support for both string and integer values

**Logging System**
- Unified logging interface using Anyka's `ak_print()` function
- Timestamp formatting and log level management
- Integration with platform abstraction for consistent logging

### API Design

The platform abstraction provides a clean, hardware-agnostic interface:

```c
// Video encoder stream for RTSP
platform_result_t platform_venc_get_stream(platform_venc_handle_t handle, 
                                          platform_venc_stream_t *stream, 
                                          uint32_t timeout_ms);

// PTZ control
platform_result_t platform_ptz_move_to_position(int pan_deg, int tilt_deg);

// Configuration management
const char* platform_config_get_string(const char *section, const char *key, 
                                      const char *default_value);
```

### Error Handling

All platform functions return standardized error codes:
- `PLATFORM_SUCCESS` - Operation completed successfully
- `PLATFORM_ERROR` - General error
- `PLATFORM_ERROR_NULL` - Null pointer parameter
- `PLATFORM_ERROR_INVALID` - Invalid parameter value
- `PLATFORM_ERROR_NOT_SUPPORTED` - Feature not supported

---

## 3. Build & Deployment

You can build in two ways:

### A. Docker Cross-Compile Environment (Recommended, esp. on Windows)

1. Build the image (from repo root or `cross-compile/` directory):

	PowerShell (Windows):

	```pwsh
	cd cross-compile
	./docker-build.ps1
	```

	Linux/macOS:

	```sh
	cd cross-compile
	./docker-build.sh
	# or manually
	docker build -t anyka-cross-compile .
	```

2. Build the ONVIF daemon inside the container (no local toolchain needed):

	```sh
	docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/onvif
	```

	(Interactive shell if you want to iterate):

	```sh
	docker run -it --rm -v ${PWD}:/workspace anyka-cross-compile
	# inside container
	make -C /workspace/onvif -j
	```

3. Result: `cross-compile/onvif/onvifd` (ARM binary) on your host filesystem (because of the bind mount).

### B. Traditional Host Toolchain (Legacy Flow)

1. Prepare Ubuntu 16.04 environment (32‑bit libs available).
2. Install / extract Anyka toolchain so binaries live at:
	`/opt/arm-anykav200-crosstool/usr/bin`
3. Export path:

	```sh
	export PATH=$PATH:/opt/arm-anykav200-crosstool/usr/bin
	```

4. Build:

	```sh
	cd cross-compile/onvif
	make -j
	```

### Deployment Options

Regardless of build method you now have `onvifd`.

1. SD-Card Hack (non‑flash, easiest test):
	 - Copy binary into SD payload tree so it lands in PATH on boot, e.g.:

		 ```sh
		 cp cross-compile/onvif/onvifd SD_card_contents/anyka_hack/usr/bin/
		 ```

	 - (Optional) Add/start via existing startup script (e.g. append to hack init script) or run manually after telnet in.

2. Live Device (temporary run):

	```sh
	scp cross-compile/onvif/onvifd root@<IP>:/tmp/
	ssh root@<IP> '/tmp/onvifd'
	```

3. Persistent Flash Install (after validation):
	- Place in `/usr/bin/onvifd` within modified userfs/rootfs build before re‑packing squashfs, or copy to `/etc/jffs2/` script that launches it at boot.

### Runtime Prerequisites
Ensure configuration file exists at `/etc/jffs2/ankya_cfg.ini` (see section 3). If absent, the daemon will apply defaults but some integrations (auth once implemented) may not work.

### Quick Verification
```sh
file cross-compile/onvif/onvifd | grep ARM
```
Should report ARM (little endian) executable.

Then on device:
```sh
./onvifd &
ps | grep onvifd
```

RTSP test:
```sh
ffplay rtsp://<IP>:554/vs0
```

---

## 4. Configuration (`/etc/jffs2/ankya_cfg.ini`)

Primary path: `/etc/jffs2/ankya_cfg.ini` (single canonical file). Relevant sections/keys used by ONVIF daemon:

```ini
[global]
user = admin         # username (also read as 'username')
secret = admin       # password (also read as 'password')

[onvif]
enabled = 1          # 0 disables ONVIF daemon
http_port = 8080     # HTTP/SOAP port

[autoir]
auto_day_night_enable = 1
# 0=auto,1=day,2=night (mapped internally: 0 auto, 1 day, 2 night)
day_night_mode = 2
# Luminance thresholds
day_to_night_lum = 6400
night_to_day_lum = 2048
# Lock time (ms) before mode re-evaluation
lock_time = 900000
# Optional (not currently parsed): quick_switch_mode, night_cnt*, day_cnt*
```

If keys are missing, defaults: enabled=1, http_port=8080, user/pass empty → no auth gate (future work).

---

## 5. Runtime Services & Endpoints

| Service | HTTP Endpoint | Notes |
|---------|---------------|-------|
| Device  | `/onvif/device_service`  | SOAP Device operations |
| Media   | `/onvif/media_service`   | Stream/profile queries |
| PTZ     | `/onvif/ptz_service`     | PTZ moves & presets |
| Imaging | `/onvif/imaging_service` | Basic imaging settings |
| RTSP    | (port 554) `rtsp://IP:554/vs0` / `vs1` | Live H.264 streams |

Example Device service request: `GetDeviceInformation` (SOAP POST to device_service).

---

## 6. RTSP Streaming Integration

RTSP server starts when video input + encoding init succeed through the platform abstraction layer.

- Main profile: `rtsp://IP:554/vs0` (configured resolution/bitrate; currently capped to ≤1080p in code)
- Sub profile: `rtsp://IP:554/vs1` (future expansion; placeholder unless implemented)
- Audio support: Synchronized audio/video streaming using platform audio encoder
- Stream management: Proper resource handling using Anyka SDK stream APIs

Sample creation snippet (see `onvifd.c`):

```c
rtsp_config.video_config.width  = min(sensor_w, 1920);
rtsp_config.video_config.height = min(sensor_h, 1080);
rtsp_config.video_config.fps = 25;
rtsp_config.video_config.bitrate = 2000; // kbps
rtsp_config.video_config.gop_size = 50;
rtsp_config.video_config.codec_type = H264_ENC_TYPE;
rtsp_config.video_config.br_mode = BR_MODE_CBR;
```

Use VLC / FFplay:

```sh
ffplay rtsp://IP:554/vs0
```

---

## 7. Imaging (Day/Night & IR LED)

Runtime logic:

- Loads `[autoir]` thresholds into internal `auto_daynight_config`
- Applies brightness/contrast/etc. via VPSS effects (scaled ranges)
- IR LED driver initialized (`ak_drv_irled`); auto mode toggles based on day/night detection (placeholder thresholds)

SOAP imaging operations implemented (basic):

- `GetImagingSettings` (returns brightness/contrast/saturation/sharpness/hue)
- `SetImagingSettings` (simple XML parse for basic fields)
- `GetImagingOptions` (range metadata)

(Advanced IR / auto switching expansions possible; arrays quick_switch_mode/night_cnt/day_cnt currently ignored.)

---

## 8. Implementation Status Summary

Implemented (high level):

- **Complete Platform Abstraction Layer** - Full hardware abstraction for Anyka AK3918
- Device / Media / PTZ base operations
- Basic Presets (in-memory)
- RTSP integration for main stream with proper video/audio encoding
- WS-Discovery responses
- Comprehensive config system (INI) with load/save functionality
- Imaging baseline + day/night thresholds parse
- Video/audio encoder stream management
- PTZ control with proper coordinate mapping
- IR LED management and day/night switching
- Unified logging system

Not yet / Partial:

- Authentication (HTTP Digest, WS-Security)
- Persistent preset storage
- Full XML namespace validation & robust parser
- Event / notification subscription
- Advanced imaging arrays (night_cnt/day_cnt) & quick_switch_mode usage

---

## 9. Usage & Testing

Start daemon:

```sh
./onvifd
```

Console output lists available endpoints + RTSP stream.

Basic functional tests:

1. Discovery: ONVIF Device Manager should list device.
2. Device Info: SOAP GetDeviceInformation → Manufacturer/Model/Firmware returned.
3. Stream URI: Media GetStreamUri → `rtsp://IP:554/vs0`.
4. PTZ: Send AbsoluteMove / ContinuousMove (if PTZ hardware present).
5. Imaging: GetImagingSettings / SetImagingSettings adjust effects.

Manual RTSP test:

```sh
ffplay rtsp://IP:554/vs0
```

---

## 10. Troubleshooting

| Symptom | Check |
|---------|-------|
| No discovery | WS-Discovery thread (logs), UDP blocks, network segment |
| 404 on service | URL path matches `/onvif/*_service`, HTTP port correct |
| RTSP fails | Video input init success, port 554 free, bitrate/res caps |
| PTZ no movement | PTZ adapter init messages, driver nodes, permissions |
| Imaging no change | VPSS effect calls returning 0, config save messages |

Log tips:

```sh
ps | grep onvifd
# (If logging redirected) tail -f /tmp/onvif.log
```

Config sanity:

```sh
grep -E "onvif|autoir" /etc/jffs2/ankya_cfg.ini
```

---

## 11. Next Steps / Roadmap

- Add HTTP Digest auth & credential enforcement
- Persist presets & imaging settings (separate config file or JSON)
- Expand imaging to consume quick_switch_mode, night_cnt[], day_cnt[]
- Implement event notifications (motion, day/night transitions)
- ONVIF Profile S compliance additions (snapshot, audio, advanced media)
- Harden SOAP parsing (XML parser library or robust tokenizer)

---

## 12. License & Attribution

Follows repository license (see root `LICENSE`). Uses Anyka SDK headers/binaries as provided in firmware extraction context.

---

## 13. Minimal Quick Start

```sh
make
scp onvifd root@IP:/tmp/
ssh root@IP '/tmp/onvifd'
ffplay rtsp://IP:554/vs0
```

---

## 14. File Map (Key Sources)

```text
src/main/onvifd.c                    # daemon entry
src/platform/platform.h              # platform abstraction interface
src/platform/platform_anyka.c        # Anyka AK3918 platform implementation
src/server/http_server.c             # HTTP/SOAP server
src/server/ws_discovery.c            # discovery
src/server/rtsp_server_*.c           # RTSP server implementation
src/services/device/...              # Device service
src/services/media/...               # Media service
src/services/ptz/...                 # PTZ service
src/services/imaging/...             # Imaging service
src/utils/onvif_config.c             # INI loader
src/utils/imaging_config.c           # Imaging config parser
```

---

## 15. Security Note

Currently no authentication; do NOT expose camera directly to untrusted networks until auth added.
