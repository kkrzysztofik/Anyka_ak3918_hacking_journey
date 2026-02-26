# RTSP UDP H.264 RFC Fixes Implementation Plan

> For Claude: REQUIRED SUB-SKILL: use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove RTCP time-related panics, improve H.264 RTP marker behavior for access units, and treat vendor-daemon overflow drops as normal loss (no noisy errors), while keeping UDP RTSP streaming stable.

**Architecture:** Keep behavior changes minimal and localized: make NTP derivation fallible and skip SR until valid time; adjust marker-bit decision in H.264 packer to reflect end-of-access-unit; downgrade/adjust dropped-frame notification handling without changing IPC wire format or main data path.

**Tech Stack:** Rust (onvif-rust + streaming-lib), RTSP/RTP/RTCP, RFC 2326/3550/6184/4566, vendor-daemon (C IPC producer).

---

### Task 1: Make NTP timestamp non-panicking (skip SR if wall clock invalid)

**Files:**
- Modify: `cross-compile/streaming-lib/src/rtsp/rtp/utils.rs:145-168`
- Modify: `cross-compile/streaming-lib/src/rtsp/rtp/rtcp/rtcp_context.rs:232-269`
- Modify: `cross-compile/streaming-lib/src/rtsp/rtsp_track.rs:110-128`
- Test: `cross-compile/streaming-lib/src/rtsp/rtp/utils.rs` (unit tests, same file) and/or `cross-compile/streaming-lib/src/rtsp/rtp/rtcp/rtcp_context.rs` (unit tests)

**Step 1: Write failing tests**
- Add a helper API to test invalid time deterministically (no system clock mocking):
  - Introduce `ntp_timestamp_from_system_time(st: SystemTime) -> Option<u64>` (or `Result<u64, _>`), and have `ntp_timestamp()` call it with `SystemTime::now()`.
- Test cases:
  - `SystemTime::UNIX_EPOCH - Duration::from_secs(1)` returns `None`/`Err`.
  - `SystemTime::UNIX_EPOCH + Duration::from_secs(1)` returns `Some(ntp)` and has the epoch offset applied.

**Step 2: Run the tests (verify failing)**
Run: `cd cross-compile && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p streaming-lib utils::tests::test_ntp_timestamp_from_system_time_*`

**Step 3: Implement minimal code**
- Replace `expect(...)` in `ntp_timestamp()` with fallible logic.
- Update `RtcpContext::generate_sr()` to return `Option<RtcpSenderReport>` (or keep signature but internally "no-op" and let caller decide); ensure it does not panic when NTP unavailable.
- Update `rtcp_send_loop` to skip sending SR when SR isn't available; log at debug once per interval (or rate-limit) to avoid spam on devices without time.

**Step 4: Run targeted tests**
Run: same as Step 2  
Expected: PASS

**Step 5: Run workspace gates**
Run:
- `cd cross-compile && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo fmt`
- `cd cross-compile && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo clippy --target x86_64-unknown-linux-gnu -p streaming-lib -- -D warnings`
- `cd cross-compile && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p streaming-lib`

**Step 6: Commit**
Commit message (example): `fix(rtcp): skip sender reports until wall clock valid`

---

### Task 2: H.264 RTP marker bit = end-of-access-unit (not last VCL)

**Files:**
- Modify: `cross-compile/streaming-lib/src/rtsp/rtp/rtp_h264.rs:230-244` (pack loop + marker decision)
- Test: `cross-compile/streaming-lib/src/rtsp/rtp/rtp_h264.rs` (existing tests around marker/FU-A)

**Step 1: Write failing tests**
- Construct an Annex-B access unit that includes trailing non-VCL NAL after VCL (e.g., `IDR (type 5)` then `SEI (type 6)`), and assert the *final* packet sent has marker=1.
- Also assert prior behavior still holds: multi-NAL frame gets exactly one marker=1 at the end; FU-A fragmentation ends with marker=1 on last fragment.

**Step 2: Run the tests (verify failing)**
Run: `cd cross-compile && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p streaming-lib rtp_h264::tests::*marker*`

**Step 3: Implement minimal code**
- In `TPacker::pack()`:
  - Determine the last NAL index *in extracted_nalus*, not last VCL index.
  - Use `mark_end_of_access_unit = index == last_index`.
- Keep FU-A rule: marker set only on FU_END when `mark_end_of_access_unit` is true.

**Step 4: Run targeted tests**
Run: same as Step 2  
Expected: PASS

**Step 5: Run streaming-lib gates**
- fmt/clippy/test as in Task 1 Step 5 but limited to `-p streaming-lib`.

**Step 6: Commit**
Commit message (example): `fix(rtp): set H264 marker on end of access unit`

---

### Task 3: Vendor "frame dropped" notification treated as loss (no warn/error semantics)

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/vendor_ipc.rs:902-915`
- (Optional) Modify: `cross-compile/onvif-rust/src/platform/anyka.rs:1633-1692` (only if needed to adjust accounting/logging)
- Test: `cross-compile/onvif-rust/src/hal/vendor_ipc.rs` unit tests if present, otherwise add a small test around the notification flag handling (mocked input path)

**Step 1: Define intended behavior (no API churn)**
- Keep returning a transient error (so the loop continues) but:
  - downgrade `warn!` to `debug!` or add rate-limiting,
  - adjust error message to avoid implying platform failure,
  - ensure it does not increment "hard error" counters downstream (if any).

**Step 2: Write failing test**
- If there's a test seam for `FrameNotification::is_frame_dropped()` and the handling branch, assert logging level/behavior isn't warn (if log capture exists), or assert returned error remains transient (e.g., `PlatformError::ResourceBusy(_)` is still classified transient by `is_push_mode_transient_error`).

**Step 3: Implement minimal code**
- Change `warn!(...)` to `debug!(...)` (or rate-limited warn) and keep transient return type.
- Do not change IPC wire structs or notification flags.

**Step 4: Run onvif-rust gates**
Run:
- `cd cross-compile && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo fmt`
- `cd cross-compile && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo clippy --target x86_64-unknown-linux-gnu -p onvif-rust -- -D warnings`
- `cd cross-compile && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust`

**Step 5: Commit**
Commit message (example): `chore(ipc): treat daemon frame drops as transient loss`

---

### Task 4 (Optional): Enforce RTSP CSeq presence

**Files:**
- Modify: `cross-compile/streaming-lib/src/rtsp/session/server_session.rs` (early request validation)
- Test: `cross-compile/streaming-lib/tests/rtsp_session_test.rs` (or existing session tests)

**Steps:**
- Add test: request without `CSeq` gets 400.
- Implement: reject missing CSeq before handler dispatch.
- Run `-p streaming-lib` clippy/test, commit.

---

### Final verification
Run full workspace gates (already known-good baseline, re-run after changes):
- `cd cross-compile && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo fmt --check`
- `cd cross-compile && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings`
- `cd cross-compile && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu`
- `cd cross-compile/vendor-daemon && make`

### Deliverables
- Code changes implementing Tasks 1–3 (Task 4 optional).
- Plan doc saved to `docs/plans/2026-02-26-rtsp-udp-h264-rfc-fixes.md` during implementation phase.

## Handover from Planning Session

---
## Discoveries

- RTP timestamp scaling already exists and assumes video `FrameData::Video.timestamp` is **milliseconds**, converting to 90kHz via `scale_rtp_timestamp()` in streaming-lib; onvif-rust feeds ms timestamps unchanged from the daemon/SDK into `FrameData` (`timestamp_ms` stored everywhere). This reduces risk of "timestamp unit mismatch" but means any change to timestamp units upstream would ripple.
- streaming-lib includes a `RtpTimestampNormalizer` and `VideoAccessUnitAssembler` in `server_session.rs` that corrects equal/regressed timestamps and coalesces same-timestamp chunks; this is relevant when adjusting marker-bit logic because the library expects a full access unit per `on_frame` call but has mitigation if publishers split NALs.
- Marker-bit behavior in the H.264 packer is currently "last VCL NAL in frame" (not last NAL overall). This can miss marking AU end if non-VCL trails after VCL; changing to last NAL index is the simplest interop improvement.
- RTCP NTP generation currently **panics** on invalid wall-clock due to `expect("System time before Unix epoch")`, making SR sending unsafe on embedded devices without time set. User decision: skip SR until time valid.
- Vendor-daemon overflow drops generate a "frame dropped" notification; Rust currently logs at warn and returns `PlatformError::ResourceBusy`, but `is_push_mode_transient_error()` treats `ResourceBusy` as transient so the main push loop continues. Changing severity/behavior should preserve transience so push loop doesn't break.
- SDP `profile-level-id` is derived by naive SPS byte indexing (`sps[1..3]`) in onvif-rust; this was flagged earlier as a potential interop risk but was not included in the fix plan (don't accidentally scope-creep into RBSP parsing).

## Relevant Files

- `cross-compile/streaming-lib/src/rtsp/rtp/utils.rs`
  - `ntp_timestamp()` currently panics on invalid system time; also holds Annex-B start-code finder (`find_start_code`).
- `cross-compile/streaming-lib/src/rtsp/rtp/rtcp/rtcp_context.rs`
  - SR generation (`generate_sr`) extrapolates RTP timestamp vs wallclock; will need to handle missing NTP.
- `cross-compile/streaming-lib/src/rtsp/rtsp_track.rs`
  - RTCP SR periodic send loop (5s interval); must skip when SR unavailable.
- `cross-compile/streaming-lib/src/rtsp/rtp/rtp_h264.rs`
  - H.264 packetization (Annex-B split, FU-A, marker-bit decisions) and existing marker-related tests.
- `cross-compile/streaming-lib/src/rtsp/session/server_session.rs`
  - Timestamp scaling (`scale_rtp_timestamp`), `RtpTimestampNormalizer`, `VideoAccessUnitAssembler`, RTP-Info builder; useful context when validating downstream behavior after marker changes.
- `cross-compile/onvif-rust/src/streaming/service.rs`
  - On subscriber connect, sends `MediaInfo` and combines SPS+PPS(+IDR) into one Annex-B AU to avoid DTS collisions; relevant to marker-bit expectations.
- `cross-compile/onvif-rust/src/streaming/helpers.rs`
  - SDP generator (payload types hardcoded 96/97; packetization-mode=1; naive profile-level-id extraction).
- `cross-compile/onvif-rust/src/hal/vendor_ipc.rs`
  - Drop notification handling currently returns `PlatformError::ResourceBusy` with warn log.
- `cross-compile/onvif-rust/src/platform/anyka.rs`
  - Push loop uses `recv_pushed_frame()`; treats `Timeout|ResourceBusy` as transient and continues; non-transient breaks the loop.
- `cross-compile/vendor-daemon/include/vd_ring_buffer.h` and `cross-compile/vendor-daemon/src/main.c`
  - IPC/ring structures and overflow-drop behavior; helpful if adjusting semantics around dropped-frame notifications.

## Implementation Notes

- Use the vendored cargo: `../../toolchain/arm-anykav200-crosstool-ng/bin/cargo` from `cross-compile/` for fmt/clippy/test; earlier attempt used a wrong toolchain path.
- Keep vendor drop handling transient; returning a non-transient `PlatformError` will break the push loop (`anyka.rs` logs and `break` on non-transient).
- If making `generate_sr()` fallible (Option/Result), propagate changes carefully through RTCP send loop to avoid altering packet counters or session state unexpectedly.
- H.264 packer marker behavior is covered by existing tests; extend tests to include "VCL then trailing non-VCL" AU so the change is locked in.
- streaming-lib's timestamp normalizer expects strictly monotonic output timestamps and will correct regressions; ensure RTCP SR timestamp correlation logic still uses the "scaled" timestamps consistently after any refactor.
---

## Todo List

- [x] Load required project memory docs for review workflow
- [x] Map RTSP/H264/UDP code paths in onvif-rust and vendor-daemon
- [x] Run deep RFC compliance review for onvif-rust/streaming-lib path
- [x] Run deep RFC compliance review for vendor-daemon IPC/frame metadata path
- [x] Run available quality checks relevant to reviewed components
- [x] Synthesize findings into full compliance review report with severity and file references
