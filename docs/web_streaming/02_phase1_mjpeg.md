# Phase 1: MJPEG Streaming Implementation

## Overview

**Priority**: High | **Complexity**: Low | **Browser Support**: Universal (100%)

MJPEG (Motion JPEG) provides immediate browser compatibility by streaming a series of JPEG images using the `multipart/x-mixed-replace` HTTP response format. This creates the foundation for web-based video streaming.

## Technical Approach

### Core Concept
Stream continuous JPEG frames over HTTP using multipart responses. Each frame is a complete JPEG image, making it universally compatible with all browsers without requiring special codecs or plugins.

### Implementation Strategy
1. **HTTP Endpoint**: Add `/stream.mjpeg` to existing HTTP server
2. **Frame Conversion**: Convert H.264 frames to JPEG using platform encoder
3. **Streaming Protocol**: Use `multipart/x-mixed-replace` MIME type
4. **Quality Control**: Configurable JPEG quality and frame rate

## Detailed Implementation

### 1. HTTP Server Extensions

#### 1.1 New Request Handler
```c
// Add to http_server.c
static int handle_mjpeg_stream_request(const http_request_t* request,
                                      char* response, size_t response_size) {
    // Extract profile token from request
    const char* profile_token = get_query_parameter(request->uri, "profile");
    if (!profile_token) {
        profile_token = "MainProfile"; // Default profile
    }

    // Validate profile exists
    if (!onvif_media_profile_exists(profile_token)) {
        return create_http_error_response(response, response_size, 404, "Profile not found");
    }

    // Create multipart response header
    create_mjpeg_stream_header(response, response_size);

    // Start streaming in separate thread
    return start_mjpeg_streaming(request->client_fd, profile_token);
}

static int create_mjpeg_stream_header(char* response, size_t response_size) {
    return snprintf(response, response_size,
        "HTTP/1.1 200 OK\r\n"
        "Content-Type: multipart/x-mixed-replace; boundary=frame\r\n"
        "Cache-Control: no-cache\r\n"
        "Pragma: no-cache\r\n"
        "Connection: close\r\n"
        "\r\n");
}
```

#### 1.2 Streaming Thread Implementation
```c
static int start_mjpeg_streaming(int client_fd, const char* profile_token) {
    mjpeg_stream_context_t* ctx = malloc(sizeof(mjpeg_stream_context_t));
    if (!ctx) return -1;

    ctx->client_fd = client_fd;
    strncpy(ctx->profile_token, profile_token, sizeof(ctx->profile_token) - 1);
    ctx->streaming_active = true;
    ctx->quality = g_mjpeg_config.quality;
    ctx->frame_rate = g_mjpeg_config.frame_rate;

    // Create streaming thread
    if (pthread_create(&ctx->stream_thread, NULL, mjpeg_streaming_thread, ctx) != 0) {
        free(ctx);
        return -1;
    }

    return 0;
}

static void* mjpeg_streaming_thread(void* arg) {
    mjpeg_stream_context_t* ctx = (mjpeg_stream_context_t*)arg;
    uint8_t jpeg_buffer[MAX_JPEG_SIZE];
    size_t jpeg_size;

    // Calculate frame interval
    int frame_interval_ms = 1000 / ctx->frame_rate;

    while (ctx->streaming_active) {
        // Get JPEG frame from platform
        if (platform_get_jpeg_frame(ctx->profile_token, jpeg_buffer,
                                   sizeof(jpeg_buffer), &jpeg_size) == 0) {

            // Send frame over HTTP
            if (send_mjpeg_frame(ctx->client_fd, jpeg_buffer, jpeg_size) != 0) {
                break; // Client disconnected
            }
        }

        // Wait for next frame interval
        usleep(frame_interval_ms * 1000);
    }

    cleanup_mjpeg_context(ctx);
    return NULL;
}
```

#### 1.3 Frame Transmission
```c
static int send_mjpeg_frame(int client_fd, const uint8_t* jpeg_data, size_t jpeg_size) {
    char frame_header[256];
    int header_len = snprintf(frame_header, sizeof(frame_header),
        "\r\n--frame\r\n"
        "Content-Type: image/jpeg\r\n"
        "Content-Length: %zu\r\n"
        "\r\n", jpeg_size);

    // Send frame header
    if (send(client_fd, frame_header, header_len, MSG_NOSIGNAL) != header_len) {
        return -1; // Client disconnected
    }

    // Send JPEG data
    if (send(client_fd, jpeg_data, jpeg_size, MSG_NOSIGNAL) != jpeg_size) {
        return -1; // Client disconnected
    }

    return 0;
}
```

### 2. Platform Integration

#### 2.1 JPEG Frame Extraction
```c
// Add to platform_anyka.c
int platform_get_jpeg_frame(const char* profile_token, uint8_t* buffer,
                           size_t buffer_size, size_t* frame_size) {
    // Get platform stream handle
    platform_venc_handle_t handle = get_platform_handle(profile_token);
    if (!handle) return -1;

    // Get H.264 frame
    platform_stream_t h264_stream;
    if (platform_venc_get_stream(handle, &h264_stream) != 0) {
        return -1;
    }

    // Convert H.264 to JPEG using hardware encoder
    ak_venc_jpeg_param_t jpeg_param = {0};
    jpeg_param.quality = g_mjpeg_config.quality;
    jpeg_param.width = h264_stream.width;
    jpeg_param.height = h264_stream.height;

    // Perform conversion
    int ret = ak_venc_encode_jpeg(handle, &h264_stream.frame_data,
                                 &jpeg_param, buffer, buffer_size, frame_size);

    platform_venc_release_stream(handle, &h264_stream);
    return ret;
}

int platform_configure_jpeg_encoder(int quality, int width, int height) {
    // Configure JPEG encoder parameters
    ak_venc_jpeg_param_t jpeg_config = {0};
    jpeg_config.quality = quality;
    jpeg_config.width = width;
    jpeg_config.height = height;
    jpeg_config.format = AK_VIDEO_FORMAT_YUV420SP;

    return ak_venc_set_jpeg_param(&jpeg_config);
}
```

### 3. Media Service Integration

#### 3.1 ONVIF Media Service Extensions
```c
// Add to onvif_media.c
int onvif_media_get_mjpeg_stream_uri(const char* profile_token,
                                   struct stream_uri* uri) {
    if (!profile_token || !uri) return -1;

    // Validate profile exists
    if (!onvif_media_profile_exists(profile_token)) {
        return -1;
    }

    // Construct MJPEG stream URI
    snprintf(uri->uri, sizeof(uri->uri),
        "http://%s:%d/stream.mjpeg?profile=%s",
        get_device_ip(), HTTP_PORT, profile_token);

    uri->invalid_after_connect = false;
    uri->invalid_after_reboot = false;
    uri->timeout = PT30S; // 30 seconds

    return 0;
}

int onvif_media_configure_mjpeg_stream(const char* profile_token,
                                     int quality, int frame_rate) {
    // Validate parameters
    if (quality < 1 || quality > 100) return -1;
    if (frame_rate < 1 || frame_rate > 30) return -1;

    // Store configuration
    mjpeg_profile_config_t* config = get_mjpeg_profile_config(profile_token);
    if (!config) return -1;

    config->quality = quality;
    config->frame_rate = frame_rate;
    config->enabled = true;

    return save_mjpeg_configuration();
}
```

### 4. Configuration and Constants

#### 4.1 Configuration Structure
```c
// Add to config.h
typedef struct {
    bool enabled;
    int quality;          // 1-100
    int frame_rate;       // 1-30 fps
    int max_clients;      // Maximum concurrent streams
    bool auth_required;   // Require authentication
} mjpeg_config_t;

// Global configuration
extern mjpeg_config_t g_mjpeg_config;
```

#### 4.2 Constants
```c
// Add to onvif_constants.h
#define MJPEG_STREAM_PATH "/stream.mjpeg"
#define MJPEG_QUALITY_DEFAULT 80
#define MJPEG_FRAME_RATE_DEFAULT 15
#define MJPEG_BOUNDARY "frame"
#define MJPEG_MAX_CLIENTS 5
#define MAX_JPEG_SIZE (1024 * 1024)  // 1MB max JPEG size

// MIME types
#define MIME_TYPE_MJPEG "multipart/x-mixed-replace; boundary=frame"
#define MIME_TYPE_JPEG "image/jpeg"
```

## Browser Integration

### Simple HTML Integration
```html
<!DOCTYPE html>
<html>
<head>
    <title>MJPEG Stream</title>
    <style>
        .stream-container {
            display: flex;
            flex-direction: column;
            align-items: center;
            margin: 20px;
        }
        .video-stream {
            max-width: 100%;
            height: auto;
            border: 2px solid #333;
            border-radius: 8px;
        }
        .controls {
            margin: 10px 0;
        }
        .control-button {
            margin: 5px;
            padding: 10px 15px;
            background-color: #007bff;
            color: white;
            border: none;
            border-radius: 5px;
            cursor: pointer;
        }
    </style>
</head>
<body>
    <div class="stream-container">
        <h1>Live Camera Stream</h1>
        <img id="mjpegStream" class="video-stream"
             src="http://192.168.1.100:8080/stream.mjpeg"
             alt="Live Stream"
             onerror="handleStreamError()">

        <div class="controls">
            <button class="control-button" onclick="refreshStream()">Refresh Stream</button>
            <button class="control-button" onclick="toggleStream()">Toggle Stream</button>
        </div>
    </div>

    <script>
        let streamActive = true;
        const streamImg = document.getElementById('mjpegStream');
        const originalSrc = streamImg.src;

        function handleStreamError() {
            console.error('Stream error occurred');
            // Retry after 5 seconds
            setTimeout(refreshStream, 5000);
        }

        function refreshStream() {
            // Force refresh by adding timestamp
            const timestamp = new Date().getTime();
            streamImg.src = originalSrc + '&t=' + timestamp;
        }

        function toggleStream() {
            if (streamActive) {
                streamImg.src = '';
                streamActive = false;
            } else {
                refreshStream();
                streamActive = true;
            }
        }
    </script>
</body>
</html>
```

### JavaScript Stream Control
```javascript
class MJPEGStreamController {
    constructor(streamUrl, imageElement) {
        this.streamUrl = streamUrl;
        this.imageElement = imageElement;
        this.isStreaming = false;
        this.retryCount = 0;
        this.maxRetries = 5;
    }

    start() {
        if (this.isStreaming) return;

        this.isStreaming = true;
        this.imageElement.src = this.streamUrl;

        // Handle stream errors
        this.imageElement.onerror = () => this.handleError();
        this.imageElement.onload = () => this.handleSuccess();
    }

    stop() {
        this.isStreaming = false;
        this.imageElement.src = '';
        this.retryCount = 0;
    }

    handleError() {
        console.warn(`MJPEG stream error (attempt ${this.retryCount + 1})`);

        if (this.retryCount < this.maxRetries && this.isStreaming) {
            this.retryCount++;
            // Exponential backoff
            const delay = Math.min(1000 * Math.pow(2, this.retryCount), 30000);
            setTimeout(() => this.retry(), delay);
        } else {
            console.error('MJPEG stream failed after maximum retries');
            this.stop();
        }
    }

    handleSuccess() {
        this.retryCount = 0; // Reset retry count on success
    }

    retry() {
        if (this.isStreaming) {
            const timestamp = new Date().getTime();
            this.imageElement.src = this.streamUrl + '&retry=' + timestamp;
        }
    }

    setQuality(quality) {
        // Assumes server supports quality parameter
        const url = new URL(this.streamUrl);
        url.searchParams.set('quality', quality);
        this.streamUrl = url.toString();

        if (this.isStreaming) {
            this.imageElement.src = this.streamUrl;
        }
    }
}
```

## Performance Optimization

### 1. Frame Rate Adaptation
```c
// Adaptive frame rate based on client processing
static void adapt_frame_rate(mjpeg_stream_context_t* ctx) {
    static time_t last_check = 0;
    time_t now = time(NULL);

    if (now - last_check >= 5) { // Check every 5 seconds
        // Monitor client socket buffer
        int send_buffer_size;
        socklen_t size = sizeof(send_buffer_size);
        getsockopt(ctx->client_fd, SOL_SOCKET, SO_SNDBUF,
                  &send_buffer_size, &size);

        // Adapt frame rate based on buffer usage
        if (send_buffer_size > BUFFER_HIGH_WATERMARK) {
            ctx->frame_rate = max(ctx->frame_rate - 2, MIN_FRAME_RATE);
        } else if (send_buffer_size < BUFFER_LOW_WATERMARK) {
            ctx->frame_rate = min(ctx->frame_rate + 1, MAX_FRAME_RATE);
        }

        last_check = now;
    }
}
```

### 2. Quality Management
```c
// Dynamic quality adjustment based on system load
static void manage_jpeg_quality(mjpeg_stream_context_t* ctx) {
    float cpu_usage = get_cpu_usage_percentage();

    if (cpu_usage > CPU_HIGH_THRESHOLD) {
        // Reduce quality to save CPU
        ctx->quality = max(ctx->quality - 5, MIN_QUALITY);
    } else if (cpu_usage < CPU_LOW_THRESHOLD) {
        // Increase quality if CPU allows
        ctx->quality = min(ctx->quality + 2, MAX_QUALITY);
    }
}
```

## Testing Strategy

### 1. Browser Compatibility Testing
```bash
# Test URLs for different browsers
curl -H "User-Agent: Chrome" http://camera-ip:8080/stream.mjpeg
curl -H "User-Agent: Firefox" http://camera-ip:8080/stream.mjpeg
curl -H "User-Agent: Safari" http://camera-ip:8080/stream.mjpeg
curl -H "User-Agent: Edge" http://camera-ip:8080/stream.mjpeg
```

### 2. Performance Testing
```c
// Performance metrics collection
typedef struct {
    uint64_t frames_sent;
    uint64_t bytes_sent;
    uint64_t errors;
    double avg_frame_size;
    double fps_actual;
    time_t start_time;
} mjpeg_performance_stats_t;

static void collect_performance_metrics(mjpeg_stream_context_t* ctx) {
    ctx->stats.frames_sent++;
    ctx->stats.bytes_sent += ctx->last_frame_size;

    time_t now = time(NULL);
    double elapsed = difftime(now, ctx->stats.start_time);

    if (elapsed > 0) {
        ctx->stats.fps_actual = ctx->stats.frames_sent / elapsed;
        ctx->stats.avg_frame_size = (double)ctx->stats.bytes_sent / ctx->stats.frames_sent;
    }
}
```

### 3. Load Testing
Test concurrent clients:
```bash
# Simulate multiple clients
for i in {1..10}; do
    curl http://camera-ip:8080/stream.mjpeg > /dev/null 2>&1 &
done

# Monitor system resources
top -p $(pgrep onvifd)
```

## Implementation Checklist

- [ ] Add MJPEG endpoint to HTTP server request routing
- [ ] Implement multipart HTTP response handling
- [ ] Create JPEG frame extraction from H.264 stream
- [ ] Add streaming thread management
- [ ] Implement client connection handling and cleanup
- [ ] Add ONVIF Media service extensions for MJPEG URIs
- [ ] Create configuration management for quality/frame rate
- [ ] Add performance monitoring and adaptive controls
- [ ] Test browser compatibility across all major browsers
- [ ] Implement error handling and client disconnection detection

## Success Criteria

- **Universal Browser Support**: Stream displays in Chrome, Firefox, Safari, Edge
- **Performance**: < 10% CPU overhead for MJPEG streaming
- **Quality**: Configurable JPEG quality (50-95) with visible difference
- **Concurrent Streams**: Support 5+ simultaneous MJPEG streams
- **Reliability**: Stream remains stable for 24+ hours continuous operation

## Next Steps

After implementing MJPEG streaming, proceed to [03_phase2_websocket.md](03_phase2_websocket.md) for real-time streaming capabilities.

## Related Documentation

- **Previous**: [01_current_architecture.md](01_current_architecture.md) - Understanding current system
- **Next**: [03_phase2_websocket.md](03_phase2_websocket.md) - WebSocket streaming implementation
- **Platform**: [06_integration_platform.md](06_integration_platform.md) - Platform API details
- **Templates**: [templates/](templates/) - Implementation code patterns