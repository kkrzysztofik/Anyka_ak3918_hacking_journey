# PR #55 Coverage Improvement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Raise PR #55 Sonar new coverage from ~60% to ≥80% by adding behavior-asserting host unit tests for imaging IR/lamp paths and PTZ auxiliary dispatch, without new Sonar exclusions or abstractions.

**Architecture:** Reuse existing injectable seams (`AnykaImagingControl::with_ffi_and_paths`, temp `NodePaths`, `MockImagingHalTrait`, `MockImagingControl` + `service_with_imaging`). Phase 1 imaging → Phase 2 PTZ → Phase 3 mop-up only if still under 80%.

**Tech Stack:** Rust, tokio, mockall, tempfile, vendored cargo at `toolchain/arm-anykav200-crosstool-ng/bin/cargo`, SonarCloud PR measures.

**Design:** `docs/plans/2026-08-05-pr55-coverage-improvement-design.md`

## Global Constraints

- Host tests: `--target x86_64-unknown-linux-gnu`
- Prefer `source ./setenv.sh` then `$CARGO`
- No new `sonar.coverage.exclusions` for platform/imaging/night_mode
- No `NightMode` trait extraction
- Prefer zero production code changes; only add a tiny seam if a branch is otherwise unreachable
- Assert observable behavior (settings + GPIO + Ok/Err), not coverage-only calls
- Do not re-test existing PTZ aux error-mapping cases
- Ponytail: fewest files, reuse harnesses already in those modules

## File map

| File | Role |
|---|---|
| `cross-compile/onvif-rust/src/platform/anyka/imaging.rs` | Phase 1 tests in existing `mod tests` |
| `cross-compile/onvif-rust/src/onvif/ptz/service.rs` | Phase 2 tests in existing `mod tests` |
| `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs` | Phase 3 only if needed |
| `cross-compile/onvif-rust/src/platform/common/traits.rs` / stub | Phase 3 only if needed |
| `docs/plans/2026-08-05-pr55-coverage-improvement-design.md` | Spec (already written) |

---

### Task 1: Imaging — `set_settings` ON / OFF / AUTO

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/imaging.rs` (`mod tests`, after existing get_options tests)

**Interfaces:**
- Consumes: `AnykaImagingControl::with_ffi_and_paths`, `NodePaths::rooted`, `MockImagingHalTrait`, `ImagingSettings`, `IrCutFilterMode`, `NightModeController` via `control.night_mode()`
- Produces: three tests covering ON/OFF/AUTO branches of `set_settings`

- [ ] **Step 1: Add a small local helper in `mod tests` (optional but DRY)**

```rust
fn seeded_imaging() -> (tempfile::TempDir, AnykaImagingControl) {
    use crate::hal::common::imaging::MockImagingHalTrait;
    use crate::platform::anyka::night_mode::{Node, NodePaths};

    let dir = tempfile::tempdir().expect("tempdir");
    let paths = NodePaths::rooted(dir.path(), dir.path());
    for n in [Node::IrCutA, Node::IrCutB, Node::IrLed, Node::WhiteLed] {
        std::fs::write(paths.node(n), "0").unwrap();
    }
    let mut ffi = MockImagingHalTrait::new();
    // apply() may call set_ir_filter; allow any for helpers that share the mock,
    // or set expectations per-test instead of here.
    ffi.expect_set_ir_filter().returning(|_| 0);
    let control = AnykaImagingControl::with_ffi_and_paths(
        Arc::new(ffi),
        paths,
        crate::config::types::ImagingConfig::default(),
    );
    (dir, control)
}
```

Prefer per-test FFI expectations when asserting call counts (see Step 2).

- [ ] **Step 2: Write failing tests for ON and OFF**

```rust
#[tokio::test]
async fn test_set_settings_on_applies_day_and_writes_gpio() {
    use crate::hal::common::imaging::MockImagingHalTrait;
    use crate::onvif::types::common::IrCutFilterMode;
    use crate::platform::anyka::night_mode::{DayNight, Node, NodePaths};

    let dir = tempfile::tempdir().expect("tempdir");
    let paths = NodePaths::rooted(dir.path(), dir.path());
    for n in [Node::IrCutA, Node::IrCutB, Node::IrLed, Node::WhiteLed] {
        std::fs::write(paths.node(n), "9").unwrap();
    }

    let mut ffi = MockImagingHalTrait::new();
    ffi.expect_set_ir_filter()
        .withf(|enabled| !*enabled) // Day → set_ir_filter(false)
        .times(1)
        .returning(|_| 0);
    // Day → set_ir_filter(false); Night → set_ir_filter(true)
    // (see night_mode apply: set_ir_filter(matches!(target, DayNight::Night)))

    let control = AnykaImagingControl::with_ffi_and_paths(
        Arc::new(ffi),
        paths.clone(),
        crate::config::types::ImagingConfig::default(),
    );

    let mut settings = control.get_settings().await.unwrap();
    settings.ir_cut_filter = IrCutFilterMode::ON;
    control.set_settings(&settings).await.unwrap();

    assert_eq!(control.get_settings().await.unwrap().ir_cut_filter, IrCutFilterMode::ON);
    assert_eq!(control.night_mode().current_mode().await, DayNight::Day);
    assert_eq!(std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(), "0");
}

#[tokio::test]
async fn test_set_settings_off_applies_night_and_writes_gpio() {
    // Same harness; IrCutFilterMode::OFF; expect set_ir_filter(true); IrLed == "1";
    // current_mode == DayNight::Night
}

#[tokio::test]
async fn test_set_settings_auto_enables_auto_without_forced_apply() {
    use crate::hal::common::imaging::MockImagingHalTrait;
    use crate::onvif::types::common::IrCutFilterMode;
    use crate::platform::anyka::night_mode::{Node, NodePaths};

    let dir = tempfile::tempdir().expect("tempdir");
    let paths = NodePaths::rooted(dir.path(), dir.path());
    for n in [Node::IrCutA, Node::IrCutB, Node::IrLed, Node::WhiteLed] {
        std::fs::write(paths.node(n), "0").unwrap();
    }
    // No set_ir_filter expectation — AUTO must not call apply()
    let ffi = MockImagingHalTrait::new();

    let control = AnykaImagingControl::with_ffi_and_paths(
        Arc::new(ffi),
        paths.clone(),
        crate::config::types::ImagingConfig::default(),
    );

    let before_led = std::fs::read_to_string(paths.node(Node::IrLed)).unwrap();
    let mut settings = control.get_settings().await.unwrap();
    settings.ir_cut_filter = IrCutFilterMode::AUTO;
    control.set_settings(&settings).await.unwrap();

    assert_eq!(control.get_settings().await.unwrap().ir_cut_filter, IrCutFilterMode::AUTO);
    assert_eq!(std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(), before_led);
}
```

**Implementer note:** Confirm `set_ir_filter` polarity from `night_mode::plan` / existing `test_apply_night_*` before locking `.withf`. Day → IrLed `"0"`; Night → IrLed `"1"`.

- [ ] **Step 3: Run tests — expect FAIL until pasted into file, then PASS**

```bash
source ./setenv.sh
cd cross-compile
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib \
  test_set_settings_on_applies_day_and_writes_gpio \
  test_set_settings_off_applies_night_and_writes_gpio \
  test_set_settings_auto_enables_auto_without_forced_apply
```

Expected: PASS

- [ ] **Step 4: Commit** (when user asks)

```bash
git add cross-compile/onvif-rust/src/platform/anyka/imaging.rs
git commit -m "$(cat <<'EOF'
test(imaging): cover set_settings ON/OFF/AUTO day-night paths

EOF
)"
```

---

### Task 2: Imaging — lamp APIs

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/imaging.rs` (`mod tests`)

**Interfaces:**
- Consumes: same harness as Task 1; `set_ir_lamp`, `set_white_light`, `enable_ir_auto`, `current_mode`
- Produces: lamp + auto tests covering lines ~319–347

- [ ] **Step 1: Write tests**

```rust
#[tokio::test]
async fn test_set_ir_lamp_writes_node_and_mirrors_filter_mode() {
    use crate::hal::common::imaging::MockImagingHalTrait;
    use crate::onvif::types::common::IrCutFilterMode;
    use crate::platform::anyka::night_mode::{DayNight, Node, NodePaths};

    let dir = tempfile::tempdir().expect("tempdir");
    let paths = NodePaths::rooted(dir.path(), dir.path());
    for n in [Node::IrCutA, Node::IrCutB, Node::IrLed, Node::WhiteLed] {
        std::fs::write(paths.node(n), "0").unwrap();
    }
    let mut ffi = MockImagingHalTrait::new();
    ffi.expect_set_ir_filter().returning(|_| 0);

    let control = AnykaImagingControl::with_ffi_and_paths(
        Arc::new(ffi),
        paths.clone(),
        crate::config::types::ImagingConfig::default(),
    );

    // Force a known mode so ir_cut_filter mirroring is deterministic
    let mut settings = control.get_settings().await.unwrap();
    settings.ir_cut_filter = IrCutFilterMode::OFF;
    control.set_settings(&settings).await.unwrap();
    assert_eq!(control.night_mode().current_mode().await, DayNight::Night);

    control.set_ir_lamp(true).await.unwrap();
    assert_eq!(std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(), "1");
    let s = control.get_settings().await.unwrap();
    assert!(s.ir_led);
    assert_eq!(s.ir_cut_filter, IrCutFilterMode::OFF); // mirrors Night

    control.set_ir_lamp(false).await.unwrap();
    assert_eq!(std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(), "0");
    assert!(!control.get_settings().await.unwrap().ir_led);
}

#[tokio::test]
async fn test_set_white_light_writes_white_led_node() {
    // seed nodes including WhiteLed; set_white_light(true) → "1"; false → "0"
    // MockImagingHalTrait::new() with no ISP expectations required
}

#[tokio::test]
async fn test_enable_ir_auto_sets_settings_to_auto() {
    // enable_ir_auto().await; get_settings().ir_cut_filter == AUTO
}
```

- [ ] **Step 2: Run**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib \
  test_set_ir_lamp_writes_node_and_mirrors_filter_mode \
  test_set_white_light_writes_white_led_node \
  test_enable_ir_auto_sets_settings_to_auto
```

Expected: PASS

- [ ] **Step 3: Commit** (when user asks)

```bash
git add cross-compile/onvif-rust/src/platform/anyka/imaging.rs
git commit -m "$(cat <<'EOF'
test(imaging): cover IR/white lamp and enable_ir_auto paths

EOF
)"
```

---

### Task 3: PTZ — auxiliary success + `lamp_support`

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/ptz/service.rs` (`mod tests`, near existing `test_dispatch_auxiliary_*`)

**Interfaces:**
- Consumes: `service_with_imaging`, `MockImagingControl`, `AuxCommand`, `LampState`, private `lamp_support` / `dispatch_auxiliary`
- Produces: success-path and lamp_support coverage for uncovered lines ~130–147, ~360–385

- [ ] **Step 1: Write success-path tests**

```rust
#[tokio::test]
async fn test_dispatch_auxiliary_ir_lamp_on_off_auto_succeed() {
    use crate::platform::common::traits::MockImagingControl;
    use auxiliary::{AuxCommand, LampState};

    let mut imaging = MockImagingControl::new();
    imaging.expect_set_ir_lamp().with(eq(true)).times(1).returning(|_| Ok(()));
    imaging.expect_set_ir_lamp().with(eq(false)).times(1).returning(|_| Ok(()));
    imaging.expect_enable_ir_auto().times(1).returning(|| Ok(()));

    let service = service_with_imaging(imaging);
    service.dispatch_auxiliary(AuxCommand::IrLamp(LampState::On)).await.unwrap();
    service.dispatch_auxiliary(AuxCommand::IrLamp(LampState::Off)).await.unwrap();
    service.dispatch_auxiliary(AuxCommand::IrLamp(LampState::Auto)).await.unwrap();
}

#[tokio::test]
async fn test_dispatch_auxiliary_white_light_succeeds() {
    let mut imaging = MockImagingControl::new();
    imaging.expect_set_white_light().with(eq(true)).times(1).returning(|_| Ok(()));
    let service = service_with_imaging(imaging);
    service.dispatch_auxiliary(auxiliary::AuxCommand::WhiteLight(true)).await.unwrap();
}
```

- [ ] **Step 2: Write `lamp_support` tests**

```rust
#[tokio::test]
async fn test_lamp_support_maps_imaging_options() {
    use crate::platform::common::traits::{ImagingOptions, MockImagingControl};

    let mut imaging = MockImagingControl::new();
    imaging.expect_get_options().returning(|| {
        Ok(ImagingOptions {
            ir_led_supported: true,
            white_light_supported: true,
            ..ImagingOptions::default_options()
        })
    });

    let service = service_with_imaging(imaging);
    let lamps = service.lamp_support().await;
    assert!(lamps.ir_lamp);
    assert!(lamps.white_light);
}

#[tokio::test]
async fn test_lamp_support_defaults_when_get_options_fails() {
    use crate::platform::common::{PlatformError, traits::MockImagingControl};

    let mut imaging = MockImagingControl::new();
    imaging
        .expect_get_options()
        .returning(|| Err(PlatformError::HardwareFailure("boom".into())));

    let service = service_with_imaging(imaging);
    let lamps = service.lamp_support().await;
    assert!(!lamps.ir_lamp);
    assert!(!lamps.white_light);
}
```

**Implementer note:** Confirm `ImagingOptions` field names / `default_options()` and `eq` import (`mockall::predicate::eq`) match the crate’s existing test style in this file.

- [ ] **Step 3: Run**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib \
  test_dispatch_auxiliary_ir_lamp_on_off_auto_succeed \
  test_dispatch_auxiliary_white_light_succeeds \
  test_lamp_support_maps_imaging_options \
  test_lamp_support_defaults_when_get_options_fails
```

Expected: PASS

- [ ] **Step 4: Commit** (when user asks)

```bash
git add cross-compile/onvif-rust/src/onvif/ptz/service.rs
git commit -m "$(cat <<'EOF'
test(ptz): cover auxiliary success paths and lamp_support

EOF
)"
```

---

### Task 4: Quality gate + conditional mop-up

**Files:**
- Possibly modify: `night_mode.rs` tests, stub/`traits.rs` exercise, WebUI tests — **only if needed**

**Interfaces:**
- Consumes: Sonar PR #55 measures API / dashboard
- Produces: `new_coverage ≥ 80%`

- [ ] **Step 1: Run full onvif-rust lib tests + clippy + fmt**

```bash
source ./setenv.sh
cd cross-compile
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -p onvif-rust --all-targets -- -D warnings
$CARGO fmt --check
```

Expected: all green

- [ ] **Step 2: Estimate coverage locally (optional) or push and re-query Sonar**

```bash
curl -sS "https://sonarcloud.io/api/measures/component?component=kkrzysztofik_Anyka_ak3918_hacking_journey&metricKeys=new_coverage,new_uncovered_lines,new_lines_to_cover&pullRequest=55"
```

Target: `new_coverage` ≥ `80`

- [ ] **Step 3: If still &lt;80%, mop up in this order**

1. `night_mode.rs` uncovered risk branches only (sensor-absent hold, IrCutB-only probe, join/GPIO error arms) — copy style from existing tests in that file
2. Exercise `ImagingControl` default `set_ir_lamp` / `set_white_light` / `enable_ir_auto` via stub platform type that does not override them
3. WebUI / `ptz/ops/config.rs` leftovers only if still short

Do **not** add Sonar exclusions.

- [ ] **Step 4: Final commit(s) when user asks; PR quality gate coverage OK**

---

## Self-review

1. **Spec coverage:** Goals D1–D5, Phases 1–3, verification, non-goals → Tasks 1–4.
2. **Placeholders:** Implementer notes call out polarity/`ImagingOptions` confirmation against live code (not TBD features).
3. **Types:** `IrCutFilterMode`, `DayNight`, `AuxCommand`, `LampState`, `LampSupport`, `MockImagingControl`, `MockImagingHalTrait` match current tree.

## Execution handoff

Plan saved to `docs/plans/2026-08-05-pr55-coverage-improvement.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — fresh subagent per task, review between tasks
2. **Inline Execution** — execute tasks in this session with checkpoints

Which approach?
