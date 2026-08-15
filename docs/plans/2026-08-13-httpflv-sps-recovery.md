# HTTP-FLV SPS Recovery Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refuse HTTP-FLV subscribe when SPS/PPS are missing (after a coalesced IDR kick), and stop that failure from killing the stream event loop.

**Architecture:** Mirror RTSP’s `send_information` IDR kick inside `send_prior_data` for `HttpFlvPull` only, return `Err` with existing `StreamHubError::Other`. Change hub `handle_subscribe_event` so prior-data `Err` returns `false` (oneshot drop still fails that subscribe; loop stays up).

**Tech Stack:** onvif-rust `LiveStreamHandler`, streaming-lib `StreamsHub`, mockall `MockIdrRequester`, tokio tests.

**Design:** `docs/plans/2026-08-13-httpflv-sps-recovery-design.md`

---

### Task 1: Hub — prior-data Err must not break the stream loop

**Files:**
- Modify: `cross-compile/streaming-lib/src/hub/mod.rs:569-575`
- Test: add in `cross-compile/streaming-lib/src/hub/tests.rs` (or `mod.rs` hub tests if that is where subscribe transceiver tests live)

**Step 1: Write the failing test**

Add a test that:
1. Publishes a stream with a handler whose `send_prior_data` returns `Err` once, then `Ok` on a second subscribe (or use two handlers / mock).
2. First subscribe fails (oneshot `RecvError` / subscribe `Err` is fine).
3. Second subscribe with a handler that succeeds still works — proves the event loop did not exit.

If existing hub tests lack a mock handler hook, the smallest proof is unit-testing `handle_subscribe_event` behavior by asserting the bool: extract is overkill — instead add an integration-style test around `StreamsHub` if one already publishes+subscribes; otherwise add a focused test next to `receive_event_loop` helpers.

Practical minimal test (prefer this if mocks are heavy): document the one-line change and add a unit test that calls a small extracted function — **ponytail:** do not extract. Change the line and add a comment referencing the bug; cover behavior from onvif-rust side + a streaming-lib test that spies via a custom `TStreamHandler`.

Use existing `hub/tests.rs` mock publisher pattern (`expect_send_prior_data`). Pattern:

```rust
#[tokio::test]
async fn test_subscribe_prior_data_error_does_not_kill_stream_for_later_subscriber() {
    // Mock: first send_prior_data -> Err, second -> Ok with empty prior
    // Publish stream, subscribe twice
    // First subscribe Err; second Ok
}
```

**Step 2: Run test — expect FAIL** (second subscribe fails or hangs because loop died)

```bash
source ./setenv.sh
cd cross-compile
rtk cargo test --target x86_64-unknown-linux-gnu -p streaming-lib --lib \
  test_subscribe_prior_data_error_does_not_kill_stream -- --nocapture
```

**Step 3: Minimal fix**

In `handle_subscribe_event`:

```rust
if let Err(err) = stream_handler
    .send_prior_data(sender.clone(), info.sub_type.clone())
    .await
{
    error!(error = %err, "receive_event_loop_send_prior_data_error");
    return false; // was true — dropping result_sender still fails this subscribe
}
```

**Step 4: Re-run test — expect PASS**

**Step 5: Commit**

```bash
git add cross-compile/streaming-lib/src/hub/mod.rs cross-compile/streaming-lib/src/hub/tests.rs
git commit -m "$(cat <<'EOF'
fix(streaming-lib): keep stream loop alive when prior-data fails

EOF
)"
```

---

### Task 2: RED — HttpFlvPull without SPS requests IDR and returns Err

**Files:**
- Test: `cross-compile/onvif-rust/src/streaming/service.rs` (`mod tests`)
- Modify (later): same file `send_prior_data`

**Step 1: Write failing tests**

```rust
#[tokio::test]
async fn test_send_prior_data_httpflv_missing_sps_requests_idr_and_errs() {
    let (bridge, handler) = make_main_handler();
    expect_idr_requests(&bridge, true, 1);
    let (frame_tx, mut frame_rx) = streaming_lib::frame_data_channel();
    let sender = DataSender::Frame { sender: frame_tx };

    let result = handler
        .send_prior_data(sender, SubscribeType::HttpFlvPull)
        .await;
    assert!(result.is_err());
    assert!(frame_rx.try_recv().is_err()); // no MediaInfo on refuse path
}

#[tokio::test]
async fn test_send_prior_data_httpflv_missing_sps_requests_idr_only_once() {
    let (bridge, handler) = make_main_handler();
    expect_idr_requests(&bridge, true, 1);
    for _ in 0..2 {
        let (frame_tx, _frame_rx) = streaming_lib::frame_data_channel();
        let sender = DataSender::Frame { sender: frame_tx };
        let _ = handler
            .send_prior_data(sender, SubscribeType::HttpFlvPull)
            .await;
    }
}
```

Keep existing `test_live_stream_handler_send_prior_data_rtsp_no_sps` green (still Ok + MediaInfo).

**Step 2: Run — expect FAIL** (currently Ok / no IDR)

```bash
source ./setenv.sh
cd cross-compile
rtk cargo test --target x86_64-unknown-linux-gnu -p onvif-rust --lib \
  test_send_prior_data_httpflv_missing_sps -- --nocapture
```

**Step 3: Commit test-only (optional TDD checkpoint)**

```bash
git add cross-compile/onvif-rust/src/streaming/service.rs
git commit -m "$(cat <<'EOF'
test(onvif-rust): RED HttpFlvPull refuses subscribe without SPS/PPS

EOF
)"
```

---

### Task 3: GREEN — implement HttpFlvPull refuse + IDR kick

**Files:**
- Modify: `cross-compile/onvif-rust/src/streaming/service.rs` (`send_prior_data`, ~134–210; optionally share kick helper with `send_information`)

**Step 1: Implement**

Before sending MediaInfo, when `HttpFlvPull` and SPS or PPS missing:

```rust
if matches!(sub_type, SubscribeType::HttpFlvPull) && (sps.is_none() || pps.is_none()) {
    tracing::error!(
        stream = stream_name,
        has_sps = sps.is_some(),
        has_pps = pps.is_some(),
        "HTTP-FLV subscribe refused: SPS/PPS missing"
    );
    if !self.idr_requested.swap(true, Ordering::Relaxed)
        && let Some(requester) = self.bridge.idr_requester.read().deref()
    {
        requester.request_idr(self.is_main);
    }
    return Err(StreamHubError {
        value: StreamHubErrorValue::Other(
            "http-flv: SPS/PPS not ready".into(),
        ),
    });
}
```

Use existing imports for `StreamHubError` / `StreamHubErrorValue` (already used in this module via streaming-lib). Prefer `String::into()` / `From<String>` if that is the local style.

Ponytail: if the IDR kick block duplicates `send_information`, extract one private `fn request_idr_once(&self)` used by both — only if it shortens.

When prior-data later succeeds with SPS present, ensure `idr_requested` is cleared (same as `send_information` success path) so a later regression can kick again:

```rust
if let (Some(sps), Some(pps)) = (sps, pps) {
    self.idr_requested.store(false, Ordering::Relaxed);
    // existing send_parameter_sets ...
}
```

**Step 2: Run tests**

```bash
rtk cargo test --target x86_64-unknown-linux-gnu -p onvif-rust --lib \
  streaming::service::tests -- --nocapture
```

Expected: all pass, including RTSP no-SPS still Ok.

**Step 3: Commit**

```bash
git add cross-compile/onvif-rust/src/streaming/service.rs
git commit -m "$(cat <<'EOF'
fix(onvif-rust): refuse HTTP-FLV subscribe until SPS/PPS cached

EOF
)"
```

---

### Task 4: Quality gates

**Step 1: streaming-lib**

```bash
source ./setenv.sh
cd cross-compile
rtk cargo fmt -p streaming-lib
rtk cargo test --target x86_64-unknown-linux-gnu -p streaming-lib --lib
rtk cargo clippy --target x86_64-unknown-linux-gnu -p streaming-lib -- -D warnings
```

**Step 2: onvif-rust**

```bash
rtk cargo fmt -p onvif-rust
rtk cargo test --target x86_64-unknown-linux-gnu -p onvif-rust --lib
rtk cargo clippy --target x86_64-unknown-linux-gnu -p onvif-rust -- -D warnings
```

**Step 3: Device smoke (optional, if 198 up)**

1. Restart or wait until main lacks SPS (hard to force); or temporarily verify logs on next cold boot.
2. `curl -u admin:admin -m 3 http://192.168.2.198:8080/live/main.flv` while SPS missing → connection fails/ends quickly (not 200/0-byte hang).
3. After IDR fills cache → FLV `FLV` magic + bytes; RTSP `/main` still works.

---

### Task 5: Final commit if fmt touched files

```bash
git status
# commit any fmt-only leftovers if needed
```
