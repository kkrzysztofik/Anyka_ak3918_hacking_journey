# Phase 4: WebRTC Implementation (Advanced)

## Overview

**Priority**: Low | **Complexity**: Very High | **Browser Support**: Excellent (98%)

WebRTC provides ultra-low latency peer-to-peer streaming with built-in NAT traversal, adaptive bitrate, and real-time communication capabilities. This is the most advanced streaming option.

## Technical Approach

### Core Concept
Establish direct peer-to-peer RTP connections between camera and browsers using WebRTC signaling. Bypasses server-side streaming bottlenecks for minimal latency.

### Implementation Strategy
1. **Signaling Server**: WebSocket-based SDP/ICE exchange
2. **Media Pipeline**: RTP streaming integration
3. **NAT Traversal**: STUN/TURN server support
4. **Codec Negotiation**: H.264/VP8/VP9 support

## Implementation Details

### 1. WebRTC Signaling Server

```c
// New file: src/networking/webrtc/webrtc_server.h
typedef struct {
    char session_id[64];
    int client_fd;
    char sdp_offer[4096];
    char sdp_answer[4096];
    char ice_candidates[2048];
    bool media_active;
    int rtp_port;
    int rtcp_port;
    time_t created_time;
} webrtc_session_t;

// Core functions
int webrtc_server_init(int signaling_port);
int webrtc_handle_offer(const char* session_id, const char* sdp_offer, char* sdp_answer);
int webrtc_add_ice_candidate(const char* session_id, const char* candidate);
int webrtc_start_media_stream(const char* session_id, const char* profile_token);
```

### 2. SDP Generation

```c
// Generate SDP answer for WebRTC offer
static int generate_sdp_answer(const char* offer, char* answer, size_t answer_size) {
    // Parse offer SDP
    sdp_session_t offer_session;
    if (parse_sdp(offer, &offer_session) != 0) {
        return -1;
    }

    // Create answer SDP
    int written = snprintf(answer, answer_size,
        "v=0\r\n"
        "o=- %llu 2 IN IP4 %s\r\n"
        "s=-\r\n"
        "t=0 0\r\n"
        "a=group:BUNDLE 0\r\n"
        "a=msid-semantic: WMS\r\n",
        (unsigned long long)time(NULL),
        get_device_ip());

    // Add video media description
    written += snprintf(answer + written, answer_size - written,
        "m=video %d RTP/SAVPF 96\r\n"
        "c=IN IP4 %s\r\n"
        "a=rtcp:%d IN IP4 %s\r\n"
        "a=ice-ufrag:%s\r\n"
        "a=ice-pwd:%s\r\n"
        "a=fingerprint:sha-256 %s\r\n"
        "a=setup:active\r\n"
        "a=mid:0\r\n"
        "a=rtcp-mux\r\n"
        "a=rtpmap:96 H264/90000\r\n"
        "a=fmtp:96 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f\r\n",
        get_rtp_port(),
        get_device_ip(),
        get_rtcp_port(),
        get_device_ip(),
        generate_ice_ufrag(),
        generate_ice_pwd(),
        get_dtls_fingerprint());

    return written;
}
```

### 3. RTP Media Pipeline

```c
// RTP packet generation from H.264 stream
static int webrtc_stream_h264_rtp(webrtc_session_t* session, const char* profile_token) {
    uint8_t h264_buffer[65536];
    size_t frame_size;
    uint32_t timestamp = 0;
    uint16_t sequence = 0;

    while (session->media_active) {
        // Get H.264 frame
        if (platform_get_h264_frame(profile_token, h264_buffer,
                                   sizeof(h264_buffer), &frame_size) == 0) {

            // Fragment H.264 frame into RTP packets
            int packets_sent = fragment_h264_to_rtp(session, h264_buffer,
                                                  frame_size, timestamp, &sequence);

            timestamp += 3000; // 90kHz clock, ~30fps
        }

        usleep(33333); // ~30 FPS
    }

    return 0;
}

// H.264 RTP packetization
static int fragment_h264_to_rtp(webrtc_session_t* session, const uint8_t* h264_data,
                               size_t data_size, uint32_t timestamp, uint16_t* sequence) {
    const int MAX_RTP_PAYLOAD = 1200; // MTU consideration
    uint8_t rtp_packet[1500];
    int packets_sent = 0;

    size_t offset = 0;
    while (offset < data_size) {
        size_t payload_size = min(data_size - offset, MAX_RTP_PAYLOAD);
        bool is_last_packet = (offset + payload_size >= data_size);

        // Create RTP header
        create_rtp_header(rtp_packet, *sequence, timestamp, 96, is_last_packet);

        // Add H.264 payload
        memcpy(rtp_packet + RTP_HEADER_SIZE, h264_data + offset, payload_size);

        // Send RTP packet
        if (send_rtp_packet(session, rtp_packet, RTP_HEADER_SIZE + payload_size) == 0) {
            packets_sent++;
        }

        offset += payload_size;
        (*sequence)++;
    }

    return packets_sent;
}
```

## Client-Side WebRTC Implementation

### JavaScript WebRTC Client

```javascript
class WebRTCVideoPlayer {
    constructor(signalingUrl, videoElement) {
        this.signalingUrl = signalingUrl;
        this.video = videoElement;
        this.pc = null;
        this.ws = null;
        this.sessionId = this.generateSessionId();

        this.initSignaling();
    }

    async initSignaling() {
        this.ws = new WebSocket(this.signalingUrl);

        this.ws.onopen = () => {
            console.log('WebRTC signaling connected');
            this.startCall();
        };

        this.ws.onmessage = (event) => {
            const message = JSON.parse(event.data);
            this.handleSignalingMessage(message);
        };
    }

    async startCall() {
        // Create RTCPeerConnection
        this.pc = new RTCPeerConnection({
            iceServers: [
                { urls: 'stun:stun.l.google.com:19302' }
            ]
        });

        // Handle incoming stream
        this.pc.ontrack = (event) => {
            this.video.srcObject = event.streams[0];
        };

        // Handle ICE candidates
        this.pc.onicecandidate = (event) => {
            if (event.candidate) {
                this.sendSignalingMessage({
                    type: 'ice-candidate',
                    candidate: event.candidate
                });
            }
        };

        // Create offer
        const offer = await this.pc.createOffer({
            offerToReceiveVideo: true,
            offerToReceiveAudio: false
        });

        await this.pc.setLocalDescription(offer);

        // Send offer to signaling server
        this.sendSignalingMessage({
            type: 'offer',
            sdp: offer.sdp
        });
    }

    async handleSignalingMessage(message) {
        switch (message.type) {
            case 'answer':
                await this.pc.setRemoteDescription({
                    type: 'answer',
                    sdp: message.sdp
                });
                break;

            case 'ice-candidate':
                await this.pc.addIceCandidate(message.candidate);
                break;

            case 'error':
                console.error('WebRTC signaling error:', message.error);
                break;
        }
    }

    sendSignalingMessage(message) {
        message.sessionId = this.sessionId;
        this.ws.send(JSON.stringify(message));
    }

    generateSessionId() {
        return 'session_' + Math.random().toString(36).substr(2, 9);
    }

    close() {
        if (this.pc) {
            this.pc.close();
        }
        if (this.ws) {
            this.ws.close();
        }
    }
}
```

### STUN/TURN Server Configuration

```c
// ICE server configuration
typedef struct {
    char stun_server[256];
    int stun_port;
    char turn_server[256];
    int turn_port;
    char turn_username[64];
    char turn_password[64];
} ice_config_t;

static ice_config_t g_ice_config = {
    .stun_server = "stun.l.google.com",
    .stun_port = 19302,
    .turn_server = "",
    .turn_port = 3478,
    .turn_username = "",
    .turn_password = ""
};
```

## Performance and Latency Optimization

### Adaptive Bitrate Control

```c
// Monitor RTP statistics and adjust bitrate
static void monitor_webrtc_quality(webrtc_session_t* session) {
    rtcp_stats_t stats;
    if (get_rtcp_statistics(session, &stats) == 0) {
        // Adjust based on packet loss
        if (stats.packet_loss_percent > 5.0) {
            reduce_video_bitrate(session, 0.8); // Reduce by 20%
        } else if (stats.packet_loss_percent < 1.0) {
            increase_video_bitrate(session, 1.1); // Increase by 10%
        }

        // Adjust based on RTT
        if (stats.round_trip_time > 200) {
            reduce_frame_rate(session, 0.9);
        }
    }
}
```

## Implementation Challenges

### 1. DTLS/SRTP Implementation
- **Requires**: OpenSSL or similar for encryption
- **Complexity**: Certificate generation and key exchange
- **Performance**: Hardware acceleration for encryption

### 2. ICE/STUN/TURN Support
- **NAT Traversal**: Complex network topology handling
- **TURN Server**: May require external TURN server
- **Firewall**: Corporate firewall compatibility

### 3. Codec Negotiation
- **Multiple Codecs**: H.264, VP8, VP9 support
- **Profile Levels**: Different H.264 profiles
- **Hardware Acceleration**: Leverage Anyka encoding

## Testing Strategy

### 1. Network Scenarios
```bash
# Test behind NAT
# Test symmetric NAT
# Test firewall restrictions
# Test bandwidth limitations
```

### 2. Browser Compatibility
- Chrome/Chromium: Full WebRTC support
- Firefox: Full WebRTC support
- Safari: WebRTC support with limitations
- Edge: Full WebRTC support

### 3. Performance Metrics
- **Latency**: Target < 200ms end-to-end
- **Packet Loss**: Handle up to 5% gracefully
- **Jitter**: Maintain smooth playback
- **CPU Usage**: < 25% additional overhead

## Implementation Checklist

- [ ] Implement WebRTC signaling server
- [ ] Add SDP offer/answer generation
- [ ] Create RTP media pipeline integration
- [ ] Implement H.264 RTP packetization
- [ ] Add ICE candidate handling
- [ ] Create DTLS/SRTP encryption layer
- [ ] Implement client-side WebRTC JavaScript
- [ ] Add adaptive bitrate control
- [ ] Test NAT traversal scenarios
- [ ] Verify ultra-low latency performance

## Success Criteria

- **Ultra-Low Latency**: < 200ms end-to-end latency
- **NAT Traversal**: Works behind most NAT configurations
- **Adaptive Quality**: Automatically adjusts to network conditions
- **Browser Support**: Works in 98% of modern browsers
- **P2P Performance**: Direct connection when possible

## Implementation Priority

**Note**: WebRTC implementation is complex and optional. Consider implementing only if:
- Ultra-low latency is critical requirement
- Advanced real-time communication features needed
- Team has WebRTC/RTP expertise
- STUN/TURN infrastructure available

## Next Steps

After evaluating WebRTC needs, proceed to [06_integration_platform.md](06_integration_platform.md) for detailed platform integration requirements.

## Related Documentation

- **Previous**: [04_phase3_hls.md](04_phase3_hls.md) - HLS adaptive streaming
- **Next**: [06_integration_platform.md](06_integration_platform.md) - Platform integration
- **Security**: [07_security_performance.md](07_security_performance.md) - Security considerations
- **Templates**: [templates/webrtc_player.js](templates/webrtc_player.js) - Client implementation