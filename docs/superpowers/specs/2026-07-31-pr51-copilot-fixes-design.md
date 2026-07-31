# PR #51 Copilot Review Fixes — Design

**Date:** 2026-07-31  
**PR:** https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/pull/51  
**Review:** https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/pull/51#pullrequestreview-4831635329  
**Branch:** `feat/vendor-daemon-restart-resilience` (same PR; Approach 1 — surgical patches)

## Goal

Address all seven Copilot comments on the restart-resilience PR without extracting shared helpers or expanding into CodeRabbit findings.

## Decisions (locked)

| Topic | Choice |
|-------|--------|
| Close / unregister policy | **B** — unregister only when SDK close returns 0 |
| Delivery | Same PR #51 |
| Missing peer-loss receiver | `spawn_supervisor` returns `Result`; app init **hard-fails** |
| Verification | Host gates only (no device matrix) |
| Implementation style | Approach 1 — minimal call-site patches |

## Finding map

| # | File | Action |
|---|------|--------|
| 1 | `cross-compile/vendor-daemon/src/globals.c` | Add `#include <stdlib.h>` (do not rely on transitive `ak_global.h`) |
| 2 | `handlers_venc.c` `handle_venc_close` | Unregister only if `ak_venc_close` returns 0; warn otherwise |
| 3 | `handlers_vi.c` `handle_vi_close` | Same for VI |
| 4 | `handlers_venc.c` `handle_venc_request_stream` | On register failure after successful `ak_venc_request_stream`, bounded-cancel the stream (never registered → no unregister), then return error |
| 5 | `handlers_audio.c` `handle_ai_close` / `handle_aenc_close` | Unregister only on successful close; warn otherwise |
| 6 | `scripts/debugging/cam_exec.py` | Incomplete IAC: `if i + 2 >= len(data): break` |
| 7 | `supervisor.rs` / `mod.rs` / `app.rs` | Remove `expect`; `PlatformAttachTarget::new` and `spawn_supervisor` return `Result`; app uses `?` |

## Daemon lifecycle

### Close handlers

After a successful `vd_obj_resolve`:

```c
int ret = ak_*_close(handle);
if (ret == 0)
    vd_obj_unregister(KIND, handle);
else
    log_warn("... close failed ret=%d; keeping object tracked for reclaim", ret);
return send_response(fd, ret, NULL, 0);
```

**Rationale:** A non-zero SDK close means the object is still live. Keeping the table entry preserves reclaim via `vd_obj_close_all` on control-client loss. Client may also retry close with the same token.

**Out of scope for this change:** `handle_venc_cancel_stream` continues to unregister before cancel (token must die even if cancel times out). That is intentional and not the Copilot close-failure finding.

### Stream register failure (#4)

Mirror other open paths (VI/VENC/AI already `ak_*_close` on register failure). For streams:

1. `ak_venc_request_stream` succeeded → live stream pointer.
2. `vd_obj_register` fails → table full.
3. Run the existing bounded cancel in `handlers_venc.c` (`cancel_thread_fn` + `CANCEL_STREAM_TIMEOUT_SEC` wait) on that pointer **without** `vd_obj_unregister` (never entered the table).
4. Return `STATUS_ERROR`.

Do **not** call bare `ak_venc_cancel_stream` on the accept/dispatch thread (hang risk). Mild local duplication of the wait loop is acceptable; do not export `vd_cancel_stream_bounded` from `globals.c` in this change.

### Includes (#1)

Add `#include <stdlib.h>` to `globals.c` alongside `<pthread.h>` / `<time.h>`.

## Rust supervisor spawn (#7)

### API

- `PlatformAttachTarget::new(platform) -> Result<Self, PlatformError>` using the existing
  platform error type: map a missing receiver to a clear programming-error variant/message
  such as `"peer-loss receiver already taken; supervisor must be spawned once"`.
- `AnykaPlatform::spawn_supervisor(self: &Arc<Self>) -> Result<watch::Receiver<Availability>, PlatformError>`: build target with `?`, then `tokio::spawn(run_supervisor(...))` as today.

### App init

In `app.rs` Anyka path, after `Arc::new(p)`:

```rust
let availability = platform.spawn_supervisor()?;
```

A missing loss receiver fails `PlatformInit` with `Err` (hard fail). This is stricter than `with_isp_config` failure, which remains degraded — double-take of the loss channel is a programming bug, not an absent daemon.

### Tests

- Update constructors/call sites for `Result`.
- Add a focused test that a second `take_loss_rx` / failed `PlatformAttachTarget::new` returns `Err` rather than panicking.

Supervisor loop, S3 breaker behavior, and availability semantics after a successful spawn stay unchanged.

## cam_exec.py (#6)

Require a full three-byte IAC negotiation before consuming it:

```python
if i + 2 >= len(data):
    break
```

Refuse WILL→DONT and DO→WONT unchanged. No new test harness required for this debug script unless a tiny pure-function extract is trivial; correctness is the off-by-one fix itself.

## Verification

Host only:

1. `make -C cross-compile/vendor-daemon clean && make` (exit 0, no warnings)
2. Vendored toolchain from repo root `source ./setenv.sh`
3. `$CARGO fmt` / `fmt --check`
4. `PATH=$TOOLBIN:$PATH $CARGO clippy --target x86_64-unknown-linux-gnu --lib --tests -- -D warnings`
5. `$CARGO test --target x86_64-unknown-linux-gnu --lib`
6. `$CARGO doc --no-deps`

No device redeploy or Task 5 matrix for this follow-up.

## PR hygiene

- Commit and push to `feat/vendor-daemon-restart-resilience` (PR #51).
- Reply on each of the seven Copilot threads as **fixed** (including #2/#3/#5: unregister
  only on successful close so reclaim via `vd_obj_close_all` remains possible).
- Do not address CodeRabbit threads in this work.

## Non-goals

- Shared `vd_obj_close_and_unregister` helpers
- Exporting a single cancel helper across `globals.c` / `handlers_venc.c`
- Hardware re-verification
- Breaker / backoff tuning
- CodeRabbit findings (supervisor JoinHandle, zero-epoch samples, imaging test epochs, etc.)
