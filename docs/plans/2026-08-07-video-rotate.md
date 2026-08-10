# Image Flip (180° Rotate) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expose a 180°-rotate control through ONVIF's `VideoSourceConfiguration.Extension.Rotate` and a WebUI toggle, applied live to the running video pipeline and persisted to `profiles.toml`.

**Architecture:** vendor-daemon gets one new IPC command (`CMD_VI_SET_FLIP_MIRROR`) wrapping the vendor SDK's existing `ak_vi_set_flip_mirror()`. onvif-rust threads it through the same HAL/platform layers every other VI command uses, then exposes it via a new `VideoControl` platform trait mirroring the existing `ImagingControl` pattern — live apply on `SetVideoSourceConfiguration`, persisted in `profiles.toml`, and re-applied automatically every time the supervisor's attach sequence runs (cold boot or vendor-daemon-crash reattach alike).

**Tech Stack:** C (vendor-daemon), Rust 2021 (onvif-rust), TypeScript/React (Camera WebUI).

**Design doc:** `docs/plans/2026-08-07-video-rotate-design.md`

---

## Before you start

Source the vendored toolchain from repo root: `source ./setenv.sh`. All `cargo` commands below assume this, and use `--target x86_64-unknown-linux-gnu` for anything host-side (build, test, clippy) per `AGENTS.md`. Never target `armv5te-unknown-linux-uclibceabi` except for the final cross-compile sanity check in Task 14.

There is no C test harness for `vendor-daemon/src/handlers_*.c` (checked: `vendor-daemon/tests/` only has `test_ring_epoch.c`). Task 1 has no automated test step for that reason — it gets a manual on-device smoke check instead, folded into Task 14.

---

## Task 1: vendor-daemon — `CMD_VI_SET_FLIP_MIRROR`

**Files:**
- Modify: `cross-compile/vendor-daemon/src/protocol.h`
- Modify: `cross-compile/vendor-daemon/src/handlers_vi.h`
- Modify: `cross-compile/vendor-daemon/src/handlers_vi.c`
- Modify: `cross-compile/vendor-daemon/src/dispatcher.c`

**Step 1: Add the command ID**

In `protocol.h`, the VI block (1–7) is followed immediately by VPSS (8–9) and VENC (10–20) — no free numbers there. `CMD_AI_OPEN` starts at 50, leaving 21–49 unused. Add after `CMD_VENC_STOP_PUSH = 20`:

```c
    /* Video Input (continued — appended here, not in the VI block above,
     * because this is a wire protocol: renumbering existing commands would
     * break any client/daemon pair mid-upgrade). */
    CMD_VI_SET_FLIP_MIRROR        = 21,
```

**Step 2: Declare the handler**

In `handlers_vi.h`, add after `handle_vi_capture_off`:

```c
int handle_vi_set_flip_mirror(int fd, const uint8_t *req, uint32_t req_len);
```

**Step 3: Implement the handler**

In `handlers_vi.c`, add after `handle_vi_capture_off` (mirrors `handle_vi_capture_on`'s handle-resolution shape, extended with the two flag bytes):

```c
/**
 * handle_vi_set_flip_mirror - IPC handler for CMD_VI_SET_FLIP_MIRROR.
 *
 * Calls ak_vi_set_flip_mirror() on the VI device identified by the handle.
 *
 * Wire format: [u64 handle][u8 flip][u8 mirror] = 10 bytes.
 *
 * @param fd      Client socket file descriptor, used to send the response.
 * @param req     Request payload bytes (little-endian, layout described above).
 * @param req_len Length of @p req in bytes.
 * @return        0 on success, -1 on I/O error.
 */
int handle_vi_set_flip_mirror(int fd, const uint8_t *req, uint32_t req_len)
{
    if (req_len < sizeof(uint64_t) + 2) {
        log_warn("[vi] set_flip_mirror: req too short (%u)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    void *handle;
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &handle) != 0)
        return send_response(fd, VD_STATUS_STALE_EPOCH, NULL, 0);

    int flip = req[8];
    int mirror = req[9];

    log_debug("[vi] set_flip_mirror handle=%p flip=%d mirror=%d", handle, flip, mirror);
    int ret = ak_vi_set_flip_mirror(handle, flip, mirror);
    if (ret != 0)
        log_error("[vi] set_flip_mirror failed: %d", ret);
    return send_response(fd, ret, NULL, 0);
}
```

**Step 4: Register it in the dispatcher**

In `dispatcher.c`, add `CMD_VI_SET_FLIP_MIRROR` to `is_lifecycle_cmd()`'s switch (it mutates hardware state, same as `CMD_VI_SET_CHANNEL_ATTR`):

```c
    case CMD_VI_SET_CHANNEL_ATTR:
    case CMD_VI_SET_FLIP_MIRROR:
```

And add a case in `process_request()`'s dispatch switch, in the `/* --- Video Input --- */` block:

```c
    case CMD_VI_SET_FLIP_MIRROR:
        ret = handle_vi_set_flip_mirror(fd, req_buf, req_len);
        break;
```

**Step 5: Commit**

```bash
git add cross-compile/vendor-daemon/src/protocol.h cross-compile/vendor-daemon/src/handlers_vi.h cross-compile/vendor-daemon/src/handlers_vi.c cross-compile/vendor-daemon/src/dispatcher.c
git commit -m "feat(vendor-daemon): add CMD_VI_SET_FLIP_MIRROR"
```

---

## Task 2: onvif-rust HAL — IPC client wrapper

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs`
- Modify: `cross-compile/onvif-rust/src/hal/common/video.rs`
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/video.rs`
- Modify: `cross-compile/onvif-rust/src/hal/stub/video.rs`

**Step 1: Add the command constant and debug name**

In `hal/anyka/ipc/mod.rs`, add after `const CMD_VENC_STOP_PUSH: i32 = 20;`:

```rust
const CMD_VI_SET_FLIP_MIRROR: i32 = 21;
```

And in the debug-name match (near `CMD_VI_CAPTURE_OFF => "VI_CAPTURE_OFF",`):

```rust
            CMD_VI_SET_FLIP_MIRROR => "VI_SET_FLIP_MIRROR",
```

**Step 2: Add the trait method and safe wrapper**

In `hal/common/video.rs`, add to `VideoHalTrait` (after `vi_capture_off`):

```rust
    fn vi_set_flip_mirror(&self, handle: *mut c_void, flip: bool, mirror: bool) -> i32;
```

Add the wrapper function after `video_input_capture_off`:

```rust
/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_set_flip_mirror(
    handle: &VideoInputHandle,
    flip: bool,
    mirror: bool,
    ffi: &dyn VideoHalTrait,
) -> PlatformResult<()> {
    let ret = ffi.vi_set_flip_mirror(handle.as_ptr(), flip, mirror);
    check_result(ret, "ak_vi_set_flip_mirror")
}
```

**Step 3: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `hal/common/video.rs`, after `test_video_input_capture_off_ffi_failure`:

```rust
    #[test]
    fn test_video_input_set_flip_mirror_calls_ffi_with_correct_flags() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_set_flip_mirror()
            .withf(|handle, flip, mirror| handle.is_null() && *flip && *mirror)
            .times(1)
            .returning(|_, _, _| AK_SUCCESS_I32);

        let result = video_input_set_flip_mirror(&vi_handle, true, true, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_input_set_flip_mirror_propagates_error() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_set_flip_mirror()
            .times(1)
            .returning(|_, _, _| AK_FAILED_I32);

        let result = video_input_set_flip_mirror(&vi_handle, true, true, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_set_flip_mirror"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }
```

**Step 4: Run — confirm it fails to compile (trait not implemented yet)**

```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust --lib hal::common::video
```
Expected: compile error, `vi_set_flip_mirror` not a member of `VideoHalTrait` implementors (`AnykaIpc`, `StubVideoHal` don't implement it yet).

**Step 5: Implement `AnykaIpc::vi_set_flip_mirror`**

In `hal/anyka/ipc/video.rs`, add to the `impl VideoHalTrait for AnykaIpc` block, after `vi_capture_off`, and add `CMD_VI_SET_FLIP_MIRROR` to the `use super::{...}` import list at the top of the file:

```rust
    fn vi_set_flip_mirror(&self, handle: *mut c_void, flip: bool, mirror: bool) -> i32 {
        let handle_val = handle as u64;
        let mut req_data = handle_val.to_le_bytes().to_vec();
        req_data.push(flip as u8);
        req_data.push(mirror as u8);
        match self.send_request(CMD_VI_SET_FLIP_MIRROR, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "vi_set_flip_mirror IPC failed");
                AK_FAILED_I32
            }
        }
    }
```

**Step 6: Implement the stub**

In `hal/stub/video.rs`, add to `impl VideoHalTrait for StubVideoHal`, after `vi_capture_off`:

```rust
    fn vi_set_flip_mirror(&self, _handle: *mut c_void, _flip: bool, _mirror: bool) -> i32 {
        AK_SUCCESS_I32
    }
```

**Step 7: Run tests — confirm they pass**

```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust --lib hal::
```
Expected: PASS, including the two new tests.

**Step 8: Commit**

```bash
git add cross-compile/onvif-rust/src/hal
git commit -m "feat(onvif-rust): add vi_set_flip_mirror to the video HAL"
```

---

## Task 3: `VideoControl` platform trait

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/common/traits.rs`

**Step 1: Add the trait**

Add after the `ImagingControl` trait definition (after its closing brace, before `/// Implements the trivial field-accessor methods...`):

```rust
/// Video geometry control — currently just 180° flip/mirror.
///
/// Mirrors [`ImagingControl`]'s shape: live-apply on set, with the caller
/// responsible for persistence. Kept separate from `ImagingControl` because
/// it operates on the VI device, not the ISP.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait VideoControl: Send + Sync {
    /// Set 180° flip/mirror. `true` rotates the image 180°, `false` restores
    /// normal orientation. There is no intermediate state — the vendor VI API
    /// only exposes independent flip/mirror flags, and this crate always
    /// drives them together (see the design doc for why).
    async fn set_flip_mirror(&self, rotated: bool) -> PlatformResult<()>;
}
```

**Step 2: Add the trait method to `Platform`**

Add to the `Platform` trait, after `fn imaging_control(&self) -> Option<Arc<dyn ImagingControl>>;`:

```rust
    /// Get video geometry control interface (optional).
    fn video_control(&self) -> Option<Arc<dyn VideoControl>>;
```

Note: deliberately **not** added to `impl_platform_accessors!` — that macro assumes a same-named struct field, and `AnykaPlatform` derives this one from its existing `video_input` field instead of holding a separate field (Task 5). `StubPlatform` does get a real field (Task 6), implemented manually there too, for symmetry with how `imaging_control` already works.

**Step 3: Run — confirm every `Platform` impl now fails to compile**

```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo build --target x86_64-unknown-linux-gnu -p onvif-rust 2>&1 | grep "not all trait items implemented"
```
Expected: errors for `AnykaPlatform`, `StubPlatform`, `ValidationPlatform` — fixed in Tasks 5–6.

**Step 4: Commit**

Fold into Task 5's commit — this task alone doesn't compile.

---

## Task 4: `AnykaVideoInput` — apply/store the flip state

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/video_input.rs`

**Step 1: Add the field**

Add to `AnykaVideoInput`'s struct definition, after `channel_layout`:

```rust
    pub(super) rotated: AtomicBool,
```

Initialize it in `with_ffi` (the only constructor — `new()` calls `with_ffi`), in the `Self { ... }` literal, after `channel_layout: RwLock::new(...)`:

```rust
            rotated: AtomicBool::new(false),
```

**Step 2: Add the apply method and getter**

Add after `capture_off`, before `close_blocking`:

```rust
    /// Store the flip/mirror flag and, if the VI is currently open, apply it
    /// immediately. If the VI is closed (construction-time seed, or between
    /// attach cycles), the value is stored and picked up the next time
    /// `init_video_input()` reaches this point after `capture_on()`.
    ///
    /// Called from three places: the boot-time seed in
    /// `AnykaPlatform::with_isp_config`, the post-`capture_on` reapply in
    /// `AnykaPlatform::init_video_input` (both via direct calls on the
    /// concrete type), and `VideoControl::set_flip_mirror` (via the trait,
    /// for live ONVIF-triggered changes).
    pub(super) fn apply_flip_mirror(&self, rotated: bool) -> PlatformResult<()> {
        self.rotated.store(rotated, Ordering::SeqCst);

        if !self.opened.load(Ordering::SeqCst) {
            return Ok(());
        }

        let guard = self.handle.read();
        let handle = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("Video input not opened".to_string())
        })?;

        video_input_set_flip_mirror(handle, rotated, rotated, self.ffi.as_ref())
    }

    /// Current flip/mirror flag (may not yet be applied if the VI is closed).
    pub(super) fn rotated(&self) -> bool {
        self.rotated.load(Ordering::SeqCst)
    }
```

Add `video_input_set_flip_mirror` to the `use crate::hal::common::video::{...}` import list at the top of the file.

**Step 3: Implement `VideoControl`**

Add at the bottom of the file, after the `impl VideoInput for AnykaVideoInput` block:

```rust
#[async_trait]
impl crate::platform::VideoControl for AnykaVideoInput {
    async fn set_flip_mirror(&self, rotated: bool) -> PlatformResult<()> {
        self.apply_flip_mirror(rotated)
    }
}
```

**Step 4: Run — this file alone should now compile in isolation checks**

```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo check --target x86_64-unknown-linux-gnu -p onvif-rust 2>&1 | grep "video_input.rs"
```
Expected: no errors attributable to this file (the crate as a whole still fails — `AnykaPlatform` doesn't implement `video_control()` yet; that's Task 5).

**Step 5: Commit**

Fold into Task 5's commit.

---

## Task 5: `AnykaPlatform` — wire construction, boot seed, and reattach

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/mod.rs`

**Step 1: Add `video_control()` to `impl Platform for AnykaPlatform`**

Find the `impl Platform for AnykaPlatform` block (uses `impl_platform_accessors!()`). Add, right after the macro invocation:

```rust
    fn video_control(&self) -> Option<Arc<dyn VideoControl>> {
        Some(self.video_input.clone() as Arc<dyn VideoControl>)
    }
```

**Step 2: Thread `initial_rotated` through `with_isp_config`**

Change the signature (add the new parameter at the end, after `imaging_cfg`):

```rust
    pub fn with_isp_config(
        isp_config_path: Option<PathBuf>,
        ptz_enabled: bool,
        main_encoder: StreamOpenParams,
        sub_encoder: StreamOpenParams,
        imaging_cfg: crate::config::types::ImagingConfig,
        initial_rotated: bool,
    ) -> PlatformResult<Self> {
```

Right after `let video_input = Arc::new(AnykaVideoInput::with_ffi(...));` inside the constructor block, seed it (VI is not open yet, so this only stores the flag — see Task 4 Step 2):

```rust
            let _ = video_input.apply_flip_mirror(initial_rotated);
```

**Step 3: Reapply after every `capture_on()`, not just at construction**

In `init_video_input()`, right after Step 5's `capture_on()` block succeeds (after the `if let Err(e) = self.video_input.capture_on() { ... }` block, before the stabilization `sleep`), add:

```rust
        // Step 5.5: Reapply flip/mirror. VI state does not survive
        // close/reopen, so a vendor-daemon crash-and-reattach needs this
        // too, not just cold boot — which is why it lives here rather than
        // only in the constructor. Soft-fail: an upside-down stream is still
        // a working stream, so this must not abort the whole bring-up.
        if let Err(e) = self.video_input.apply_flip_mirror(self.video_input.rotated()) {
            tracing::warn!("Failed to reapply flip/mirror after capture_on: {}", e);
        }
```

**Step 4: Run — crate should compile now**

```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo check --target x86_64-unknown-linux-gnu -p onvif-rust
```
Expected: fails only on the two call sites of `with_isp_config` that need the new argument — `app.rs` (fixed in Task 10) and any test helper constructing `AnykaPlatform` directly (grep for `with_isp_config(` and fix each with `false` unless the test specifically exercises rotation).

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka/mod.rs cross-compile/onvif-rust/src/platform/anyka/video_input.rs cross-compile/onvif-rust/src/platform/common/traits.rs
git commit -m "feat(onvif-rust): add VideoControl and wire flip/mirror through AnykaPlatform bring-up"
```

---

## Task 6: `StubPlatform` / `ValidationPlatform` — `video_control()`

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/stub/mod.rs`

**Step 1: Add a minimal stub `VideoControl`**

Add near `create_imaging_control` (same file):

```rust
/// In-memory `VideoControl` for host-side / stub builds. No hardware to
/// flip, so this just remembers the last value it was given — enough for
/// ops-layer tests to exercise the live-apply path end to end.
#[derive(Default)]
struct StubVideoControl {
    rotated: std::sync::atomic::AtomicBool,
}

#[async_trait]
impl crate::platform::VideoControl for StubVideoControl {
    async fn set_flip_mirror(&self, rotated: bool) -> PlatformResult<()> {
        self.rotated
            .store(rotated, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}
```

**Step 2: Add the field to `StubPlatform` and populate it**

Add `video_control: Option<Arc<dyn VideoControl>>,` next to the existing `imaging_control: Option<Arc<dyn ImagingControl>>,` field. In the constructor where `imaging_control` is built (near line 247–264), add:

```rust
        let video_control = Some(Arc::new(StubVideoControl::default()) as Arc<dyn VideoControl>);
```

and include `video_control,` in the struct literal alongside `imaging_control,`.

**Step 3: `ValidationPlatform` delegation**

`ValidationPlatform` wraps an inner platform manually (see its existing `imaging_control` at line ~577). Add alongside it:

```rust
    fn video_control(&self) -> Option<Arc<dyn VideoControl>> {
        self.inner.as_ref().video_control()
    }
```

**Step 4: Run**

```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo build --target x86_64-unknown-linux-gnu -p onvif-rust
```
Expected: crate compiles (modulo Task 10's `app.rs` call site, if not yet fixed).

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/stub/mod.rs
git commit -m "feat(onvif-rust): add StubVideoControl for host-side VideoControl testing"
```

---

## Task 7: Persistence — `StoredVideoSourceConfig.rotated`

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/profiles/mod.rs`
- Modify: `cross-compile/onvif-rust/src/onvif/media/profile_manager.rs`

**Step 1: Add the field**

In `config/profiles/mod.rs`, add to `StoredVideoSourceConfig`, after `height`:

```rust
    #[serde(default)]
    pub rotated: bool,
```

**Step 2: Write the round-trip test**

Add near existing serde tests in this file (grep for `#[test]` in the same file to match the existing style — likely a `mod tests` at the bottom testing `ProfilesFile` TOML round-trips). Add:

```rust
    #[test]
    fn stored_video_source_config_rotated_defaults_false_when_absent() {
        let toml = r#"
            token = "VideoSourceConfig_0"
            source_token = "VideoSource_1"
            name = "Main"
            width = 1920
            height = 1080
        "#;
        let cfg: StoredVideoSourceConfig = toml::from_str(toml).unwrap();
        assert!(!cfg.rotated);
    }
```

**Step 3: Run — confirm it passes (default field, should work immediately)**

```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust --lib config::profiles
```

**Step 4: Wire the conversions**

In `profile_manager.rs`, `video_source_config_to_stored`, add after `height: c.bounds.height as u32,`:

```rust
            rotated: c
                .extension
                .as_ref()
                .and_then(|ext| ext.rotate.as_ref())
                .map(|r| r.mode == crate::onvif::types::common::RotateMode::On)
                .unwrap_or(false),
```

In `stored_to_video_source_config`, replace `extension: None,` with:

```rust
            extension: Some(crate::onvif::types::common::VideoSourceConfigurationExtension {
                rotate: Some(crate::onvif::types::common::Rotate {
                    mode: if s.rotated {
                        crate::onvif::types::common::RotateMode::On
                    } else {
                        crate::onvif::types::common::RotateMode::Off
                    },
                    degree: None,
                }),
            }),
```

This won't compile until Task 8 adds `VideoSourceConfigurationExtension`/`Rotate`/`RotateMode` — that's expected; keep going.

**Step 5: Commit**

Fold into Task 8's commit (the two tasks don't compile independently).

---

## Task 8: ONVIF types — typed `Rotate` extension

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/types/common.rs`
- Modify: `cross-compile/onvif-rust/src/onvif/types/media.rs`

**Step 1: Add `RotateMode`, `Rotate`, `VideoSourceConfigurationExtension`**

In `common.rs`, add before the `VideoSourceConfiguration` struct (around line 462):

```rust
/// Rotate mode (ONVIF `tt:RotateMode`). `Auto` is deliberately not modeled:
/// this hardware can never honor or report it (see design doc), so a request
/// specifying it is rejected at the ops-validation boundary instead of being
/// represented as a dead enum arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotateMode {
    #[serde(rename = "OFF")]
    Off,
    #[serde(rename = "ON")]
    On,
}

/// Image rotation (ONVIF `tt:Rotate`). `degree` is always `None` on this
/// device — omitting it for `On` means 180° per spec, the only degree this
/// hardware's flip+mirror trick can produce.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rotate {
    #[serde(rename = "tt:Mode", alias = "Mode")]
    pub mode: RotateMode,

    #[serde(
        rename = "tt:Degree",
        alias = "Degree",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub degree: Option<i32>,
}

/// Typed `VideoSourceConfiguration.Extension` (ONVIF
/// `tt:VideoSourceConfigurationExtension`). Only `Rotate` is modeled; this
/// device has nothing else to put there.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoSourceConfigurationExtension {
    #[serde(
        rename = "tt:Rotate",
        alias = "Rotate",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub rotate: Option<Rotate>,
}
```

**Step 2: Change `VideoSourceConfiguration.extension`'s type**

In the same file, change the `VideoSourceConfiguration` struct's `extension` field:

```rust
    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<VideoSourceConfigurationExtension>,
```

(Was `Option<Extension>`.) Every existing `extension: None` literal for this type still compiles unchanged — `None` is generic. Grep to confirm no site does `extension: Some(Extension { .. })` for `VideoSourceConfiguration` specifically (the earlier survey found only `None` literals for this struct).

**Step 3: Add `RotateOptions` and wire it into `VideoSourceConfigurationOptions`**

In `media.rs`, add before `VideoSourceConfigurationOptions`:

```rust
/// Supported rotate options (ONVIF `tt:RotateOptions`). `reboot = Some(true)`
/// signals ONVIF clients this device does not apply rotation changes across
/// a full restart without re-sending the config — see the design doc's
/// "profiles.toml is the single source of truth" note. `degree_list` is
/// omitted: this device supports exactly one non-zero degree (180), which is
/// already implied by omitting `Degree` on `Mode: On`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotateOptions {
    #[serde(rename = "tt:Mode", alias = "Mode")]
    pub mode: Vec<crate::onvif::types::common::RotateMode>,

    #[serde(rename = "@Reboot", default, skip_serializing_if = "Option::is_none")]
    pub reboot: Option<bool>,
}
```

Add to `VideoSourceConfigurationOptions`, after `video_source_tokens_available`:

```rust
    /// Rotate options.
    #[serde(
        rename = "tt:Rotate",
        alias = "Rotate",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub rotate: Option<RotateOptions>,
```

**Step 4: Write serialization round-trip tests**

Add to `common.rs`'s test module:

```rust
    #[test]
    fn rotate_mode_serializes_to_onvif_tokens() {
        assert_eq!(quick_xml::se::to_string(&RotateMode::Off).unwrap(), "OFF");
        assert_eq!(quick_xml::se::to_string(&RotateMode::On).unwrap(), "ON");
    }

    #[test]
    fn rotate_omits_degree_when_none() {
        let r = Rotate {
            mode: RotateMode::On,
            degree: None,
        };
        let xml = quick_xml::se::to_string(&r).unwrap();
        assert!(!xml.contains("Degree"));
    }
```

Adjust the exact serialization call to match however this file's existing tests invoke the XML (de)serializer — check an existing round-trip test in the same `#[cfg(test)]` block for the established helper/pattern before writing these; the shape above is illustrative, not necessarily the exact API.

**Step 5: Run**

```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust --lib onvif::types
```
Expected: PASS. This also unblocks Task 7's conversions — run that module's tests too:
```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust --lib onvif::media::profile_manager
```

**Step 6: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/types cross-compile/onvif-rust/src/config/profiles cross-compile/onvif-rust/src/onvif/media/profile_manager.rs
git commit -m "feat(onvif-rust): add typed Rotate extension, wire VideoSourceConfiguration persistence"
```

---

## Task 9: `GetVideoSourceConfigurationOptions` — advertise Rotate

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/media/profile_manager.rs`

**Step 1: Populate the new options field**

Correction from the original Task 8 draft: `Rotate` is nested inside `VideoSourceConfigurationOptionsExtension` (the type of `.extension`), not a top-level sibling field — verified against `wsdl/onvif.xsd` during Task 7-8's review after the first pass got this wrong and had to be fixed. In `get_video_source_configuration_options`, replace the `extension: None,` field of the `VideoSourceConfigurationOptions { ... }` literal:

```rust
            extension: Some(crate::onvif::types::media::VideoSourceConfigurationOptionsExtension {
                rotate: Some(crate::onvif::types::media::RotateOptions {
                    mode: vec![
                        crate::onvif::types::common::RotateMode::Off,
                        crate::onvif::types::common::RotateMode::On,
                    ],
                    // false, not true: per the WSDL, Reboot=true means the device
                    // needs an actual power-cycle to apply the change. This
                    // feature applies live via VideoControl::set_flip_mirror
                    // (Task 3-5) — no reboot required.
                    reboot: Some(false),
                }),
            }),
```

**Step 2: Write the test**

Find this function's existing test (grep `get_video_source_configuration_options` in this file's `#[cfg(test)]` block) and extend it, or add a new one:

```rust
    #[test]
    fn video_source_configuration_options_advertise_rotate_off_and_on() {
        let pm = ProfileManager::new_for_test(); // match whatever the existing tests in this file use to construct one
        let options = pm.get_video_source_configuration_options();
        let rotate = options.rotate.expect("rotate options should be present");
        assert_eq!(
            rotate.mode,
            vec![
                crate::onvif::types::common::RotateMode::Off,
                crate::onvif::types::common::RotateMode::On
            ]
        );
    }
```

Check the file for the actual `ProfileManager` test-construction helper name before writing this — it's used throughout the existing test module.

**Step 3: Run**

```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust --lib onvif::media::profile_manager
```

**Step 4: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/media/profile_manager.rs
git commit -m "feat(onvif-rust): advertise Off/On Rotate in GetVideoSourceConfigurationOptions"
```

---

## Task 10: `SetVideoSourceConfiguration` — validate and live-apply

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/media/ops/video_sources.rs`
- Modify: `cross-compile/onvif-rust/src/onvif/media/service.rs`

**Step 1: Add Mode/Degree validation to the op**

In `video_sources.rs`, change `set_video_source_configuration`'s signature to also take the platform (mirrors how `service.rs` already reaches into `self.platform` for other Set* validation — check the `GetVideoEncoderConfigurationOptions`-adjacent validation at `service.rs:270-275` for the exact style to match before writing this):

```rust
/// Handle SetVideoSourceConfiguration request.
///
/// Updates a video source configuration and, if a Rotate extension is
/// present, applies it to the platform live before persisting. Mode is
/// restricted to Off/On (see design doc — Auto is not representable on this
/// hardware and is rejected here rather than modeled).
pub async fn set_video_source_configuration(
    pm: &ProfileManagerRef,
    platform: Option<&std::sync::Arc<dyn crate::platform::Platform>>,
    request: SetVideoSourceConfiguration,
) -> OnvifResult<SetVideoSourceConfigurationResponse> {
    tracing::debug!(
        "SetVideoSourceConfiguration request for token: {}",
        request.configuration.token
    );

    if let Some(rotate) = request
        .configuration
        .extension
        .as_ref()
        .and_then(|ext| ext.rotate.as_ref())
    {
        if let Some(degree) = rotate.degree
            && degree != 180
        {
            return Err(crate::onvif::error::OnvifError::invalid_arg_val(format!(
                "Unsupported rotate degree {}: this device only supports 180",
                degree
            )));
        }

        let rotated = rotate.mode == crate::onvif::types::common::RotateMode::On;
        if let Some(platform) = platform
            && let Some(control) = platform.video_control()
        {
            control.set_flip_mirror(rotated).await.map_err(|e| {
                crate::onvif::error::OnvifError::action_failed(format!(
                    "Failed to apply rotation: {}",
                    e
                ))
            })?;
        }
    }

    pm.set_video_source_configuration(request.configuration)?;
    Ok(SetVideoSourceConfigurationResponse {})
}
```

Check `crate::onvif::error::OnvifError`'s actual constructor names (`invalid_arg_val`, `action_failed` are guesses at the ONVIF fault vocabulary — grep this crate's `error.rs` for existing fault constructors used by sibling Set* ops, e.g. `set_video_encoder_configuration` or an Imaging Set*, and match exactly).

**Step 2: Update the call site**

Find where `service.rs` currently calls `ops::set_video_source_configuration(pm, request)` (or wherever `video_sources::set_video_source_configuration` is dispatched from) and pass `self.platform.as_ref()` as the new second argument. This function becomes `async` if it wasn't already (it now `.await`s `control.set_flip_mirror`) — confirm the dispatch call site already `.await`s it (Media service SOAP handlers are async throughout this codebase, so this should already be the case; verify rather than assume).

**Step 3: Write the failing test**

Add to `video_sources.rs`'s `#[cfg(test)] mod tests` (near the existing `set_video_source_configuration` roundtrip tests):

```rust
    #[tokio::test]
    async fn set_video_source_configuration_rejects_non_180_degree() {
        let pm = /* match existing test setup in this file */;
        let mut config = pm.get_video_source_configuration("VideoSourceConfig_0").unwrap();
        config.extension = Some(crate::onvif::types::common::VideoSourceConfigurationExtension {
            rotate: Some(crate::onvif::types::common::Rotate {
                mode: crate::onvif::types::common::RotateMode::On,
                degree: Some(90),
            }),
        });

        let result = set_video_source_configuration(
            &pm,
            None,
            SetVideoSourceConfiguration { configuration: config },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn set_video_source_configuration_persists_rotate_on() {
        let pm = /* match existing test setup in this file */;
        let mut config = pm.get_video_source_configuration("VideoSourceConfig_0").unwrap();
        config.extension = Some(crate::onvif::types::common::VideoSourceConfigurationExtension {
            rotate: Some(crate::onvif::types::common::Rotate {
                mode: crate::onvif::types::common::RotateMode::On,
                degree: None,
            }),
        });

        let result = set_video_source_configuration(
            &pm,
            None,
            SetVideoSourceConfiguration { configuration: config },
        )
        .await;

        assert!(result.is_ok());
        let stored = pm.get_video_source_configuration("VideoSourceConfig_0").unwrap();
        let rotate = stored.extension.unwrap().rotate.unwrap();
        assert_eq!(rotate.mode, crate::onvif::types::common::RotateMode::On);
    }
```

Match the existing tests' exact `ProfileManagerRef`/fixture construction in this file before writing these — the placeholders above need the real helper name.

**Step 4: Run**

```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust --lib onvif::media::ops::video_sources
```
Expected: PASS.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/media/ops/video_sources.rs cross-compile/onvif-rust/src/onvif/media/service.rs
git commit -m "feat(onvif-rust): validate and live-apply Rotate in SetVideoSourceConfiguration"
```

---

## Task 11: `app.rs` — reorder boot, seed from `profiles.toml`

**Files:**
- Modify: `cross-compile/onvif-rust/src/app.rs`

**Step 1: Reorder `wire_profile_persistence` before `init_platform`**

In `Application::run` (or wherever Phase 2/3 startup is sequenced — the `init_platform` call around line 982 and `wire_profile_persistence` around line 1011), move the `wire_profile_persistence` call to run first. Neither depends on the other's output today (`init_platform` only reads `config_runtime`; `wire_profile_persistence` only reads `config_path`/`save_delay`/`shutdown_coordinator`), so this is a pure reorder, not a rewrite. `wire_imaging_persistence` (which *does* need `platform.as_ref()`) stays after `init_platform`, unchanged.

**Step 2: Extract the boot value and pass it to `init_platform`**

After the (now-earlier) `wire_profile_persistence` call, add:

```rust
        let initial_rotated = profile_storage
            .snapshot()
            .video_source_configs
            .first()
            .map(|c| c.rotated)
            .unwrap_or(false);
```

Add `initial_rotated: bool` as a parameter to `init_platform` (threaded through to the `AnykaPlatform::with_isp_config(..., imaging_cfg, initial_rotated)` call inside it — Task 5 Step 2 added the parameter on the receiving end), and pass `initial_rotated` at the call site.

**Step 3: Fix the stub-build branch too**

`init_platform`'s `#[cfg(use_stubs)]` branch builds a `StubPlatformBuilder` — no change needed there (`StubPlatform`'s `video_control` always returns the fixed `StubVideoControl` from Task 6, independent of any boot seed). Just make sure the new `initial_rotated` parameter is accepted by the function signature regardless of which `#[cfg]` branch compiles (add `let _ = initial_rotated;` in the stub branch to avoid an unused-parameter warning under that cfg, matching the existing `let _ = shutdown;` pattern already there).

**Step 4: Run**

```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo build --target x86_64-unknown-linux-gnu -p onvif-rust
toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust --lib
```
Expected: crate builds clean; full host-side suite passes.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/app.rs
git commit -m "feat(onvif-rust): seed boot-time flip/mirror from profiles.toml, not config.toml"
```

---

## Task 12: WebUI — `getVideoSourceConfiguration` / `setVideoSourceConfiguration`

**Files:**
- Modify: `cross-compile/www/src/services/profileService.ts`
- Modify: `cross-compile/www/src/services/profileService.test.ts`

**Step 1: Add the types**

Add near this file's other config types (check how `VideoEncoderConfiguration` is typed/exported at the top of the file, and match its shape):

```typescript
export type RotateMode = 'OFF' | 'ON';

export interface VideoSourceConfiguration {
  token: string;
  name: string;
  sourceToken: string;
  rotate: RotateMode;
}
```

**Step 2: Write the failing tests**

In `profileService.test.ts`, add tests modeled directly on the existing `getVideoEncoderConfiguration`/`setVideoEncoderConfiguration` tests in the same file (grep for those to match the existing `soapRequest` mocking pattern exactly):

```typescript
describe('getVideoSourceConfiguration', () => {
  it('parses Rotate ON from the response', async () => {
    // mock soapRequest to return a Configuration with Extension/Rotate/Mode=ON,
    // matching how the existing getVideoEncoderConfiguration test mocks its response
  });

  it('defaults to OFF when Extension is absent', async () => {
    // mock a response with no Extension element
  });
});

describe('setVideoSourceConfiguration', () => {
  it('sends Extension/Rotate/Mode in the request body', async () => {
    // assert the SOAP body sent to soapRequest contains <tt:Rotate><tt:Mode>ON</tt:Mode></tt:Rotate>
  });
});
```

**Step 3: Implement**

Add to `profileService.ts`, modeled on `getVideoEncoderConfiguration`/`setVideoEncoderConfiguration`:

```typescript
export async function getVideoSourceConfiguration(
  token: string,
): Promise<VideoSourceConfiguration | null> {
  const body = `<trt:GetVideoSourceConfiguration>
    <trt:ConfigurationToken>${token}</trt:ConfigurationToken>
  </trt:GetVideoSourceConfiguration>`;

  try {
    const data = await soapRequest<Record<string, unknown>>(
      ENDPOINTS.media,
      body,
      'GetVideoSourceConfigurationResponse',
    );
    const config = data?.Configuration as Record<string, unknown> | undefined;
    if (!config) {
      return null;
    }

    const extension = config.Extension as Record<string, unknown> | undefined;
    const rotate = extension?.Rotate as Record<string, unknown> | undefined;

    return {
      token: safeString(config['@_token'], ''),
      name: safeString(config.Name, ''),
      sourceToken: safeString(config.SourceToken, ''),
      rotate: (safeString(rotate?.Mode, 'OFF') as RotateMode) || 'OFF',
    };
  } catch (error) {
    console.warn('Failed to get video source configuration:', error);
    return null;
  }
}

export async function setVideoSourceConfiguration(
  config: VideoSourceConfiguration,
): Promise<void> {
  const escapedToken = escapeXmlAttribute(config.token);
  const escapedName = escapeXml(config.name);
  const escapedSourceToken = escapeXml(config.sourceToken);

  const body = `<trt:SetVideoSourceConfiguration>
    <trt:Configuration token="${escapedToken}">
      <tt:Name>${escapedName}</tt:Name>
      <tt:UseCount>0</tt:UseCount>
      <tt:SourceToken>${escapedSourceToken}</tt:SourceToken>
      <tt:Bounds x="0" y="0" width="0" height="0" />
      <tt:Extension>
        <tt:Rotate>
          <tt:Mode>${config.rotate}</tt:Mode>
        </tt:Rotate>
      </tt:Extension>
    </trt:Configuration>
    <trt:ForcePersistence>true</trt:ForcePersistence>
  </trt:SetVideoSourceConfiguration>`;

  await soapRequest(ENDPOINTS.media, body, 'SetVideoSourceConfigurationResponse');
}
```

Check the real `Bounds` values the device expects (this repo's `GetVideoSourceConfigurationOptions` bounds_range in `profile_manager.rs` fixes `x`/`y` at 0 and width/height at the sensor's actual resolution) — fetch the current configuration first and round-trip its real `Bounds` rather than hardcoding zeros, matching whatever `setVideoEncoderConfiguration`-adjacent callers already do for fields they don't intend to change.

**Step 4: Run**

```bash
cd cross-compile/www && npx vitest run src/services/profileService.test.ts
```
Expected: PASS.

**Step 5: Commit**

```bash
git add cross-compile/www/src/services/profileService.ts cross-compile/www/src/services/profileService.test.ts
git commit -m "feat(webui): add getVideoSourceConfiguration/setVideoSourceConfiguration"
```

---

## Task 13: WebUI — "Flip image 180°" toggle

**Files:**
- Modify: `cross-compile/www/src/pages/settings/ImagingPage.tsx`
- Modify: `cross-compile/www/src/pages/settings/ImagingPage.test.tsx` (or wherever this page's existing tests live — check the file list next to it)

**Step 1: Write the failing component test**

Add a test asserting the switch renders, reflects the fetched state, and calls `setVideoSourceConfiguration` on toggle — follow this file's existing pattern for the IR-cut or WDR switch tests exactly (same `renderWithProviders`/MSW mocking conventions used throughout, per the `anyka-webui-testing` skill).

**Step 2: Implement**

In `ImagingPage.tsx`:
- Add a `useQuery` for `getVideoSourceConfiguration('VideoSourceConfig_0')` (the fixed single-video-source token used throughout this backend — confirm via `defaults.rs`'s `VIDEO_SOURCE_CONFIG_PREFIX` constant rather than assuming).
- Add a `useMutation` calling `setVideoSourceConfiguration`, following the exact `mutation`/`toast.success`/`toast.error`/`queryClient.invalidateQueries` shape already used for `imagingSettings` in this file.
- Add a `Switch` + `Label` pair ("Flip image 180°") in the JSX, near the other day/night or lamp controls, following this file's existing `SettingsCard`/`Switch` layout conventions.

**Step 3: Run**

```bash
cd cross-compile/www && npx vitest run src/pages/settings/ImagingPage.test.tsx
```
Expected: PASS.

**Step 4: Commit**

```bash
git add cross-compile/www/src/pages/settings/ImagingPage.tsx cross-compile/www/src/pages/settings/ImagingPage.test.tsx
git commit -m "feat(webui): add flip-image-180 toggle to the Imaging settings page"
```

---

## Task 14: Full verification

**Step 1: Host-side Rust — full suite, clippy, fmt**

```bash
source ./setenv.sh
cd cross-compile
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
```
Expected: all PASS. Fix any clippy/fmt issue surfaced by the new code before continuing.

**Step 2: WebUI — typecheck, lint, full test suite**

```bash
cd cross-compile/www
npm run type-check
npm run lint
npx vitest run
```
Expected: all PASS.

**Step 3: Cross-compile sanity (ARM ELF, no hardware needed)**

```bash
cd /home/kmk/dev/anyka-dev
bash scripts/build_sd_contents.sh --skip-www
```
Expected: `SD_card_contents assembly complete`, `onvif-rust.bin`/`vendor-daemon.bin` verified as ARMv5 32-bit ELF (this script's own `require_arm_elf` check does this).

**Step 4: Manual on-device smoke test**

No C test harness exists for `handlers_vi.c` (see "Before you start"), so this is the only check for Task 1's C code. Deploy per the same jumphost/FTP/atomic-rename workflow used for the last binary deploy (`.deploy/anyka-121.toml`, `scripts/deploy_onvif.sh` or the manual `.new`-then-`mv` sequence), then over telnet:
1. `GetVideoSourceConfiguration` via the WebUI toggle or `curl`/SOAP client — flip it on, confirm the RTSP/HTTP-FLV stream visibly rotates without restarting `onvif-rust.bin`.
2. Flip it off, confirm it reverts.
3. `killall onvif-rust.bin vendor-daemon.bin` (both together, per the known restart-resilience requirement), wait for the supervisor to respawn them, confirm the last-set rotation is still applied without touching the WebUI again — this is the "reapply on reattach" path from Task 5 Step 3.

**Step 5: Update the design doc status**

Edit `docs/plans/2026-08-07-video-rotate-design.md`'s header from `Status: proposed` to `Status: implemented 2026-08-07` (or whatever date this actually lands), and commit.

```bash
git add docs/plans/2026-08-07-video-rotate-design.md
git commit -m "docs(plan): mark the video rotate design implemented"
```
