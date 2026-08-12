# Diagnostics Vision + Network Text Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show network rates as text in Device Information and expose live day/night vision (AE, ain0, full lamp set) on Diagnostics via an extended `/api/diagnostics` snapshot.

**Architecture:** Add `NightModeController::live_diagnostics()`, surface it through `ImagingControl::vision_diagnostics()`, await it from async `DiagnosticsState::snapshot`, and render net text + a Vision card in the WebUI. Design: `docs/plans/2026-08-11-diagnostics-vision-design.md`.

**Tech Stack:** Rust (onvif-rust, mockall, tokio), axum, React 19, TanStack Query, Vitest

---

## Task 1: Vision types + ImagingControl hook

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/common/traits.rs`
- Modify: `cross-compile/onvif-rust/src/platform/stub/mod.rs` (stub imaging — default already enough if trait default returns `None`)
- Test: unit tests adjacent to the new types / trait default

**Step 1: Write the failing test**

Add a test that a default/mock imaging control returns `None` for vision:

```rust
#[tokio::test]
async fn test_imaging_control_vision_diagnostics_defaults_to_none() {
    let imaging = crate::platform::stub::StubImagingControl::default(); // or existing stub ctor
    assert!(imaging.vision_diagnostics().await.unwrap().is_none());
}
```

(Adapt to the real stub constructor already used in `stub/mod.rs`.)

**Step 2: Run test to verify it fails**

```bash
source ./setenv.sh
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu test_imaging_control_vision_diagnostics_defaults_to_none -- --nocapture
```

Expected: FAIL (method missing)

**Step 3: Write minimal implementation**

Add serializable types (in `traits.rs` near imaging, or a small `vision.rs` re-exported from `platform::common`):

```rust
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VisionSupported {
    pub ir_led: bool,
    pub ircut: bool,
    pub white_led: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VisionDiagnostics {
    /// `"day"` | `"night"` when hardware has been driven; else `None`.
    pub mode: Option<String>,
    pub ae_luma: Option<u8>,
    pub ain0: Option<i32>,
    pub ir_led: Option<bool>,
    pub ircut_a: Option<bool>,
    pub ircut_b: Option<bool>,
    pub white_led: Option<bool>,
    pub supported: VisionSupported,
}
```

On `ImagingControl`:

```rust
async fn vision_diagnostics(&self) -> PlatformResult<Option<VisionDiagnostics>> {
    Ok(None)
}
```

**Step 4: Run test to verify it passes**

Same command as Step 2. Expected: PASS

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/common/traits.rs cross-compile/onvif-rust/src/platform/stub/mod.rs
git commit -m "$(cat <<'EOF'
feat(platform): add ImagingControl::vision_diagnostics hook

EOF
)"
```

---

## Task 2: NightModeController::live_diagnostics

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/imaging.rs` (delegate)
- Test: tempdir GPIO + ain0 + mock FFI in `night_mode.rs` tests

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_live_diagnostics_reads_ae_ain0_and_lamps() {
    let dir = tempfile::tempdir().unwrap();
    let paths = NodePaths::rooted(dir.path(), dir.path());
    for n in [Node::IrCutA, Node::IrCutB, Node::IrLed, Node::WhiteLed] {
        std::fs::write(paths.node(n), "0").unwrap();
    }
    std::fs::write(paths.node(Node::IrLed), "1").unwrap();
    std::fs::write(paths.light_sensor(), "306").unwrap();

    let mut ffi = MockNightModeFfi::new(); // use existing mock type name
    ffi.expect_get_ae_luma().times(1).returning(|| Some(42));

    let ctl = NightModeController::new(/* paths, cfg, ffi, … same as other tests */);
    let v = ctl.live_diagnostics().await;
    assert_eq!(v.ae_luma, Some(42));
    assert_eq!(v.ain0, Some(306));
    assert_eq!(v.ir_led, Some(true));
    assert_eq!(v.ircut_a, Some(false));
    assert!(v.supported.ir_led);
}
```

**Step 2: Run test to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu test_live_diagnostics_reads_ae_ain0_and_lamps -- --nocapture
```

Expected: FAIL

**Step 3: Write minimal implementation**

```rust
fn read_gpio_on(paths: &NodePaths, node: Node) -> Option<bool> {
    let raw = std::fs::read_to_string(paths.node(node)).ok()?;
    let v: u32 = raw.trim().parse().ok()?;
    Some(v != 0)
}

impl NightModeController {
    pub(crate) async fn live_diagnostics(&self) -> crate::platform::common::VisionDiagnostics {
        let ae_luma = self.ffi.get_ae_luma().await;
        let ain0 = read_light_sensor(&self.paths);
        let mode = self.current_mode().await.map(|m| match m {
            DayNight::Day => "day".to_string(),
            DayNight::Night => "night".to_string(),
        });
        VisionDiagnostics {
            mode,
            ae_luma,
            ain0,
            ir_led: self.caps.ir_led.then(|| read_gpio_on(&self.paths, Node::IrLed)).flatten(),
            ircut_a: self.caps.ircut.then(|| read_gpio_on(&self.paths, Node::IrCutA)).flatten(),
            ircut_b: self.caps.ircut.then(|| read_gpio_on(&self.paths, Node::IrCutB)).flatten(),
            white_led: self.caps.white_led.then(|| read_gpio_on(&self.paths, Node::WhiteLed)).flatten(),
            supported: VisionSupported {
                ir_led: self.caps.ir_led,
                ircut: self.caps.ircut,
                white_led: self.caps.white_led,
            },
        }
    }
}
```

In `AnykaImagingControl` / imaging impl:

```rust
async fn vision_diagnostics(&self) -> PlatformResult<Option<VisionDiagnostics>> {
    Ok(Some(self.night.live_diagnostics().await))
}
```

**Step 4: Run tests**

```bash
$CARGO test --target x86_64-unknown-linux-gnu live_diagnostics -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka/night_mode.rs cross-compile/onvif-rust/src/platform/anyka/imaging.rs
git commit -m "$(cat <<'EOF'
feat(anyka): live day/night vision diagnostics snapshot

EOF
)"
```

---

## Task 3: Wire vision into DiagnosticsState snapshot

**Files:**
- Modify: `cross-compile/onvif-rust/src/diagnostics/state.rs`
- Modify: `cross-compile/onvif-rust/src/diagnostics/http.rs`
- Test: `state.rs` tests — vision null without platform imaging

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_snapshot_vision_none_without_platform() {
    let state = DiagnosticsState::new(None, Vec::new());
    let snap = state.snapshot().await;
    assert!(snap.vision.is_none());
}
```

Update existing sync snapshot tests to `.await`.

**Step 2: Run test — expect fail** (method still sync / field missing)

**Step 3: Implement**

```rust
pub struct Snapshot {
    // …existing…
    pub vision: Option<crate::platform::common::VisionDiagnostics>,
}

pub async fn snapshot(&self) -> Snapshot {
    // …existing sync /proc work…
    let vision = match self.platform.as_ref().and_then(|p| p.imaging_control()) {
        Some(imaging) => imaging.vision_diagnostics().await.ok().flatten(),
        None => None,
    };
    Snapshot { /* … */, vision }
}

// http.rs
pub async fn handle_diagnostics(...) -> Json<Snapshot> {
    Json(state.snapshot().await)
}
```

**Step 4: Run diagnostics + imaging tests**

```bash
$CARGO test --target x86_64-unknown-linux-gnu diagnostics:: -- --nocapture
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

Expected: PASS / clean

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/diagnostics/
git commit -m "$(cat <<'EOF'
feat(diagnostics): include live vision block in snapshot

EOF
)"
```

---

## Task 4: Frontend types + Device Information net text

**Files:**
- Modify: `cross-compile/www/src/services/diagnosticsService.ts`
- Modify: `cross-compile/www/src/services/diagnosticsService.test.ts` (if present)
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx`
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.test.tsx`

**Step 1: Failing tests**

- Zod/TS type includes `vision`
- Device Information shows download/upload when `network` present; `—` when null

**Step 2: Run**

```bash
cd cross-compile/www && npx vitest run src/pages/DiagnosticsPage.test.tsx src/services/diagnosticsService.test.ts
```

Expected: FAIL

**Step 3: Implement**

Extend `Diagnostics` interface; in Device Information `dl`, add rows:

- Download → `formatKbps(network.rx_bps)` or `—`
- Upload → `formatKbps(network.tx_bps)` or `—`

Reuse the same kbps formatting as the network chart helpers on the page.

**Step 4: Tests PASS + commit**

```bash
git add cross-compile/www/src/services/diagnosticsService.ts cross-compile/www/src/pages/DiagnosticsPage.tsx cross-compile/www/src/pages/DiagnosticsPage.test.tsx
git commit -m "$(cat <<'EOF'
feat(www): show network rates in diagnostics device info

EOF
)"
```

---

## Task 5: Day / Night Vision card

**Files:**
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx`
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.test.tsx`

**Step 1: Failing tests** for title, AE, ain0, lamps, `n/a` when unsupported / null vision

**Step 2: Implement card**

New card (same visual language as Device Information / Stream Health):

| Row | Value |
| --- | --- |
| Mode | day / night / — |
| AE luma | number / — |
| ain0 | number / — |
| IR LED | On / Off / n/a |
| IR-CUT A/B | On / Off / n/a |
| White LED | On / Off / n/a |

`data-testid`s: `diagnostics-vision-*`

**Step 3: Vitest PASS + commit**

```bash
git commit -m "$(cat <<'EOF'
feat(www): add day/night vision diagnostics section

EOF
)"
```

---

## Task 6: Quality gates + deploy to `.198`

**Step 1: Rust gates**

```bash
source ./setenv.sh
cd cross-compile/onvif-rust
$CARGO fmt --check
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO doc --no-deps
$CARGO build --release --target armv5te-unknown-linux-uclibceabi
```

**Step 2: WWW gates**

```bash
cd cross-compile/www
npm run lint && npm run type-check && npm run test && npm run build
```

**Step 3: Deploy**

```bash
# ARM binary (vision is server-side)
./scripts/deploy_onvif.sh 192.168.2.198 root "${DEPLOY_PASSWORD:?set DEPLOY_PASSWORD}"
# Restart via telnet :24 (stop anyka-init briefly if needed — same as prior deploys)

# WWW assets
# Mirror SD_card_contents/anyka_hack/onvif/www → /mnt/anyka_hack/onvif/www (FTP or scp)
```

**Step 4: Hardware smoke**

```bash
curl -su "admin:${DIAGNOSTICS_PASSWORD:?set DIAGNOSTICS_PASSWORD}" http://192.168.2.198/api/diagnostics | jq '.network, .vision'
```

Expected: vision fields populated on camera; lamps match sysfs.

**Step 5: Final commit if any fixups; push only if user asks**
