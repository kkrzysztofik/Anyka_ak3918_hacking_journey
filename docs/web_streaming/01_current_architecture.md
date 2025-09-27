# Current Architecture and Browser Challenges

## Existing Infrastructure

### Video Pipeline Components
- **RTSP Streaming**: Port 554 with H.264 encoding
  - Main stream: 1280x720 resolution
  - Sub stream: 640x360 resolution
- **HTTP Server**: Port 8080 serving ONVIF SOAP services
- **Snapshot Service**: Port 3000 providing JPEG snapshots
- **Platform Support**: Anyka AK3918 with hardware H.264 encoding

### Hardware Capabilities
- **Video Input (VI)**: Camera sensor interface
- **Video Processing (VPSS)**: Scaling, cropping, rotation
- **Video Encoding (VENC)**: Hardware-accelerated H.264 encoding
- **Stream Management**: Multi-stream handling with different resolutions

### Current Video Flow
```
Camera Sensor → VI → VPSS → VENC → RTSP Server → Network
                           ↓
                     JPEG Encoder → Snapshot Service
```

### Existing Platform APIs
```c
// Video Input
int platform_vi_start(platform_vi_handle_t handle);
int platform_vi_get_frame(platform_vi_handle_t handle, platform_frame_t* frame);

// Video Processing
int platform_vpss_start(platform_vpss_handle_t handle);
int platform_vpss_process_frame(platform_vpss_handle_t handle, platform_frame_t* frame);

// Video Encoding
int platform_venc_start(platform_venc_handle_t handle);
int platform_venc_encode_frame(platform_venc_handle_t handle, platform_frame_t* frame);
int platform_venc_get_stream(platform_venc_handle_t handle, platform_stream_t* stream);
```

## Browser Compatibility Challenges

### RTSP Protocol Limitations
- **No Native Support**: Browsers don't support RTSP protocol natively
- **Plugin Dependencies**: Requires browser plugins (deprecated/unsupported)
- **Security Issues**: RTSP over HTTP tunneling has security concerns
- **Firewall Problems**: RTSP uses multiple ports, complicating NAT traversal

### H.264 Container Format Issues
- **Raw H.264**: Browsers can't play raw H.264 streams
- **Container Required**: Need proper container (MP4, WebM, TS)
- **Fragmentation**: Browser support varies by container format
- **Codec Profiles**: Different browsers support different H.264 profiles

### Real-time Streaming Challenges
- **Latency Requirements**: Real-time applications need <500ms latency
- **Buffering Issues**: Standard video tags introduce buffering delays
- **Browser Differences**: Each browser handles streaming differently
- **Mobile Limitations**: Mobile browsers have additional restrictions

### Cross-Browser Support Matrix

| Protocol | Chrome | Firefox | Safari | Edge | Mobile | Support % |
|----------|--------|---------|--------|------|--------|-----------|
| RTSP | ❌ | ❌ | ❌ | ❌ | ❌ | 0% |
| MJPEG | ✅ | ✅ | ✅ | ✅ | ✅ | 100% |
| WebSocket | ✅ | ✅ | ✅ | ✅ | ✅ | 95% |
| HLS | Plugin | Plugin | ✅ | Plugin | ✅ | 90% |
| WebRTC | ✅ | ✅ | ✅ | ✅ | ✅ | 98% |

## Technical Constraints

### Hardware Limitations
- **CPU Resources**: Anyka AK3918 has limited CPU for transcoding
- **Memory Constraints**: Limited RAM for frame buffering
- **Encoding Limitations**: Hardware encoder optimized for H.264
- **Concurrent Streams**: Limited number of simultaneous encoders

### Network Considerations
- **Bandwidth**: Multiple streaming protocols increase bandwidth usage
- **Port Management**: Need to manage multiple streaming ports
- **NAT Traversal**: Some protocols require complex NAT handling
- **Quality vs Bandwidth**: Balance between quality and network usage

### ONVIF Compliance Requirements
- **Profile S Compatibility**: Must maintain existing ONVIF Profile S support
- **GetStreamUri**: Need to extend for browser-compatible URIs
- **Media Service**: Extend without breaking existing functionality
- **Authentication**: Maintain ONVIF authentication mechanisms

## Gap Analysis

### Missing Components for Browser Support

1. **MJPEG Streaming Server**
   - Current: Only JPEG snapshots available
   - Needed: Continuous MJPEG stream with multipart HTTP response
   - Implementation: HTTP server extension

2. **WebSocket Server**
   - Current: No WebSocket support
   - Needed: WebSocket upgrade handling and frame streaming
   - Implementation: New WebSocket server module

3. **HLS Segment Generator**
   - Current: Only live H.264 stream available
   - Needed: MP4 segment creation and M3U8 playlist generation
   - Implementation: New HLS server module

4. **Browser-Compatible Stream URIs**
   - Current: Only RTSP URIs provided
   - Needed: HTTP-based streaming URIs for browsers
   - Implementation: Media service extensions

### Required Platform Extensions

```c
// New functions needed in platform layer
int platform_get_jpeg_frame(void* stream_handle, uint8_t* buffer,
                           size_t buffer_size, size_t* frame_size);
int platform_configure_jpeg_encoder(int quality, int width, int height);
int platform_create_mp4_segment(const uint8_t* h264_data, size_t data_size,
                               const char* output_path);
int platform_get_h264_nal_units(void* stream_handle, uint8_t* buffer,
                               size_t buffer_size, size_t* units_size);
```

## Architecture Impact

### HTTP Server Extensions Required
```c
// New endpoints to add
GET /stream.mjpeg              // MJPEG streaming
GET /ws/stream                 // WebSocket upgrade
GET /hls/playlist.m3u8         // HLS playlist
GET /hls/segment_N.ts          // HLS segments
GET /webrtc/signaling          // WebRTC signaling
```

### Media Service Extensions Required
```c
// New ONVIF operations to implement
int onvif_media_get_browser_stream_uri(const char* profile_token,
                                     const char* protocol,
                                     struct stream_uri* uri);
int onvif_media_get_supported_protocols(char* protocols[], int max_count);
int onvif_media_configure_streaming(const char* profile_token,
                                   const streaming_config_t* config);
```

### Configuration Extensions Required
```c
// New configuration options
typedef struct {
    bool mjpeg_enabled;
    int mjpeg_quality;          // 1-100
    int mjpeg_frame_rate;       // 1-30
    bool websocket_enabled;
    int websocket_port;
    bool hls_enabled;
    int hls_segment_duration;   // seconds
    int hls_max_segments;
    bool webrtc_enabled;
    int webrtc_signaling_port;
} browser_streaming_config_t;
```

## Performance Implications

### CPU Overhead Estimates
- **MJPEG**: 5-10% additional CPU for JPEG encoding
- **WebSocket**: 10-15% for frame packaging and WebSocket protocol
- **HLS**: 15-20% for MP4 segment creation and playlist management
- **WebRTC**: 20-25% for RTP packaging and signaling

### Memory Requirements
- **MJPEG**: Minimal - single frame buffering
- **WebSocket**: Low - frame queue for smooth streaming
- **HLS**: Medium - segment storage (10 segments × 10 seconds)
- **WebRTC**: Medium - RTP buffers and RTCP handling

### Network Bandwidth Impact
- **MJPEG**: 1-5 Mbps depending on quality and frame rate
- **WebSocket**: 2-8 Mbps similar to RTSP
- **HLS**: 3-10 Mbps with adaptive bitrate
- **WebRTC**: 2-8 Mbps with adaptive bitrate control

## Solution Strategy

### Incremental Implementation Approach
1. **Start Simple**: MJPEG provides immediate browser compatibility
2. **Add Capabilities**: WebSocket enables real-time streaming
3. **Enhance Experience**: HLS adds adaptive streaming
4. **Advanced Features**: WebRTC for peer-to-peer communication

### Leverage Existing Infrastructure
- **Reuse Video Pipeline**: Use existing VI/VPSS/VENC components
- **Extend HTTP Server**: Add streaming endpoints to existing server
- **Maintain ONVIF**: Keep existing ONVIF functionality intact
- **Platform Integration**: Build on existing platform abstraction

## Next Steps

Understanding the current architecture and browser challenges leads to the implementation strategy outlined in subsequent modules:

1. **Phase 1**: [02_phase1_mjpeg.md](02_phase1_mjpeg.md) - Universal browser support
2. **Phase 2**: [03_phase2_websocket.md](03_phase2_websocket.md) - Real-time streaming
3. **Phase 3**: [04_phase3_hls.md](04_phase3_hls.md) - Adaptive streaming
4. **Phase 4**: [05_phase4_webrtc.md](05_phase4_webrtc.md) - Advanced features

## Related Documentation

- **Overview**: [00_overview.md](00_overview.md) - Implementation strategy and timeline
- **Platform Integration**: [06_integration_platform.md](06_integration_platform.md) - Detailed platform APIs
- **Templates**: [templates/](templates/) - Code implementation patterns