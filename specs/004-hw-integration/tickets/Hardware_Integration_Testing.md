# Hardware Integration Testing

## Overview

Validate all hardware integrations on actual AK3918 hardware with comprehensive integration tests.

## Scope

**In Scope:**
- Hardware integration tests on AK3918:
  - Video input initialization and configuration
  - Dual video encoder operation (1080p + 720p)
  - Audio input and AAC encoding
  - PTZ motor control (absolute, relative, continuous)
  - Imaging settings (brightness, contrast, etc.)
  - Network info detection
- ONVIF protocol tests with real hardware:
  - GetCapabilities, GetDeviceInformation
  - GetProfiles, GetStreamUri
  - PTZ commands (AbsoluteMove, GetStatus)
  - Imaging commands (GetSettings, SetSettings)
- Streaming protocol tests:
  - RTSP session (DESCRIBE, SETUP, PLAY, TEARDOWN)
  - HTTP-FLV streaming (GET /live.flv)
- Concurrent client tests (4 simultaneous connections)
- Error recovery tests (encoder failure, PTZ failure)
- Resource cleanup validation (no leaks)

**Out of Scope:**
- Memory profiling (T16)
- Performance benchmarking (T16)
- 24-hour stress test (T16)

## Technical Details

**Test Environment:**
- Deploy via SD card payload system
- Test on actual AK3918 hardware
- Use ONVIF test tools (ONVIF Device Test Tool)
- Use VLC for RTSP validation
- Use browsers for HTTP-FLV validation

**Test Categories:**
1. Hardware initialization tests
2. ONVIF protocol compliance tests
3. Streaming protocol tests
4. Concurrent operation tests
5. Error recovery tests

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/b45347e9-3eee-42d0-9e77-2c8cfa54db6f - Success Metrics (functional validation)
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - All flows (1-11)

## Dependencies

- T13: Main Entry Point (needs complete system)
- T14: Memory Management (needs monitoring)

## Acceptance Criteria

- [ ] All hardware components initialize successfully
- [ ] Dual video encoders produce valid H.264 streams
- [ ] Audio encoder produces valid AAC stream
- [ ] PTZ responds within 200ms
- [ ] Imaging settings apply correctly
- [ ] ONVIF protocol tests pass (GetCapabilities, GetProfiles, etc.)
- [ ] RTSP streaming works (VLC playback)
- [ ] HTTP-FLV streaming works (browser playback)
- [ ] 4 concurrent clients supported
- [ ] Error recovery works (graceful degradation)
- [ ] No resource leaks detected
