# Phase 2: WebSocket Streaming Implementation

## Overview

**Priority**: Medium | **Complexity**: Medium | **Browser Support**: Modern browsers (95%)

WebSocket streaming provides low-latency, real-time video streaming by transmitting H.264 frames directly to browsers over WebSocket connections. This enables bidirectional communication for PTZ control and metadata streaming.

## Technical Approach

### Core Concept
Stream H.264 NAL units over WebSocket binary frames, allowing client-side JavaScript H.264 decoders to render video with minimal latency. WebSocket's persistent connection enables real-time bidirectional communication.

### Implementation Strategy
1. **WebSocket Server**: Implement WebSocket upgrade in HTTP server
2. **Frame Streaming**: Stream H.264 NAL units as binary WebSocket messages
3. **Client Decoder**: Provide JavaScript H.264 decoder integration
4. **Control Channel**: Enable PTZ commands and metadata via WebSocket

## Detailed Implementation

### 1. WebSocket Server Module

#### 1.1 WebSocket Server Structure
```c
// New file: src/networking/websocket/websocket_server.h
#ifndef WEBSOCKET_SERVER_H
#define WEBSOCKET_SERVER_H

#include <pthread.h>
#include <stdbool.h>

#define WS_MAX_CLIENTS 10
#define WS_FRAME_BUFFER_SIZE 65536
#define WS_GUID "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"

typedef enum {
    WS_FRAME_TEXT = 0x1,
    WS_FRAME_BINARY = 0x2,
    WS_FRAME_CLOSE = 0x8,
    WS_FRAME_PING = 0x9,
    WS_FRAME_PONG = 0xA
} ws_frame_type_t;

typedef struct {
    uint8_t* data;
    size_t size;
    ws_frame_type_t type;
    bool fin;
} ws_frame_t;

typedef struct {
    int client_fd;
    char profile_token[32];
    bool streaming_active;
    bool audio_enabled;
    pthread_t stream_thread;
    pthread_mutex_t send_mutex;
    uint64_t frames_sent;
    uint64_t bytes_sent;
    time_t connect_time;
} ws_client_t;

typedef struct {
    int server_fd;
    int port;
    ws_client_t clients[WS_MAX_CLIENTS];
    int client_count;
    pthread_mutex_t clients_mutex;
    bool server_running;
} ws_server_t;

// Function declarations
int ws_server_init(int port);
int ws_server_start(void);
int ws_server_stop(void);
int ws_handle_upgrade(const http_request_t* request);
int ws_send_video_frame(int client_fd, const uint8_t* frame_data, size_t frame_size);
int ws_send_control_response(int client_fd, const char* response);
int ws_handle_client_message(int client_fd, const ws_frame_t* frame);

#endif // WEBSOCKET_SERVER_H
```

#### 1.2 WebSocket Upgrade Handling
```c
// Add to websocket_server.c
int ws_handle_upgrade(const http_request_t* request) {
    char response[512];
    char accept_key[64];

    // Extract WebSocket key from headers
    const char* ws_key = get_header_value(request->headers, "Sec-WebSocket-Key");
    if (!ws_key) {
        return -1; // Invalid WebSocket request
    }

    // Generate accept key
    if (generate_ws_accept_key(ws_key, accept_key, sizeof(accept_key)) != 0) {
        return -1;
    }

    // Create WebSocket upgrade response
    int response_len = snprintf(response, sizeof(response),
        "HTTP/1.1 101 Switching Protocols\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        "Sec-WebSocket-Accept: %s\r\n"
        "\r\n", accept_key);

    // Send upgrade response
    if (send(request->client_fd, response, response_len, 0) != response_len) {
        return -1;
    }

    // Add client to WebSocket server
    return ws_add_client(request->client_fd, request->uri);
}

static int generate_ws_accept_key(const char* client_key, char* accept_key, size_t accept_size) {
    char combined_key[256];
    uint8_t sha1_result[20];

    // Combine client key with WebSocket GUID
    snprintf(combined_key, sizeof(combined_key), "%s%s", client_key, WS_GUID);

    // Calculate SHA1 hash
    sha1((uint8_t*)combined_key, strlen(combined_key), sha1_result);

    // Base64 encode the result
    return base64_encode(sha1_result, 20, accept_key, accept_size);
}
```

#### 1.3 Client Management
```c
static int ws_add_client(int client_fd, const char* uri) {
    pthread_mutex_lock(&g_ws_server.clients_mutex);

    // Find empty client slot
    int client_index = -1;
    for (int i = 0; i < WS_MAX_CLIENTS; i++) {
        if (g_ws_server.clients[i].client_fd == -1) {
            client_index = i;
            break;
        }
    }

    if (client_index == -1) {
        pthread_mutex_unlock(&g_ws_server.clients_mutex);
        return -1; // No available slots
    }

    // Initialize client
    ws_client_t* client = &g_ws_server.clients[client_index];
    client->client_fd = client_fd;
    client->streaming_active = true;
    client->audio_enabled = false;
    client->frames_sent = 0;
    client->bytes_sent = 0;
    client->connect_time = time(NULL);

    // Extract profile token from URI
    extract_profile_token(uri, client->profile_token, sizeof(client->profile_token));

    // Initialize mutex
    pthread_mutex_init(&client->send_mutex, NULL);

    // Start streaming thread
    if (pthread_create(&client->stream_thread, NULL, ws_streaming_thread, client) != 0) {
        ws_cleanup_client(client);
        pthread_mutex_unlock(&g_ws_server.clients_mutex);
        return -1;
    }

    g_ws_server.client_count++;
    pthread_mutex_unlock(&g_ws_server.clients_mutex);

    return 0;
}
```

### 2. Video Frame Streaming

#### 2.1 H.264 Frame Transmission
```c
static void* ws_streaming_thread(void* arg) {
    ws_client_t* client = (ws_client_t*)arg;
    uint8_t frame_buffer[WS_FRAME_BUFFER_SIZE];
    size_t frame_size;

    while (client->streaming_active) {
        // Get H.264 frame from platform
        if (platform_get_h264_frame(client->profile_token, frame_buffer,
                                   sizeof(frame_buffer), &frame_size) == 0) {

            // Send frame over WebSocket
            if (ws_send_binary_frame(client->client_fd, frame_buffer, frame_size) == 0) {
                client->frames_sent++;
                client->bytes_sent += frame_size;
            } else {
                // Client disconnected
                break;
            }
        }

        // Small delay to prevent overwhelming the client
        usleep(33333); // ~30 FPS
    }

    ws_cleanup_client(client);
    return NULL;
}

static int ws_send_binary_frame(int client_fd, const uint8_t* data, size_t data_size) {
    uint8_t header[14]; // Maximum WebSocket frame header size
    size_t header_size = 0;

    // First byte: FIN=1, RSV=0, OPCODE=BINARY
    header[0] = 0x80 | WS_FRAME_BINARY;

    // Payload length encoding
    if (data_size < 126) {
        header[1] = (uint8_t)data_size;
        header_size = 2;
    } else if (data_size < 65536) {
        header[1] = 126;
        header[2] = (uint8_t)((data_size >> 8) & 0xFF);
        header[3] = (uint8_t)(data_size & 0xFF);
        header_size = 4;
    } else {
        header[1] = 127;
        // 64-bit length (simplified for our use case)
        uint64_t len = data_size;
        for (int i = 9; i >= 2; i--) {
            header[i] = (uint8_t)(len & 0xFF);
            len >>= 8;
        }
        header_size = 10;
    }

    // Send header
    if (send(client_fd, header, header_size, MSG_NOSIGNAL) != header_size) {
        return -1;
    }

    // Send payload
    if (send(client_fd, data, data_size, MSG_NOSIGNAL) != data_size) {
        return -1;
    }

    return 0;
}
```

#### 2.2 Platform H.264 Frame Extraction
```c
// Add to platform_anyka.c
int platform_get_h264_frame(const char* profile_token, uint8_t* buffer,
                           size_t buffer_size, size_t* frame_size) {
    platform_venc_handle_t handle = get_platform_handle(profile_token);
    if (!handle) return -1;

    // Get H.264 stream
    platform_stream_t h264_stream;
    if (platform_venc_get_stream(handle, &h264_stream) != 0) {
        return -1;
    }

    // Extract NAL units
    size_t nal_size = 0;
    uint8_t* nal_data = h264_stream.data;
    size_t remaining = h264_stream.size;

    // Copy NAL units with start codes for browser decoder
    while (remaining > 4 && nal_size + 4 < buffer_size) {
        // Add start code (0x00 0x00 0x00 0x01)
        buffer[nal_size++] = 0x00;
        buffer[nal_size++] = 0x00;
        buffer[nal_size++] = 0x00;
        buffer[nal_size++] = 0x01;

        // Find next NAL unit
        size_t nal_unit_size = find_nal_unit_size(nal_data, remaining);
        if (nal_unit_size == 0 || nal_size + nal_unit_size > buffer_size) {
            break;
        }

        // Copy NAL unit data
        memcpy(buffer + nal_size, nal_data, nal_unit_size);
        nal_size += nal_unit_size;
        nal_data += nal_unit_size;
        remaining -= nal_unit_size;
    }

    *frame_size = nal_size;
    platform_venc_release_stream(handle, &h264_stream);

    return 0;
}
```

### 3. Control Message Handling

#### 3.1 PTZ Control Integration
```c
static int ws_handle_client_message(int client_fd, const ws_frame_t* frame) {
    if (frame->type == WS_FRAME_TEXT) {
        // Parse JSON control message
        json_object* json = json_tokener_parse((char*)frame->data);
        if (!json) return -1;

        const char* command_type;
        if (!json_object_object_get_ex(json, "type", &command_type)) {
            json_object_put(json);
            return -1;
        }

        if (strcmp(command_type, "ptz") == 0) {
            return handle_ptz_command(client_fd, json);
        } else if (strcmp(command_type, "config") == 0) {
            return handle_config_command(client_fd, json);
        } else if (strcmp(command_type, "ping") == 0) {
            return send_pong_response(client_fd);
        }

        json_object_put(json);
    } else if (frame->type == WS_FRAME_PING) {
        return ws_send_pong(client_fd);
    }

    return 0;
}

static int handle_ptz_command(int client_fd, json_object* json) {
    ws_client_t* client = find_client_by_fd(client_fd);
    if (!client) return -1;

    const char* command;
    if (!json_object_object_get_ex(json, "command", &command)) {
        return -1;
    }

    // Execute PTZ command
    int result = -1;
    if (strcmp(command, "move_up") == 0) {
        result = onvif_ptz_continuous_move(client->profile_token, 0.0, 0.5, 0.0);
    } else if (strcmp(command, "move_down") == 0) {
        result = onvif_ptz_continuous_move(client->profile_token, 0.0, -0.5, 0.0);
    } else if (strcmp(command, "move_left") == 0) {
        result = onvif_ptz_continuous_move(client->profile_token, -0.5, 0.0, 0.0);
    } else if (strcmp(command, "move_right") == 0) {
        result = onvif_ptz_continuous_move(client->profile_token, 0.5, 0.0, 0.0);
    } else if (strcmp(command, "stop") == 0) {
        result = onvif_ptz_stop(client->profile_token);
    } else if (strcmp(command, "home") == 0) {
        result = onvif_ptz_goto_home_position(client->profile_token);
    }

    // Send response
    return send_ptz_response(client_fd, command, result);
}
```

### 4. Client-Side JavaScript Implementation

#### 4.1 WebSocket Video Player Class
```javascript
// templates/websocket_player.js
class WebSocketVideoPlayer {
    constructor(wsUrl, canvasElement) {
        this.wsUrl = wsUrl;
        this.canvas = canvasElement;
        this.ctx = this.canvas.getContext('2d');
        this.ws = null;
        this.decoder = null;
        this.isConnected = false;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;

        this.initH264Decoder();
    }

    initH264Decoder() {
        // Using Broadway H.264 decoder (or similar)
        this.decoder = new H264Decoder();

        this.decoder.onPictureDecoded = (buffer, width, height) => {
            this.displayFrame(buffer, width, height);
        };
    }

    connect() {
        try {
            this.ws = new WebSocket(this.wsUrl);
            this.ws.binaryType = 'arraybuffer';

            this.ws.onopen = (event) => {
                console.log('WebSocket connected');
                this.isConnected = true;
                this.reconnectAttempts = 0;
                this.onConnectionStatusChanged(true);
            };

            this.ws.onmessage = (event) => {
                if (event.data instanceof ArrayBuffer) {
                    this.handleVideoFrame(event.data);
                } else {
                    this.handleControlMessage(JSON.parse(event.data));
                }
            };

            this.ws.onclose = (event) => {
                console.log('WebSocket disconnected:', event.code, event.reason);
                this.isConnected = false;
                this.onConnectionStatusChanged(false);
                this.attemptReconnect();
            };

            this.ws.onerror = (error) => {
                console.error('WebSocket error:', error);
            };

        } catch (error) {
            console.error('Failed to create WebSocket:', error);
            this.attemptReconnect();
        }
    }

    handleVideoFrame(frameData) {
        const uint8Array = new Uint8Array(frameData);

        // Decode H.264 frame
        try {
            this.decoder.decode(uint8Array);
        } catch (error) {
            console.error('H.264 decode error:', error);
        }
    }

    displayFrame(yuvBuffer, width, height) {
        // Convert YUV to RGB and display on canvas
        const imageData = this.ctx.createImageData(width, height);
        this.convertYUVToRGB(yuvBuffer, imageData.data, width, height);

        // Resize canvas if necessary
        if (this.canvas.width !== width || this.canvas.height !== height) {
            this.canvas.width = width;
            this.canvas.height = height;
        }

        this.ctx.putImageData(imageData, 0, 0);
    }

    sendPTZCommand(command) {
        if (this.isConnected) {
            const message = {
                type: 'ptz',
                command: command,
                timestamp: Date.now()
            };

            this.ws.send(JSON.stringify(message));
        }
    }

    sendConfigCommand(config) {
        if (this.isConnected) {
            const message = {
                type: 'config',
                config: config,
                timestamp: Date.now()
            };

            this.ws.send(JSON.stringify(message));
        }
    }

    disconnect() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        this.isConnected = false;
    }

    attemptReconnect() {
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);

            console.log(`Attempting reconnect ${this.reconnectAttempts}/${this.maxReconnectAttempts} in ${delay}ms`);

            setTimeout(() => {
                this.connect();
            }, delay);
        } else {
            console.error('Max reconnection attempts reached');
            this.onConnectionFailed();
        }
    }

    // Override these methods in your implementation
    onConnectionStatusChanged(connected) {
        // Handle connection status changes
    }

    onConnectionFailed() {
        // Handle permanent connection failure
    }
}
```

#### 4.2 PTZ Control Integration
```javascript
class PTZController {
    constructor(videoPlayer) {
        this.videoPlayer = videoPlayer;
        this.isMoving = false;
        this.currentCommand = null;

        this.setupEventListeners();
    }

    setupEventListeners() {
        // Mouse control
        document.addEventListener('mouseup', () => {
            if (this.isMoving) {
                this.stopPTZ();
            }
        });

        // Keyboard control
        document.addEventListener('keydown', (event) => {
            this.handleKeyDown(event);
        });

        document.addEventListener('keyup', (event) => {
            this.handleKeyUp(event);
        });
    }

    startPTZMove(direction) {
        if (this.isMoving && this.currentCommand === direction) {
            return; // Already moving in this direction
        }

        this.stopPTZ(); // Stop any current movement
        this.videoPlayer.sendPTZCommand('move_' + direction);
        this.isMoving = true;
        this.currentCommand = direction;
    }

    stopPTZ() {
        if (this.isMoving) {
            this.videoPlayer.sendPTZCommand('stop');
            this.isMoving = false;
            this.currentCommand = null;
        }
    }

    goHome() {
        this.stopPTZ();
        this.videoPlayer.sendPTZCommand('home');
    }

    handleKeyDown(event) {
        switch(event.key) {
            case 'ArrowUp':
                this.startPTZMove('up');
                event.preventDefault();
                break;
            case 'ArrowDown':
                this.startPTZMove('down');
                event.preventDefault();
                break;
            case 'ArrowLeft':
                this.startPTZMove('left');
                event.preventDefault();
                break;
            case 'ArrowRight':
                this.startPTZMove('right');
                event.preventDefault();
                break;
            case 'Home':
                this.goHome();
                event.preventDefault();
                break;
        }
    }

    handleKeyUp(event) {
        if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(event.key)) {
            this.stopPTZ();
            event.preventDefault();
        }
    }
}
```

### 5. Performance and Error Handling

#### 5.1 Connection Management
```c
// Monitor client connections and cleanup inactive ones
static void* ws_connection_monitor_thread(void* arg) {
    while (g_ws_server.server_running) {
        pthread_mutex_lock(&g_ws_server.clients_mutex);

        for (int i = 0; i < WS_MAX_CLIENTS; i++) {
            ws_client_t* client = &g_ws_server.clients[i];

            if (client->client_fd != -1) {
                // Check if client is still connected
                if (!is_socket_connected(client->client_fd)) {
                    ws_cleanup_client(client);
                    continue;
                }

                // Send periodic ping
                time_t now = time(NULL);
                if (now - client->last_ping > WS_PING_INTERVAL) {
                    ws_send_ping(client->client_fd);
                    client->last_ping = now;
                }
            }
        }

        pthread_mutex_unlock(&g_ws_server.clients_mutex);

        sleep(WS_MONITOR_INTERVAL);
    }

    return NULL;
}
```

#### 5.2 Frame Rate Adaptation
```c
// Adapt frame rate based on client processing speed
static void adapt_streaming_rate(ws_client_t* client) {
    static time_t last_check = 0;
    time_t now = time(NULL);

    if (now - last_check >= 5) { // Check every 5 seconds
        // Calculate actual frame rate
        double actual_fps = (double)client->frames_sent /
                           difftime(now, client->connect_time);

        // Monitor send buffer
        int send_buffer_size;
        socklen_t size = sizeof(send_buffer_size);
        getsockopt(client->client_fd, SOL_SOCKET, SO_SNDBUF,
                  &send_buffer_size, &size);

        // Adapt based on buffer usage and frame rate
        if (send_buffer_size > BUFFER_HIGH_THRESHOLD || actual_fps < TARGET_FPS * 0.8) {
            // Reduce quality or frame rate
            client->target_fps = max(client->target_fps - 2, MIN_FPS);
        } else if (send_buffer_size < BUFFER_LOW_THRESHOLD && actual_fps >= TARGET_FPS * 0.95) {
            // Increase quality or frame rate
            client->target_fps = min(client->target_fps + 1, MAX_FPS);
        }

        last_check = now;
    }
}
```

## Integration with HTTP Server

### Add WebSocket Endpoint
```c
// Add to http_server.c route handling
static int route_websocket_request(const http_request_t* request,
                                 char* response, size_t response_size) {
    // Check for WebSocket upgrade headers
    const char* upgrade = get_header_value(request->headers, "Upgrade");
    const char* connection = get_header_value(request->headers, "Connection");

    if (upgrade && strcasecmp(upgrade, "websocket") == 0 &&
        connection && strstr(connection, "Upgrade")) {

        // Handle WebSocket upgrade
        return ws_handle_upgrade(request);
    }

    return create_http_error_response(response, response_size, 400, "Bad Request");
}
```

## Testing Strategy

### 1. Connection Testing
```bash
# Test WebSocket connection
wscat -c ws://camera-ip:8081/ws/stream

# Test with different browsers
# Chrome DevTools -> Network -> WS tab
# Firefox DevTools -> Network -> WS tab
```

### 2. Performance Testing
```javascript
// Performance monitoring client-side
class WebSocketPerformanceMonitor {
    constructor(player) {
        this.player = player;
        this.frameCount = 0;
        this.startTime = Date.now();
        this.lastFrameTime = 0;

        setInterval(() => this.reportStats(), 5000);
    }

    onFrameReceived() {
        this.frameCount++;
        this.lastFrameTime = Date.now();
    }

    reportStats() {
        const elapsed = (Date.now() - this.startTime) / 1000;
        const fps = this.frameCount / elapsed;
        const latency = Date.now() - this.lastFrameTime;

        console.log(`FPS: ${fps.toFixed(2)}, Latency: ${latency}ms`);
    }
}
```

## Implementation Checklist

- [ ] Implement WebSocket server module with client management
- [ ] Add WebSocket upgrade handling to HTTP server
- [ ] Create H.264 frame extraction and NAL unit packaging
- [ ] Implement binary frame transmission over WebSocket
- [ ] Add PTZ control message handling
- [ ] Create client-side JavaScript WebSocket video player
- [ ] Integrate H.264 decoder (Broadway.js or similar)
- [ ] Implement PTZ control interface
- [ ] Add connection monitoring and error recovery
- [ ] Test real-time streaming performance and latency

## Success Criteria

- **Low Latency**: < 500ms end-to-end latency for video streaming
- **Real-time Control**: PTZ commands respond within 100ms
- **Browser Support**: Works in Chrome, Firefox, Safari, Edge (modern versions)
- **Connection Stability**: Maintains connection for 1+ hour continuous streaming
- **Performance**: < 20% CPU overhead compared to RTSP streaming

## Next Steps

After implementing WebSocket streaming, proceed to [04_phase3_hls.md](04_phase3_hls.md) for adaptive bitrate streaming with HLS.

## Related Documentation

- **Previous**: [02_phase1_mjpeg.md](02_phase1_mjpeg.md) - Foundation MJPEG streaming
- **Next**: [04_phase3_hls.md](04_phase3_hls.md) - HLS adaptive streaming
- **Platform**: [06_integration_platform.md](06_integration_platform.md) - Platform integration details
- **Templates**: [templates/websocket_player.js](templates/websocket_player.js) - Client implementation