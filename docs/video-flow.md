# ONVIF Video Flow - Developer Guide & Troubleshooting

## Overview

This document provides a comprehensive guide for developers working with the ONVIF video pipeline from hardware sensor to RTSP stream delivery. It covers the complete video flow, common troubleshooting scenarios, debugging techniques, and validation procedures.

**Target Audience**: Developers, system administrators, and QA engineers working with the ONVIF video system.

## Quick Reference

### Key Files
- `src/core/lifecycle/video_lifecycle.c` - Complete video pipeline initialization and management
- `src/platform/platform_anyka.c` - Anyka AK3918 hardware abstraction layer
- `src/networking/rtsp/rtsp_multistream.c` - Multi-stream RTSP server implementation
- `src/services/common/video_config_types.h` - Video configuration structures

### Common Issues Quick Fix
| Issue | Quick Fix | File Location |
|-------|-----------|---------------|
| No video stream | Check sensor detection | `video_lifecycle.c:46-61` |
| RTSP connection fails | Verify RTSP server start | `video_lifecycle.c:363-381` |
| Poor video quality | Check encoder config | `video_lifecycle.c:278-320` |
| Sensor not detected | Check sensor paths | `video_lifecycle.c:47-57` |
| Memory leaks | Verify cleanup order | `video_lifecycle.c:438-468` |

### Build & Test Commands
```bash
# Debug build with video logging
cd cross-compile/onvif && make debug

# Test video pipeline
./out/onvifd &
# Check RTSP stream
ffplay rtsp://admin:admin@localhost:554/vs0

# Check video logs
journalctl -f | grep -E "(video|RTSP|sensor)"
```

## Video Pipeline Architecture

### Current Implementation Overview

The ONVIF video system uses a **7-phase initialization pipeline** with these characteristics:

1. **Sensor Detection**: Multi-path sensor configuration matching
2. **Video Input Setup**: Hardware video input channel initialization
3. **Channel Configuration**: Main/sub stream channel setup
4. **Global Capture**: Hardware video capture activation
5. **Stream Configuration**: Video/audio parameter validation
6. **RTSP Server**: Multi-stream server creation and stream registration
7. **Server Start**: Threading and network activation

### Video Flow Architecture

```mermaid
graph TD
    A["🎥 Camera Sensor"] --> B["ISP Processing"]
    B --> C["Video Input Driver (ak_vi)"]
    C --> D["Platform Abstraction (platform_anyka.c)"]
    D --> E["Video Lifecycle (video_lifecycle.c)"]
    E --> F["Video Encoder (ak_venc)"]
    F --> G["RTSP Server (rtsp_multistream.c)"]
    G --> H["RTP Packetization"]
    H --> I["Network Transport"]
    I --> J["🖥️ ONVIF Client"]

    style A fill:#ff9800,color:#fff
    style E fill:#2196f3,color:#fff
    style G fill:#4caf50,color:#fff
    style J fill:#9c27b0,color:#fff
```

## Troubleshooting Guide

### 1. Video System Initialization Failures

**Symptoms**: RTSP streaming disabled, no video streams available

**Root Causes**:
- Sensor configuration not found
- Video input hardware failure
- Channel configuration mismatch
- Video capture failure

**Debugging Steps**:
```bash
# 1. Check sensor detection
journalctl -f | grep "sensor"
# Look for: "warning: failed to match sensor at /etc/jffs2"

# 2. Test hardware access
ls -la /dev/video* /dev/ak*
# Should show video device nodes

# 3. Check video input initialization
journalctl -f | grep "video input"
# Look for: "Video input initialized: 1920x1080"

# 4. Manual sensor detection test
find /etc/jffs2 /data/sensor /data -name "*sensor*" -o -name "*.cfg" 2>/dev/null
```

**Code Locations**:
- **Sensor matching**: `video_lifecycle.c:46-61` - Multi-path sensor detection
- **Video input init**: `video_lifecycle.c:69-92` - Hardware initialization
- **Channel config**: `video_lifecycle.c:100-131` - Main/sub channel setup
- **Global capture**: `video_lifecycle.c:138-146` - Video capture start

**Fixes**:
1. **Missing sensor config**: Copy sensor files to `/etc/jffs2/`
2. **Hardware permissions**: Check device node permissions
3. **Resolution limits**: Verify sensor resolution capabilities
4. **Hardware reset**: Restart hardware components

### 2. RTSP Server Creation Failures

**Symptoms**: "Failed to create multi-stream RTSP server", no RTSP port listening

**Root Causes**:
- Port 554 already in use
- Video input handle not available
- Memory allocation failure
- Socket creation failure

**Debugging Steps**:
```bash
# 1. Check port availability
netstat -tlnp | grep :554
# Should be empty before daemon start

# 2. Check video handle
journalctl -f | grep "vi_handle"
# Look for valid handle creation

# 3. Test socket creation
nc -l 554  # Should succeed if port available

# 4. Check memory availability
free -m && cat /proc/meminfo | grep Available
```

**Code Locations**:
- **Server creation**: `video_lifecycle.c:328-357` - RTSP server creation
- **Stream addition**: `video_lifecycle.c:345-352` - Stream registration
- **Server start**: `video_lifecycle.c:363-381` - Network activation

**Fixes**:
1. **Port conflict**: Stop conflicting services or change port
2. **Memory issues**: Free memory or optimize configuration
3. **Handle validation**: Ensure video input initialization succeeded
4. **Permission issues**: Run with appropriate privileges

### 3. Video Encoding Issues

**Symptoms**: RTSP connects but no video data, black screen, encoding errors

**Root Causes**:
- Invalid video configuration
- Encoder initialization failure
- Frame rate mismatch
- Resolution not supported

**Debugging Steps**:
```bash
# 1. Check encoder configuration
journalctl -f | grep "platform_venc_init"
# Look for: "Video encoder initialized successfully"

# 2. Check frame processing
journalctl -f | grep "venc_get_stream"
# Should show regular frame processing

# 3. Test video configuration
journalctl -f | grep "Main stream configuration"
# Shows actual config being used

# 4. Monitor encoder stats
watch "journalctl -n 50 | grep -E '(frames_sent|bytes_sent)'"
```

**Code Locations**:
- **Config validation**: `video_lifecycle.c:296-304` - Stream config validation
- **Encoder setup**: `platform_anyka.c:794-940` - Hardware encoder init
- **Frame processing**: `rtsp_multistream.c` - Video frame handling

**Fixes**:
1. **Invalid config**: Use `stream_config_validate()` for validation
2. **Resolution issues**: Ensure 4-byte alignment for width/height
3. **Frame rate**: Match sensor capabilities (5-60fps range)
4. **Profile/level**: Use supported H.264 profiles

### 4. Sensor Detection Failures

**Symptoms**: "failed to match sensor", "video input disabled"

**Root Causes**:
- Sensor configuration files missing
- Wrong sensor driver
- Hardware connection issues
- Path access permissions

**Debugging Steps**:
```bash
# 1. Check sensor file paths
find /etc/jffs2 -name "*sensor*" -o -name "*.conf" 2>/dev/null
find /data/sensor -name "*" 2>/dev/null
find /data -name "*sensor*" 2>/dev/null

# 2. Check hardware detection
dmesg | grep -i sensor
lsmod | grep -i sensor

# 3. Test sensor access
cat /proc/ak39/ak39_global 2>/dev/null || echo "No hardware access"

# 4. Check mount points
mount | grep -E "(jffs2|data)"
```

**Code Locations**:
- **Multi-path detection**: `video_lifecycle.c:46-61` - Sensor path fallback
- **Platform call**: `platform_anyka.c:459-470` - Hardware sensor matching

**Fixes**:
1. **Missing files**: Copy sensor configs to expected paths
2. **Permission issues**: Fix mount permissions or file ownership
3. **Hardware issues**: Check physical sensor connection
4. **Driver issues**: Verify sensor driver loading

### 5. Video Quality and Performance Issues

**Symptoms**: Poor video quality, low frame rate, high latency

**Root Causes**:
- Incorrect bitrate settings
- GOP size misconfiguration
- Frame rate limitations
- Network congestion

**Debugging Steps**:
```bash
# 1. Check current configuration
journalctl -f | grep "stream configuration"
# Shows bitrate, fps, resolution

# 2. Monitor performance
top -p $(pgrep onvifd)
# Check CPU usage

# 3. Network monitoring
iftop -i eth0  # Monitor network usage
ss -tuln | grep 554  # Check RTSP connections

# 4. Frame rate analysis
journalctl -f | grep "frames_sent" | tail -20
```

**Code Locations**:
- **Config validation**: `video_lifecycle.c:236-256` - GOP size validation
- **Quality settings**: `platform_anyka.c:850-880` - Encoder parameters
- **Frame rate config**: `video_lifecycle.c:179-196` - FPS validation

**Fixes**:
1. **Bitrate optimization**: Adjust based on network capacity
2. **GOP size**: Use 2x frame rate for optimal quality
3. **Resolution scaling**: Reduce resolution for performance
4. **Network optimization**: Check bandwidth and latency

## Developer Guidelines

### Adding New Video Streams

When adding additional video streams:

1. **Check stream limits** in `rtsp_multistream.h:23`:
   ```c
   #define RTSP_MAX_STREAMS 4
   ```

2. **Add stream configuration**:
   ```c
   video_config_t stream_config = {
       .width = 640,
       .height = 480,
       .fps = 15,
       .bitrate = 1000,
       .gop_size = 30,
       .profile = PLATFORM_PROFILE_BASELINE
   };
   ```

3. **Register with RTSP server**:
   ```c
   rtsp_multistream_server_add_stream(server, "/vs2", "sub2",
                                      &stream_config, &audio_config, false);
   ```

### Video Configuration Best Practices

1. **Resolution Alignment**: Ensure width/height are 4-byte aligned
2. **Frame Rate Validation**: Keep within sensor capabilities (5-60fps)
3. **Bitrate Calculation**: Use ~1-5 Mbps for 1080p, ~500kbps-2Mbps for 720p
4. **GOP Size**: Set to 2x frame rate for optimal seeking
5. **Profile Selection**: Use Baseline for compatibility, Main for quality

### Error Handling Patterns

For video components:

1. **Graceful Degradation**: Video failure doesn't crash daemon
   ```c
   if (video_lifecycle_init(cfg) != 0) {
       platform_log_warning("Video system disabled, continuing without streaming\n");
       // Continue with other services
   }
   ```

2. **Cleanup Idempotency**: Safe to call cleanup multiple times
   ```c
   void video_lifecycle_cleanup(void) {
       static bool cleanup_done = false;
       if (cleanup_done) return;
       // Cleanup logic
       cleanup_done = true;
   }
   ```

3. **Resource Management**: Always release resources in reverse order
   ```c
   // Cleanup order: RTSP server -> Video input -> Platform
   rtsp_multistream_server_destroy(server);
   platform_vi_close(vi_handle);
   platform_cleanup();
   ```

### Performance Optimization

1. **Memory Management**: Use platform memory pools
2. **Thread Safety**: Protect shared video resources with mutexes
3. **Frame Buffering**: Implement efficient buffer management
4. **Network Optimization**: Use optimal packet sizes for RTP

### Testing Video Pipeline

```bash
# 1. Basic functionality test
./out/onvifd &
PID=$!
sleep 5
curl -s http://localhost:8080/onvif/device_service | grep -q "Device"
echo "ONVIF service: $?"

# 2. RTSP stream test
timeout 10s ffmpeg -i rtsp://admin:admin@localhost:554/vs0 -frames:v 10 -f null - 2>&1 | grep -E "(fps|bitrate)"

# 3. Multi-client test
for i in {1..3}; do
  timeout 10s ffplay -nodisp rtsp://admin:admin@localhost:554/vs0 &
done
sleep 5
ps aux | grep ffplay | wc -l  # Should show 3 clients

# 4. Cleanup test
kill -INT $PID
sleep 2
ps -p $PID >/dev/null && echo "FAIL: Process still running" || echo "PASS: Clean shutdown"
```

## Video Configuration Reference

### Video Configuration Structure
```c
typedef struct {
    int width;              // Video width (4-byte aligned)
    int height;             // Video height (4-byte aligned)
    int fps;                // Frame rate (5-60 range)
    int bitrate;            // Bitrate in kbps (100-20000)
    int gop_size;           // GOP size (auto: fps * 2)
    int profile;            // H.264 profile
    int codec_type;         // Codec type (H.264)
    int br_mode;            // Bitrate mode (CBR/VBR)
} video_config_t;
```

### Audio Configuration Structure
```c
typedef struct {
    int sample_rate;        // Sample rate (8000-48000)
    int channels;           // Channel count (1-2)
    int bits_per_sample;    // Bits per sample (16/24)
    int codec_type;         // Audio codec type
    int bitrate;            // Audio bitrate
} audio_config_t;
```

### Stream Path Mapping
- **Main Stream**: `/vs0` - Full resolution, high quality
- **Sub Stream**: `/vs1` - Lower resolution, optimized for bandwidth
- **Custom Streams**: `/vs2`, `/vs3` - Additional streams as needed

### Hardware Limitations (Anyka AK3918)
- **Max Resolution**: 1920x1080 @ 30fps
- **Max Streams**: 4 concurrent streams
- **Encoder**: H.264 hardware encoder
- **Memory**: Limited buffer pools for video frames

## Expected Log Messages

### Normal Video Initialization
```
[INFO] Initializing video input...
[INFO] Video input initialized: 1920x1080
[INFO] Detected sensor frame rate: 25 fps
[INFO] Global video capture started successfully
[INFO] Main stream configuration: 1920x1080@25fps, 2000kbps, H264-Main, GOP:50
[INFO] Multi-stream RTSP server created successfully
[INFO] Multi-stream RTSP server started successfully
```

### Warning Messages (Non-Fatal)
```
[WARNING] warning: failed to match sensor at /etc/jffs2, trying backup path
[WARNING] warning: failed to detect sensor frame rate, using default 15fps
[WARNING] Config FPS 60 outside valid range (5-50), using sensor FPS 25
[WARNING] warning: using default main stream configuration with sensor fps 25
```

### Error Messages (Investigation Required)
```
[ERROR] Failed to open video input, RTSP streaming disabled
[ERROR] Failed to set video channel attributes, RTSP streaming disabled
[ERROR] Failed to start global video capture, RTSP streaming disabled
[ERROR] Failed to create multi-stream RTSP server
[ERROR] Failed to start multi-stream RTSP server
```

## Performance Expectations

| Component | Expected Duration | Max Acceptable | Warning Signs |
|-----------|------------------|----------------|---------------|
| Sensor detection | < 100ms | < 500ms | Multiple path failures |
| Video input init | 200-500ms | 2s | Hardware timeouts |
| Channel config | < 100ms | < 500ms | Invalid parameters |
| Global capture | 100-300ms | 1s | Hardware not ready |
| RTSP server creation | < 200ms | 1s | Port conflicts |
| Server start | 200-500ms | 2s | Threading issues |
| **Total initialization** | **500ms-1.5s** | **7s** | **System overload** |

## Integration with Build System

### Debug Build with Video Logging
```bash
# Enable video debug logging
make debug CFLAGS="-DPLATFORM_LOG_LEVEL=0"

# Video-specific static analysis
clang-tidy --checks='-*,bugprone-*,performance-*' src/core/lifecycle/video_lifecycle.c

# Memory leak detection for video
valgrind --leak-check=full --track-origins=yes ./out/onvifd
```

### Automated Testing for CI
```bash
# Video pipeline test
test_video_pipeline() {
  timeout 30s ./out/onvifd &
  local pid=$!
  sleep 5

  # Test RTSP connection
  timeout 10s ffprobe rtsp://admin:admin@localhost:554/vs0 2>&1 | grep -q "Video:"
  local video_ok=$?

  kill -INT $pid
  wait $pid

  [[ $video_ok -eq 0 ]] || exit 1
}
```

## Conclusion

The ONVIF video pipeline provides a robust, production-ready solution for camera streaming with comprehensive error handling and performance optimization. Key features:

1. **Multi-Path Sensor Detection** - Automatic fallback for sensor configuration
2. **Hardware Abstraction** - Clean separation between hardware and application layers
3. **Configuration Validation** - Automatic parameter validation and correction
4. **Error Recovery** - Graceful degradation when video components fail
5. **Performance Monitoring** - Built-in statistics and logging for troubleshooting
6. **Multi-Stream Support** - Concurrent main/sub stream delivery
7. **Thread Safety** - Mutex-protected operations throughout the pipeline

For development, always test the complete pipeline from sensor detection through RTSP streaming. The comprehensive logging system provides detailed visibility into each phase of the video flow for effective debugging.