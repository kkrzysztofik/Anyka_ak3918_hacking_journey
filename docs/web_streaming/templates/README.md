# Web Streaming Implementation Templates

This directory contains working code templates for implementing browser-compatible video streaming for the Anyka AK3918 ONVIF camera.

## Available Templates

### Client-Side JavaScript Templates

- **[websocket_player.js](websocket_player.js)** - Complete WebSocket H.264 video player with:
  - H.264 decoding using Broadway.js or similar
  - Real-time PTZ control integration
  - Performance monitoring and statistics
  - Automatic reconnection with exponential backoff
  - YUV to RGB conversion for canvas rendering

- **[hls_player.js](hls_player.js)** - HLS video player with adaptive bitrate:
  - Cross-browser HLS support (native Safari + HLS.js fallback)
  - Adaptive bitrate streaming with quality selection
  - Complete player UI with controls
  - Buffer monitoring and error recovery
  - Performance statistics and monitoring

### Server-Side C Templates

- **[mjpeg_server.c](mjpeg_server.c)** - MJPEG streaming server implementation
- **[websocket_server.c](websocket_server.c)** - WebSocket streaming server
- **[hls_server.c](hls_server.c)** - HLS segment generation server
- **[platform_integration.c](platform_integration.c)** - Anyka platform integration

### Web Interface Templates

- **[streaming_interface.html](streaming_interface.html)** - Complete web interface
- **[streaming_styles.css](streaming_styles.css)** - Styling for streaming UI

## Quick Start Guide

### 1. MJPEG Streaming (Universal Support)

```html
<!DOCTYPE html>
<html>
<head>
    <title>MJPEG Stream</title>
</head>
<body>
    <h1>Live Camera Stream</h1>
    <img src="http://192.168.1.100:8080/stream.mjpeg"
         alt="Live Stream"
         style="max-width: 100%; height: auto;">
</body>
</html>
```

### 2. WebSocket Real-time Streaming

```html
<!DOCTYPE html>
<html>
<head>
    <title>WebSocket Stream</title>
    <script src="broadway/Decoder.js"></script>
</head>
<body>
    <canvas id="videoCanvas" width="1280" height="720"></canvas>
    <div id="ptz-controls"></div>

    <script src="websocket_player.js"></script>
    <script>
        const canvas = document.getElementById('videoCanvas');
        const player = new WebSocketVideoPlayer(
            'ws://192.168.1.100:8081/ws/stream',
            canvas
        );

        const ptzController = new WebSocketPTZController(player);

        player.connect();

        const controlsContainer = document.getElementById('ptz-controls');
        ptzController.createControlButtons(controlsContainer);
    </script>
</body>
</html>
```

### 3. HLS Adaptive Streaming

```html
<!DOCTYPE html>
<html>
<head>
    <title>HLS Stream</title>
    <script src="https://cdn.jsdelivr.net/npm/hls.js@latest"></script>
</head>
<body>
    <video id="hlsVideo" controls width="1280" height="720"></video>
    <div id="hlsControls"></div>

    <script src="hls_player.js"></script>
    <script>
        const video = document.getElementById('hlsVideo');
        const controlsContainer = document.getElementById('hlsControls');

        const player = new HLSVideoPlayer(
            video,
            'http://192.168.1.100:8080/hls/playlist.m3u8'
        );

        const ui = new HLSPlayerUI(player, controlsContainer);

        player.init();
    </script>
</body>
</html>
```

## Template Customization

### Modifying WebSocket Player

```javascript
// Custom event handlers
player.onConnectionStatusChanged = (connected) => {
    const statusElement = document.getElementById('connection-status');
    statusElement.textContent = connected ? 'Connected' : 'Disconnected';
    statusElement.className = connected ? 'status-connected' : 'status-disconnected';
};

player.onPerformanceUpdate = (stats) => {
    document.getElementById('fps').textContent = stats.averageFPS.toFixed(1);
    document.getElementById('bandwidth').textContent =
        (stats.bytesReceived / 1024 / 1024).toFixed(1) + ' MB';
};

// Custom PTZ commands
ptzController.sendCustomCommand = (command, params) => {
    player.sendPTZCommand(command, params);
};
```

### Customizing HLS Player

```javascript
// Custom quality selection
player.onQualityChanged = (level) => {
    console.log(`Quality: ${level.height}p @ ${level.bitrate}bps`);
    updateQualityIndicator(level);
};

// Custom error handling
player.onError = (message) => {
    showErrorMessage(message);
    // Attempt restart after 5 seconds
    setTimeout(() => player.restart(), 5000);
};

// Custom statistics display
player.onStatsUpdate = (stats) => {
    updateBandwidthGraph(stats.currentBitrate);
    updateBufferIndicator(stats.bufferLength);
};
```

## Integration with ONVIF Services

### Getting Stream URIs from ONVIF

```javascript
class ONVIFStreamManager {
    constructor(deviceUrl, username, password) {
        this.deviceUrl = deviceUrl;
        this.credentials = btoa(`${username}:${password}`);
    }

    async getBrowserStreamURI(profile, protocol) {
        const soapRequest = this.createGetStreamURIRequest(profile, protocol);

        const response = await fetch(`${this.deviceUrl}/onvif/media`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/soap+xml',
                'Authorization': `Basic ${this.credentials}`
            },
            body: soapRequest
        });

        const responseText = await response.text();
        return this.parseStreamURI(responseText);
    }

    createGetStreamURIRequest(profile, protocol) {
        return `<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"
               xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
    <soap:Header/>
    <soap:Body>
        <trt:GetStreamUri>
            <trt:StreamSetup>
                <trt:Stream>RTP-Unicast</trt:Stream>
                <trt:Transport>
                    <trt:Protocol>${protocol}</trt:Protocol>
                </trt:Transport>
            </trt:StreamSetup>
            <trt:ProfileToken>${profile}</trt:ProfileToken>
        </trt:GetStreamUri>
    </soap:Body>
</soap:Envelope>`;
    }
}

// Usage
const onvif = new ONVIFStreamManager(
    'http://192.168.1.100:8080',
    'admin',
    'admin'
);

// Get WebSocket stream URI
const wsUri = await onvif.getBrowserStreamURI('MainProfile', 'WebSocket');
const player = new WebSocketVideoPlayer(wsUri, canvas);
```

## Performance Optimization

### Optimizing WebSocket Player

```javascript
// Reduce canvas updates for better performance
player.displayFrame = debounce(player.displayFrame.bind(player), 33); // ~30fps

// Monitor performance and adapt
setInterval(() => {
    const stats = player.getPerformanceStats();
    if (stats.averageFPS < 15) {
        // Request lower quality or frame rate
        player.sendConfigCommand({
            quality: 'medium',
            framerate: 15
        });
    }
}, 5000);

function debounce(func, delay) {
    let timeoutId;
    return function(...args) {
        clearTimeout(timeoutId);
        timeoutId = setTimeout(() => func.apply(this, args), delay);
    };
}
```

### Optimizing HLS Player

```javascript
// Configure HLS.js for low latency
const hlsConfig = {
    lowLatencyMode: true,
    maxBufferLength: 10,        // Reduce buffer for lower latency
    maxMaxBufferLength: 20,
    liveBackBufferLength: 5,    // Keep less back buffer
    liveSyncDurationCount: 1,   // Sync to live edge
    liveMaxLatencyDurationCount: 3
};

const player = new HLSVideoPlayer(video, playlistUrl, hlsConfig);
```

## Troubleshooting

### Common Issues

1. **WebSocket Connection Fails**
   - Check WebSocket URL format (`ws://` not `http://`)
   - Verify WebSocket server is running on specified port
   - Check firewall settings

2. **H.264 Decoder Not Working**
   - Ensure Broadway.js or equivalent decoder is loaded
   - Check browser H.264 support
   - Verify H.264 profile compatibility

3. **HLS Playback Issues**
   - Check M3U8 playlist accessibility
   - Verify segment files are being generated
   - Ensure HLS.js is loaded for non-Safari browsers

4. **PTZ Commands Not Responding**
   - Verify WebSocket connection is established
   - Check PTZ service configuration on camera
   - Ensure proper authentication

### Debug Helpers

```javascript
// WebSocket debug logging
player.ws.addEventListener('message', (event) => {
    if (event.data instanceof ArrayBuffer) {
        console.log(`Received video frame: ${event.data.byteLength} bytes`);
    } else {
        console.log('Received message:', JSON.parse(event.data));
    }
});

// HLS debug information
player.hls.on(Hls.Events.ERROR, (event, data) => {
    console.log('HLS Error Details:', {
        type: data.type,
        details: data.details,
        fatal: data.fatal,
        reason: data.reason
    });
});
```

## Browser Compatibility

| Feature | Chrome | Firefox | Safari | Edge | Mobile |
|---------|--------|---------|--------|------|--------|
| MJPEG | ✅ | ✅ | ✅ | ✅ | ✅ |
| WebSocket | ✅ | ✅ | ✅ | ✅ | ✅ |
| HLS (native) | ❌ | ❌ | ✅ | ❌ | ✅ (iOS) |
| HLS (HLS.js) | ✅ | ✅ | ✅ | ✅ | ✅ |
| H.264 Decoding | ✅* | ✅* | ✅* | ✅* | ✅* |

*Requires JavaScript decoder library for WebSocket streaming

## Security Considerations

### Authentication Integration

```javascript
// Add authentication headers to streaming requests
class AuthenticatedWebSocketPlayer extends WebSocketVideoPlayer {
    constructor(wsUrl, canvasElement, authToken) {
        super(wsUrl, canvasElement);
        this.authToken = authToken;
    }

    connect() {
        // Add authentication to WebSocket URL
        const authenticatedUrl = `${this.wsUrl}?token=${this.authToken}`;
        this.wsUrl = authenticatedUrl;
        super.connect();
    }
}

// Usage with authentication
const authToken = localStorage.getItem('authToken');
const player = new AuthenticatedWebSocketPlayer(wsUrl, canvas, authToken);
```

### HTTPS/WSS Support

```javascript
// Automatically use secure protocols on HTTPS pages
function getSecureStreamingURL(baseUrl) {
    if (window.location.protocol === 'https:') {
        return baseUrl.replace('ws://', 'wss://').replace('http://', 'https://');
    }
    return baseUrl;
}

const wsUrl = getSecureStreamingURL('ws://192.168.1.100:8081/ws/stream');
const hlsUrl = getSecureStreamingURL('http://192.168.1.100:8080/hls/playlist.m3u8');
```

## Next Steps

1. Choose appropriate streaming protocol based on requirements
2. Customize templates for your specific use case
3. Test across target browsers and devices
4. Implement server-side components following the C templates
5. Integrate with existing ONVIF services
6. Add authentication and security measures
7. Optimize for your specific deployment environment

For detailed implementation guidance, refer to the corresponding phase documentation:
- **Phase 1**: [02_phase1_mjpeg.md](../02_phase1_mjpeg.md)
- **Phase 2**: [03_phase2_websocket.md](../03_phase2_websocket.md)
- **Phase 3**: [04_phase3_hls.md](../04_phase3_hls.md)
- **Phase 4**: [05_phase4_webrtc.md](../05_phase4_webrtc.md)