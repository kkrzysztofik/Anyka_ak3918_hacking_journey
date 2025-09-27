# Web Streaming Implementation Overview

## Purpose

This modular plan outlines implementing browser-compatible video streaming for the Anyka AK3918 ONVIF camera. The original RTSP-only streaming will be extended to support web browsers through multiple protocols while maintaining ONVIF compliance.

## Module Structure

- **00_overview.md** - This file - high-level goals and navigation
- **01_current_architecture.md** - Existing infrastructure and browser challenges
- **02_phase1_mjpeg.md** - MJPEG streaming implementation (universal browser support)
- **03_phase2_websocket.md** - WebSocket streaming for real-time video
- **04_phase3_hls.md** - HTTP Live Streaming for adaptive bitrate
- **05_phase4_webrtc.md** - WebRTC for ultra-low latency (advanced)
- **06_integration_platform.md** - Integration with existing Anyka platform
- **07_security_performance.md** - Security considerations and performance targets
- **templates/** - Working code templates for implementation

## Implementation Strategy

### Phased Approach
1. **Phase 1: MJPEG** - Foundation with universal browser support (100%)
2. **Phase 2: WebSocket** - Real-time streaming for modern browsers (95%)
3. **Phase 3: HLS** - Adaptive streaming with mobile support (90%)
4. **Phase 4: WebRTC** - Ultra-low latency peer-to-peer (98%)

### Key Benefits
- **Universal Compatibility**: MJPEG works in all browsers
- **Low Latency Options**: WebSocket and WebRTC for real-time needs
- **Adaptive Streaming**: HLS handles varying network conditions
- **ONVIF Compliance**: Maintains existing ONVIF functionality
- **Hardware Acceleration**: Leverages existing Anyka video pipeline

## Quick Start for Agents

1. **Understand Current State**: Read [01_current_architecture.md](01_current_architecture.md)
2. **Start with MJPEG**: Implement [02_phase1_mjpeg.md](02_phase1_mjpeg.md) first
3. **Add Real-time**: Follow [03_phase2_websocket.md](03_phase2_websocket.md)
4. **Enable Adaptive**: Implement [04_phase3_hls.md](04_phase3_hls.md)
5. **Advanced Features**: Consider [05_phase4_webrtc.md](05_phase4_webrtc.md)
6. **Platform Integration**: Review [06_integration_platform.md](06_integration_platform.md)
7. **Security & Performance**: Follow [07_security_performance.md](07_security_performance.md)

## Success Criteria

### Technical Requirements
- **Latency**: < 500ms for WebSocket, < 2s for HLS
- **Performance**: < 20% CPU overhead for streaming
- **Quality**: Maintain video quality across all protocols
- **Compatibility**: 95%+ browser support

### Implementation Checkpoints
- **Phase 1 Complete**: MJPEG streaming works in all browsers
- **Phase 2 Complete**: WebSocket streaming with PTZ control
- **Phase 3 Complete**: HLS streaming with adaptive bitrate
- **Phase 4 Complete**: WebRTC with NAT traversal

## File Structure Impact

### New Components
```
src/networking/websocket/     # WebSocket streaming server
src/networking/hls/           # HLS segment generation
src/networking/webrtc/        # WebRTC signaling (optional)
src/web/                      # Web interface files
```

### Modified Components
```
src/networking/http/          # Extended for streaming endpoints
src/services/media/           # Browser streaming URI support
src/platform/                 # JPEG frame extraction
src/common/                   # New streaming constants
```

## Development Timeline

- **Phase 1 (MJPEG)**: 2-3 weeks - Foundation streaming
- **Phase 2 (WebSocket)**: 3-4 weeks - Real-time streaming
- **Phase 3 (HLS)**: 4-5 weeks - Adaptive streaming
- **Phase 4 (WebRTC)**: 6-8 weeks - Advanced features

## Agent Context Management

Each module is designed to fit within agent context limits. Modules can be processed independently while maintaining comprehensive understanding through cross-references.

## Next Steps

Begin with [01_current_architecture.md](01_current_architecture.md) to understand the existing infrastructure and browser compatibility challenges that drive the implementation approach.