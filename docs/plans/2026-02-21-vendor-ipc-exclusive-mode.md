# Vendor IPC Exclusive Mode — Remove Direct FFI from onvif-rust

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** When `use_vendor_ipc` is enabled, eliminate all direct C FFI calls and vendor library/header dependencies from onvif-rust, making vendor-daemon the exclusive SDK bridge.

**Architecture:** The `use_vendor_ipc` Cargo feature becomes a hard switch: when enabled, `build.rs` sets `use_stubs` (even on ARM), skipping bindgen and vendor `.so` linking entirely. The stub module provides all needed type definitions. `Real*Ffi` fallback paths are removed — if vendor-daemon is unreachable, the platform returns `PlatformError` instead of silently falling back to direct FFI. PTZ is unaffected (already a pure-Rust ioctl driver).

**Tech Stack:** Rust, Cargo features, `cfg` conditional compilation, `build.rs`, `mockall`

---

## Context

### Current Architecture (dual-mode)

```
AnykaVideoInput::new()
    |
    +--[use_vendor_ipc]--→ VendorIpc::new()  → /tmp/vendor-daemon.sock
    |                           |
    |                       [on error]
    |                           |
    +--[fallback]------------→ RealVideoFfi → unsafe { ak_vi_open() } → linked vendor .so
```

- `build.rs` always generates FFI bindings + links 20 vendor `.so` files on ARM
- 5 structs have dual-mode constructors with VendorIpc→RealFfi fallback
- PTZ is already pure Rust (`ptz_driver.rs` → `/dev/ak-motor*` ioctls)

### Target Architecture (IPC-exclusive)

```
AnykaVideoInput::new()
    |
    +--[use_vendor_ipc]--→ VendorIpc::new()  → /tmp/vendor-daemon.sock
    |                           |
    |                       [on error]
    |                           |
    +------------------------→ PlatformError (no fallback)

build.rs:
    [use_vendor_ipc] → set use_stubs → NO bindgen, NO vendor linking
    [default]        → existing behavior (bindgen + vendor linking)
```

### Files Inventory

| File | Role | Changes |
|------|------|---------|
| `build.rs` | FFI generation + linking | Add `use_vendor_ipc` feature check → set `use_stubs` early |
| `src/ffi/mod.rs` | Module gating | No change (stubs already gated on `use_stubs`) |
| `src/ffi/video.rs` | `RealVideoFfi` | Gate `RealVideoFfi` behind `#[cfg(not(feature = "use_vendor_ipc"))]` |
| `src/ffi/audio.rs` | `RealAudioFfi` | Gate `RealAudioFfi` behind `#[cfg(not(feature = "use_vendor_ipc"))]` |
| `src/ffi/imaging.rs` | `RealImagingFfi` | Gate `RealImagingFfi` behind `#[cfg(not(feature = "use_vendor_ipc"))]` |
| `src/platform/anyka.rs` | 5 dual-mode constructors | Remove fallback to `Real*Ffi` when `use_vendor_ipc` is set |
| `Cargo.toml` | Feature declarations | No structural change (feature already exists) |

### What Does NOT Change

- `src/ffi/ptz.rs` — PTZ is already pure Rust (ioctl driver), no C FFI
- `src/platform/hw_ptz.rs` — Uses `NativePtzFfi`, not VendorIpc
- `src/ffi/vendor_ipc.rs` — Already correct, no changes needed
- `src/ffi/ptz_driver.rs` — Pure Rust, no changes needed
- Host-side testing (`use_stubs` on x86_64) — Unaffected
- Default ARM build (without `use_vendor_ipc`) — Unaffected

---

## Task 1: Gate FFI generation in build.rs behind use_vendor_ipc

**Files:**
- Modify: `cross-compile/onvif-rust/build.rs` (lines 9-29)

**Step 1: Add use_vendor_ipc feature check to build.rs main()**

Insert a feature check before the cross-compile branch. When `use_vendor_ipc` is enabled,
set `use_stubs` and skip FFI generation — even on ARM.

```rust
// In main(), after line 11 (cargo::rustc-check-cfg), before line 14:

// When use_vendor_ipc is enabled, all SDK access goes through vendor-daemon IPC.
// No need for FFI bindings or vendor library linking — use stub type definitions.
if std::env::var("CARGO_FEATURE_USE_VENDOR_IPC").is_ok() {
    println!("cargo:rustc-cfg=use_stubs");
    println!("cargo:warning=use_vendor_ipc enabled: using stub types, skipping vendor FFI generation");
    // System libs (pthread, m, dl) are still needed for the Rust runtime
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=dl");
    return;
}
```

**Step 2: Verify host build still works**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu --lib 2>&1 | tail -5`
Expected: Tests pass (host build already uses stubs, unaffected)

**Step 3: Verify ARM build without feature still works**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo check --release 2>&1 | tail -5`
Expected: Compiles with FFI bindings as before (default path unchanged)

**Step 4: Verify ARM build WITH feature uses stubs**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo check --release --features use_vendor_ipc 2>&1 | grep -E "(use_vendor_ipc|use_stubs|warning)"`
Expected: Warning printed: "use_vendor_ipc enabled: using stub types..."

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/build.rs
git commit -m "feat(build): skip vendor FFI generation when use_vendor_ipc is enabled"
```

---

## Task 2: Gate RealVideoFfi behind not(use_vendor_ipc)

**Files:**
- Modify: `cross-compile/onvif-rust/src/ffi/video.rs` (lines 74-351)

When `use_vendor_ipc` is enabled, `RealVideoFfi` is dead code — it would only be
called as a fallback which we're removing in Task 5. Gate the struct to prevent
compilation of the C FFI call paths.

**Step 1: Gate the RealVideoFfi struct definition and impl block**

The struct is at line 74. The impl block runs from ~line 75 to ~line 351.
Each method has dual `#[cfg(not(use_stubs))]` / `#[cfg(use_stubs)]` implementations.

Wrap the entire struct + impl in:

```rust
#[cfg(not(feature = "use_vendor_ipc"))]
pub(crate) struct RealVideoFfi;

#[cfg(not(feature = "use_vendor_ipc"))]
impl VideoFfiTrait for RealVideoFfi {
    // ... all existing methods unchanged ...
}
```

This is a single gate at the struct level, not per-method.

**Step 2: Run host tests**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu --lib 2>&1 | tail -5`
Expected: Tests pass (host build doesn't use `use_vendor_ipc` feature, `RealVideoFfi` still compiled)

**Step 3: Verify ARM+IPC build compiles**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo check --release --features use_vendor_ipc 2>&1 | tail -5`
Expected: May show unused import warnings for `RealVideoFfi` in `anyka.rs` — these are fixed in Task 5.

**Step 4: Commit**

```bash
git add cross-compile/onvif-rust/src/ffi/video.rs
git commit -m "refactor(ffi): gate RealVideoFfi behind not(use_vendor_ipc)"
```

---

## Task 3: Gate RealAudioFfi behind not(use_vendor_ipc)

**Files:**
- Modify: `cross-compile/onvif-rust/src/ffi/audio.rs` (lines 52-146)

Same pattern as Task 2.

**Step 1: Gate the RealAudioFfi struct definition and impl block**

```rust
#[cfg(not(feature = "use_vendor_ipc"))]
pub(crate) struct RealAudioFfi;

#[cfg(not(feature = "use_vendor_ipc"))]
impl AudioFfiTrait for RealAudioFfi {
    // ... all existing methods unchanged ...
}
```

**Step 2: Run host tests**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu --lib 2>&1 | tail -5`
Expected: Tests pass

**Step 3: Commit**

```bash
git add cross-compile/onvif-rust/src/ffi/audio.rs
git commit -m "refactor(ffi): gate RealAudioFfi behind not(use_vendor_ipc)"
```

---

## Task 4: Gate RealImagingFfi behind not(use_vendor_ipc)

**Files:**
- Modify: `cross-compile/onvif-rust/src/ffi/imaging.rs` (lines 43-124)

Same pattern as Tasks 2-3.

**Step 1: Gate the RealImagingFfi struct definition and impl block**

```rust
#[cfg(not(feature = "use_vendor_ipc"))]
pub(crate) struct RealImagingFfi;

#[cfg(not(feature = "use_vendor_ipc"))]
impl ImagingFfiTrait for RealImagingFfi {
    // ... all existing methods unchanged ...
}
```

**Step 2: Run host tests**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu --lib 2>&1 | tail -5`
Expected: Tests pass

**Step 3: Commit**

```bash
git add cross-compile/onvif-rust/src/ffi/imaging.rs
git commit -m "refactor(ffi): gate RealImagingFfi behind not(use_vendor_ipc)"
```

---

## Task 5: Remove Real*Ffi fallback from platform constructors

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka.rs` (5 constructor sites)

This is the key behavioral change: when `use_vendor_ipc` is enabled and VendorIpc
connection fails, return `PlatformError` instead of falling back to direct FFI.

### 5a: AnykaVideoInput::new() (lines 710-729)

**Step 1: Replace the constructor**

Current code:
```rust
fn new(isp_config_path: Option<PathBuf>) -> Self {
    #[cfg(feature = "use_vendor_ipc")]
    {
        match VendorIpc::new() {
            Ok(ipc) => {
                tracing::info!("Using VendorIpc for vendor library access");
                return Self::with_ffi(Arc::new(ipc), isp_config_path);
            }
            Err(e) => {
                tracing::warn!(
                    "VendorIpc connection failed, falling back to RealVideoFfi: {}",
                    e
                );
            }
        }
    }
    Self::with_ffi(Arc::new(RealVideoFfi), isp_config_path)
}
```

New code:
```rust
fn new(isp_config_path: Option<PathBuf>) -> PlatformResult<Self> {
    #[cfg(feature = "use_vendor_ipc")]
    {
        let ipc = VendorIpc::new().map_err(|e| {
            PlatformError::InitializationFailed(format!(
                "VendorIpc connection failed (is vendor-daemon running?): {}", e
            ))
        })?;
        tracing::info!("AnykaVideoInput: using VendorIpc for vendor library access");
        return Ok(Self::with_ffi(Arc::new(ipc), isp_config_path));
    }
    #[cfg(not(feature = "use_vendor_ipc"))]
    Ok(Self::with_ffi(Arc::new(RealVideoFfi), isp_config_path))
}
```

> **NOTE:** Changing `new()` return type from `Self` to `PlatformResult<Self>` will
> propagate to all call sites. The implementer must find and update every call to
> `AnykaVideoInput::new()` in the codebase to handle the `Result`. Search for
> `AnykaVideoInput::new(` to find all call sites and add `?` or `.unwrap()`.

### 5b: AnykaVideoEncoder::new() (lines 1612-1634)

Same pattern — return `PlatformResult<Self>`, replace fallback with error propagation.

```rust
fn new() -> PlatformResult<Self> {
    #[cfg(feature = "use_vendor_ipc")]
    {
        let ipc = crate::ffi::vendor_ipc::VendorIpc::new().map_err(|e| {
            PlatformError::InitializationFailed(format!(
                "AnykaVideoEncoder: VendorIpc connection failed: {}", e
            ))
        })?;
        tracing::info!("AnykaVideoEncoder: using VendorIpc for vendor library access");
        return Ok(Self::with_ffi(Arc::new(ipc)));
    }
    #[cfg(not(feature = "use_vendor_ipc"))]
    Ok(Self::with_ffi(Arc::new(crate::ffi::video::RealVideoFfi)))
}
```

### 5c: AnykaAudioInput::new() (lines 2396-2417)

```rust
fn new() -> PlatformResult<Self> {
    #[cfg(feature = "use_vendor_ipc")]
    {
        let ipc = crate::ffi::vendor_ipc::VendorIpc::new().map_err(|e| {
            PlatformError::InitializationFailed(format!(
                "AnykaAudioInput: VendorIpc connection failed: {}", e
            ))
        })?;
        tracing::info!("AnykaAudioInput: using VendorIpc for vendor library access");
        return Ok(Self {
            ffi: Arc::new(ipc),
            opened: AtomicBool::new(false),
        });
    }
    #[cfg(not(feature = "use_vendor_ipc"))]
    Ok(Self {
        ffi: Arc::new(crate::ffi::audio::RealAudioFfi),
        opened: AtomicBool::new(false),
    })
}
```

### 5d: AnykaAudioEncoder::new() (lines 2463-2499)

```rust
fn new() -> PlatformResult<Self> {
    #[cfg(feature = "use_vendor_ipc")]
    let ffi: Arc<dyn crate::ffi::audio::AudioFfiTrait> = {
        let ipc = crate::ffi::vendor_ipc::VendorIpc::new().map_err(|e| {
            PlatformError::InitializationFailed(format!(
                "AnykaAudioEncoder: VendorIpc connection failed: {}", e
            ))
        })?;
        tracing::info!("AnykaAudioEncoder: using VendorIpc for vendor library access");
        Arc::new(ipc)
    };
    #[cfg(not(feature = "use_vendor_ipc"))]
    let ffi: Arc<dyn crate::ffi::audio::AudioFfiTrait> =
        Arc::new(crate::ffi::audio::RealAudioFfi);

    Ok(Self {
        ffi,
        configurations: RwLock::new(vec![/* ... existing default configs ... */]),
    })
}
```

### 5e: AnykaImagingControl::new() (lines 2557-2603)

```rust
fn new() -> PlatformResult<Self> {
    #[cfg(feature = "use_vendor_ipc")]
    let ffi: Arc<dyn crate::ffi::imaging::ImagingFfiTrait> = {
        let ipc = crate::ffi::vendor_ipc::VendorIpc::new().map_err(|e| {
            PlatformError::InitializationFailed(format!(
                "AnykaImagingControl: VendorIpc connection failed: {}", e
            ))
        })?;
        Arc::new(ipc)
    };
    #[cfg(not(feature = "use_vendor_ipc"))]
    let ffi = Arc::new(crate::ffi::imaging::RealImagingFfi);

    Ok(Self {
        ffi,
        settings: RwLock::new(ImagingSettings::default()),
    })
}
```

### 5f: Gate imports

At the top of `anyka.rs` (line 38), the `RealVideoFfi` import must be gated:

```rust
#[cfg(not(feature = "use_vendor_ipc"))]
use crate::ffi::video::RealVideoFfi;
```

Similarly for any other `Real*Ffi` imports.

### 5g: Update call sites

Search for all `::new()` calls on the 5 modified structs. Each needs to handle
the new `PlatformResult<Self>` return type. Common patterns:

```rust
// Before:
let vi = AnykaVideoInput::new(isp_config_path);

// After:
let vi = AnykaVideoInput::new(isp_config_path)?;
```

**Step 2: Run host tests**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu --lib 2>&1 | tail -20`
Expected: Tests pass (host uses stubs, `use_vendor_ipc` not enabled, `RealVideoFfi` still compiled)

**Step 3: Verify ARM+IPC build compiles**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo check --release --features use_vendor_ipc 2>&1 | tail -20`
Expected: Compiles with zero errors, zero warnings about `RealVideoFfi`

**Step 4: Verify default ARM build still compiles**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo check --release 2>&1 | tail -5`
Expected: Compiles with vendor FFI as before

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka.rs
git commit -m "refactor(platform): remove Real*Ffi fallback when use_vendor_ipc is enabled

When vendor-daemon is unreachable, AnykaVideoInput, AnykaVideoEncoder,
AnykaAudioInput, AnykaAudioEncoder, and AnykaImagingControl now return
PlatformError::InitializationFailed instead of silently falling back
to direct C FFI calls."
```

---

## Task 6: Run full quality gate

**Files:** None (verification only)

**Step 1: Format check**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo fmt --check`
Expected: No formatting issues

**Step 2: Clippy (host)**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings 2>&1 | tail -10`
Expected: Zero warnings

**Step 3: Clippy (ARM, default — without use_vendor_ipc)**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo clippy --release -- -D warnings 2>&1 | tail -10`
Expected: Zero warnings

**Step 4: Clippy (ARM, with use_vendor_ipc)**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo clippy --release --features use_vendor_ipc -- -D warnings 2>&1 | tail -10`
Expected: Zero warnings (no dead code, no unused imports)

**Step 5: Full test suite (host)**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu 2>&1 | tail -10`
Expected: All tests pass

**Step 6: ARM release build (with use_vendor_ipc)**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo build --release --features use_vendor_ipc 2>&1 | tail -5`
Expected: Builds successfully, binary does NOT link vendor .so files

**Step 7: Verify binary has no vendor library dependencies**

Run: `readelf -d cross-compile/onvif-rust/target/armv5te-unknown-linux-uclibceabi/release/onvif-rust 2>/dev/null | grep -E "libplat_|libmpi_|libak" || echo "CLEAN: No vendor library dependencies"`
Expected: "CLEAN: No vendor library dependencies"

**Step 8: ARM release build (without use_vendor_ipc — backward compat)**

Run: `cd cross-compile/onvif-rust && ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo build --release 2>&1 | tail -5`
Expected: Builds successfully with vendor .so linking (backward compatible)

**Step 9: Commit (if any fixups needed)**

If Steps 1-8 required fixups, commit them here.

---

## Task 7: Update build script to default to use_vendor_ipc

**Files:**
- Modify: `cross-compile/onvif-rust/scripts/build.sh`

**Step 1: Change default features**

The build script currently requires `--features use_vendor_ipc` to be passed manually.
Update the default so production ARM builds use IPC mode by default:

```bash
# Current (lines 132-135):
FEATURES_ARGS=()
if [[ -n "${EXTRA_FEATURES}" ]]; then
  FEATURES_ARGS=(--features "${EXTRA_FEATURES}")
fi

# New:
DEFAULT_FEATURES="use_vendor_ipc"
FEATURES_ARGS=(--features "${EXTRA_FEATURES:+${EXTRA_FEATURES},}${DEFAULT_FEATURES}")
```

Add a `--no-ipc` flag option (or `--direct-ffi`) that removes `use_vendor_ipc` from
the features for developers who need the old behavior:

```bash
--no-ipc|--direct-ffi)
  DEFAULT_FEATURES=""
  shift
  ;;
```

Update the help text to document the default and override.

**Step 2: Test the build script**

Run: `cd cross-compile/onvif-rust && ./scripts/build.sh --release 2>&1 | grep -E "(use_vendor_ipc|warning|Compiling|Finished)" | head -10`
Expected: Shows "use_vendor_ipc enabled" warning from build.rs

Run: `cd cross-compile/onvif-rust && ./scripts/build.sh --release --no-ipc 2>&1 | grep -E "(use_vendor_ipc|warning|Compiling|Finished)" | head -10`
Expected: No "use_vendor_ipc" warning (direct FFI mode)

**Step 3: Commit**

```bash
git add cross-compile/onvif-rust/scripts/build.sh
git commit -m "feat(build): default to use_vendor_ipc for production ARM builds

The --no-ipc flag can be used to fall back to direct vendor FFI linking
for development/debugging without vendor-daemon."
```

---

## Task 8: Update run_onvif_rust.sh — vendor-daemon is mandatory

**Files:**
- Modify: `SD_card_contents/anyka_hack/onvif/run_onvif_rust.sh`

With `use_vendor_ipc` as default, vendor-daemon is no longer optional. The launcher
already starts vendor-daemon first, but the error messaging should be updated.

**Step 1: Update the vendor-daemon failure message**

The existing code (lines 111-115) already aborts on failure. Update the error message:

```bash
# Current:
log ERROR "Failed to start vendor-daemon; aborting"

# New:
log ERROR "Failed to start vendor-daemon; aborting (vendor-daemon is required for IPC mode)"
```

**Step 2: Commit**

```bash
git add SD_card_contents/anyka_hack/onvif/run_onvif_rust.sh
git commit -m "docs(launcher): clarify vendor-daemon is required for IPC mode"
```

---

## Task 9: Create bd issue for future vendor/ directory cleanup

**Files:** None (issue tracking only)

Once `use_vendor_ipc` is the default and confirmed stable in production, the
`vendor/lib/` (20 `.so` files, ~2.8 MB) and `vendor/include/` (40+ headers) can
be removed from the repository. They would only be needed by developers building
with `--no-ipc`.

**Step 1: Create issue**

```bash
bd create "Remove vendor/lib and vendor/include from onvif-rust repo" \
  --description="Now that use_vendor_ipc is the default ARM build mode, the vendor/ directory (20 .so files + 40 headers) is only needed for --no-ipc builds. Consider moving to a git submodule, download-on-demand, or removing entirely once IPC mode is proven stable in production." \
  -t task -p 3 --json
```

---

## Risk Assessment

| Risk | Mitigation |
|------|-----------|
| vendor-daemon not running on device | Explicit `PlatformError` with "is vendor-daemon running?" hint |
| Backward compatibility broken | Default ARM build (without feature) unchanged |
| PTZ breaks | PTZ is pure Rust ioctl — verified independent of vendor libs |
| Host tests break | Host tests use `use_stubs`, unaffected by `use_vendor_ipc` |
| Binary size changes | Removing 20 vendor `.so` links reduces binary NEEDED list |
| Runtime lib path issues | No change — vendor-daemon still needs libs via its own `LD_LIBRARY_PATH` |

## Verification Matrix

| Build Configuration | Expected Behavior |
|---------------------|-------------------|
| Host (x86_64, tests) | Stubs, no vendor linking, all tests pass |
| ARM (default, no feature) | FFI bindings + vendor linking (backward compatible) |
| ARM (`--features use_vendor_ipc`) | Stubs, no vendor linking, IPC-only |
| ARM (`--features use_vendor_ipc`, vendor-daemon down) | `PlatformError::InitializationFailed` |
| ARM (`--features use_vendor_ipc`, vendor-daemon up) | Full functionality via IPC |
