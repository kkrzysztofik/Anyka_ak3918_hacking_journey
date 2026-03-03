# Streaming-lib Refactoring - Tracking Issues

**Status:** Planning  
**Created:** 2026-03-03  
**Reference:** See [REFACTORING_PLAN.md](./REFACTORING_PLAN.md) for full details

---

## Phase 1: Remove Dead Code

**Priority:** P1 (High)  
**Type:** Task  
**Estimated Effort:** 1 day  
**Status:** Pending

### Description
Remove ~9,500 lines of unused code to reduce codebase from 53k to ~44k lines.

### Tasks
- [ ] Delete `src/rtsp/session/client_session.rs` (2,686 lines)
  - Remove export from `src/lib.rs:28`
  - Remove `RtspClientSessionError` from `src/streamhub/errors.rs`
  - Clean up tests referencing client session
  
- [ ] Strip RTMP/WebRTC/HLS abstractions (~3,500 lines)
  - Edit `src/streamhub/define.rs`:
    - Remove from `SubscribeType`: `RtmpPull`, `RtmpRemux2HttpFlv`, `RtmpRemux2Hls`, `RtmpRelay`, `RtspRemux2Rtmp`, `WhepPull`, `WebRTCRemux2Rtmp`, `WhipRelay`, `RtpPull`
    - Remove from `PublishType`: `RtmpPush`, `RtmpRelay`, `WhipPush`, `WhepRelay`, `RtpPush`
    - Keep only: `RtspPush`, `RtspPull`, `RtspRelay`
  - Remove from `StreamIdentifier` in `streamhub/stream.rs`: `Rtmp`, `WebRTC` variants
  - Clean up `tests/stream_routing_test.rs`
  
- [ ] Remove duplicate auth system (1,153 lines)
  - Delete `src/common/auth.rs`
  - Update `rtsp/rtsp.rs` and `httpflv/server.rs` to use `onvif_rust::security::digest_auth`
  - Update RTSP server_session auth handling
  
- [ ] Gate validation file readers (~2,100 lines)
  - Add `#[cfg(feature = "validation-mode")]` to:
    - `src/codec/h264_file_reader.rs` (1,277 lines)
    - `src/codec/aac_file_reader.rs` (804 lines)
  - Move or gate `src/container/demuxer.rs` (909 lines)
  - Update `Cargo.toml` to add `validation-mode` feature

### Validation
- [ ] Run tests: `cargo test --target x86_64-unknown-linux-gnu`
- [ ] Check binary size reduction
- [ ] Verify RTSP streaming with VLC: `rtsp://localhost:554/main`
- [ ] Verify HTTP-FLV: `http://localhost:8080/live/main.flv`

### Expected Outcome
- Codebase reduced by ~9,500 lines
- Binary size reduced by ~10-15%
- Compile time reduced by ~15%
- No functional changes to RTSP/HTTP-FLV servers

---

## Phase 2: Split Oversized Files

**Priority:** P1 (High)  
**Type:** Task  
**Estimated Effort:** 2-3 days  
**Status:** Blocked by Phase 1  
**Dependencies:** Phase 1 complete

### Description
Split 6 files >1,500 lines into smaller, focused modules. Target: no file >2,000 lines.

### 2.1 Split `server_session.rs` (6,206 lines → 5 files)

**New structure:**
```
rtsp/session/
├── mod.rs               (~800 lines)
├── protocol.rs          (~1,500 lines)
├── rtp_sender.rs        (~1,500 lines)
├── playback_policy.rs   (~800 lines)
└── auth_handler.rs      (~600 lines)
```

**Tasks:**
- [ ] Create `rtsp/session/protocol.rs` - Extract RTSP message handlers (OPTIONS, DESCRIBE, SETUP, PLAY, TEARDOWN)
- [ ] Create `rtsp/session/rtp_sender.rs` - Extract RTP packet transmission, pacing, telemetry
- [ ] Create `rtsp/session/playback_policy.rs` - Extract `PlaybackLatencyPolicy`, `LagTracker`, `LagRecoveryMode`
- [ ] Create `rtsp/session/auth_handler.rs` - Extract auth logic
- [ ] Update `rtsp/session/mod.rs` - Keep only public API and coordinator
- [ ] Update all imports and re-exports
- [ ] Run tests: `cargo test --target x86_64-unknown-linux-gnu`

### 2.2 Split `streamhub/mod.rs` (2,799 lines → 4 files)

**New structure:**
```
streamhub/
├── mod.rs               (~400 lines)
├── hub.rs               (~800 lines)
├── transceiver.rs       (~800 lines)
└── registry.rs          (~600 lines)
```

**Tasks:**
- [ ] Create `streamhub/hub.rs` - Extract `StreamsHub` core logic
- [ ] Create `streamhub/transceiver.rs` - Extract `StreamDataTransceiver`
- [ ] Create `streamhub/registry.rs` - Extract publisher/subscriber tracking
- [ ] Update `streamhub/mod.rs` - Keep only public API
- [ ] Run tests

### 2.3 Split `container/mpeg4_aac.rs` (1,930 lines → 2 files)

**New structure:**
```
container/aac/
├── demuxer.rs           (~1,000 lines)
└── muxer.rs             (~900 lines)
```

**Tasks:**
- [ ] Create `container/aac/` directory
- [ ] Split demuxer and muxer logic
- [ ] Update imports
- [ ] Run tests

### 2.4 Split `rtsp/rtp/rtp_h264.rs` (1,724 lines → 2 files)

**New structure:**
```
rtp/h264/
├── packer.rs            (~1,000 lines)
└── depacketizer.rs      (~700 lines)
```

**Tasks:**
- [ ] Create `rtp/h264/` directory
- [ ] Split packer and depacketizer
- [ ] Update imports
- [ ] Run tests

### 2.5 Split `httpflv/httpflv.rs` (1,739 lines → 2 files)

**New structure:**
```
httpflv/
├── subscriber.rs        (~900 lines)
└── muxer.rs             (~800 lines)
```

**Tasks:**
- [ ] Split subscriber logic from muxing
- [ ] Update imports
- [ ] Run tests

### Validation
- [ ] Run full test suite after each split
- [ ] Verify no duplicate definitions
- [ ] Check imports/re-exports work
- [ ] Verify documentation builds: `cargo doc --no-deps`

---

## Phase 3: Architectural Alignment

**Priority:** P1 (High)  
**Type:** Task  
**Estimated Effort:** 3-4 days  
**Status:** Blocked by Phase 2  
**Dependencies:** Phases 1 & 2 complete

### Description
Align streaming-lib with onvif-rust architectural patterns: traits for testability, tracing, config structs, managed lifecycle.

### 3.1 Add Traits for Testability

**Tasks:**
- [ ] Create `src/protocol/rtsp/traits.rs` with:
  - `RtpSender` trait with `#[automock]`
  - `FrameRouter` trait with `#[automock]`
  - `StreamRegistry` trait with `#[automock]`
- [ ] Update components to use traits instead of concrete types
- [ ] Write unit tests using `MockRtpSender`, `MockFrameRouter`
- [ ] Add `mockall` to dependencies (not just dev-dependencies)

### 3.2 Replace Global State with Config

**Tasks:**
- [ ] Create `src/config.rs` with `StreamingConfig` struct
- [ ] Remove `static RTP_SAMPLE_INTERVAL` from `server_session.rs`
- [ ] Pass config via constructor/method parameters
- [ ] Update all code using global state

### 3.3 Switch from `log` to `tracing`

**Tasks:**
- [ ] Update `Cargo.toml`:
  - Add `tracing = "0.1"`
  - Add `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`
  - Remove `log = "0.4"`
  - Remove `env_logger = "0.11"`
- [ ] Replace all `log::*!()` macros with `tracing::*!()`
- [ ] Convert string formatting to structured fields: `key = value`
- [ ] Update initialization code to use `tracing_subscriber`

### 3.4 Add Managed Service Lifecycle

**Tasks:**
- [ ] Create `src/service.rs` with `StreamingService` struct
- [ ] Use `tokio::task::JoinSet` for managed tasks
- [ ] Add graceful shutdown with `broadcast::channel`
- [ ] Add `ShutdownReport` struct
- [ ] Write integration test for service lifecycle

### 3.5 Standardize Error Handling

**Tasks:**
- [ ] Create `StreamingError` enum using `thiserror`
- [ ] Replace `anyhow` usage in public APIs
- [ ] Keep `anyhow` only for internal/test code
- [ ] Update error propagation with `?` operator

### Validation
- [ ] Run all tests with mocks
- [ ] Verify graceful shutdown works
- [ ] Check tracing output format
- [ ] Ensure no global state remains
- [ ] Benchmark RTP throughput (target: >30fps @ 1080p)

---

## Phase 4: Module Reorganization

**Priority:** P2 (Medium)  
**Type:** Task  
**Estimated Effort:** 1-2 days  
**Status:** Blocked by Phase 3  
**Dependencies:** Phases 1, 2, 3 complete

### Description
Reorganize module hierarchy for clearer separation of concerns: protocol vs codec vs transport.

### New Structure
```
streaming-lib/src/
├── lib.rs
├── config.rs           (StreamingConfig)
├── service.rs          (StreamingService with lifecycle)
├── protocol/
│   ├── rtsp/
│   │   ├── server.rs
│   │   ├── session/
│   │   ├── sdp/
│   │   ├── transport.rs
│   │   └── traits.rs
│   └── httpflv/
│       ├── server.rs
│       └── subscriber.rs
├── codec/
│   ├── h264/
│   │   ├── packer.rs
│   │   ├── depacketizer.rs
│   │   └── sps.rs
│   └── aac/
│       └── muxer.rs
├── container/
│   └── flv/
│       ├── muxer.rs
│       └── demuxer.rs
├── io/
│   ├── bytes_reader.rs
│   ├── bytes_writer.rs
│   └── net_io.rs
├── hub/
│   ├── mod.rs
│   ├── hub.rs
│   ├── transceiver.rs
│   └── statistics/
└── common/
    ├── http.rs
    └── utils.rs
```

### Tasks
- [ ] Rename `bytesio/` → `io/`
- [ ] Rename `streamhub/` → `hub/`
- [ ] Move `rtsp/` → `protocol/rtsp/`
- [ ] Move `httpflv/` → `protocol/httpflv/`
- [ ] Organize `codec/` with `h264/` and `aac/` subdirs
- [ ] Organize `container/` with `flv/` subdir
- [ ] Update all imports across codebase
- [ ] Update `lib.rs` exports
- [ ] Run full test suite
- [ ] Verify documentation builds

### Validation
- [ ] All tests pass
- [ ] Public API unchanged (semver check)
- [ ] Documentation builds successfully
- [ ] No broken imports

---

## Integration Tests (New)

**Priority:** P2 (Medium)  
**Type:** Task  
**Estimated Effort:** 1 day  
**Status:** Can be done in parallel with phases

### Tasks
- [ ] Create `tests/rtsp_integration_test.rs`
  - Test full RTSP session: DESCRIBE → SETUP → PLAY → TEARDOWN
  - Verify RTP packet delivery
  - Test multiple simultaneous sessions
  
- [ ] Create `tests/httpflv_integration_test.rs`
  - Test HTTP-FLV streaming
  - Verify FLV container format
  - Test simultaneous connections
  
- [ ] Create `tests/streaming_service_test.rs`
  - Test service lifecycle: start → run → shutdown
  - Verify graceful shutdown
  - Test error handling

---

## Documentation Updates

**Priority:** P3 (Low)  
**Type:** Task  
**Estimated Effort:** 0.5 day  
**Status:** After all phases complete

### Tasks
- [ ] Update `README.md` with new architecture
- [ ] Update module-level documentation
- [ ] Add examples for common use cases
- [ ] Document trait-based testing approach
- [ ] Add migration guide from old API (if breaking changes)
- [ ] Generate and review rustdoc: `cargo doc --no-deps --open`

---

## Success Criteria

### Code Quality
- ✅ No file >2,000 lines
- ✅ All code uses `tracing` (no `log`)
- ✅ No global state (all in config structs)
- ✅ All dependencies injected via traits
- ✅ Test coverage >80% (with mockall)

### Performance
- ✅ Binary size reduced by 15%
- ✅ Compile time reduced by 20%
- ✅ RTP throughput maintained (>30fps @ 1080p)
- ✅ No memory leaks in streaming sessions

### Functionality
- ✅ RTSP server works with VLC/ffplay
- ✅ HTTP-FLV server works in browsers
- ✅ Graceful shutdown completes in <2s
- ✅ No panics in production code paths

---

## Notes

- Each phase is independently valuable (can stop after any phase)
- Phases should be completed in order (dependencies listed)
- Run validation after each phase before proceeding
- Keep PRs small and focused for easier review
- Integration with onvif-rust must remain stable throughout

---

## Rollout Timeline

| Week | Days | Phase | Deliverable |
|------|------|-------|-------------|
| 1    | 1    | Phase 1 | Dead code removed, ~44k LOC |
| 1    | 2-3  | Phase 2.1 | server_session.rs split |
| 1    | 4    | Phase 2.2-2.5 | Other large files split |
| 2    | 1-2  | Phase 3.1-3.3 | Traits, config, tracing |
| 2    | 3    | Phase 3.4-3.5 | Service lifecycle, errors |
| 2    | 4    | Phase 4 | Module reorganization |
| 2    | 5    | Final | Integration tests + docs |

**Total Estimated Effort:** 7-10 days
