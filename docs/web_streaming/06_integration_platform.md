# Platform Integration and Implementation

## Overview

This module details the integration points between the web streaming implementations and the existing Anyka AK3918 platform, including API extensions, video pipeline integration, and ONVIF service modifications.

## Existing Platform Architecture

### Current Video Pipeline
```
Camera Sensor → VI (Video Input) → VPSS (Video Processing) → VENC (Video Encoding) → RTSP Server
                                                            ↓
                                                       JPEG Encoder → Snapshot Service
```

### Existing Platform APIs
```c
// Video Input APIs
int platform_vi_start(platform_vi_handle_t handle);
int platform_vi_get_frame(platform_vi_handle_t handle, platform_frame_t* frame);
int platform_vi_stop(platform_vi_handle_t handle);

// Video Processing APIs
int platform_vpss_start(platform_vpss_handle_t handle);
int platform_vpss_process_frame(platform_vpss_handle_t handle, platform_frame_t* frame);

// Video Encoding APIs
int platform_venc_start(platform_venc_handle_t handle);
int platform_venc_encode_frame(platform_venc_handle_t handle, platform_frame_t* frame);
int platform_venc_get_stream(platform_venc_handle_t handle, platform_stream_t* stream);
```

## Required Platform Extensions

### 1. JPEG Frame Extraction (Phase 1 - MJPEG)

```c
// Add to platform_anyka.c
int platform_get_jpeg_frame(const char* profile_token, uint8_t* buffer,
                           size_t buffer_size, size_t* frame_size) {
    // Get platform handle for profile
    platform_venc_handle_t handle = get_platform_handle_by_profile(profile_token);
    if (!handle) {
        return PLATFORM_ERROR_INVALID_HANDLE;
    }

    // Get current frame from VI
    platform_frame_t raw_frame;
    int ret = platform_vi_get_frame(handle->vi_handle, &raw_frame);
    if (ret != 0) {
        return ret;
    }

    // Configure JPEG encoder
    ak_venc_jpeg_param_t jpeg_param = {0};
    jpeg_param.quality = g_jpeg_config.quality;
    jpeg_param.width = raw_frame.width;
    jpeg_param.height = raw_frame.height;
    jpeg_param.format = AK_VIDEO_FORMAT_YUV420SP;

    // Encode to JPEG
    ret = ak_venc_encode_jpeg(handle->jpeg_handle, &raw_frame,
                             &jpeg_param, buffer, buffer_size, frame_size);

    // Release frame
    platform_vi_release_frame(handle->vi_handle, &raw_frame);

    return ret;
}

int platform_configure_jpeg_encoder(const char* profile_token, int quality,
                                   int width, int height) {
    platform_venc_handle_t handle = get_platform_handle_by_profile(profile_token);
    if (!handle) {
        return PLATFORM_ERROR_INVALID_HANDLE;
    }

    // Update JPEG encoder configuration
    ak_venc_jpeg_param_t jpeg_config = {0};
    jpeg_config.quality = quality;
    jpeg_config.width = width;
    jpeg_config.height = height;
    jpeg_config.format = AK_VIDEO_FORMAT_YUV420SP;

    return ak_venc_set_jpeg_param(handle->jpeg_handle, &jpeg_config);
}
```

### 2. H.264 NAL Unit Extraction (Phase 2 - WebSocket)

```c
// Add to platform_anyka.c
int platform_get_h264_nal_units(const char* profile_token, uint8_t* buffer,
                               size_t buffer_size, size_t* frame_size) {
    platform_venc_handle_t handle = get_platform_handle_by_profile(profile_token);
    if (!handle) {
        return PLATFORM_ERROR_INVALID_HANDLE;
    }

    // Get H.264 stream
    platform_stream_t h264_stream;
    int ret = platform_venc_get_stream(handle->h264_handle, &h264_stream);
    if (ret != 0) {
        return ret;
    }

    // Extract NAL units with start codes
    size_t output_size = 0;
    uint8_t* input_ptr = h264_stream.data;
    size_t remaining = h264_stream.size;

    while (remaining > 4 && output_size + 4 < buffer_size) {
        // Find NAL unit
        size_t nal_size = find_nal_unit_length(input_ptr, remaining);
        if (nal_size == 0) break;

        // Add start code (0x00 0x00 0x00 0x01)
        buffer[output_size++] = 0x00;
        buffer[output_size++] = 0x00;
        buffer[output_size++] = 0x00;
        buffer[output_size++] = 0x01;

        // Copy NAL unit
        if (output_size + nal_size <= buffer_size) {
            memcpy(buffer + output_size, input_ptr, nal_size);
            output_size += nal_size;
        }

        input_ptr += nal_size;
        remaining -= nal_size;
    }

    *frame_size = output_size;
    platform_venc_release_stream(handle->h264_handle, &h264_stream);

    return 0;
}
```

### 3. MP4 Segment Creation (Phase 3 - HLS)

```c
// Add to platform_anyka.c
int platform_create_mp4_segment(const uint8_t* h264_data, size_t data_size,
                               const char* output_path, int duration_ms) {
    // Use lightweight MP4 muxer or FFmpeg
    mp4_muxer_t* muxer = mp4_muxer_create(output_path);
    if (!muxer) {
        return PLATFORM_ERROR_MP4_CREATE_FAILED;
    }

    // Configure video track
    mp4_track_config_t video_config = {0};
    video_config.type = MP4_TRACK_VIDEO;
    video_config.codec = MP4_CODEC_H264;
    video_config.width = get_video_width();
    video_config.height = get_video_height();
    video_config.framerate = get_video_framerate();

    int track_id = mp4_muxer_add_track(muxer, &video_config);
    if (track_id < 0) {
        mp4_muxer_destroy(muxer);
        return PLATFORM_ERROR_MP4_TRACK_FAILED;
    }

    // Process H.264 data into MP4 samples
    uint32_t timestamp = 0;
    uint32_t sample_duration = duration_ms * 90; // 90kHz timescale

    int ret = mp4_muxer_write_sample(muxer, track_id, h264_data, data_size,
                                   timestamp, sample_duration, true);

    mp4_muxer_finalize(muxer);
    mp4_muxer_destroy(muxer);

    return ret;
}
```

### 4. RTP Packet Generation (Phase 4 - WebRTC)

```c
// Add to platform_anyka.c
int platform_create_rtp_packets(const char* profile_token, uint8_t* h264_data,
                               size_t data_size, rtp_packet_t* packets,
                               int max_packets, int* packet_count) {
    const int MAX_PAYLOAD_SIZE = 1200; // MTU consideration
    int packets_created = 0;
    size_t offset = 0;
    uint32_t timestamp = get_current_rtp_timestamp();

    while (offset < data_size && packets_created < max_packets) {
        rtp_packet_t* packet = &packets[packets_created];

        // Calculate payload size for this packet
        size_t payload_size = min(data_size - offset, MAX_PAYLOAD_SIZE);
        bool is_last = (offset + payload_size >= data_size);

        // Create RTP header
        create_rtp_header(&packet->header, packets_created, timestamp, 96, is_last);

        // Copy payload
        packet->payload_size = payload_size;
        memcpy(packet->payload, h264_data + offset, payload_size);

        offset += payload_size;
        packets_created++;
    }

    *packet_count = packets_created;
    return 0;
}
```

## ONVIF Media Service Extensions

### 1. Browser Stream URIs

```c
// Add to onvif_media.c
int onvif_media_get_browser_stream_uri(const char* profile_token,
                                     const char* protocol,
                                     struct stream_uri* uri) {
    if (!profile_token || !protocol || !uri) {
        return ONVIF_ERROR_INVALID_PARAMS;
    }

    // Validate profile exists
    if (!onvif_media_profile_exists(profile_token)) {
        return ONVIF_ERROR_PROFILE_NOT_FOUND;
    }

    const char* device_ip = get_device_ip();

    // Generate URI based on requested protocol
    if (strcmp(protocol, "MJPEG") == 0) {
        snprintf(uri->uri, sizeof(uri->uri),
                "http://%s:%d/stream.mjpeg?profile=%s",
                device_ip, HTTP_PORT, profile_token);

    } else if (strcmp(protocol, "WebSocket") == 0) {
        snprintf(uri->uri, sizeof(uri->uri),
                "ws://%s:%d/ws/stream?profile=%s",
                device_ip, WEBSOCKET_PORT, profile_token);

    } else if (strcmp(protocol, "HLS") == 0) {
        snprintf(uri->uri, sizeof(uri->uri),
                "http://%s:%d/hls/playlist.m3u8?profile=%s",
                device_ip, HTTP_PORT, profile_token);

    } else if (strcmp(protocol, "WebRTC") == 0) {
        snprintf(uri->uri, sizeof(uri->uri),
                "ws://%s:%d/webrtc/signaling?profile=%s",
                device_ip, WEBRTC_SIGNALING_PORT, profile_token);
    } else {
        return ONVIF_ERROR_UNSUPPORTED_PROTOCOL;
    }

    uri->invalid_after_connect = false;
    uri->invalid_after_reboot = false;
    uri->timeout = PT30S;

    return ONVIF_SUCCESS;
}

// Get list of supported browser streaming protocols
int onvif_media_get_supported_browser_protocols(char* protocols[], int max_count) {
    int count = 0;

    if (g_streaming_config.mjpeg_enabled && count < max_count) {
        protocols[count++] = strdup("MJPEG");
    }

    if (g_streaming_config.websocket_enabled && count < max_count) {
        protocols[count++] = strdup("WebSocket");
    }

    if (g_streaming_config.hls_enabled && count < max_count) {
        protocols[count++] = strdup("HLS");
    }

    if (g_streaming_config.webrtc_enabled && count < max_count) {
        protocols[count++] = strdup("WebRTC");
    }

    return count;
}
```

### 2. Streaming Configuration Management

```c
// Add streaming configuration to ONVIF media service
int onvif_media_configure_browser_streaming(const char* profile_token,
                                          const browser_streaming_config_t* config) {
    if (!profile_token || !config) {
        return ONVIF_ERROR_INVALID_PARAMS;
    }

    // Get profile configuration
    media_profile_t* profile = get_media_profile(profile_token);
    if (!profile) {
        return ONVIF_ERROR_PROFILE_NOT_FOUND;
    }

    // Update streaming configuration
    profile->browser_config = *config;

    // Apply configuration to platform
    if (config->mjpeg_enabled) {
        platform_configure_jpeg_encoder(profile_token, config->mjpeg_quality,
                                       profile->video_config.width,
                                       profile->video_config.height);
    }

    // Save configuration
    return save_media_profile_configuration(profile);
}

// Get current streaming configuration
int onvif_media_get_browser_streaming_config(const char* profile_token,
                                            browser_streaming_config_t* config) {
    media_profile_t* profile = get_media_profile(profile_token);
    if (!profile) {
        return ONVIF_ERROR_PROFILE_NOT_FOUND;
    }

    *config = profile->browser_config;
    return ONVIF_SUCCESS;
}
```

## HTTP Server Integration

### 1. Request Routing Extensions

```c
// Add to http_server.c
static const http_route_t g_browser_streaming_routes[] = {
    {"/stream.mjpeg",         HTTP_GET, handle_mjpeg_stream_request},
    {"/ws/stream",           HTTP_GET, handle_websocket_upgrade_request},
    {"/hls/playlist.m3u8",   HTTP_GET, handle_hls_playlist_request},
    {"/hls/segment_*.ts",    HTTP_GET, handle_hls_segment_request},
    {"/webrtc/signaling",    HTTP_GET, handle_webrtc_signaling_request},
    {NULL, 0, NULL}
};

// Enhanced route matching with streaming support
static int match_streaming_route(const char* path, const char* method) {
    for (int i = 0; g_browser_streaming_routes[i].path; i++) {
        if (match_route_pattern(g_browser_streaming_routes[i].path, path) &&
            strcmp(g_browser_streaming_routes[i].method, method) == 0) {
            return i;
        }
    }
    return -1;
}

// Pattern matching for HLS segments
static bool match_route_pattern(const char* pattern, const char* path) {
    if (strstr(pattern, "*")) {
        // Handle wildcard patterns like "/hls/segment_*.ts"
        char* wildcard_pos = strstr(pattern, "*");
        size_t prefix_len = wildcard_pos - pattern;

        if (strncmp(pattern, path, prefix_len) != 0) {
            return false;
        }

        // Check suffix after wildcard
        const char* suffix = wildcard_pos + 1;
        size_t path_len = strlen(path);
        size_t suffix_len = strlen(suffix);

        if (suffix_len > 0) {
            return strcmp(path + path_len - suffix_len, suffix) == 0;
        }

        return true;
    } else {
        return strcmp(pattern, path) == 0;
    }
}
```

### 2. CORS and Security Headers

```c
// Add CORS support for browser compatibility
static void add_cors_headers(char* response, size_t response_size) {
    strncat(response, "Access-Control-Allow-Origin: *\r\n",
            response_size - strlen(response) - 1);
    strncat(response, "Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n",
            response_size - strlen(response) - 1);
    strncat(response, "Access-Control-Allow-Headers: Content-Type, Authorization\r\n",
            response_size - strlen(response) - 1);
}

// Handle OPTIONS preflight requests
static int handle_options_request(const http_request_t* request,
                                char* response, size_t response_size) {
    int written = snprintf(response, response_size,
        "HTTP/1.1 200 OK\r\n"
        "Content-Length: 0\r\n");

    add_cors_headers(response + written, response_size - written);
    strncat(response, "\r\n", response_size - strlen(response) - 1);

    return strlen(response);
}
```

## Configuration Management

### 1. Streaming Configuration Structure

```c
// Add to config.h
typedef struct {
    // MJPEG Configuration
    bool mjpeg_enabled;
    int mjpeg_quality;              // 1-100
    int mjpeg_frame_rate;           // 1-30 fps
    int mjpeg_max_clients;          // Max concurrent MJPEG streams

    // WebSocket Configuration
    bool websocket_enabled;
    int websocket_port;
    int websocket_max_clients;
    bool websocket_auth_required;

    // HLS Configuration
    bool hls_enabled;
    int hls_segment_duration;       // seconds
    int hls_max_segments;           // sliding window size
    char hls_storage_path[256];

    // WebRTC Configuration
    bool webrtc_enabled;
    int webrtc_signaling_port;
    char webrtc_stun_server[256];
    char webrtc_turn_server[256];

    // General Settings
    bool cors_enabled;
    bool rate_limiting_enabled;
    int max_total_streams;
} browser_streaming_config_t;

extern browser_streaming_config_t g_streaming_config;
```

### 2. Configuration Loading and Validation

```c
// Add to config.c
int load_browser_streaming_config(const char* config_file) {
    FILE* fp = fopen(config_file, "r");
    if (!fp) {
        // Use defaults
        set_default_streaming_config();
        return 0;
    }

    char line[256];
    while (fgets(line, sizeof(line), fp)) {
        parse_config_line(line, &g_streaming_config);
    }

    fclose(fp);
    return validate_streaming_config(&g_streaming_config);
}

static void set_default_streaming_config(void) {
    g_streaming_config.mjpeg_enabled = true;
    g_streaming_config.mjpeg_quality = 80;
    g_streaming_config.mjpeg_frame_rate = 15;
    g_streaming_config.mjpeg_max_clients = 5;

    g_streaming_config.websocket_enabled = true;
    g_streaming_config.websocket_port = 8081;
    g_streaming_config.websocket_max_clients = 3;

    g_streaming_config.hls_enabled = true;
    g_streaming_config.hls_segment_duration = 10;
    g_streaming_config.hls_max_segments = 10;
    strcpy(g_streaming_config.hls_storage_path, "/tmp/hls");

    g_streaming_config.webrtc_enabled = false; // Advanced feature
    g_streaming_config.webrtc_signaling_port = 8082;
    strcpy(g_streaming_config.webrtc_stun_server, "stun:stun.l.google.com:19302");
}
```

## Performance Monitoring and Resource Management

### 1. Resource Monitoring

```c
// Add resource monitoring for streaming services
typedef struct {
    uint64_t mjpeg_frames_sent;
    uint64_t mjpeg_bytes_sent;
    int mjpeg_active_clients;

    uint64_t websocket_frames_sent;
    uint64_t websocket_bytes_sent;
    int websocket_active_clients;

    uint64_t hls_segments_created;
    uint64_t hls_bytes_written;
    int hls_active_clients;

    double cpu_usage_percent;
    size_t memory_usage_bytes;
    size_t disk_usage_bytes;
} streaming_performance_stats_t;

int get_streaming_performance_stats(streaming_performance_stats_t* stats) {
    memset(stats, 0, sizeof(streaming_performance_stats_t));

    // Collect statistics from each streaming module
    collect_mjpeg_stats(stats);
    collect_websocket_stats(stats);
    collect_hls_stats(stats);

    // System resource usage
    stats->cpu_usage_percent = get_cpu_usage_percentage();
    stats->memory_usage_bytes = get_memory_usage_bytes();
    stats->disk_usage_bytes = get_disk_usage_bytes();

    return 0;
}
```

### 2. Load Balancing and Rate Limiting

```c
// Streaming load balancer
static int check_streaming_capacity(streaming_protocol_t protocol) {
    streaming_performance_stats_t stats;
    get_streaming_performance_stats(&stats);

    // Check CPU usage
    if (stats.cpu_usage_percent > MAX_CPU_THRESHOLD) {
        return STREAMING_ERROR_CPU_OVERLOAD;
    }

    // Check protocol-specific limits
    switch (protocol) {
        case STREAMING_MJPEG:
            if (stats.mjpeg_active_clients >= g_streaming_config.mjpeg_max_clients) {
                return STREAMING_ERROR_MAX_CLIENTS;
            }
            break;

        case STREAMING_WEBSOCKET:
            if (stats.websocket_active_clients >= g_streaming_config.websocket_max_clients) {
                return STREAMING_ERROR_MAX_CLIENTS;
            }
            break;
    }

    return STREAMING_SUCCESS;
}
```

## Integration Testing

### 1. Platform API Testing

```c
// Test platform integration functions
void test_platform_integration(void) {
    // Test JPEG frame extraction
    uint8_t jpeg_buffer[1024 * 1024];
    size_t jpeg_size;

    int ret = platform_get_jpeg_frame("MainProfile", jpeg_buffer,
                                    sizeof(jpeg_buffer), &jpeg_size);
    assert(ret == 0);
    assert(jpeg_size > 0);
    assert(is_valid_jpeg(jpeg_buffer, jpeg_size));

    // Test H.264 NAL unit extraction
    uint8_t h264_buffer[1024 * 1024];
    size_t h264_size;

    ret = platform_get_h264_nal_units("MainProfile", h264_buffer,
                                    sizeof(h264_buffer), &h264_size);
    assert(ret == 0);
    assert(h264_size > 0);
    assert(has_h264_start_codes(h264_buffer, h264_size));
}
```

### 2. ONVIF Service Testing

```c
// Test ONVIF media service extensions
void test_onvif_media_extensions(void) {
    struct stream_uri uri;

    // Test MJPEG URI generation
    int ret = onvif_media_get_browser_stream_uri("MainProfile", "MJPEG", &uri);
    assert(ret == ONVIF_SUCCESS);
    assert(strstr(uri.uri, "stream.mjpeg") != NULL);

    // Test WebSocket URI generation
    ret = onvif_media_get_browser_stream_uri("MainProfile", "WebSocket", &uri);
    assert(ret == ONVIF_SUCCESS);
    assert(strstr(uri.uri, "ws://") != NULL);

    // Test protocol enumeration
    char* protocols[10];
    int count = onvif_media_get_supported_browser_protocols(protocols, 10);
    assert(count > 0);
    assert(count <= 4); // MJPEG, WebSocket, HLS, WebRTC
}
```

## Implementation Checklist

- [ ] Extend platform APIs for JPEG frame extraction
- [ ] Add H.264 NAL unit extraction functions
- [ ] Implement MP4 segment creation capability
- [ ] Add RTP packet generation for WebRTC
- [ ] Extend ONVIF media service with browser streaming URIs
- [ ] Add streaming configuration management
- [ ] Integrate browser streaming routes in HTTP server
- [ ] Implement resource monitoring and load balancing
- [ ] Add CORS support for browser compatibility
- [ ] Create platform integration tests
- [ ] Verify ONVIF service extensions
- [ ] Test performance under load

## Next Steps

After completing platform integration, proceed to [07_security_performance.md](07_security_performance.md) for security considerations and performance optimization.

## Related Documentation

- **Previous**: [05_phase4_webrtc.md](05_phase4_webrtc.md) - WebRTC implementation
- **Next**: [07_security_performance.md](07_security_performance.md) - Security and performance
- **Current Architecture**: [01_current_architecture.md](01_current_architecture.md) - Existing system
- **Templates**: [templates/](templates/) - Implementation code patterns