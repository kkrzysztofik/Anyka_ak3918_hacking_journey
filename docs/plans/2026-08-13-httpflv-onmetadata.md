# HTTP-FLV onMetaData tag — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Emit a single FLV `onMetaData` script tag (tag type 18) carrying the configured framerate so mpegts.js, ffprobe, and VLC read ~15 fps instead of mpegts.js's hardcoded 23.976 fallback.

**Architecture:** streaming-lib's `HttpFlv` connection learns to write `FrameData::MetaData` frames as FLV script tags (generic capability). onvif-rust's `ValidationHttpFlvRemuxer` builds the AMF0 `onMetaData` payload from the configured `video_framerate`; `send_httpflv_prior_frames` emits it as the first tag after the FLV header.

**Tech Stack:** Rust (streaming-lib + onvif-rust), AMF0 encoding, FLV tag serialization.

## Global Constraints

- No `unwrap()` / `expect()` / `panic!()` in production paths (onvif-rust AGENTS.md).
- Rust host-side gates: `$CARGO fmt --check`, `$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings`, `$CARGO test --target x86_64-unknown-linux-gnu` (from `cross-compile/onvif-rust` after `source ./setenv.sh`).
- Test naming: `test_<component>_<scenario>_<expected_outcome>`.
- onMetaData carries only: `videocodecid` (7), `hasVideo` (true), `hasAudio` (bool), `framerate` (configured). No width/height (SPS covers them).
- AMF0 body format must match mpegts.js `amf-parser.js`: `02 <u16be> "onMetaData"` then ECMA array `08 <u32be count>` + `(u16be keylen + key + typebyte + value)*` + `00 00 09` terminator.
- The framerate value advertised is the configured `video_framerate` (same as SDP `a=framerate`), default 15.

---

### Task 1: streaming-lib `HttpFlv` accepts MetaData frames

**Files:**
- Modify: `cross-compile/streaming-lib/src/protocol/httpflv/httpflv.rs`
- Test: same file, `mod tests`

**Interfaces:**
- Consumes: `FrameData::MetaData { timestamp: u32, data: BytesMut }` (already exists in `crate::hub::define::FrameData`).
- Produces: `HttpFlv` writes `FrameData::MetaData` as an FLV tag with tag type `tag_type::SCRIPT_DATA_AMF` (18), both in the header phase (cached after the FLV header) and post-header.

- [ ] **Step 1: Write the failing test — MetaData is written as a script tag (replaces `test_write_flv_tag_metadata_rejected`)**

Replace the body of the existing `test_write_flv_tag_metadata_rejected` (httpflv.rs:1161-1189) with:

```rust
#[tokio::test]
async fn test_write_flv_tag_metadata_writes_script_tag() {
    let (event_sender, _event_receiver, response_sender, mut response_receiver) =
        create_test_channels();
    let remote_addr = create_test_socket_addr();

    let mut httpflv = HttpFlv::new(
        "live".to_string(),
        "stream1".to_string(),
        event_sender,
        response_sender,
        "http://localhost/live/stream1.flv".to_string(),
        remote_addr,
    );

    let mut meta = BytesMut::new();
    meta.extend_from_slice(&[0x02, 0x00, 0x0a]);
    meta.extend_from_slice(b"onMetaData");
    let frame = FrameData::MetaData {
        timestamp: 0,
        data: meta,
    };
    let result = httpflv.write_flv_tag(frame);
    assert!(result.is_ok(), "MetaData frame should be writable as an FLV script tag");

    let chunk = response_receiver.next().await.expect("response chunk");
    let bytes = chunk.expect("chunk is Ok");
    assert_eq!(bytes[0], tag_type::SCRIPT_DATA_AMF, "tag type byte must be 18");
    assert_eq!(bytes[11], 0x02, "body must start with AMF string type");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run (from `cross-compile/streaming-lib`): `$CARGO test --target x86_64-unknown-linux-gnu httpflv::tests::test_write_flv_tag_metadata_writes_script_tag`
Expected: FAIL — current `extract_flv_tag_data` returns `Err(UnexpectedFrameData)` for MetaData.

- [ ] **Step 3: Implement — accept MetaData in `extract_flv_tag_data`**

In `HttpFlv::extract_flv_tag_data` (httpflv.rs:181-199), add a `MetaData` arm before the `other` arm:

```rust
FrameData::MetaData { timestamp, data } => (data, timestamp, tag_type::SCRIPT_DATA_AMF),
```

- [ ] **Step 4: Implement — cache MetaData in the header phase**

In `HttpFlv::process_header_phase` (httpflv.rs:120-137), change the match so MetaData frames are cached too:

```rust
match &data {
    FrameData::Audio { .. } => {
        self.has_audio = true;
        header_state.cached_frames.push(data);
    }
    FrameData::Video { .. } => {
        self.has_video = true;
        header_state.cached_frames.push(data);
    }
    FrameData::MetaData { .. } => {
        header_state.cached_frames.push(data);
    }
    _ => {}
}
```

`finalize_header` already writes cached frames in order after `write_flv_header`, so a MetaData frame sent first lands immediately after the FLV header — correct FLV ordering.

- [ ] **Step 5: Run tests to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu httpflv::tests`
Expected: PASS, including `test_send_media_stream_header_phase_metadata_then_audio_video` (sends MetaData then Audio then Video through the hub — now the MetaData is cached and written rather than dropped).

- [ ] **Step 6: Commit**

```bash
git add cross-compile/streaming-lib/src/protocol/httpflv/httpflv.rs
git commit -m "feat(streaming-lib): write FrameData::MetaData as FLV script tags"
```

---

### Task 2: onvif-rust `ValidationHttpFlvRemuxer` builds the onMetaData tag

**Files:**
- Modify: `cross-compile/onvif-rust/src/validation/httpflv_remux.rs`
- Test: same file, `mod tests`

**Interfaces:**
- Consumes: existing `ValidationHttpFlvRemuxer::new(sps, pps, audio_config, audio_sample_rate)` — gains a 5th param `video_framerate: u32`.
- Produces:
  - `ValidationHttpFlvRemuxer::new(sps: Vec<u8>, pps: Vec<u8>, audio_config: Option<Vec<u8>>, audio_sample_rate: u32, video_framerate: u32) -> Self`
   - `ValidationHttpFlvRemuxer::on_metadata_tag(&self, timestamp: u32) -> FrameData`

- [ ] **Step 1: Write the failing test for the AMF0 payload**

Append to `mod tests` in httpflv_remux.rs:

```rust
#[test]
fn test_on_metadata_tag_builds_framerate_ecma_array() {
    let remuxer = ValidationHttpFlvRemuxer::new(
        vec![0x67, 0x42, 0xE0, 0x1E],
        vec![0x68, 0xCE, 0x06, 0xE2],
        None,
        0,
        15,
    );
    let FrameData::MetaData { timestamp, data } = remuxer.on_metadata_tag(0) else {
        panic!("expected MetaData frame");
    };
    assert_eq!(timestamp, 0);

    // AMF string: 0x02, u16be len=10, "onMetaData"
    assert_eq!(&data[..3], &[0x02, 0x00, 0x0a]);
    assert_eq!(&data[3..13], b"onMetaData");

    // ECMA array marker + framerate key/value (15.0 = 0x402E000000000000)
    let body = &data[13..];
    assert_eq!(body[0], 0x08, "ECMA array marker");
    let framerate = body.windows(9)
        .position(|w| w == b"framerate".as_slice())
        .expect("framerate key present");
    assert_eq!(&body[framerate + 9..framerate + 10], &[0x00], "number type");
    assert_eq!(
        &body[framerate + 10..framerate + 18],
        &[0x40, 0x2E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        "15.0 as f64"
    );
    assert!(body.ends_with(&[0x00, 0x00, 0x09]), "ECMA array terminator");
}

#[test]
fn test_on_metadata_tag_reflects_audio_presence() {
    let remuxer = ValidationHttpFlvRemuxer::new(
        vec![0x67, 0x42, 0xE0, 0x1E],
        vec![0x68, 0xCE, 0x06, 0xE2],
        Some(vec![0x11, 0x90]),
        48_000,
        15,
    );
    let FrameData::MetaData { data, .. } = remuxer.on_metadata_tag(0) else {
        panic!("expected MetaData frame");
    };
    // hasAudio bool (0x01) true (0x01) follows the "hasAudio" key
    let body = &data[13..];
    let idx = body.windows(8)
        .position(|w| w == b"hasAudio".as_slice())
        .expect("hasAudio key present");
    assert_eq!(&body[idx + 8..idx + 10], &[0x01, 0x01], "hasAudio = true");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run (from `cross-compile/onvif-rust`): `$CARGO test --target x86_64-unknown-linux-gnu httpflv_remux`
Expected: FAIL — `on_metadata_tag` does not exist / `new` takes 4 args.

- [ ] **Step 3: Implement — add `video_framerate` field and `on_metadata_tag`**

Update the struct and constructor:

```rust
#[derive(Debug, Clone)]
pub struct ValidationHttpFlvRemuxer {
    sps: Option<Vec<u8>>,
    pps: Option<Vec<u8>>,
    audio_config: Option<Vec<u8>>,
    audio_sample_rate: u32,
    video_framerate: u32,
}

impl ValidationHttpFlvRemuxer {
    pub fn new(
        sps: Vec<u8>,
        pps: Vec<u8>,
        audio_config: Option<Vec<u8>>,
        audio_sample_rate: u32,
        video_framerate: u32,
    ) -> Self {
        Self {
            sps: (!sps.is_empty()).then_some(sps),
            pps: (!pps.is_empty()).then_some(pps),
            audio_config,
            audio_sample_rate,
            video_framerate,
        }
    }

    /// Build the FLV onMetaData script-tag payload (AMF0 ECMA array).
    pub fn on_metadata_tag(&self, timestamp: u32) -> FrameData {
        let mut data = BytesMut::new();
        amf0_string(&mut data, "onMetaData");
        amf0_ecma_array(
            &mut data,
            &[
                ("videocodecid", amf0_number(7.0)),
                ("hasVideo", amf0_bool(true)),
                ("hasAudio", amf0_bool(self.audio_config.is_some())),
                ("framerate", amf0_number(self.video_framerate as f64)),
            ],
        );
        FrameData::MetaData { timestamp, data }
    }
}

fn amf0_string(out: &mut BytesMut, s: &str) {
    out.put_u8(0x02);
    out.put_u16(s.len() as u16);
    out.extend_from_slice(s.as_bytes());
}

fn amf0_number(value: f64) -> BytesMut {
    let mut v = BytesMut::with_capacity(9);
    v.put_u8(0x00);
    v.put_f64(value);
    v
}

fn amf0_bool(value: bool) -> BytesMut {
    BytesMut::from(&[0x01, value as u8][..])
}

/// AMF0 ECMA array: `08 <u32be count> (u16be keylen + key + value)* 00 00 09`.
fn amf0_ecma_array(out: &mut BytesMut, entries: &[(&str, BytesMut)]) {
    out.put_u8(0x08);
    out.put_u32(entries.len() as u32);
    for (key, value) in entries {
        out.put_u16(key.len() as u16);
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(value);
    }
    out.extend_from_slice(&[0x00, 0x00, 0x09]);
}
```

- [ ] **Step 4: Fix existing `new()` call sites in this file's tests**

`test_remux_video_sequence_header_generation_success`, `test_remux_annexb_video_to_avcc_nalu_payload`, `test_remux_single_raw_nal_without_annexb_start_code`, `test_remux_audio_timestamp_scaling_success`, `test_remux_runtime_sps_pps_filtering_skips_headers_only_frame` all call `ValidationHttpFlvRemuxer::new(sps, pps, audio_config, audio_sample_rate)` — append `, 15` (or a chosen framerate) as the 5th argument.

- [ ] **Step 5: Run tests to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu httpflv_remux`
Expected: PASS (all 5 remux tests updated + 2 new onMetaData tests).

- [ ] **Step 6: Commit**

```bash
git add cross-compile/onvif-rust/src/validation/httpflv_remux.rs
git commit -m "feat(onvif-rust): build FLV onMetaData tag with configured framerate"
```

---

### Task 3: Wire onMetaData into the FLV prior-data path and handlers

**Files:**
- Modify: `cross-compile/onvif-rust/src/streaming/helpers.rs` (`send_httpflv_prior_frames`)
- Modify: `cross-compile/onvif-rust/src/streaming/service.rs` (`LiveStreamHandler::send_parameter_sets`, `FanoutTask::new`, `FanoutTask::update_remuxer`, `publish_stream`)
- Modify: `cross-compile/onvif-rust/src/main.rs` (`ValidationAvStreamHandler`)

**Interfaces:**
- Consumes: `ValidationHttpFlvRemuxer::on_metadata_tag(timestamp) -> FrameData` (Task 2).
- Produces: every HTTP-FLV subscriber receives an `onMetaData` tag as the first FLV tag.

- [ ] **Step 1: Add a failing helper test in `helpers.rs`**

Append to `mod tests` in helpers.rs:

```rust
#[test]
fn test_send_httpflv_prior_frames_sends_metadata_first() {
    use tokio::sync::mpsc;
    let (tx, mut rx) = mpsc::channel(16);
    let mut remuxer = ValidationHttpFlvRemuxer::new(
        vec![0x67, 0x42, 0xE0, 0x1E],
        vec![0x68, 0xCE, 0x06, 0xE2],
        None,
        0,
        15,
    );
    send_httpflv_prior_frames(&tx, &mut remuxer, 123, None).expect("prior frames sent");

    let first = rx.try_recv().expect("first frame");
    assert!(
        matches!(first, FrameData::MetaData { .. }),
        "first FLV frame must be onMetaData, got {first:?}"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `$CARGO test --target x86_64-unknown-linux-gnu helpers::tests::test_send_httpflv_prior_frames_sends_metadata_first`
Expected: FAIL — first frame is the video sequence header (a `FrameData::Video`), not MetaData.

- [ ] **Step 3: Implement — emit onMetaData first in `send_httpflv_prior_frames`**

In `send_httpflv_prior_frames` (helpers.rs:104-149), after the tracing::debug and before the video sequence header block, add:

```rust
send_frame(frame_sender, remuxer.on_metadata_tag(timestamp))?;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `$CARGO test --target x86_64-unknown-linux-gnu helpers::tests::test_send_httpflv_prior_frames_sends_metadata_first`
Expected: PASS.

- [ ] **Step 5: Fix all `ValidationHttpFlvRemuxer::new` call sites to pass `video_framerate`**

`service.rs`:
- `LiveStreamHandler::send_parameter_sets` (~line 96): add `self.video_framerate` as 5th arg.
- `FanoutTask::update_remuxer` (~line 402): add the task's `video_framerate` field as 5th arg.

`FanoutTask` gains a field. In `FanoutTask::new` (service.rs:291-313) add `video_framerate: u32` param + field; in `publish_stream` (~line 719) pass `self.config.video_framerate`.

`main.rs`:
- `ValidationAvStreamHandler` struct + `new()` (main.rs:441-467): add `video_framerate: u32` field/param.
- Handler construction (main.rs:681-688): pass `config.frame_rate` (the `H264PlaybackConfig` field, default 25).
- `send_parameter_sets`-equivalent in `ValidationAvStreamHandler::send_prior_data` (main.rs:496-501): add `self.video_framerate` as 5th arg.

- [ ] **Step 6: Run onvif-rust build + tests**

Run (from `cross-compile/onvif-rust`): `$CARGO build --target x86_64-unknown-linux-gnu` then `$CARGO test --target x86_64-unknown-linux-gnu`
Expected: builds clean; all tests pass.

- [ ] **Step 7: Commit**

```bash
git add cross-compile/onvif-rust/src/streaming/helpers.rs cross-compile/onvif-rust/src/streaming/service.rs cross-compile/onvif-rust/src/main.rs
git commit -m "feat(onvif-rust): send onMetaData tag before FLV sequence headers"
```

---

### Task 4: Full quality gates + on-device verification

**Files:** none (verification only).

- [ ] **Step 1: Format check**

Run (from `cross-compile/onvif-rust`): `$CARGO fmt --check` (and `$CARGO fmt` to fix).
Run (from `cross-compile/streaming-lib`): `$CARGO fmt --check`.

- [ ] **Step 2: Clippy**

Run: `$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings`
Expected: clean (both crates).

- [ ] **Step 3: Full tests**

Run: `$CARGO test --target x86_64-unknown-linux-gnu` in both `cross-compile/onvif-rust` and `cross-compile/streaming-lib`.
Expected: all pass.

- [ ] **Step 4: On-device verification (192.168.2.198)**

Deploy the rebuilt binary via the A/B slot flow (per anyka-firmware-upgrade skill), then:

```bash
ffprobe -v error -show_entries stream=codec_name,profile,width,height,r_frame_rate \
  http://admin:admin@192.168.2.198:8080/live/main.flv
```

Expected: `r_frame_rate` ~ `15/1` (or equivalent near-15 value). Additionally open the WebUI Live View: the Frame Rate stat must show ~15 (measured) and the Codec stat `H.264 Main@L4.0`.

- [ ] **Step 5: Commit any remaining changes**

```bash
git status  # clean; nothing to commit if gates passed without edits
```

---

## Self-review

- **Spec coverage:** Task 1 (streaming-lib script-tag support) ✓; Task 2 (AMF0 onMetaData builder, no width/height) ✓; Task 3 (wiring + framerate threading) ✓; Task 4 (gates + on-device) ✓. Non-goals (width/height, measured rate, SPS VUI rewrite) are not implemented.
- **Placeholder scan:** all steps contain concrete code; no TBD/TODO.
- **Type consistency:** `ValidationHttpFlvRemuxer::new` gains 5th param in Task 2 and every call site is updated in Task 3; `on_metadata_tag` returns `FrameData` (constructing the `FrameData::MetaData` variant); `send_httpflv_prior_frames` signature unchanged (remuxer already carries framerate). `FanoutTask::new` gains `video_framerate: u32` — updated at its single call site in `publish_stream`.
