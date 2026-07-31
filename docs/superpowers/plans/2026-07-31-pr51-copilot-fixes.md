# PR #51 Copilot Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve all seven Copilot comments on PR #51 with surgical patches (unregister-on-successful-close, stream register-failure cancel, stdlib include, `spawn_supervisor` `Result`, cam_exec IAC fix).

**Architecture:** Approach 1 from the design — call-site patches only. No shared cross-file helpers. C close handlers keep the object table entry when SDK close fails so `vd_obj_close_all` can reclaim; stream open failure uses the existing bounded-cancel worker in `handlers_venc.c` without unregister; Rust surfaces a missing peer-loss receiver as `PlatformError` and hard-fails platform startup.

**Tech Stack:** C99 vendor-daemon (uClibc ARMv5), Rust onvif-rust (vendored toolchain), Python3 cam_exec debug script.

**Spec:** `docs/superpowers/specs/2026-07-31-pr51-copilot-fixes-design.md`

## Global Constraints

- Land on branch `feat/vendor-daemon-restart-resilience` (PR #51); push when tasks complete.
- Unregister VI/VENC/AI/AENC **only** when the matching `ak_*_close` returns 0.
- Never call bare `ak_venc_cancel_stream` on the dispatch thread; use bounded cancel (`cancel_thread_fn` + timeout).
- No `expect()` / `unwrap()` on production paths in onvif-rust.
- Host verification only — no device Task 5 matrix.
- Do not address CodeRabbit findings in this plan.
- Always `source ./setenv.sh` from repo root; use `$CARGO`; clippy needs `PATH=$TOOLBIN:$PATH`.
- No file deletion without explicit user permission.

---

## File map

| File | Role |
|------|------|
| `cross-compile/vendor-daemon/src/globals.c` | Add `#include <stdlib.h>` |
| `cross-compile/vendor-daemon/src/handlers_venc.c` | Close unregister policy; stream register-failure cancel |
| `cross-compile/vendor-daemon/src/handlers_vi.c` | Close unregister policy |
| `cross-compile/vendor-daemon/src/handlers_audio.c` | AI/AENC close unregister policy |
| `cross-compile/onvif-rust/src/platform/anyka/supervisor.rs` | `PlatformAttachTarget::new` → `Result` |
| `cross-compile/onvif-rust/src/platform/anyka/mod.rs` | `spawn_supervisor` → `Result` |
| `cross-compile/onvif-rust/src/app.rs` | Hard-fail `StartupError::Platform` if spawn fails |
| `scripts/debugging/cam_exec.py` | IAC off-by-one |

---

### Task 1: `#include <stdlib.h>` in globals.c

**Files:**
- Modify: `cross-compile/vendor-daemon/src/globals.c` (includes near top)

**Interfaces:**
- Consumes: none
- Produces: explicit `malloc`/`free` declarations for `vd_cancel_stream_bounded`

- [ ] **Step 1: Add the include**

After the existing `#include <time.h>` line, ensure the block is:

```c
#include <pthread.h>
#include <stdlib.h>
#include <time.h>
```

(Order among the three system headers may match nearby files; `stdlib.h` must be present.)

- [ ] **Step 2: Rebuild daemon**

```bash
cd /home/kmk/dev/anyka-dev
make -C cross-compile/vendor-daemon clean
make -C cross-compile/vendor-daemon 2>&1 | grep -iE "warning|error"; echo "make_exit=${PIPESTATUS[0]}"
```

Expected: `make_exit=0`, no warning/error lines.

- [ ] **Step 3: Commit**

```bash
git add cross-compile/vendor-daemon/src/globals.c
git commit -m "$(cat <<'EOF'
fix(vendor-daemon): include stdlib.h for malloc in globals.c

Copilot #1: do not rely on transitive ak_global.h for malloc/free.

EOF
)"
```

---

### Task 2: Unregister only on successful SDK close

**Files:**
- Modify: `cross-compile/vendor-daemon/src/handlers_venc.c` — `handle_venc_close`
- Modify: `cross-compile/vendor-daemon/src/handlers_vi.c` — `handle_vi_close`
- Modify: `cross-compile/vendor-daemon/src/handlers_audio.c` — `handle_ai_close`, `handle_aenc_close`

**Interfaces:**
- Consumes: existing `vd_obj_resolve` / `vd_obj_unregister` / `ak_*_close`
- Produces: close handlers that keep table entries when close fails

No C unit tests; verify with rebuild. Leave `handle_venc_cancel_stream` unchanged (still unregisters before cancel).

- [ ] **Step 1: Update `handle_venc_close`**

Replace the close+unregister tail with:

```c
    log_debug("[venc] close handle=%p", handle);
    int ret = ak_venc_close(handle);
    if (ret == 0)
        vd_obj_unregister(VD_OBJ_KIND_VENC, handle);
    else
        log_warn("[venc] close failed ret=%d; keeping object tracked for reclaim", ret);
    return send_response(fd, ret, NULL, 0);
```

- [ ] **Step 2: Update `handle_vi_close`**

```c
    log_debug("[vi] close handle=%p", handle);
    int ret = ak_vi_close(handle);
    if (ret == 0)
        vd_obj_unregister(VD_OBJ_KIND_VI, handle);
    else
        log_warn("[vi] close failed ret=%d; keeping object tracked for reclaim", ret);
    return send_response(fd, ret, NULL, 0);
```

- [ ] **Step 3: Update `handle_ai_close` and `handle_aenc_close`**

Same pattern with `VD_OBJ_KIND_AI` / `VD_OBJ_KIND_AENC` and `[ai]` / `[aenc]` log tags.

- [ ] **Step 4: Rebuild**

```bash
make -C cross-compile/vendor-daemon clean
make -C cross-compile/vendor-daemon 2>&1 | grep -iE "warning|error"; echo "make_exit=${PIPESTATUS[0]}"
```

Expected: `make_exit=0`.

- [ ] **Step 5: Commit**

```bash
git add cross-compile/vendor-daemon/src/handlers_venc.c \
        cross-compile/vendor-daemon/src/handlers_vi.c \
        cross-compile/vendor-daemon/src/handlers_audio.c
git commit -m "$(cat <<'EOF'
fix(vendor-daemon): unregister objects only after successful SDK close

Failed close keeps the table entry so vd_obj_close_all can reclaim on
client loss (Copilot #2/#3/#5).

EOF
)"
```

---

### Task 3: Cancel stream when register fails after request_stream

**Files:**
- Modify: `cross-compile/vendor-daemon/src/handlers_venc.c` — add static helper; update `handle_venc_request_stream`

**Interfaces:**
- Consumes: `cancel_thread_fn`, `CANCEL_STREAM_TIMEOUT_SEC`, `ak_venc_request_stream`, `vd_obj_register`
- Produces: `cancel_untracked_stream(void *handle)` — bounded cancel without unregister

- [ ] **Step 1: Add static helper above `handle_venc_cancel_stream`**

Place after `cancel_thread_fn`, before `handle_venc_cancel_stream`:

```c
/*
 * cancel_untracked_stream - Bounded cancel for a stream that was never registered.
 *
 * Used when ak_venc_request_stream succeeded but vd_obj_register failed: the
 * SDK capture_thread must still be stopped. No vd_obj_unregister — the pointer
 * never entered the table.
 */
static void cancel_untracked_stream(void *handle)
{
    struct cancel_arg *ca = (struct cancel_arg *)malloc(sizeof(*ca));
    if (ca == NULL) {
        log_error("[venc] cancel_untracked: malloc failed ptr=%p", handle);
        return;
    }
    ca->handle = handle;
    ca->result = -1;
    ca->done = 0;

    pthread_t tid;
    if (pthread_create(&tid, NULL, cancel_thread_fn, ca) != 0) {
        log_error("[venc] cancel_untracked: pthread_create failed ptr=%p", handle);
        free(ca);
        return;
    }
    pthread_detach(tid);

    struct timespec deadline;
    clock_gettime(CLOCK_MONOTONIC, &deadline);
    deadline.tv_sec += CANCEL_STREAM_TIMEOUT_SEC;

    while (!__atomic_load_n(&ca->done, __ATOMIC_ACQUIRE)) {
        struct timespec now;
        clock_gettime(CLOCK_MONOTONIC, &now);
        if (now.tv_sec > deadline.tv_sec ||
            (now.tv_sec == deadline.tv_sec && now.tv_nsec >= deadline.tv_nsec)) {
            log_error("[venc] cancel_untracked timed out after %ds ptr=%p (leaking arg)",
                      CANCEL_STREAM_TIMEOUT_SEC, handle);
            return;
        }
        struct timespec ts = { .tv_sec = 0, .tv_nsec = 10 * 1000000L };
        nanosleep(&ts, NULL);
    }

    log_info("[venc] cancelled untracked stream ptr=%p ret=%d", handle, ca->result);
    free(ca);
}
```

- [ ] **Step 2: Call it from `handle_venc_request_stream`**

Replace the register-failure arm:

```c
    int slot = vd_obj_register(VD_OBJ_KIND_STREAM, stream_handle);
    if (slot < 0) {
        log_error("[venc] object table full; refusing open");
        cancel_untracked_stream(stream_handle);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_token_response(fd, vd_obj_token(slot));
```

- [ ] **Step 3: Rebuild**

```bash
make -C cross-compile/vendor-daemon clean
make -C cross-compile/vendor-daemon 2>&1 | grep -iE "warning|error"; echo "make_exit=${PIPESTATUS[0]}"
```

Expected: `make_exit=0`.

- [ ] **Step 4: Commit**

```bash
git add cross-compile/vendor-daemon/src/handlers_venc.c
git commit -m "$(cat <<'EOF'
fix(vendor-daemon): cancel stream if object-table register fails

Prevents leaking an SDK capture_thread when request_stream succeeds but
vd_obj_register fails (Copilot #4).

EOF
)"
```

---

### Task 4: `spawn_supervisor` returns `Result` (no `expect`)

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/supervisor.rs` — `PlatformAttachTarget::new`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/mod.rs` — `spawn_supervisor`
- Modify: `cross-compile/onvif-rust/src/app.rs` — hard-fail on spawn error
- Test: add test near platform tests / supervisor (see Step 1)

**Interfaces:**
- Consumes: `AnykaIpc::take_loss_rx() -> Option<Receiver<PeerLoss>>`
- Produces:
  - `PlatformAttachTarget::new(platform: Arc<AnykaPlatform>) -> PlatformResult<Self>`
  - `AnykaPlatform::spawn_supervisor(self: &Arc<Self>) -> PlatformResult<watch::Receiver<Availability>>`

- [ ] **Step 1: Write the failing test**

Add to `cross-compile/onvif-rust/src/platform/anyka/tests/platform_tests.rs` (or `supervisor.rs` `mod tests` if mocks are easier — prefer platform_tests with `with_mocked_hal`). Follow the existing `with_mocked_hal` setup in that file for video/audio mocks:

```rust
#[tokio::test]
async fn spawn_supervisor_fails_if_loss_receiver_already_taken() {
    let platform = Arc::new(AnykaPlatform::with_mocked_hal(
        // same mock args as neighboring tests in this file
        video_ffi,
        audio_ffi,
        None,
    ));
    assert!(
        platform.ipc().take_loss_rx().is_some(),
        "first take must succeed"
    );
    let err = platform
        .spawn_supervisor()
        .expect_err("second ownership of loss rx must fail without panicking");
    match err {
        PlatformError::InitializationFailed(msg) => {
            assert!(
                msg.contains("peer-loss") || msg.contains("already taken"),
                "got {msg}"
            );
        }
        other => panic!("expected InitializationFailed, got {other:?}"),
    }
}
```

Fill `video_ffi` / `audio_ffi` exactly as the nearest existing `with_mocked_hal` test in that file.

- [ ] **Step 2: Run test — expect FAIL (panic or compile error until API changes)**

```bash
cd /home/kmk/dev/anyka-dev && source ./setenv.sh
export HOST=x86_64-unknown-linux-gnu
cd cross-compile/onvif-rust
$CARGO test --target $HOST --lib -- spawn_supervisor_fails_if_loss_receiver_already_taken 2>&1 | tail -30
```

Expected: FAIL (current `expect` panics, or test does not compile until `spawn_supervisor` returns `Result`).

- [ ] **Step 3: Implement `PlatformAttachTarget::new`**

```rust
impl PlatformAttachTarget {
    pub fn new(platform: Arc<AnykaPlatform>) -> PlatformResult<Self> {
        let reports = platform.ipc().take_loss_rx().ok_or_else(|| {
            PlatformError::InitializationFailed(
                "peer-loss receiver already taken; supervisor must be spawned once".to_string(),
            )
        })?;
        Ok(Self {
            platform,
            reports: tokio::sync::Mutex::new(reports),
        })
    }
}
```

Ensure `PlatformError` / `PlatformResult` are already imported in `supervisor.rs`.

- [ ] **Step 4: Implement `spawn_supervisor`**

```rust
pub fn spawn_supervisor(self: &Arc<Self>) -> PlatformResult<watch::Receiver<Availability>> {
    let (tx, rx) = watch::channel(Availability::Unavailable);
    let target: Arc<dyn AttachTarget> = Arc::new(PlatformAttachTarget::new(Arc::clone(self))?);
    tokio::spawn(async move {
        run_supervisor(target, &tx).await;
    });
    Ok(rx)
}
```

- [ ] **Step 5: Wire `app.rs` hard-fail**

In `init_platform`, replace the infallible spawn with:

```rust
                    let platform = Arc::new(p);
                    let availability = platform.spawn_supervisor().map_err(|e| {
                        StartupError::Platform(format!(
                            "failed to start attach supervisor: {e}"
                        ))
                    })?;
                    tracing::info!(
                        "AnykaPlatform created; attach supervisor started (degraded until attached)"
                    );
                    return Ok(PlatformInit {
                        platform: Some(platform as Arc<dyn Platform>),
                        availability: Some(availability),
                    });
```

Do **not** treat this as degraded-continue (unlike `with_isp_config` failure).

- [ ] **Step 6: Run the new test + supervisor suite**

```bash
$CARGO test --target $HOST --lib -- spawn_supervisor_fails_if_loss_receiver_already_taken 2>&1 | tail -15
$CARGO test --target $HOST --lib -- supervisor 2>&1 | tail -20
```

Expected: all PASS.

- [ ] **Step 7: fmt + clippy + commit**

```bash
$CARGO fmt
PATH=$TOOLBIN:$PATH $CARGO clippy --target $HOST --lib --tests -- -D warnings 2>&1 | tail -5
$CARGO fmt --check && echo fmt-clean
cd /home/kmk/dev/anyka-dev
git add cross-compile/onvif-rust/src/platform/anyka/supervisor.rs \
        cross-compile/onvif-rust/src/platform/anyka/mod.rs \
        cross-compile/onvif-rust/src/app.rs \
        cross-compile/onvif-rust/src/platform/anyka/tests/platform_tests.rs
git commit -m "$(cat <<'EOF'
fix(platform): return Result from spawn_supervisor instead of expect

Missing peer-loss receiver is a programming error and hard-fails platform
init (Copilot #7).

EOF
)"
```

(If the test lives in `supervisor.rs` instead, stage that file.)

---

### Task 5: Fix cam_exec IAC off-by-one

**Files:**
- Modify: `scripts/debugging/cam_exec.py` — `negotiate()`

**Interfaces:**
- Consumes: none
- Produces: incomplete IAC sequences left in buffer for the next recv

- [ ] **Step 1: Fix the guard**

Change:

```python
        if i + 2 >= len(data) + 1:
            break
```

to:

```python
        if i + 2 >= len(data):
            break
```

Remove the now-redundant `if i + 2 < len(data) else 0` on the next line — after the guard, `i + 2 < len(data)` is always true:

```python
        cmd, opt = data[i + 1], data[i + 2]
```

- [ ] **Step 2: Sanity-check with a tiny inline Python assertion**

```bash
cd /home/kmk/dev/anyka-dev
python3 - <<'PY'
import importlib.util
from pathlib import Path
spec = importlib.util.spec_from_file_location("cam_exec", Path("scripts/debugging/cam_exec.py"))
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

class FakeSock:
    def __init__(self):
        self.sent = []
    def sendall(self, b):
        self.sent.append(b)

# Truncated IAC+WILL (missing opt) must not consume or reply
sock = FakeSock()
out = mod.negotiate(sock, bytes([65, mod.IAC, mod.WILL]))
assert out == b"A", out
assert sock.sent == [], sock.sent

# Complete WILL must refuse with DONT
sock = FakeSock()
out = mod.negotiate(sock, bytes([mod.IAC, mod.WILL, 1, 66]))
assert out == b"B", out
assert sock.sent == [bytes([mod.IAC, mod.DONT, 1])], sock.sent
print("cam_exec negotiate checks ok")
PY
```

Expected: `cam_exec negotiate checks ok`.

- [ ] **Step 3: Commit**

```bash
git add scripts/debugging/cam_exec.py
git commit -m "$(cat <<'EOF'
fix(scripts): require full 3-byte telnet IAC before negotiating

Truncated IAC+cmd fragments no longer synthesize opt=0 (Copilot #6).

EOF
)"
```

---

### Task 6: Host gates, push, reply on Copilot threads

**Files:** none (verification + PR hygiene)

- [ ] **Step 1: Full host gates**

```bash
cd /home/kmk/dev/anyka-dev && source ./setenv.sh
export HOST=x86_64-unknown-linux-gnu
export TOOLBIN=/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin

make -C cross-compile/vendor-daemon clean
make -C cross-compile/vendor-daemon 2>&1 | grep -iE "warning|error"; echo "make_exit=${PIPESTATUS[0]}"

cd cross-compile/onvif-rust
$CARGO test --target $HOST --lib 2>&1 | grep -E "test result:|FAILED" | head
$CARGO fmt --check && echo fmt-clean
PATH=$TOOLBIN:$PATH $CARGO clippy --target $HOST --lib --tests -- -D warnings 2>&1 | tail -3
```

Expected: `make_exit=0`, all test results ok, `fmt-clean`, clippy Finished with no warnings.

- [ ] **Step 2: Push branch**

```bash
cd /home/kmk/dev/anyka-dev
git push origin HEAD
```

- [ ] **Step 3: Reply on each Copilot thread**

Use `gh api` (or `gh pr comment` / reply endpoints) on PR #51 discussions:

| Finding | Reply gist |
|---------|------------|
| #1 stdlib | Fixed: `#include <stdlib.h>` in `globals.c`. |
| #2 venc close | Fixed: unregister only when `ak_venc_close` returns 0 so reclaim remains possible. |
| #3 vi close | Fixed: same policy for VI. |
| #4 stream register | Fixed: bounded cancel of untracked stream before returning error. |
| #5 audio close | Fixed: same policy for AI/AENC. |
| #6 cam_exec | Fixed: require `i+2 < len(data)` before consuming IAC. |
| #7 expect | Fixed: `spawn_supervisor` returns `Result`; app hard-fails with `StartupError::Platform`. |

Do not open or reply on CodeRabbit threads.

- [ ] **Step 4: Done**

Confirm `git status` clean and PR #51 shows the new commits.

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| #1 stdlib include | Task 1 |
| #2/#3/#5 unregister-on-success | Task 2 |
| #4 stream cancel on register fail | Task 3 |
| #7 Result / hard-fail / test | Task 4 |
| #6 cam_exec IAC | Task 5 |
| Host gates only | Task 6 |
| Push + Copilot replies | Task 6 |
| No CodeRabbit / no device matrix / no shared helpers | Global Constraints |

## Placeholder / consistency self-review

- No TBD/TODO left.
- `PlatformError::InitializationFailed` used consistently for missing loss RX.
- `StartupError::Platform` used for app hard-fail.
- `cancel_untracked_stream` does not unregister; `handle_venc_cancel_stream` still does.
