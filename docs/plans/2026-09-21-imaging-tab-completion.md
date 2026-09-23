# Imaging Tab Completion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make every control on the WebUI Imaging tab actually reach the camera's ISP, and add the exposure, white balance, hue, anti-flicker and live-preview capability the shipped SDK already supports.

**Architecture:** ONVIF SOAP carries everything ONVIF models (brightness…sharpness, IR cut, WDR, BLC, `WhiteBalance20`, `Exposure20`); a new REST `/api/imaging` carries the three knobs ONVIF does not model (hue, mains Hz, ISP style). All value mapping and validation lives in `onvif-rust` where the tests are; the C daemon only calls the SDK. No new libraries — every symbol needed is already linked.

**Tech Stack:** Rust (vendored armv5te toolchain, `mockall`, `tokio`), C99 (vendor-daemon, Anyka SDK), React 19 + TanStack Query + Vitest.

**Design doc:** `docs/plans/2026-09-21-imaging-tab-completion-design.md`

---

## Before you start

**Read these first:**
- `docs/plans/2026-09-21-imaging-tab-completion-design.md` — the spike findings and why each decision was made
- `AGENTS.md` — toolchain, naming, error handling, test naming
- Relevant skills: @anyka-embedded-build, @anyka-rust-testing, @vendor-daemon-ipc, @camera-webui-components, @anyka-webui-testing, @onvif-service-impl

**Toolchain setup — do this in every shell:**

```bash
cd /home/kmk/dev/anyka-dev
source ./setenv.sh
```

This exports `$CARGO` and puts the vendored toolchain first on `PATH`. System `cargo` will fail with version/target mismatches, and `cargo clippy` dies with E0514 without the PATH prefix.

**Command reference:**

```bash
# Rust host tests — run from cross-compile/onvif-rust/
$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt

# Rust ARM build — MUST run from cross-compile/onvif-rust/, not the workspace root,
# or cargo silently links with the host toolchain
cd cross-compile/onvif-rust && $CARGO build --release

# Daemon
cd cross-compile/vendor-daemon && make && make test

# WebUI — run from cross-compile/www/
pnpm test
pnpm verify
```

**Hardware:** `.198` is the reference camera (`192.168.2.198`). Shell via
`uv run python3 scripts/debugging/cam_exec.py --host 192.168.2.198 '<cmd>'`.
ONVIF is on **port 80**, not 8080. Credentials `admin:admin`.

**Two traps that will waste your afternoon:**
1. The camera log level defaults to `error`, so a `grep -c` returning 0 on a `warn!` proves nothing. Check `/mnt/logs/vendor_daemon.log` for SDK-level complaints — that is where the real rejections land.
2. `git status` in this repo usually shows files already staged. **Always pass an explicit pathspec to `git add`** or you will sweep unrelated binaries into your commit.

---

# Phase 1 — Make the existing six controls work

## Task 1: Fix the ONVIF→SDK effect mapping  `[COMPLETED]`

The SDK takes a **signed offset from the ISP tuning profile** in `[-50, 50]`, where 0 means "use the config file value". It hard-rejects anything outside that range. We currently send `(v/100)*255`, so every value ≥20 fails. The vendor's own ONVIF layer subtracts 50 (`ja_onvif.c:299`, `IMG_EFFECT_DEF_VAL = 50`).

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/common/imaging.rs:30` (delete `SDK_MAX_VALUE`), `:112-124` (the mapping), `:132,143,154,165` (call sites)
- Test: same file, `mod tests` — five existing assertions encode the *wrong* contract and must be rewritten

**Step 1: Rewrite the mapping test to assert the real contract**

Replace `test_onvif_to_sdk_value` (currently at `:213-223`) with:

```rust
    #[test]
    fn test_onvif_to_effect_value_maps_neutral_to_profile_default() {
        // The SDK treats 0 as "use the value from the ISP config file", so the
        // ONVIF midpoint must map to 0 and not to a register value.
        assert_eq!(onvif_to_effect_value(50.0), 0);
    }

    #[test]
    fn test_onvif_to_effect_value_maps_full_range_within_sdk_limits() {
        assert_eq!(onvif_to_effect_value(0.0), -50);
        assert_eq!(onvif_to_effect_value(100.0), 50);
        assert_eq!(onvif_to_effect_value(25.0), -25);
        assert_eq!(onvif_to_effect_value(75.0), 25);
    }

    /// Regression: every ONVIF value must land inside the SDK's accepted
    /// `[-50, 50]`. Sending 20.0 previously produced 51, which
    /// `isp_set_effect` rejects outright with AK_FAILED.
    #[test]
    fn test_onvif_to_effect_value_never_exceeds_sdk_range() {
        for percent in 0..=100 {
            let value = onvif_to_effect_value(percent as f32);
            assert!(
                (-50..=50).contains(&value),
                "ONVIF {percent} produced out-of-range SDK value {value}"
            );
        }
    }
```

**Step 2: Run the tests to verify they fail**

```bash
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu --lib onvif_to_effect_value
```

Expected: FAIL — `cannot find function 'onvif_to_effect_value'`.

**Step 3: Implement the mapping**

Delete `SDK_MAX_VALUE` at `:30` and replace `onvif_to_sdk_value` at `:112-124` with:

```rust
/// Convert an ONVIF parameter value (0.0-100.0) to an Anyka ISP effect offset.
///
/// The SDK does not take a register value. `ak_vpss_effect_set` accepts a
/// signed offset in `[-50, 50]` where **0 means "use the value in the ISP
/// config file"**, and `isp_set_effect` rejects anything outside that range
/// with `AK_FAILED` rather than clamping it. The vendor's own ONVIF layer
/// maps the two scales by subtracting the midpoint, and so do we.
///
/// See `docs/plans/2026-09-21-imaging-tab-completion-design.md`.
pub(crate) fn onvif_to_effect_value(onvif_value: f32) -> i32 {
    (onvif_value - ONVIF_MIDPOINT).round() as i32
}
```

Add next to `ONVIF_MAX`:

```rust
/// ONVIF midpoint, which the SDK represents as offset 0 ("ISP config default").
/// Mirrors the vendor's `IMG_EFFECT_DEF_VAL` (`libapp/src/onvif/ja_media.h:10`).
const ONVIF_MIDPOINT: f32 = 50.0;
```

Then update the four call sites (`:132,143,154,165`), each from
`let sdk_value = onvif_to_sdk_value(value, SDK_MAX_VALUE);` to
`let sdk_value = onvif_to_effect_value(value);`.

**Step 4: Fix the four FFI tests that assert the old register values**

At `:231` change `.with(eq(128))` to `.with(eq(0))` and the comment to `// 50.0 is the profile default`.
At `:279` change `.with(eq(255))` to `.with(eq(50))`.
At `:293` `.with(eq(0))` becomes `.with(eq(-50))` — 0.0 is now the floor, not the default.
At `:307` `.with(eq(64))` becomes `.with(eq(-25))`.

Read each test's body before editing; match the value to the input it passes, do not blanket-replace.

**Step 5: Run the full imaging test module**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib imaging
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

Expected: PASS, no warnings. Clippy will flag `SDK_MAX_VALUE` if you left it behind.

**Step 6: Commit**

```bash
git add cross-compile/onvif-rust/src/hal/common/imaging.rs
git commit -m "fix(imaging): map ONVIF values to the SDK's [-50,50] effect offset

isp_set_effect rejects anything outside [-50,50] with AK_FAILED, and treats
0 as 'use the ISP config file value'. We were sending (v/100)*255, so every
slider value at or above 20 failed on hardware. Match the vendor's own
mapping: subtract the 50 midpoint.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 2: Widen the platform imaging model to carry WDR and BLC levels  `[COMPLETED]`

WDR and backlight compensation are `bool` in the platform model (`traits.rs:294-296`), but ONVIF and the WebUI both carry mode **plus** a 0-100 level. The level currently has nowhere to go. WDR is also never applied at all: nothing calls `imaging_set_wdr`.

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/common/traits.rs:294-296`
- Modify: `cross-compile/onvif-rust/src/onvif/imaging/store.rs:582,593` (the two conversion sites)
- Modify: `cross-compile/onvif-rust/src/platform/anyka/imaging.rs` (`set_settings`)

**Step 1: Write the failing test**

In `cross-compile/onvif-rust/src/platform/common/traits.rs` `mod tests`:

```rust
    /// WDR and BLC carry a level in ONVIF, so the platform model must too —
    /// a bare bool silently discards whatever the operator set.
    #[test]
    fn test_imaging_settings_carries_wdr_and_blc_levels() {
        let settings = ImagingSettings {
            wdr: ToggleWithLevel { enabled: true, level: 70.0 },
            backlight_compensation: ToggleWithLevel { enabled: false, level: 30.0 },
            ..ImagingSettings::default()
        };
        assert!(settings.wdr.enabled);
        assert_eq!(settings.wdr.level, 70.0);
        assert_eq!(settings.backlight_compensation.level, 30.0);
    }
```

**Step 2: Run it to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_imaging_settings_carries
```

Expected: FAIL — `cannot find type 'ToggleWithLevel'`.

**Step 3: Implement**

Add near `ImagingSettings` in `traits.rs`:

```rust
/// An imaging feature that is either off, or on at a given strength.
///
/// ONVIF models WDR and backlight compensation as a mode plus a 0-100 level;
/// the SDK takes a single signed offset where 0 is the profile default. Keeping
/// both here means the level survives a round trip instead of being flattened
/// to a bool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToggleWithLevel {
    /// Whether the feature is active.
    pub enabled: bool,
    /// Strength, 0.0 to 100.0. Ignored when `enabled` is false.
    pub level: f32,
}

impl Default for ToggleWithLevel {
    fn default() -> Self {
        // 50.0 is the ONVIF midpoint, i.e. the ISP profile's own value.
        Self { enabled: false, level: 50.0 }
    }
}
```

Change the two fields at `:294-296`:

```rust
    /// Wide dynamic range.
    pub wdr: ToggleWithLevel,
    /// Backlight compensation.
    pub backlight_compensation: ToggleWithLevel,
```

**Step 4: Fix every compile error the change surfaces**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib 2>&1 | head -60
```

Work through each error. The known sites are `store.rs:582` (`if settings.wdr {` → `if settings.wdr.enabled {`, and pass `settings.wdr.level` into the `Level` field instead of the hardcoded value) and `store.rs:593` for BLC. Do not paper over an error by adding `.enabled` without checking whether that call site should be carrying the level through.

**Step 5: Run the full suite**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
```

Expected: PASS.

**Step 6: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/common/traits.rs cross-compile/onvif-rust/src/onvif/imaging/store.rs
git commit -m "refactor(imaging): carry WDR and BLC levels through the platform model

Both were bools, so the 0-100 level ONVIF and the WebUI send was discarded
before it could reach the SDK.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 3: Wire WDR through the effect path  `[COMPLETED]`

`handle_isp_set_wdr` is a `log_debug("no-op")` that returns OK (`handlers_isp.c:47-54`). But `VPSS_EFFECT_WDR` is a normal effect: `ak_vpss_effect_set` routes `type <= VPSS_EFFECT_WDR` straight to `isp_set_effect` in the already-linked `libplat_vi`. No new library, no new command — just stop lying.

**Files:**
- Modify: `cross-compile/vendor-daemon/src/dispatcher.c:351` (the `CMD_ISP_SET_WDR` case)
- Delete: `handle_isp_set_wdr` from `cross-compile/vendor-daemon/src/handlers_isp.c:46-54` and its declaration in `handlers_isp.h`
- Modify: `cross-compile/onvif-rust/src/hal/common/imaging.rs:180-186` and the `ImagingHalTrait::set_wdr` signature at `:76`
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/imaging.rs:79`, `cross-compile/onvif-rust/src/hal/stub/imaging.rs:34`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/imaging.rs` (`set_settings` — add the WDR apply)

**Step 1: Point the dispatcher at the generic effect handler**

In `dispatcher.c`, replace the `CMD_ISP_SET_WDR` case with:

```c
    case CMD_ISP_SET_WDR:
        ret = handle_isp_effect(fd, req_buf, req_len,
                                VPSS_EFFECT_WDR, "set_wdr");
        break;
```

Then delete `handle_isp_set_wdr` and its header declaration. Build:

```bash
cd cross-compile/vendor-daemon && make
```

Expected: builds clean. A leftover declaration will warn.

**Step 2: Write the failing Rust test**

In `hal/common/imaging.rs` `mod tests`, replace `test_imaging_set_wdr_calls_ffi_enabled` / `_disabled` (`:363-390`) with:

```rust
    #[tokio::test]
    async fn test_imaging_set_wdr_sends_effect_offset() {
        let mut mock_ffi = MockImagingHalTrait::new();
        mock_ffi
            .expect_set_wdr()
            .with(eq(20)) // 70.0 -> +20 offset from the profile default
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_wdr(70.0, &mock_ffi).await;
        assert!(result.is_ok());
    }

    /// WDR off must mean "profile default", not "minimum", so it cannot be
    /// mapped through the same offset as an explicit level.
    #[tokio::test]
    async fn test_imaging_set_wdr_disabled_sends_profile_default() {
        let mut mock_ffi = MockImagingHalTrait::new();
        mock_ffi
            .expect_set_wdr()
            .with(eq(0))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_wdr_disabled(&mock_ffi).await;
        assert!(result.is_ok());
    }
```

**Step 3: Run it to verify it fails**

```bash
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu --lib imaging_set_wdr
```

Expected: FAIL — signature mismatch, `imaging_set_wdr_disabled` not found.

**Step 4: Implement**

Change the trait method at `:76` from `async fn set_wdr(&self, enabled: bool) -> i32;` to
`async fn set_wdr(&self, level: i32) -> i32;`, then:

```rust
/// Apply a wide-dynamic-range level, as an ONVIF 0-100 value.
pub(crate) async fn imaging_set_wdr(value: f32, ffi: &dyn ImagingHalTrait) -> PlatformResult<()> {
    validate_onvif_range(value, "wdr level")?;
    let ret = ffi.set_wdr(onvif_to_effect_value(value)).await;
    check_result(ret, "imaging_set_wdr")
}

/// Return WDR to the ISP profile's own setting.
pub(crate) async fn imaging_set_wdr_disabled(ffi: &dyn ImagingHalTrait) -> PlatformResult<()> {
    let ret = ffi.set_wdr(0).await;
    check_result(ret, "imaging_set_wdr_disabled")
}
```

Update the IPC impl (`hal/anyka/ipc/imaging.rs:79`) and the stub (`hal/stub/imaging.rs:34`) to take `level: i32` and forward it as the i32 payload — the wire format is already `[i32 value]`, so only the parameter type changes.

**Step 5: Apply WDR in `set_settings`**

In `platform/anyka/imaging.rs::set_settings`, after the sharpness block, add:

```rust
        if current.wdr != settings.wdr {
            if settings.wdr.enabled {
                crate::hal::common::imaging::imaging_set_wdr(
                    settings.wdr.level,
                    self.ffi.as_ref(),
                )
                .await?;
            } else {
                crate::hal::common::imaging::imaging_set_wdr_disabled(self.ffi.as_ref()).await?;
            }
        }
```

**Step 6: Run tests, clippy, fmt**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt
```

**Step 7: Commit**

```bash
git add cross-compile/vendor-daemon/src/dispatcher.c cross-compile/vendor-daemon/src/handlers_isp.c cross-compile/vendor-daemon/src/handlers_isp.h cross-compile/onvif-rust/src/hal cross-compile/onvif-rust/src/platform/anyka/imaging.rs
git commit -m "fix(imaging): actually apply WDR instead of no-oping it

handle_isp_set_wdr logged 'no-op' and returned OK, and nothing in the
platform layer called imaging_set_wdr anyway. VPSS_EFFECT_WDR is a normal
effect, so route it through the generic handler like the other five.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 4: Add backlight compensation over the ISP SDK  `[COMPLETED]`

BLC has no `ak_vpss` equivalent — it needs `AK_ISP_get_blc_attr` / `AK_ISP_set_blc_attr` from `libakispsdk`. That is safe: `libplat_vi` holds 188 undefined `AK_ISP_*` references and the daemon already links `-lakispsdk` (`Makefile:96`), so the SDK is loaded and initialised in-process. **Do not call `AK_ISP_sdk_init`.**

**Read-modify-write is mandatory.** Constructing a fresh attr struct zeroes fields we do not model, which is worse than the feature being missing.

**Files:**
- Modify: `cross-compile/vendor-daemon/src/protocol.h` (add `CMD_ISP_SET_BLC = 111`)
- Modify: `cross-compile/vendor-daemon/src/handlers_isp.c`, `handlers_isp.h`, `dispatcher.c`
- Modify: `cross-compile/onvif-rust/src/hal/common/imaging.rs` (trait + helper), `hal/anyka/ipc/imaging.rs`, `hal/stub/imaging.rs`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/imaging.rs`

**Step 1: Add the command id**

In `protocol.h`, after `CMD_ISP_GET_AWB_STAT = 110`:

```c
    CMD_ISP_SET_BLC                = 111,
```

> **Superseded:** the wire protocol is append-only — slot 109 was never reclaimed; the AE commands were appended at 119-121 (120 GET_RUN_INFO is the permanent one; 119/121 were temporary and removed after the measurement, see docs/reference/anyka-ae-units.md).

**Step 2: Write the daemon handler**

In `handlers_isp.c`, include `ak_isp_sdk.h` (vendor it into `cross-compile/vendor-daemon/include/` from `cross-compile/anyka_reference/component/ispsdk_lib/ak_isp_sdk.h` if it is not already there) and add:

```c
/* CMD_ISP_SET_BLC. Wire format: [i32 level] = 4 bytes, SDK range [-50, 50]. */
int handle_isp_set_blc(int fd, const uint8_t *req, uint32_t req_len)
{
    AK_ISP_BLC_ATTR attr;
    int32_t level;

    if (req_len < 4) {
        log_warn("[isp] set_blc: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    level = req_read_i32(req, 0);

    /* Read-modify-write: a fresh struct would zero the tuning fields we do
     * not model, which is worse than leaving BLC alone. */
    if (AK_ISP_get_blc_attr(&attr)) {
        log_warn("[isp] set_blc: read failed; not writing");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    log_debug("[isp] set_blc level=%d", (int)level);
    return send_response(fd, AK_ISP_set_blc_attr(&attr), NULL, 0);
}
```

**Before writing the field assignment**, open the vendored `ak_isp_sdk.h` and read `AK_ISP_BLC_ATTR`'s real members. Set only the field that corresponds to strength and leave the rest as read. Do not guess the field name from this plan — it is deliberately left blank above.

Declare it in `handlers_isp.h`, and dispatch it in `dispatcher.c` beside the other ISP cases.

**Step 3: Build the daemon and its host tests**

```bash
cd cross-compile/vendor-daemon && make && make test
```

Expected: both clean.

**Step 4: Add the Rust side, test first**

Add to the trait: `async fn set_blc(&self, level: i32) -> i32;`. Then in `mod tests`:

```rust
    #[tokio::test]
    async fn test_imaging_set_blc_sends_effect_offset() {
        let mut mock_ffi = MockImagingHalTrait::new();
        mock_ffi.expect_set_blc().with(eq(-10)).times(1).returning(|_| AK_SUCCESS_I32);
        assert!(imaging_set_blc(40.0, &mock_ffi).await.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_blc_rejects_out_of_range() {
        let mock_ffi = MockImagingHalTrait::new();
        // Must fail validation before any IPC call; the mock expects none.
        assert!(imaging_set_blc(150.0, &mock_ffi).await.is_err());
    }
```

Run (expect FAIL), implement `imaging_set_blc` / `imaging_set_blc_disabled` mirroring the WDR pair from Task 3, wire the IPC and stub impls, and apply it in `set_settings` next to the WDR block.

**Step 5: Verify**

```bash
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

**Step 6: Commit**

```bash
git add cross-compile/vendor-daemon/src cross-compile/vendor-daemon/include cross-compile/onvif-rust/src
git commit -m "feat(imaging): apply backlight compensation via AK_ISP_set_blc_attr

BLC was persisted in the imaging store and never reached the SDK. It has no
ak_vpss equivalent, so go through libakispsdk, which the daemon already links
and libplat_vi already initialises. Read-modify-write so the untouched tuning
fields survive.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 5: Make `set_settings` validate before it applies  `[COMPLETED]`

`set_settings` (`platform/anyka/imaging.rs:216-242`) uses `?` between each knob. A rejected value aborts mid-batch, leaving the camera partially applied with *nothing* persisted — so the ISP and the store disagree and the UI shows neither. Now that more knobs are involved, validate everything first.

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/imaging.rs`
- Test: same file, `mod tests`

**Step 1: Write the failing test**

```rust
    /// A value the SDK will reject must be caught before anything is applied,
    /// so a bad request cannot leave the ISP half-configured.
    #[tokio::test]
    async fn test_set_settings_rejects_invalid_batch_without_applying() {
        let mut mock_ffi = MockImagingHalTrait::new();
        // No setter may be called at all.
        mock_ffi.expect_set_brightness().times(0);
        mock_ffi.expect_set_contrast().times(0);

        let imaging = imaging_with_mock(mock_ffi);
        let settings = ImagingSettings {
            brightness: 50.0,
            contrast: 150.0, // out of ONVIF range
            ..ImagingSettings::default()
        };

        assert!(imaging.set_settings(&settings).await.is_err());
    }
```

Follow the existing mock-construction pattern in that file's test module for `imaging_with_mock`; do not invent a new harness.

**Step 2: Run it to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_set_settings_rejects_invalid_batch
```

Expected: FAIL — brightness gets applied before contrast is validated.

**Step 3: Implement**

At the top of `set_settings`, before the day/night block:

```rust
        // Validate the whole batch up front. Applying knob-by-knob and
        // bailing on the first rejection leaves the ISP inconsistent with the
        // store, and the store is what the UI reads back.
        validate_onvif_range(settings.brightness, "brightness")?;
        validate_onvif_range(settings.contrast, "contrast")?;
        validate_onvif_range(settings.saturation, "saturation")?;
        validate_onvif_range(settings.sharpness, "sharpness")?;
        if settings.wdr.enabled {
            validate_onvif_range(settings.wdr.level, "wdr level")?;
        }
        if settings.backlight_compensation.enabled {
            validate_onvif_range(settings.backlight_compensation.level, "backlight level")?;
        }
```

Import `validate_onvif_range` from `crate::hal::common::imaging`. Leave the per-knob `?` in place — an IPC failure mid-batch is still possible and still an error; this only removes the *avoidable* partial applies.

**Step 4: Run tests**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
```

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka/imaging.rs
git commit -m "fix(imaging): validate the whole settings batch before applying any

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 6: Phase 1 hardware gate  `[COMPLETED]`

**Do not proceed to Phase 2 until this passes.** Everything so far is host-tested, and a green host suite is exactly what we had while the tab was broken.

**Step 1: Build and deploy**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/onvif-rust && $CARGO build --release
cd /home/kmk/dev/anyka-dev/cross-compile/vendor-daemon && make
```

Then deploy per @anyka-firmware-upgrade. Two things that will silently waste a deploy:
- `push_binary_dev` writes `/mnt/anyka_hack/onvif`, but a slots camera runs `slots/<active>/onvif`. Confirm the path matches the camera.
- Killing `onvif-rust` also kills the daemon; the pair restarts together. That is expected.

**Step 2: Verify the sliders across the full range**

```bash
cd /home/kmk/dev/anyka-dev
for v in 0 20 50 80 100; do
  printf "brightness=%s -> " $v
  cat > /tmp/set.xml <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
<s:Body><timg:SetImagingSettings><timg:VideoSourceToken>VideoSource_1</timg:VideoSourceToken><timg:ImagingSettings><tt:Brightness>$v</tt:Brightness></timg:ImagingSettings></timg:SetImagingSettings></s:Body>
</s:Envelope>
EOF
  curl -s -u admin:admin -m 15 -H 'Content-Type: application/soap+xml' \
    --data-binary @/tmp/set.xml http://192.168.2.198/onvif/imaging_service \
    | grep -qi fault && echo FAULT || echo ok
done
```

Expected: `ok` for all five. Any `FAULT` means the mapping is still wrong.

**Step 3: Confirm the SDK stopped complaining**

```bash
uv run python3 scripts/debugging/cam_exec.py --host 192.168.2.198 \
  'grep -c "value range" /mnt/logs/vendor_daemon.log'
```

Note the count before Step 2 and after. It must not increase.

**Step 4: Confirm the image actually changes**

Set brightness 0, grab a frame, set brightness 100, grab another, and compare mean luma — they must differ. Use the ffmpeg harness from @anyka-validation rather than eyeballing a stream.

Restore brightness to 50 when done.

**Step 5: Record the result**

Append the measured numbers to the design doc's Phase 1 gate section and commit. Evidence in the repo, not just in the transcript.

---

# Phase 2a — White Balance

## Task 7: Daemon commands for white balance  `[COMPLETED]`

`ak_vpss_isp_set_wb_type` (`WB_OPS_TYPE_MANU = 0`, `WB_OPS_TYPE_AUTO = 1`) and `ak_vpss_isp_set_mwb_attr` are both exported by the already-linked `libplat_vpss.so`.

**Files:**
- Modify: `cross-compile/vendor-daemon/src/protocol.h` (`CMD_ISP_SET_WB_TYPE = 112`, `CMD_ISP_SET_MWB_ATTR = 113`, `CMD_ISP_GET_MWB_ATTR = 114`)
- Modify: `handlers_isp.c`, `handlers_isp.h`, `dispatcher.c`

**Steps:** follow Task 4's shape exactly — `[i32]` wire format for wb_type; for mwb use read-modify-write via `ak_vpss_isp_get_mwb_attr`, setting only `r_gain` and `b_gain` and preserving `g_gain` and all three offsets. Wire format for the setter is `[u16 r_gain][u16 b_gain]`; the getter returns the raw `struct vpss_isp_mwb_attr` bytes and the daemon must not interpret them. Build with `make && make test` and commit.

## Task 8: ONVIF white balance plumbing  `[COMPLETED]`

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/common/imaging.rs` (trait + helpers), `hal/anyka/ipc/imaging.rs`, `hal/stub/imaging.rs`
- Modify: `cross-compile/onvif-rust/src/platform/common/traits.rs` (add white balance to `ImagingSettings`/`ImagingOptions`)
- Modify: `cross-compile/onvif-rust/src/onvif/imaging/ops/settings.rs:234,259` and `store.rs:606,700,719,772` — stop hardcoding `white_balance: None`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/imaging.rs`

`WhiteBalance20 { mode, cr_gain, cb_gain }` and `WhiteBalanceOptions20` already exist (`types/imaging.rs:178,504`) — this is plumbing, not new types.

Mapping: `Mode::AUTO` → wb_type 1; `Mode::MANUAL` → wb_type 0 plus an mwb write. `cr_gain` → `r_gain`, `cb_gain` → `b_gain`. Both scales are unitless multipliers, so no conversion constant — but **assert that in a test** rather than assuming it, and confirm on hardware in Task 9.

Test first, one test per behaviour:
- AUTO sends wb_type 1 and no mwb write
- MANUAL sends wb_type 0 and the two gains
- `GetImagingSettings` returns the gains read back from the device
- `GetOptions` advertises both modes

Then `$CARGO test`, `clippy`, `fmt`, commit.

## Task 9: White balance hardware gate + UI  `[COMPLETED]`

**GATE RESULT (2026-09-21, firmware `0152b67c`, camera `.198`): FAILED — no visible effect; investigation done, control marked unavailable.**

Measurements (`scripts/debugging/wb_gate.sh`): MANUAL with cr=3.0/cb=1.0 and cr=4.0/cb=1.0 left the frame's mean U/V identical to the AUTO baseline (U≈93, V≈54 both before and after; a real effect moves one of them by tens of counts in either units scale — 8.8 fixed-point or raw multiplier). The plumbing is verifiably live: `SetImagingSettings` is SOAP-OK, `GetImagingSettings` reads the gains back from the driver, and the daemon's `AK_ISP_get_mwb_attr` ioctl returns exactly the bytes written — so the values land in the kernel ISP instance's `mwb_para`/`wb_type_para` and are never applied to the pipeline.

Investigation trail (all source in `cross-compile/anyka_reference/`):
1. `ak_vpss_isp_set_wb_type/set_mwb_attr` (libplat_vpss) are pure passthroughs to the same `AK_ISP_*` SDK ioctls — switching layers changes nothing (Task 7's VPSS-preference deviation was therefore moot).
2. The SDK is a real kernel ioctl on `/dev/isp_char` (`component/ispsdk_lib/ak_isp_sdk.c`); the 3A manual branch (`if (WB_OPS_TYPE_MANU == isp->wb_type_para.wb_type)` in `component/ispdrv_lib/ak39_isp2_3a.c:1353`, GAIN_SHIFT=8) lives in the kernel ISP module (`[aec_*]` kernel threads on the camera), so write and 3A share one `isp` instance.
3. The camera's user-space `libplat_drv.so` is a 26 KB slim build containing none of the 3A code; the vendor's own aipc never touches white balance at all; the AWB attr struct has no auto/manual mode field and the ISP conf has no WB entry. The shipped kernel 3.4.35 is not built from the available UVC source tree (no `ISP:` printks in its ring buffer), so whether its 3A even contains the manual branch is unresolvable from the sources we have.

Decision per the gate's own rule and the plan's fallback ("mark it unsupported, document it, never fake it"): the ONVIF plumbing from Tasks 7–8 **stays** (it is an honest, spec-conformant surface: Set persists into the driver state the vendor exposes, Get reads it back, no in-memory echo), but the Imaging tab keeps its **"Unavailable" stub card** (disabled, no fake controls). If a future kernel/firmware applies the manual branch, the same SetImagingSettings call will start working with no plumbing change.

**Step 1 (gate, done — see above):** On `.198`, set MANUAL with a strongly red-biased `cr_gain`, grab a frame, confirm the colour cast is visible and in the expected direction. Then AUTO, confirm it corrects. A gain that changes nothing means the mapping or the units are wrong — stop and measure rather than shipping it.

**Step 2:** In `cross-compile/www/src/pages/settings/ImagingPage.tsx`, replace the stubbed White Balance card (`:423-450`) with a real mode select plus two gain sliders shown only in MANUAL. Add `getWhiteBalance`-style parsing to `cross-compile/www/src/services/imagingService.ts` following the existing `parseWideDynamicRangeSettings` pattern.

**Step 3:** Vitest per @anyka-webui-testing. Note the `radix-select-value-is-undefined-in-jsdom` trap: a Radix Select value set after mount reads back `undefined` in jsdom, so assert the mapping function directly and verify the select itself in Chromium.

```bash
cd cross-compile/www && pnpm test && pnpm verify
```

`pnpm verify` runs type-check, lint and format. Do **not** trust `rtk prettier --check` here — it has reported "All files formatted correctly" on a real exit-1; run the raw binary and read `$?`.

Commit.

---

# Phase 2b — Exposure, with calibration

## Task 10: Measure the AE units  `[COMPLETED]`

ONVIF mandates µs for exposure time and dB for gain. The device reports raw fixed-point and line units — `.198` currently reports `ae_a_gain_max: 16384`, `ae_exp_time_max: 2250`, `ae_target_luminance: 55`. Publishing those under ONVIF field names would lie to every third-party client.

**This task produces a measurement document, not production code.**

> **Superseded:** implemented as an appended `CMD_ISP_AE_SET_ATTR = 119` (append-only wire protocol); the set-attr command was temporary and has been removed.

**Step 1 (as done):** the AE attr was read via `CMD_ISP_GET_AE_ATTR = 108` for the options surface, read-modify-write via `ak_vpss_isp_get_ae_attr`, overriding only named fields. The cancelled implementation sketch in `docs/plans/2026-08-14-day-night-gaps.md:554-596` is a working starting point — reuse it rather than rewriting from scratch. **Never construct a fresh `vpss_isp_ae_attr`**: it would zero `hist_weight`, `envi_gain_range` and `target_lumiance`.

**Step 2:** Add `CMD_ISP_GET_AE_RUN_INFO` over `ak_vpss_isp_get_ae_run_info`, which returns `current_exp_time`, `current_a_gain`, `current_d_gain` and `current_darked_flag`. Surface it in `/api/diagnostics` next to the existing vision block.

**Step 3:** Write `scripts/debugging/measure_ae_units.py` that sweeps `exp_time_max` and `a_gain_max`, and after each step records the run-info readback plus achieved luma. Sweep under stable indoor light, not at dusk.

**Step 4:** Derive the conversions. Exposure time: get sensor fps via the frame interval and solve for µs-per-unit. Gain: dB = 20·log₁₀(ratio), with the fixed-point divisor determined from the sweep (16384 is Q8→64× or Q10→16×; the sweep tells you which).

**Step 5:** Write the findings to `docs/reference/anyka-ae-units.md` with the raw sweep table, the derived constants and the confidence in each. Commit.

**Gate:** if the sweep does not yield a consistent conversion, **stop**. Ship Task 11's AUTO/MANUAL only and leave the numeric limits out of ONVIF. Wrong units are worse than absent ones.

## Task 11: Exposure implementation  `[COMPLETED]`

`Exposure20` and `ExposureOptions20` already exist (`types/imaging.rs:138,301`).

**Step 1:** Mode only — `ExposureMode::AUTO`/`MANUAL` over `ak_vpss_isp_set_exp_type`. Test first, ship this regardless of Task 10's outcome.

**Step 2:** Only if Task 10 produced constants: add `MinExposureTime`/`MaxExposureTime`/`MinGain`/`MaxGain`, converting with the named constants from `docs/reference/anyka-ae-units.md`. Every conversion gets a unit test with a value from the measured table, cited by row.

**Step 3:** Replace the stubbed Exposure card (`ImagingPage.tsx:453-480`). Show the numeric limits only if the backend advertises them in `GetOptions` — the UI must degrade to mode-only rather than showing dead inputs, which is the defect we are removing.

**Step 4:** Hardware gate — MANUAL with a short max exposure visibly darkens a dim scene. Vitest, `pnpm verify`, commit.

---

# Phase 3 — Hue, anti-flicker, style

## Task 12: Daemon dispatch for the remaining effect types  `[COMPLETED]`

`handle_isp_effect` is already generic over `enum vpss_effect_type`; the dispatcher just hardwires 4 of the 8 values.

**Files:** `protocol.h` (`CMD_ISP_SET_HUE = 115`, `CMD_ISP_SET_POWER_HZ = 116`, `CMD_ISP_SET_STYLE_ID = 117`), `dispatcher.c`

Three cases, two lines each:

```c
    case CMD_ISP_SET_HUE:
        ret = handle_isp_effect(fd, req_buf, req_len, VPSS_EFFECT_HUE, "set_hue");
        break;
    case CMD_ISP_SET_POWER_HZ:
        ret = handle_isp_effect(fd, req_buf, req_len, VPSS_POWER_HZ, "set_power_hz");
        break;
    case CMD_ISP_SET_STYLE_ID:
        ret = handle_isp_effect(fd, req_buf, req_len, VPSS_STYLE_ID, "set_style_id");
        break;
```

`ak_vpss_effect_set` silently ignores a `VPSS_POWER_HZ` that is not exactly 50 or 60 (it `break`s without calling `isp_set_hz`), so **validate in Rust** — the daemon cannot report that rejection. `make && make test`, commit.

## Task 13: REST `/api/imaging`

**Files:**
- Create: `cross-compile/onvif-rust/src/http/imaging.rs` (match the module layout of the existing `/api/network` handler)
- Modify: the router, `cross-compile/onvif-rust/src/config/types.rs` (`[imaging]` keys)

`GET` returns the three current values; `PUT` accepts them. Validation in Rust: hue 0-100 (mapped with `onvif_to_effect_value`), `power_hz` ∈ {50, 60}, `style_id` ∈ 0..=2.

Config keys get **code defaults, not a new required section** — per the `anyka-toml-config-key-is-a-rollback-hazard` finding, `deny_unknown_fields` makes a new section a hard parse error for the older `anyka-init` in the other A/B slot, which breaks rollback.

Test first: valid PUT applies; `power_hz = 55` is rejected with 400 and no IPC call; GET round-trips. Also add the auth-requirement entry so the endpoint is not open — check `onvif/auth_requirements.rs` for the pattern used by `/api/network`.

Commit.

## Task 14: UI for the three knobs  `[COMPLETED]`

Add an "Advanced" card to `ImagingPage.tsx`: hue slider, anti-flicker select (50 Hz / 60 Hz), style select. New `cross-compile/www/src/services/imagingAdvancedService.ts` using the REST client, not the SOAP one.

Label anti-flicker for humans — "Mains frequency (reduces flicker under artificial light)", not "POWER_HZ". Vitest, `pnpm verify`, hardware check that 50↔60 visibly changes banding under a mains lamp, commit.

---

# Phase 4 — Live preview and honest cards

## Task 15: Apply on commit, not on drag  `[COMPLETED]`
Every apply calls `mark_imaging_update_and_request_idr`. Per the `main-keyframes-exceed-shm-slot` finding, a 184KB main IDR does not fit the 131008-byte ring slot, so an IDR per drag tick would hammer the main stream.

**Files:** `cross-compile/www/src/pages/settings/ImagingPage.tsx`

Radix Slider exposes `onValueCommit`, which fires once on release. Keep `onValueChange` for the local display value, and move the mutation to `onValueCommit`.

**Step 1:** Write the failing Vitest — dragging a slider through several values fires exactly one mutation.

```bash
cd cross-compile/www && pnpm test ImagingPage
```

Expected: FAIL (one per tick, or zero).

**Step 2:** Implement. **Step 3:** Re-run. **Step 4:** Commit.

## Task 16: Live preview  `[COMPLETED]`
Reuse `cross-compile/www/src/components/common/LiveVideoPlayer.tsx` — it already handles auth, CORS and late joiners, so no Rust is needed. Place it in a sticky column beside the cards on `lg:` and above; stack it on mobile.

Measure the layout headless before hand-tuning CSS (jsdom sees no layout): cached chromium + playwright-core against the dev server, per the `webui-layout-can-be-measured-headless` finding. Assert `scrollHeight` rather than guessing.

Vitest, `pnpm verify`, commit.

## Task 17: Honest cards  `[COMPLETED]`
- Fold Sharpness into "Color & Brightness"; delete the "Focus & Sharpness" card. The device has no motorised focus — `GetMoveOptions` returns empty defaults — so the title promises hardware that does not exist.
- Give the illumination switches real state: `/api/diagnostics` already returns `ir_led` and `white_led`. Replace `useState(false)` (`:86-87`) with the fetched values so they survive a reload.
- Mention in the Illumination card's description that white light is the effective night illuminator on this hardware: 110 vs 3 YAVG against the IR lamp indoors.

Vitest for the switch read-back, `pnpm verify`, commit.

---

## Definition of done  `[IN PROGRESS]`

> **Open finding (bdc08439 firmware, 2026-09-21):** the definition-of-done gate
> (`scripts/debugging/imaging_dod_gate.sh`) passed the 20-call sweep (all HTTP 200) and
> the brightness luma delta (0 -> 33.9, 100 -> 212.2, delta 178), but **WDR ON returned a
> receiver fault (500)** while WDR OFF succeeded.
>
> **Root cause (2026-09-23, on `.198` at firmware 31b9c554):** the 500 was
> `s:Receiver / ter:HardwareFailure / "Hardware query failed"` — an `ImagingSettingsError::PlatformError`.
> The WDR *set* path (`store.set_settings` → platform `set_settings` →
> `imaging_set_wdr`) drives the vendor daemon's `ak_vpss_effect_set(VPSS_EFFECT_WDR)`,
> which routes to the **closed** `libplat_vi`/`libakispsdk` `AK_ISP_set_wdr_attr`. That
> symbol is linked (not a stub) but the GC1084 driver **rejects it at runtime**, so the
> daemon returns non-zero. WDR "OFF" only ever returned 200 because the
> `current.wdr != settings.wdr` diff-guard skipped an already-off value — it never
> touched the daemon. Every ON level (0/50/80/100 → SDK −50…+50) failed identically.
> So WDR is genuinely **not supported on this sensor build** — the plan's "or mark as
> unavailable" branch.
>
> **Fix:** `store.validate_settings` now rejects a `WideDynamicRange` element when
> `GetOptions` does not advertise it (platform `wdr_supported=false`) with a clean
> `400 InvalidArgVal` ("WideDynamicRange is not supported on this device") instead of
> driving the failing SDK path into a 500. `GetOptions` already omits WDR
> (`wdr_supported` defaults false); the daemon/ONVIF plumbing is kept for a sensor that
> does support it. Unit test `test_validate_settings_wdr_unsupported_is_rejected` proves
> the 400; the gate now asserts WDR is *marked unavailable* (not advertised + clean 400
> on set) as its pass criterion. Host-verified (2340 tests, clippy, fmt clean).
>
> **On-hardware re-verification is blocked** by a pre-existing `.198` fault unrelated
> to imaging: the 64 KB `/etc/jffs2` is ~94 % full so anyka-init's wifi-config write
> fails (ENOSPC) → `wpa_supplicant` crash-loops (exit 2) → anyka-init requests shutdown
> and the supervised stack never comes up. The review-fixed bundle is built and staged
> (`active=a`, `f43da754-dirty`); re-run the gate once the camera boots cleanly.
> Also still open from the DoD list: the `value range` no-increase check, BLC frame
> visibility, and the anti-flicker 50/60 banding measurement (all need the camera).

- [ ] `SetImagingSettings` succeeds for brightness/contrast/saturation/sharpness at 0, 20, 50, 80 and 100 on `.198`
- [ ] `grep -c "value range" /mnt/logs/vendor_daemon.log` does not increase across a full slider sweep
- [ ] Brightness 0 and 100 produce measurably different mean luma
- [ ] WDR is marked unavailable (not in GetOptions + clean 400 on set) **or** BLC changes are visible in the frame
- [ ] White balance MANUAL produces the expected colour cast; AUTO corrects it
- [ ] Exposure ships mode-only, or with limits backed by `docs/reference/anyka-ae-units.md`
- [ ] Anti-flicker 50↔60 visibly changes banding under a mains lamp
- [ ] No card on the tab is labelled "Unavailable" unless the backend actually reports it unsupported
- [ ] One slider drag produces one mutation
- [ ] `$CARGO test`, `$CARGO clippy -- -D warnings`, `pnpm test`, `pnpm verify`, `make test` all clean
- [ ] ARM release build succeeds **from `cross-compile/onvif-rust/`** (from the workspace root it silently links the host toolchain)
- [ ] Code review requested per @superpowers:requesting-code-review

## Rollout notes

- CI never cross-builds armv5te on PRs — armv5te lives only in `release.yml`. Build the ARM target locally before merging or you will leave `main` unbuildable for the camera.
- A new `[services.*]` binary never ships with its `anyka.toml` stanza; this plan adds no new service, but if that changes, append the stanza before upload and validate with `tomllib`.
- Audit the fleet's stored `anyka.toml` imaging values before rollout. 50 is the default everywhere, which maps to the ISP profile default and therefore to today's actual image — but a camera with a hand-edited value will visibly change once the mapping is correct.
