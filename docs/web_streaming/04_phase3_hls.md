# Phase 3: HTTP Live Streaming (HLS) Implementation

## Overview

**Priority**: Medium | **Complexity**: High | **Browser Support**: Safari native, others via libraries (90%)

HLS provides adaptive bitrate streaming by creating MP4 segments from H.264 streams and generating M3U8 playlists. This enables high-quality streaming with automatic quality adjustment based on network conditions.

## Technical Approach

### Core Concept
Convert continuous H.264 stream into discrete MP4 segments (10-second duration) with accompanying M3U8 playlist files. Browsers use HLS.js or native HLS support to adaptively stream content.

### Implementation Strategy
1. **Segment Generation**: Create MP4 segments from H.264 stream
2. **Playlist Management**: Generate and update M3U8 playlists
3. **Adaptive Bitrates**: Multiple quality levels with automatic switching
4. **Storage Management**: Segment caching and cleanup

## Implementation Details

### 1. HLS Server Module

```c
// New file: src/networking/hls/hls_server.h
typedef struct {
    char segment_dir[256];
    char playlist_path[256];
    int segment_duration;        // seconds
    int max_segments;           // sliding window
    int target_bitrate;         // kbps
    bool cleanup_enabled;
} hls_config_t;

typedef struct {
    char filename[64];
    char filepath[256];
    uint64_t timestamp;
    int duration_ms;
    int sequence_number;
    size_t file_size;
    bool available;
} hls_segment_t;

// Function declarations
int hls_server_init(const hls_config_t* config);
int hls_create_segment(const uint8_t* h264_data, size_t data_size);
int hls_update_playlist(void);
int hls_cleanup_old_segments(void);
int hls_get_playlist_content(char* buffer, size_t buffer_size);
```

### 2. Segment Creation

```c
// Segment generation from H.264 stream
static int create_mp4_segment(const uint8_t* h264_data, size_t data_size,
                             const char* output_path) {
    // Use FFmpeg libraries or custom MP4 muxer
    AVFormatContext* fmt_ctx = avformat_alloc_context();
    AVOutputFormat* output_fmt = av_guess_format("mp4", NULL, NULL);

    // Configure MP4 container
    fmt_ctx->oformat = output_fmt;

    // Add H.264 video stream
    AVStream* video_stream = avformat_new_stream(fmt_ctx, NULL);
    video_stream->codecpar->codec_type = AVMEDIA_TYPE_VIDEO;
    video_stream->codecpar->codec_id = AV_CODEC_ID_H264;

    // Write segment
    avformat_write_header(fmt_ctx, NULL);

    // Process H.264 NAL units into MP4 packets
    process_h264_to_mp4(fmt_ctx, video_stream, h264_data, data_size);

    av_write_trailer(fmt_ctx);
    avformat_free_context(fmt_ctx);

    return 0;
}

// M3U8 playlist generation
static int generate_m3u8_playlist(char* playlist_content, size_t buffer_size) {
    hls_segment_t segments[MAX_SEGMENTS];
    int segment_count = get_available_segments(segments, MAX_SEGMENTS);

    int written = snprintf(playlist_content, buffer_size,
        "#EXTM3U\n"
        "#EXT-X-VERSION:3\n"
        "#EXT-X-TARGETDURATION:%d\n"
        "#EXT-X-MEDIA-SEQUENCE:%d\n",
        g_hls_config.segment_duration,
        segments[0].sequence_number);

    // Add segment entries
    for (int i = 0; i < segment_count; i++) {
        written += snprintf(playlist_content + written, buffer_size - written,
            "#EXTINF:%.3f,\n%s\n",
            segments[i].duration_ms / 1000.0,
            segments[i].filename);
    }

    return written;
}
```

### 3. HTTP Integration

```c
// Add HLS endpoints to HTTP server
static int handle_hls_playlist_request(const http_request_t* request,
                                     char* response, size_t response_size) {
    char playlist_content[4096];

    if (hls_get_playlist_content(playlist_content, sizeof(playlist_content)) > 0) {
        return create_http_response(response, response_size, 200,
                                  "application/x-mpegURL", playlist_content);
    }

    return create_http_error_response(response, response_size, 404, "Playlist not found");
}

static int handle_hls_segment_request(const http_request_t* request,
                                    char* response, size_t response_size) {
    // Extract segment filename from URI
    const char* segment_name = extract_segment_name(request->uri);

    // Serve segment file
    return serve_file_response(response, response_size, segment_name, "video/mp4");
}
```

## Client-Side Implementation

### Browser HLS Player

```html
<!-- HLS Video Player -->
<video id="hlsPlayer" controls width="1280" height="720">
    <source src="/hls/playlist.m3u8" type="application/x-mpegURL">
</video>

<script src="https://cdn.jsdelivr.net/npm/hls.js@latest"></script>
<script>
class HLSVideoPlayer {
    constructor(videoElement, playlistUrl) {
        this.video = videoElement;
        this.playlistUrl = playlistUrl;
        this.hls = null;
        this.isSupported = this.checkSupport();

        this.init();
    }

    checkSupport() {
        return Hls.isSupported() || this.video.canPlayType('application/vnd.apple.mpegurl');
    }

    init() {
        if (this.video.canPlayType('application/vnd.apple.mpegurl')) {
            // Native HLS support (Safari)
            this.video.src = this.playlistUrl;
        } else if (Hls.isSupported()) {
            // HLS.js for other browsers
            this.hls = new Hls({
                maxBufferLength: 30,
                maxMaxBufferLength: 60,
                enableWorker: true
            });

            this.hls.loadSource(this.playlistUrl);
            this.hls.attachMedia(this.video);

            this.hls.on(Hls.Events.MANIFEST_PARSED, () => {
                console.log('HLS manifest parsed, starting playback');
            });

            this.hls.on(Hls.Events.ERROR, (event, data) => {
                console.error('HLS error:', data);
                if (data.fatal) {
                    this.handleFatalError();
                }
            });
        }
    }

    handleFatalError() {
        console.log('Attempting HLS recovery...');
        if (this.hls) {
            this.hls.destroy();
            setTimeout(() => this.init(), 5000);
        }
    }
}

// Initialize player
const player = new HLSVideoPlayer(
    document.getElementById('hlsPlayer'),
    '/hls/playlist.m3u8'
);
</script>
```

## Performance Considerations

### Segment Management
- **Sliding Window**: Keep only last N segments in memory
- **Background Cleanup**: Periodic old segment removal
- **Disk Space Monitoring**: Prevent storage exhaustion

### Quality Adaptation
- **Multiple Bitrates**: Generate segments at different qualities
- **Network Monitoring**: Adjust based on client download speed
- **CPU Load Balancing**: Reduce quality under high system load

## Implementation Checklist

- [ ] Create HLS server module with segment management
- [ ] Implement MP4 segment generation from H.264 streams
- [ ] Add M3U8 playlist generation and updates
- [ ] Integrate HLS endpoints in HTTP server
- [ ] Add segment cleanup and storage management
- [ ] Create client-side HLS player with HLS.js integration
- [ ] Test adaptive bitrate switching
- [ ] Verify cross-browser compatibility

## Success Criteria

- **Adaptive Streaming**: Automatic quality switching based on bandwidth
- **Browser Support**: Works with HLS.js in 90%+ of browsers
- **Storage Management**: Maintains reasonable disk usage (< 100MB)
- **Seek Support**: Allows seeking within available segments
- **Mobile Compatibility**: Works on iOS and Android browsers

## Next Steps

After implementing HLS, proceed to [05_phase4_webrtc.md](05_phase4_webrtc.md) for ultra-low latency WebRTC streaming.

## Related Documentation

- **Previous**: [03_phase2_websocket.md](03_phase2_websocket.md) - WebSocket real-time streaming
- **Next**: [05_phase4_webrtc.md](05_phase4_webrtc.md) - WebRTC peer-to-peer streaming
- **Platform**: [06_integration_platform.md](06_integration_platform.md) - Platform integration
- **Templates**: [templates/hls_player.js](templates/hls_player.js) - Client implementation