# Streaming-lib Architectural Refactoring Plan

**Date:** 2026-03-03  
**Status:** Planning  
**Estimated Effort:** 7-10 days  
**Target:** Reduce from 53k to ~30k lines, align architecture with onvif-rust

---

## Executive Summary

streaming-lib is a 53k-line fork of [xiu](https://github.com/harlanc/xiu) with substantial bloat and architectural mismatches compared to onvif-rust. This plan removes ~23k lines of unused code and aligns patterns with the main onvif-rust codebase.

**Key Issues:**
- ✅ Core needed: RTSP server, HTTP-FLV server, H.264/AAC codecs, RTP/RTCP
- ❌ Excessive unused code: RTSP client (2,686 lines), RTMP/WebRTC/HLS abstractions
- ❌ Oversized files: 6 files >1,500 lines (largest: 6,206 lines)
- ❌ Architectural inconsistencies: No traits for testability, global state, different patterns

---

## Critical Files Requiring Attention

### Massive Files (Must Split)

| File | Lines | Problem | Action |
|------|-------|---------|--------|
| `rtsp/session/server_session.rs` | 6,206 | Monolithic session handler | Split into 5 files: protocol.rs, rtp_sender.rs, playback_policy.rs, auth_handler.rs, mod.rs |
| `streamhub/mod.rs` | 2,799 | Stream routing hub with mixed concerns | Split into: hub.rs, transceiver.rs, registry.rs, mod.rs |
| `rtsp/session/client_session.rs` | 2,686 | RTSP client (not needed) | **DELETE ENTIRELY** |
| `container/mpeg4_aac.rs` | 1,930 | AAC muxing/demuxing | Split into aac/demuxer.rs + aac/muxer.rs |
| `rtsp/rtp/rtp_h264.rs` | 1,724 | H.264 RTP packer | Split into h264/packer.rs + h264/depacketizer.rs |
| `httpflv/httpflv.rs` | 1,739 | HTTP-FLV subscriber | Split into subscriber.rs + muxer.rs |

---

## Phase 1: Remove Dead Code (Est. 1 day)

**Goal:** Remove ~9,500 lines of unused code  
**Target:** 53k → 44k lines

### Tasks

1. **Delete RTSP Client (2,686 lines)**
   - Remove: `src/rtsp/session/client_session.rs`
   - Remove export from: `src/lib.rs:28` (`pub use rtsp::session::client_session::RtspClientSession;`)
   - Remove references in: `src/streamhub/errors.rs` (`RtspClientSessionError`)

2. **Strip RTMP/WebRTC/HLS Abstractions (~3,500 lines)**
   - Edit `src/streamhub/define.rs`:
     - Remove from `SubscribeType`: `RtmpPull`, `RtmpRemux2HttpFlv`, `RtmpRemux2Hls`, `RtmpRelay`, `RtspRemux2Rtmp`, `WhepPull`, `WebRTCRemux2Rtmp`, `WhipRelay`, `RtpPull`
     - Remove from `PublishType`: `RtmpPush`, `RtmpRelay`, `WhipPush`, `WhepRelay`, `RtpPush`
     - Keep only: `RtspPush`, `RtspPull`, `RtspRelay` (server-only variants)
   - Remove from `StreamIdentifier` in `streamhub/stream.rs`: `Rtmp`, `WebRTC` variants (keep only `Rtsp` + `HttpFlv`)
   - Clean up tests in `tests/stream_routing_test.rs` that reference RTMP/WebRTC

3. **Remove Duplicate Auth System (1,153 lines)**
   - Delete: `src/common/auth.rs`
   - Replace RTSP auth in `server_session.rs` with:
     ```rust
     use onvif_rust::security::digest_auth::DigestAuth;
     ```
   - Update `rtsp/rtsp.rs` and `httpflv/server.rs` to use onvif-rust's auth

4. **Gate Validation Readers (~2,100 lines)**
   - Move to `#[cfg(feature = "validation-mode")]`:
     - `src/codec/h264_file_reader.rs` (1,277 lines)
     - `src/codec/aac_file_reader.rs` (804 lines)
   - Move `container/demuxer.rs` (909 lines) to validation feature or delete if unused

### Validation
- Run tests: `cargo test --target x86_64-unknown-linux-gnu`
- Check binary size reduction: `cargo build --release && ls -lh target/release/`
- Verify RTSP server still works: Test with VLC player

---

## Phase 2: Split Oversized Files (Est. 2-3 days)

**Goal:** No file >2,000 lines  
**Target:** 44k → ~43k lines (better modularity, same LOC)

### 2.1 Split `server_session.rs` (6,206 → 5 files ~1,200 each)

**New structure:**
```
rtsp/session/
├── mod.rs               (~800 lines) - Public API + RtspServerSession coordinator
├── protocol.rs          (~1,500 lines) - RTSP message handling (OPTIONS, DESCRIBE, SETUP, PLAY, TEARDOWN)
├── rtp_sender.rs        (~1,500 lines) - RTP packet transmission + pacing + telemetry
├── playback_policy.rs   (~800 lines) - Lag recovery, frame age checks, IDR skipping
└── auth_handler.rs      (~600 lines) - RTSP Digest/Basic authentication
```

**Key extractions:**
- Move `PlaybackLatencyPolicy`, `LagTracker`, `LagRecoveryMode` → `playback_policy.rs`
- Move `RtpTrackCounters`, RTP send loop → `rtp_sender.rs`
- Move `handle_options`, `handle_describe`, etc. → `protocol.rs`
- Move `Auth`, `SecretCarrier` logic → `auth_handler.rs`

### 2.2 Split `streamhub/mod.rs` (2,799 → 4 files)

**New structure:**
```
streamhub/
├── mod.rs               (~400 lines) - Public API
├── hub.rs               (~800 lines) - StreamsHub core logic
├── transceiver.rs       (~800 lines) - StreamDataTransceiver
└── registry.rs          (~600 lines) - Publisher/subscriber tracking
```

### 2.3 Split `container/mpeg4_aac.rs` (1,930 → 2 files)

**New structure:**
```
container/aac/
├── demuxer.rs           (~1,000 lines) - AAC parsing for FLV
└── muxer.rs             (~900 lines) - AAC encoding for FLV
```

### 2.4 Split `rtsp/rtp/rtp_h264.rs` (1,724 → 2 files)

**New structure:**
```
rtp/h264/
├── packer.rs            (~1,000 lines) - NAL unit fragmentation into RTP
└── depacketizer.rs      (~700 lines) - RTP → NAL reassembly
```

### 2.5 Split `httpflv/httpflv.rs` (1,739 → 2 files)

**New structure:**
```
httpflv/
├── subscriber.rs        (~900 lines) - HTTP-FLV subscriber logic
└── muxer.rs             (~800 lines) - FLV container muxing
```

### Validation
- Run full test suite after each split
- Verify no regression in functionality
- Check that imports/re-exports work correctly

---

## Phase 3: Architectural Alignment (Est. 3-4 days)

**Goal:** Match onvif-rust patterns (traits, tracing, config, lifecycle)

### 3.1 Add Traits for Testability

**Create:** `src/protocol/rtsp/traits.rs`
```rust
use async_trait::async_trait;
use mockall::automock;

#[automock]
#[async_trait]
pub trait RtpSender: Send + Sync {
    async fn send_packet(&self, packet: &RtpPacket) -> Result<(), RtpError>;
    async fn send_rtcp_sr(&self, report: &SenderReport) -> Result<(), RtpError>;
}

#[automock]
pub trait FrameRouter: Send + Sync {
    fn route_frame(&self, stream_id: StreamId, frame: FrameData);
}

#[automock]
pub trait StreamRegistry: Send + Sync {
    fn register_publisher(&self, info: PublisherInfo) -> Result<Uuid, StreamHubError>;
    fn register_subscriber(&self, info: SubscriberInfo) -> Result<Uuid, StreamHubError>;
    fn unregister(&self, id: Uuid);
}
```

**Update tests to use `MockRtpSender`, `MockFrameRouter`, etc.**

### 3.2 Replace Global State with Config

**Create:** `src/config.rs`
```rust
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    pub rtp_sample_interval: u32,
    pub max_frame_age_ms: u32,
    pub lag_recovery_mode: LagRecoveryMode,
    pub rtsp_listen_addr: String,
    pub httpflv_listen_addr: String,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            rtp_sample_interval: 0, // disabled by default
            max_frame_age_ms: 1500,
            lag_recovery_mode: LagRecoveryMode::LatestIdr,
            rtsp_listen_addr: "0.0.0.0:554".to_string(),
            httpflv_listen_addr: "0.0.0.0:8080".to_string(),
        }
    }
}
```

**Remove global statics:**
- `RTP_SAMPLE_INTERVAL` in `server_session.rs` → pass via config
- Any other `lazy_static!` or `static` globals

### 3.3 Switch from `log` to `tracing`

**Update `Cargo.toml`:**
```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Remove:
# log = "0.4"
# env_logger = "0.11"
```

**Replace logging:**
```rust
// Old:
log::info!("session run exit: session id: {} session type: {}", session_id, session.session_type);

// New:
tracing::info!(
    session_id = %session_id,
    session_type = %session.session_type,
    "session_exit"
);
```

**Update all files:**
- Replace `log::error!` → `tracing::error!`
- Replace `log::warn!` → `tracing::warn!`
- Replace `log::info!` → `tracing::info!`
- Replace `log::debug!` → `tracing::debug!`
- Replace `log::trace!` → `tracing::trace!`
- Use structured fields: `key = value` instead of string formatting

### 3.4 Add Managed Service Lifecycle

**Create:** `src/service.rs`
```rust
use tokio::task::JoinSet;
use tokio::sync::broadcast;

pub struct StreamingService {
    tasks: JoinSet<Result<(), StreamingError>>,
    shutdown_tx: broadcast::Sender<()>,
}

impl StreamingService {
    pub async fn new(config: StreamingConfig, hub: Arc<StreamsHub>) -> Result<Self, StreamingError> {
        let mut tasks = JoinSet::new();
        let (shutdown_tx, _) = broadcast::channel(1);
        
        // Spawn RTSP server
        let rtsp_shutdown = shutdown_tx.subscribe();
        let rtsp_config = config.clone();
        tasks.spawn(async move {
            let mut server = DefaultRtspServer::new(rtsp_config.rtsp_listen_addr, hub.event_sender(), None);
            tokio::select! {
                result = server.run() => result.map_err(|e| e.into()),
                _ = rtsp_shutdown.recv() => Ok(()),
            }
        });
        
        // Spawn HTTP-FLV server
        let httpflv_shutdown = shutdown_tx.subscribe();
        let httpflv_config = config.clone();
        tasks.spawn(async move {
            let mut server = DefaultHttpFlvServer::new(httpflv_config.httpflv_listen_addr, hub.event_sender(), None);
            tokio::select! {
                result = server.run() => result.map_err(|e| e.into()),
                _ = httpflv_shutdown.recv() => Ok(()),
            }
        });
        
        Ok(Self { tasks, shutdown_tx })
    }
    
    pub async fn shutdown(mut self) -> ShutdownReport {
        self.shutdown_tx.send(()).ok();
        
        let mut report = ShutdownReport::default();
        while let Some(result) = self.tasks.join_next().await {
            match result {
                Ok(Ok(())) => report.success_count += 1,
                Ok(Err(e)) => {
                    report.failed_count += 1;
                    report.errors.push(e.to_string());
                }
                Err(e) => {
                    report.failed_count += 1;
                    report.errors.push(format!("Task panicked: {}", e));
                }
            }
        }
        report
    }
}

#[derive(Default)]
pub struct ShutdownReport {
    pub success_count: usize,
    pub failed_count: usize,
    pub errors: Vec<String>,
}
```

### 3.5 Standardize Error Handling

**Replace `anyhow` with `thiserror` for public errors:**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StreamingError {
    #[error("RTSP server error: {0}")]
    RtspServer(#[from] std::io::Error),
    
    #[error("HTTP-FLV server error: {0}")]
    HttpFlv(String),
    
    #[error("Stream hub error: {0}")]
    StreamHub(#[from] StreamHubError),
}
```

**Keep `anyhow` only for internal/test convenience.**

### Validation
- Run all tests with mocks
- Verify graceful shutdown works
- Check tracing output format
- Ensure no global state remains

---

## Phase 4: Module Reorganization (Est. 1-2 days)

**Goal:** Clearer separation of concerns (protocol vs codec vs transport)

### New Module Hierarchy

```
streaming-lib/src/
├── lib.rs
├── config.rs           (StreamingConfig)
├── service.rs          (StreamingService with lifecycle)
├── protocol/
│   ├── rtsp/
│   │   ├── server.rs
│   │   ├── session/     (protocol, rtp_sender, playback_policy, auth_handler)
│   │   ├── sdp/
│   │   ├── transport.rs
│   │   ├── codec.rs
│   │   ├── track.rs
│   │   └── traits.rs
│   └── httpflv/
│       ├── server.rs
│       ├── subscriber.rs
│       └── mod.rs
├── codec/
│   ├── h264/
│   │   ├── packer.rs
│   │   ├── depacketizer.rs
│   │   ├── sps.rs
│   │   └── pps.rs
│   └── aac/
│       ├── muxer.rs
│       └── profile.rs
├── container/
│   └── flv/
│       ├── muxer.rs
│       ├── demuxer.rs  (validation feature only)
│       └── tag_header.rs
├── io/
│   ├── bytes_reader.rs
│   ├── bytes_writer.rs
│   ├── bits_reader.rs
│   ├── bits_writer.rs
│   └── net_io.rs
├── hub/               (renamed from streamhub)
│   ├── mod.rs
│   ├── hub.rs
│   ├── transceiver.rs
│   ├── registry.rs
│   ├── define.rs
│   ├── stream.rs
│   └── statistics/
└── common/
    ├── http.rs         (HTTP parsing utilities)
    ├── utils.rs
    └── errors.rs
```

### Key Changes
- `bytesio/` → `io/` (clearer purpose)
- `streamhub/` → `hub/` (shorter, clearer)
- `rtsp/` → `protocol/rtsp/` (grouped with httpflv)
- `codec/` → separate `h264/` and `aac/` subdirs
- `container/` → `container/flv/` (scoped to FLV only)
- Remove `common/auth.rs` (use onvif-rust security)

### Validation
- Update all imports across codebase
- Run full test suite
- Verify public API exports in `lib.rs`
- Check documentation builds: `cargo doc --no-deps`

---

## Architectural Inconsistencies Fixed

| Issue | Before (streaming-lib) | After (aligned with onvif-rust) |
|-------|------------------------|----------------------------------|
| **Testability** | No traits/mocking | `#[automock]` traits for RtpSender, FrameRouter |
| **Error handling** | Mixed `anyhow`/`thiserror` | Consistent `thiserror` for public errors |
| **Naming** | Mixed `snake_case`/`camelCase` | Strict `snake_case` functions, `CamelCase` types |
| **Logging** | `log` crate with string formatting | `tracing` with structured fields |
| **Global state** | `static` atomics | Config structs passed to components |
| **Async patterns** | Raw `tokio::spawn` | Managed `JoinSet` + graceful shutdown |
| **Module organization** | Flat (bytesio, common, codec) | Layered (protocol, codec, io, hub) |

---

## Success Metrics

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| **Total lines** | 53,133 | ~30,000 | -43% |
| **Largest file** | 6,206 | <2,000 | -68% |
| **Files >1.5k lines** | 6 | 0 | -100% |
| **Unused code** | ~9,500 | 0 | -100% |
| **Test coverage** | Minimal | Mockall-based | +80% |
| **Compile time** | Baseline | -20% | Faster |
| **Binary size** | Baseline | -15% | Smaller |

---

## Dependencies Changed

### Remove
```toml
log = "0.4"
env_logger = "0.11"
md5 = "0.8"            # If only used by auth.rs
```

### Add
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
mockall = "0.14"       # Move from dev-dependencies to dependencies for trait mocking
```

---

## Testing Strategy

### Per Phase
1. **Phase 1 (Remove dead code):**
   - Run existing tests: `cargo test --target x86_64-unknown-linux-gnu`
   - Verify RTSP streaming with VLC: `rtsp://localhost:554/main`
   - Verify HTTP-FLV with browser: `http://localhost:8080/live/main.flv`

2. **Phase 2 (Split files):**
   - Run tests after each file split
   - Verify imports/re-exports work
   - Check no duplicate definitions

3. **Phase 3 (Architectural alignment):**
   - Write unit tests using mocks for RtpSender, FrameRouter
   - Verify graceful shutdown with integration test
   - Check tracing output in logs

4. **Phase 4 (Module reorg):**
   - Run full test suite
   - Verify documentation builds
   - Check public API hasn't changed (semver)

### Integration Tests
- Add `tests/rtsp_integration_test.rs`: Full RTSP session (DESCRIBE → SETUP → PLAY → TEARDOWN)
- Add `tests/httpflv_integration_test.rs`: HTTP-FLV streaming with FLV parsing
- Add `tests/streaming_service_test.rs`: Service lifecycle (start → shutdown)

---

## Rollout Plan

### Week 1
- **Day 1:** Phase 1 (remove dead code) → PR #1
- **Day 2-3:** Phase 2.1 (split server_session.rs) → PR #2
- **Day 4:** Phase 2.2-2.5 (split remaining files) → PR #3

### Week 2
- **Day 1-2:** Phase 3.1-3.3 (traits, config, tracing) → PR #4
- **Day 3:** Phase 3.4-3.5 (service lifecycle, errors) → PR #5
- **Day 4:** Phase 4 (module reorg) → PR #6
- **Day 5:** Integration tests + documentation → PR #7

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| **Breaking existing integration** | Run onvif-rust integration tests after each phase |
| **Performance regression** | Benchmark RTP throughput before/after (target: >30fps @ 1080p) |
| **Merge conflicts** | Keep PRs small and sequential, merge quickly |
| **Incomplete refactor** | Each phase is independently valuable (can stop after any phase) |

---

## Notes

- **No file readers in production:** `h264_file_reader.rs` and `aac_file_reader.rs` should only exist for validation mode
- **No RTSP client needed:** You're server-only (no pulling/pushing to remote RTSP)
- **Auth duplication wasteful:** onvif-rust already has WS-Security + HTTP Digest with rate limiting
- **Keep RTP/RTCP solid:** The `rtp/` module is well-structured, don't over-refactor
- **FLV muxing is production-ready:** Container format handling can stay mostly as-is

---

## References

- Original xiu fork: https://github.com/harlanc/xiu
- onvif-rust architecture: `cross-compile/onvif-rust/src/`
- ONVIF 24.12 spec: `cross-compile/onvif-rust/wsdl/`
- Architectural review: This document

---

## Next Steps

1. **Create tracking issues in br** for each phase
2. **Get approval** on overall plan before starting
3. **Start with Phase 1** (immediate wins on binary size/compile time)
4. **Run validation** after each phase
5. **Update documentation** as modules are refactored
