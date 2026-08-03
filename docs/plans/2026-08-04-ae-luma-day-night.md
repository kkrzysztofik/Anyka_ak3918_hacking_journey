# AE Luma Day/Night AUTO Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Drive AUTO day/night from ISP `current_calc_avg_lumi`, falling back to `ain0` after three AE read failures.

**Architecture:** New daemon opcode returns one luma byte from the sole registered VI. `ImagingHalTrait::get_ae_luma` maps that to `Option<u8>`. `night_mode::tick` prefers AE, streaks failures, then uses existing ain0 classify/decide/apply. Forced modes unchanged.

**Tech Stack:** vendor-daemon C, onvif-rust (`ImagingHalTrait`, `night_mode`, `NightConfig`), vendored cargo, host tests `--target x86_64-unknown-linux-gnu`.

**Design:** `docs/plans/2026-08-04-ae-luma-day-night-design.md`

---

### Task 1: NightConfig AE thresholds

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/types.rs` (`NightConfig` / `Default`)
- Modify: `SD_card_contents/anyka_hack/onvif/config.toml` (`[imaging.night]`)
- Test: existing config deserialize tests in `types.rs` if any; else add one next to `NightConfig`

**Step 1: Write the failing test**

In `types.rs` tests (or add `#[cfg(test)]` module if missing), assert defaults deserialize:

```rust
#[test]
fn test_night_config_ae_thresholds_default() {
    let cfg = NightConfig::default();
    assert_eq!(cfg.ae_day_threshold, 80);
    assert_eq!(cfg.ae_night_threshold, 40);
}
```

(Placeholder defaults — Task 8 calibrates on hardware.)

**Step 2: Run test to verify it fails**

```bash
source ./setenv.sh
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu test_night_config_ae_thresholds_default -- --nocapture
```

Expected: FAIL (fields missing).

**Step 3: Minimal implementation**

Add to `NightConfig`:

```rust
/// At or above this AE `current_calc_avg_lumi`, treat as day.
pub ae_day_threshold: i32,
/// At or below this AE luma, treat as night.
pub ae_night_threshold: i32,
```

Defaults: `ae_day_threshold: 80`, `ae_night_threshold: 40`. Keep existing ain0 fields.

In `config.toml` under `[imaging.night]`:

```toml
# AE luma (0-255). PLACEHOLDER — calibrate on device (Task 8).
ae_day_threshold = 80
ae_night_threshold = 40
```

**Step 4: Run test to verify it passes**

Same command as Step 2. Expected: PASS.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/config/types.rs SD_card_contents/anyka_hack/onvif/config.toml
git commit -m "feat(config): add AE day/night luma thresholds"
```

---

### Task 2: ImagingHalTrait::get_ae_luma + stub

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/common/imaging.rs`
- Modify: `cross-compile/onvif-rust/src/hal/stub/imaging.rs`
- Test: `hal/common/imaging.rs` or stub tests

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_stub_get_ae_luma_returns_none() {
    let stub = StubImagingHal;
    assert!(stub.get_ae_luma().await.is_none());
}
```

**Step 2: Run — expect FAIL** (method missing).

```bash
$CARGO test --target x86_64-unknown-linux-gnu test_stub_get_ae_luma_returns_none -- --nocapture
```

**Step 3: Minimal implementation**

On trait:

```rust
async fn get_ae_luma(&self) -> Option<u8>;
```

Stub: `async fn get_ae_luma(&self) -> Option<u8> { None }`

mockall regenerates `MockImagingHalTrait` via `#[cfg_attr(test, mockall::automock)]`.

**Step 4: Run — expect PASS.** Fix any compile breaks from missing impls.

**Step 5: Commit**

```bash
git commit -am "feat(hal): add ImagingHalTrait::get_ae_luma"
```

---

### Task 3: Daemon CMD_ISP_GET_AE_LUMA

**Files:**
- Modify: `cross-compile/vendor-daemon/src/protocol.h`
- Modify: `cross-compile/vendor-daemon/src/handlers_isp.h`
- Modify: `cross-compile/vendor-daemon/src/handlers_isp.c`
- Modify: `cross-compile/vendor-daemon/src/dispatcher.c`
- Modify: `cross-compile/vendor-daemon/src/globals.h` / `globals.c` only if a tiny `vd_obj_first(kind)` helper is cleaner than an inline loop (prefer inline in handler — ponytail)

**Step 1: No C unit harness — skip to implement, verify with `make release`**

**Step 2: Implementation**

`protocol.h`:

```c
CMD_ISP_GET_AE_LUMA = 106,
```

`handlers_isp.c` — new handler:

```c
int handle_isp_get_ae_luma(int fd, const uint8_t *req, uint32_t req_len)
{
    (void)req;
    (void)req_len;
    void *vi = NULL;
    int i;
    for (i = 0; i < VD_OBJ_SLOTS; i++) {
        if (g_obj_slots[i].live && g_obj_slots[i].kind == VD_OBJ_KIND_VI) {
            vi = g_obj_slots[i].ptr;
            break;
        }
    }
    if (vi == NULL)
        return send_response(fd, STATUS_ERROR, NULL, 0);

    struct vpss_isp_ae_run_info info;
    memset(&info, 0, sizeof(info));
    if (ak_vpss_isp_get_ae_run_info(vi, &info) != 0)
        return send_response(fd, STATUS_ERROR, NULL, 0);

    uint8_t luma = info.current_calc_avg_lumi;
    return send_response(fd, STATUS_OK, &luma, 1);
}
```

Need `g_obj_slots` / `VD_OBJ_*` visible — include `globals.h`. Confirm `g_obj_slots` is declared extern there; if not, add a one-line helper `vd_obj_first_vi(void **out)` in globals to avoid leaking the array (prefer helper if array is static — check; it is global in globals.c, declared in globals.h).

Wire in `dispatcher.c` next to other ISP cases.

**Step 3: Build**

```bash
make -C cross-compile/vendor-daemon release
```

Expected: success.

**Step 4: Commit**

```bash
git add cross-compile/vendor-daemon
git commit -m "feat(vendor-daemon): CMD_ISP_GET_AE_LUMA from sole VI"
```

---

### Task 4: AnykaIpc get_ae_luma + fake-daemon test

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs` (opcode const + name table)
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/imaging.rs`
- Test: `imaging.rs` or `ipc/mod.rs` tests via `FakeDaemon`

**Step 1: Failing test**

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_ae_luma_roundtrip() {
    let daemon = test_helpers::FakeDaemon::start(|cmd_id, req| {
        assert_eq!(cmd_id, CMD_ISP_GET_AE_LUMA);
        assert!(req.is_empty());
        (AK_SUCCESS_I32, vec![42u8])
    });
    let ipc = /* attach like other imaging tests */;
    assert_eq!(ipc.get_ae_luma().await, Some(42));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_ae_luma_error_is_none() {
    let daemon = test_helpers::FakeDaemon::start(|_c, _r| (AK_FAILED_I32, vec![]));
    // ...
    assert_eq!(ipc.get_ae_luma().await, None);
}
```

Copy attach pattern from `test_set_brightness_roundtrip` in `imaging.rs`.

**Step 2: Run — expect FAIL.**

**Step 3: Implement**

```rust
const CMD_ISP_GET_AE_LUMA: i32 = 106;

async fn get_ae_luma(&self) -> Option<u8> {
    match self.request_async(CMD_ISP_GET_AE_LUMA, &[]).await {
        Ok((status, data)) if status == 0 && !data.is_empty() => Some(data[0]),
        Ok(_) | Err(_) => None,
    }
}
```

Use the same success constant other getters/setters use (`AK_SUCCESS_I32` / `0`).

**Step 4: Run — PASS.**

**Step 5: Commit**

```bash
git commit -am "feat(ipc): get_ae_luma opcode 106"
```

---

### Task 5: tick() AE-first + fail streak (TDD)

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`
- Test: same file `mod tests`

**Step 1: Failing tests**

Add `ae_fail_streak: AtomicU32` to controller (or test via public tick behavior only).

```rust
#[tokio::test]
async fn test_tick_uses_ae_luma_when_available() {
    // tempdir with ain0=999 (would be day on ain0 thresholds)
    // ae thresholds: day=80 night=40; mock get_ae_luma → Some(10) → Night
    // expect_set_ir_filter / GPIO night side effects OR assert current_mode() == Night
}

#[tokio::test]
async fn test_tick_falls_back_to_ain0_after_three_ae_failures() {
    // get_ae_luma → None × 3; ain0 dark; expect night apply on 3rd tick
}

#[tokio::test]
async fn test_tick_clears_streak_on_ae_success() {
    // None, None, Some(day_luma) — should not have switched on ain0 in between if ain0 was night
}
```

Mirror existing tempdir + MockImagingHalTrait patterns in this file (`expect_set_ir_filter`, `expect_get_ae_luma`).

**Step 2: Run — FAIL.**

**Step 3: Implement tick**

```rust
const AE_FAIL_STREAK_MAX: u32 = 3;
// field: ae_fail_streak: AtomicU32

pub(crate) async fn tick(&self) {
    if !self.auto_enabled.load(Ordering::SeqCst) {
        return;
    }

    let reading = match self.ffi.get_ae_luma().await {
        Some(luma) => {
            self.ae_fail_streak.store(0, Ordering::SeqCst);
            classify(
                luma as i32,
                Thresholds {
                    day: self.cfg.ae_day_threshold,
                    night: self.cfg.ae_night_threshold,
                    ldr_high_is_day: true, // AE high = bright = day
                },
            )
        }
        None => {
            let n = self.ae_fail_streak.fetch_add(1, Ordering::SeqCst) + 1;
            if n < AE_FAIL_STREAK_MAX {
                return;
            }
            let Some(raw) = read_light_sensor(&self.paths) else {
                return;
            };
            classify(
                raw,
                Thresholds {
                    day: self.cfg.day_threshold,
                    night: self.cfg.night_threshold,
                    ldr_high_is_day: self.cfg.ldr_high_is_day,
                },
            )
        }
    };

    let target = { /* existing decide */ };
    if let Some(target) = target
        && let Err(e) = self.apply(target).await
    {
        tracing::warn!(error = %e, "night-mode transition failed");
    }
}
```

Update `NightModeController::new` to init streak to 0. Fix existing mocks: add `.expect_get_ae_luma().returning(|| None)` (or `Some`) wherever tests construct MockImagingHalTrait so they keep compiling.

**Step 4: Run night_mode tests**

```bash
$CARGO test --target x86_64-unknown-linux-gnu night_mode -- --nocapture
```

Expected: all PASS.

**Step 5: Commit**

```bash
git commit -am "feat(night_mode): prefer AE luma with ain0 fallback"
```

---

### Task 6: Clippy + fmt quality gate

**Step 1:**

```bash
source ./setenv.sh
cd cross-compile/onvif-rust
$CARGO fmt
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu --lib
```

**Step 2:** Fix any issues.

**Step 3: Commit** if fmt touched files.

---

### Task 7: Build + deploy binaries to `.198`

**Step 1:** Rebuild vendor-daemon + onvif-rust (same flow as prior deploy).

**Step 2:** Upload via `nc` to `/tmp`, install under `/mnt/anyka_hack/...`, kill services, wait for anyka-init respawn.

**Step 3:** Verify PIDs and md5.

---

### Task 8: On-device AE calibration + AUTO verify

**Step 1:** Sample AE luma (temporary debug log in tick, or one-shot telnet after adding a tiny debug print — prefer reading via a one-off IPC if easier; else `tracing` at info for a few ticks then remove).

Measure:
- room uncovered
- dark-box whole front

Set `ae_day_threshold` slightly below bright, `ae_night_threshold` slightly above dark. Update device `config.toml` and tracked template.

**Step 2:** Restart onvif-rust. Dark-box → `IR_LED=1`. Uncover, wait `lock_time_ms` (consider lowering lock to 5000 for the test, then restore).

**Step 3:** Forced `IrCutFilter` ON/OFF on `VideoSource_1` still works.

**Step 4: Commit** calibrated thresholds + wiki one-liner if needed.

```bash
git commit -am "chore(config): calibrate AE luma thresholds on .198"
```

---

### Task 9: Ponytail-review the diff

Run ponytail-review on the branch diff vs design baseline. Cut anything that grew past the design. Commit shrinks if any.

---

## Execution handoff

After this plan is saved and committed, use **executing-plans** (or implement task-by-task with TDD). Do not expand scope into `set_ir_filter` VI-token fix unless a follow-up plan is written.
