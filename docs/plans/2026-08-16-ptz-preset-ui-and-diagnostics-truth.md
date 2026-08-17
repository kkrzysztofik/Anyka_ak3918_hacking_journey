# PTZ Preset UI and Diagnostics Truth — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the LiveView preset list reflect what the device actually holds and make the diagnostics PTZ card distinguish measured readings from dead reckoning.

**Architecture:** Three independent slices. The backend promotes a verdict the motor driver already computes (`driver.rs:474-486`) from a log line to a `PtzDiagnostics.step_readback` field. The diagnostics card then deletes its inference heuristic and regroups rows into Measured / Estimated. The LiveView preset card drops its three hardcoded slots for a device-driven list with one honest create/update dialog and a delete confirm.

**Tech Stack:** Rust (vendored ARM toolchain, host tests via `--target x86_64-unknown-linux-gnu`, `mockall`), React 19 + TypeScript, TanStack Query, shadcn/ui (`dialog`, `alert-dialog`, `input`, `button` — all already present), Vitest + React Testing Library.

**Design doc:** `docs/plans/2026-08-16-ptz-preset-ui-and-diagnostics-truth-design.md`

**Before you start:**

```bash
cd /home/kmk/dev/anyka-dev
source ./setenv.sh
```

This exports `$CARGO`. Every Rust command below assumes it. Never use system `cargo` — it fails with version/target mismatches. Clippy additionally needs the toolchain `bin/` first on `PATH`, which `setenv.sh` does.

Rust commands run from `cross-compile/onvif-rust/`. Frontend commands run from `cross-compile/www/`.

---

## Phase 1 — Backend: step-readback provenance

### Task 1: Add the `StepReadback` verdict type

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/common/ptz.rs` (near `TurnOutcome`, ~line 28)

**Step 1: Write the failing test**

Add to the `mod tests` block at the bottom of `src/hal/common/ptz.rs`:

```rust
#[test]
fn test_step_readback_worst_of_prefers_unsupported() {
    use StepReadback::{Unknown, Unsupported, Working};
    assert_eq!(StepReadback::worst_of(Working, Working), Working);
    assert_eq!(StepReadback::worst_of(Working, Unknown), Unknown);
    assert_eq!(StepReadback::worst_of(Unknown, Unsupported), Unsupported);
    assert_eq!(StepReadback::worst_of(Unsupported, Working), Unsupported);
}
```

**Step 2: Run test to verify it fails**

```bash
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu --lib step_readback_worst_of
```

Expected: FAIL to compile — `cannot find type StepReadback in this scope`.

**Step 3: Write minimal implementation**

Add above `TurnOutcome` in `src/hal/common/ptz.rs`:

```rust
/// Whether the kernel's own position accounting is live on this motor driver.
///
/// V500 boards accept `MOTOR_GET_STATUS`, return success, and write nothing into the
/// caller's buffer. A step position read back from such a board is a zero, not a
/// measurement, and rendering it as data misleads whoever is diagnosing the camera.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StepReadback {
    /// The kernel wrote plausible geometry back — step positions mean something.
    Working,
    /// The ioctl succeeded but wrote nothing. Step positions are always zero.
    Unsupported,
    /// Never probed, or the probe itself failed.
    #[default]
    Unknown,
}

impl StepReadback {
    /// Combine two per-motor verdicts into one for the device.
    ///
    /// Pessimistic on purpose: if either motor cannot report its position, the pair
    /// cannot, and a consumer has nothing useful to do with "pan works, tilt does not".
    pub fn worst_of(a: Self, b: Self) -> Self {
        use StepReadback::{Unknown, Unsupported, Working};
        match (a, b) {
            (Unsupported, _) | (_, Unsupported) => Unsupported,
            (Unknown, _) | (_, Unknown) => Unknown,
            (Working, Working) => Working,
        }
    }
}
```

Add `use serde::Serialize;` to the imports at the top of the file if not already present.

**Step 4: Run test to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib step_readback_worst_of
```

Expected: PASS, 1 passed.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
rtk git add cross-compile/onvif-rust/src/hal/common/ptz.rs
rtk git commit -m "feat(ptz): add StepReadback verdict for motor position accounting"
```

---

### Task 2: Return the verdict from `ptz_check_self`

Changing the existing return type beats adding a trait method: no new driver state, no interior mutability. The compiler will point at every call site.

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/common/ptz.rs:46` (trait signature)
- Modify: `cross-compile/onvif-rust/src/hal/stub/ptz.rs:24`
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ptz/driver.rs:700` and `:454`, `:590`

**Step 1: Change the trait signature**

In `src/hal/common/ptz.rs`, the `PtzHalTrait` method at line 46:

```rust
    /// Run the calibration sweep (turn to physical limit, then to middle).
    ///
    /// Returns whether the kernel's position accounting proved live during the sweep —
    /// the driver probes `MOTOR_GET_STATUS` once the motors have demonstrably moved,
    /// which is the only moment the answer is knowable.
    fn ptz_check_self(&self, pin_type: ptz_feedback_pin) -> PlatformResult<StepReadback>;
```

**Step 2: Update the stub**

In `src/hal/stub/ptz.rs`, add `StepReadback` to the `use crate::hal::common::ptz::{...}` import and change:

```rust
    fn ptz_check_self(&self, _pin_type: ptz_feedback_pin) -> PlatformResult<StepReadback> {
        // The host stub has no kernel to read back from.
        Ok(StepReadback::Unknown)
    }
```

**Step 3: Update the driver — `calibrate`**

In `src/hal/anyka/ptz/driver.rs`, change `calibrate` (line 454) to return the verdict. Replace the trailing probe block (lines 474-487) with:

```rust
        // Does the kernel's own position accounting actually move? This distinguishes a
        // working MOTOR_GET_STATUS from one that returns success while writing nothing.
        // The buffer goes in zeroed; steps_one_circle == 0 coming back is physically
        // impossible for a driver that wrote anything at all.
        let readback = match self.get_status() {
            Ok(msg) => {
                tracing::info!(
                    "{}: post-calibration status={} pos={} steps_one_circle={} total_steps={}",
                    self.name,
                    msg.status,
                    msg.pos,
                    msg.steps_one_circle,
                    msg.total_steps
                );
                if msg.pos == 0 && msg.steps_one_circle == 0 && msg.total_steps == 0 {
                    StepReadback::Unsupported
                } else {
                    StepReadback::Working
                }
            }
            Err(e) => {
                tracing::warn!("{}: post-calibration status read failed: {}", self.name, e);
                StepReadback::Unknown
            }
        };
        Ok(readback)
```

and its signature to:

```rust
    fn calibrate(&self, stop_flag: &AtomicBool) -> PlatformResult<StepReadback> {
```

**Step 4: Update the driver — `check_self`**

At line 590:

```rust
    pub fn check_self(&self, _pin_type: ptz_feedback_pin) -> PlatformResult<StepReadback> {
        self.stop_flag.store(false, Ordering::SeqCst);
        let (motor_h, motor_v) = self
            .both_motors()?
            .ok_or_else(|| PlatformError::HardwareUnavailable("PTZ device not opened".into()))?;
        tracing::info!("PTZ calibration: horizontal motor (limit then middle)");
        let readback_h = motor_h.calibrate(&self.stop_flag)?;
        tracing::info!("PTZ calibration: vertical motor (limit then middle)");
        let readback_v = motor_v.calibrate(&self.stop_flag)?;
        let readback = StepReadback::worst_of(readback_h, readback_v);
        tracing::info!("PTZ calibration complete, step readback: {:?}", readback);
        Ok(readback)
    }
```

And the trait impl at line 700:

```rust
    fn ptz_check_self(&self, pin_type: ptz_feedback_pin) -> PlatformResult<StepReadback> {
        self.check_self(pin_type)
    }
```

Add `StepReadback` to the driver's `use crate::hal::common::ptz::{...}` import.

**Step 5: Verify it compiles**

```bash
cd cross-compile/onvif-rust
$CARGO check --target x86_64-unknown-linux-gnu
```

Expected: compiles, with errors only in `ptz_open` (Task 3 fixes that) if any. If `ptz_open` errors with a type mismatch on the `match ffi.ptz_check_self(...)` arms, that is expected — proceed to Task 3 before committing.

**Step 6: Commit**

```bash
cd /home/kmk/dev/anyka-dev
rtk git add cross-compile/onvif-rust/src/hal/
rtk git commit -m "refactor(ptz): return the calibration step-readback verdict instead of logging it"
```

---

### Task 3: Store the verdict on `PTZHandle`

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/common/ptz.rs:77-129`

**Step 1: Write the failing tests**

Add to the `mod tests` block in `src/hal/common/ptz.rs`:

```rust
#[test]
fn test_ptz_open_records_working_step_readback() {
    let mut ffi = MockPtzHalTrait::new();
    ffi.expect_ptz_open().returning(|| Ok(()));
    ffi.expect_ptz_close().returning(|| Ok(()));
    ffi.expect_ptz_check_self()
        .returning(|_| Ok(StepReadback::Working));

    let handle = ptz_open(std::sync::Arc::new(ffi)).expect("open should succeed");
    assert_eq!(handle.step_readback(), StepReadback::Working);
    assert!(handle.self_check_error().is_none());
}

#[test]
fn test_ptz_open_records_unsupported_step_readback() {
    let mut ffi = MockPtzHalTrait::new();
    ffi.expect_ptz_open().returning(|| Ok(()));
    ffi.expect_ptz_close().returning(|| Ok(()));
    ffi.expect_ptz_check_self()
        .returning(|_| Ok(StepReadback::Unsupported));

    let handle = ptz_open(std::sync::Arc::new(ffi)).expect("open should succeed");
    assert_eq!(handle.step_readback(), StepReadback::Unsupported);
}

#[test]
fn test_ptz_open_self_check_failure_leaves_readback_unknown() {
    let mut ffi = MockPtzHalTrait::new();
    ffi.expect_ptz_open().returning(|| Ok(()));
    ffi.expect_ptz_close().returning(|| Ok(()));
    ffi.expect_ptz_check_self()
        .returning(|_| Err(PlatformError::HardwareFailure("sweep timed out".into())));

    let handle = ptz_open(std::sync::Arc::new(ffi)).expect("open still succeeds");
    // A failed sweep says nothing about whether the status ioctl works.
    assert_eq!(handle.step_readback(), StepReadback::Unknown);
    assert!(handle.self_check_error().is_some());
    assert!(handle.is_opened(), "a failed sweep must not close the device");
}
```

**Step 2: Run tests to verify they fail**

```bash
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu --lib ptz_open_records
```

Expected: FAIL to compile — `no method named step_readback`.

**Step 3: Write the implementation**

In `src/hal/common/ptz.rs`, add the field to `PTZHandle`:

```rust
pub struct PTZHandle {
    opened: bool,
    ffi: std::sync::Arc<dyn PtzHalTrait>,
    /// Calibration sweep failure, kept for diagnostics. `None` = the sweep succeeded.
    self_check_error: Option<String>,
    /// Whether the motor driver's position accounting proved live during the sweep.
    step_readback: StepReadback,
}
```

Add the accessor next to `self_check_error()`:

```rust
    /// Whether motor step positions from this device are measurements or always zero.
    pub(crate) fn step_readback(&self) -> StepReadback {
        self.step_readback
    }
```

And update `ptz_open`:

```rust
    // PTZ_FEEDBACK_PIN_NONE = 0 (no feedback pin on this hardware).
    let (self_check_error, step_readback) =
        match ffi.ptz_check_self(ptz_feedback_pin::PTZ_FEEDBACK_PIN_NONE) {
            Ok(readback) => (None, readback),
            Err(e) => {
                tracing::warn!("PTZ self-check failed, continuing anyway: {}", e);
                // The sweep never reached its probe, so the readback question is open.
                (Some(e.to_string()), StepReadback::Unknown)
            }
        };

    Ok(PTZHandle {
        opened: true,
        ffi,
        self_check_error,
        step_readback,
    })
```

**Step 4: Run tests to verify they pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib ptz
```

Expected: PASS. All pre-existing `ptz` tests must still pass — if `test_ptz_open_clean_self_check_records_no_error` fails, its mock needs `Ok(StepReadback::Unknown)` instead of `Ok(())`.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
rtk git add cross-compile/onvif-rust/src/hal/common/ptz.rs
rtk git commit -m "feat(ptz): keep the step-readback verdict on PTZHandle"
```

---

### Task 4: Expose `step_readback` through `PtzDiagnostics`

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/common/traits.rs:545-586`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/ptz_control.rs:205-227`

**Step 1: Write the failing test**

In `src/platform/anyka/ptz_control.rs`, alongside the existing diagnostics tests (near line 547):

```rust
#[tokio::test]
async fn test_diagnostics_reports_unsupported_step_readback() {
    let mut ffi = MockPtzHalTrait::new();
    ffi.expect_ptz_open().returning(|| Ok(()));
    ffi.expect_ptz_close().returning(|| Ok(()));
    ffi.expect_ptz_check_self()
        .returning(|_| Ok(StepReadback::Unsupported));

    let ptz = build_test_ptz(ffi).await;
    let d = ptz.diagnostics();
    assert_eq!(d.step_readback, StepReadback::Unsupported);
}
```

Match the existing helper used by `test_diagnostics_*` in that file for constructing `ptz` — reuse it verbatim rather than inventing `build_test_ptz` if a helper already exists.

**Step 2: Run test to verify it fails**

```bash
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu --lib diagnostics_reports_unsupported
```

Expected: FAIL to compile — `no field step_readback on type PtzDiagnostics`.

**Step 3: Write the implementation**

In `src/platform/common/traits.rs`, add to `PtzDiagnostics`:

```rust
    /// Whether `last_step_pos` is a measurement or a driver artefact.
    ///
    /// `Unsupported` means the motor driver accepts `MOTOR_GET_STATUS` and writes
    /// nothing, so every step position it reports is zero regardless of where the
    /// camera is actually pointing.
    pub step_readback: StepReadback,
```

Import `StepReadback` from `crate::hal::common::ptz`. Add it to `PtzDiagnostics::disabled()`:

```rust
            step_readback: StepReadback::Unknown,
```

In `src/platform/anyka/ptz_control.rs`, alongside the existing `self_check` read at line 205:

```rust
        let step_readback = handle
            .as_ref()
            .map_or(StepReadback::Unknown, |h| h.step_readback());
```

and add `step_readback,` to the `PtzDiagnostics { .. }` literal at line 222.

**Step 4: Run tests to verify they pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib ptz
```

Expected: PASS.

**Step 5: Verify the JSON shape**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib diagnostics
```

Expected: PASS. `#[serde(rename_all = "lowercase")]` on the enum means the field serialises as `"working"` / `"unsupported"` / `"unknown"`. If a snapshot test in `diagnostics/state.rs` compares serialised JSON, update its expectation to include the new key.

**Step 6: Full backend gate**

```bash
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
```

Expected: all pass, zero warnings.

**Step 7: Commit**

```bash
cd /home/kmk/dev/anyka-dev
rtk git add cross-compile/onvif-rust/src/
rtk git commit -m "feat(ptz): report step-readback support in the diagnostics snapshot"
```

---

## Phase 2 — Frontend: the diagnostics card

### Task 5: Type and validate `step_readback`

**Files:**
- Modify: `cross-compile/www/src/services/diagnosticsService.ts:36-46`, `:102-114`
- Test: `cross-compile/www/src/services/diagnosticsService.test.ts`

**Step 1: Write the failing tests**

Add to `diagnosticsService.test.ts`, following the existing `isPtz` validation tests:

```ts
it('accepts a ptz block carrying step_readback', async () => {
  const payload = { ...validSnapshot, ptz: { ...validPtz, step_readback: 'unsupported' } };
  mockFetchJson(payload);
  const result = await getDiagnostics();
  expect(result.ptz?.step_readback).toBe('unsupported');
});

it('accepts a ptz block from an older bundle without step_readback', async () => {
  // A camera running a pre-readback build must not fail validation outright.
  const { step_readback: _omitted, ...legacyPtz } = { ...validPtz, step_readback: 'working' };
  mockFetchJson({ ...validSnapshot, ptz: legacyPtz });
  const result = await getDiagnostics();
  expect(result.ptz?.step_readback).toBeUndefined();
});

it('rejects a ptz block with an unknown step_readback value', async () => {
  mockFetchJson({ ...validSnapshot, ptz: { ...validPtz, step_readback: 'maybe' } });
  await expect(getDiagnostics()).rejects.toThrow(/unexpected shape/);
});
```

Reuse whatever `validSnapshot` / `validPtz` / `mockFetchJson` fixtures the file already defines; do not add new ones.

**Step 2: Run tests to verify they fail**

```bash
cd cross-compile/www
npm test -- diagnosticsService
```

Expected: the third test FAILS (the payload is currently accepted).

**Step 3: Write the implementation**

In `diagnosticsService.ts`, add to the `ptz` type:

```ts
    /** Whether last_step_pos is a measurement. Absent on pre-2026-08-16 bundles. */
    step_readback?: 'working' | 'unsupported' | 'unknown';
```

And to `isPtz`:

```ts
    (value.step_readback === undefined ||
      value.step_readback === 'working' ||
      value.step_readback === 'unsupported' ||
      value.step_readback === 'unknown') &&
```

**Step 4: Run tests to verify they pass**

```bash
npm test -- diagnosticsService
```

Expected: PASS.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
rtk git add cross-compile/www/src/services/diagnosticsService.ts cross-compile/www/src/services/diagnosticsService.test.ts
rtk git commit -m "feat(webui): validate the step_readback field on the diagnostics snapshot"
```

---

### Task 6: Rebuild `PtzCard` around provenance

This task is a net deletion in logic: two derived booleans and a conditional paragraph go, one field arrives.

**Files:**
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx:182-290`
- Test: `cross-compile/www/src/pages/DiagnosticsPage.test.tsx`

**Step 1: Write the failing tests**

```ts
it('hides the step position and states the reason when readback is unsupported', async () => {
  mockDiagnostics({ ...baseDiag, ptz: { ...basePtz, step_readback: 'unsupported' } });
  renderWithProviders(<DiagnosticsPage />);
  await waitFor(() => {
    expect(screen.getByTestId('diagnostics-ptz-step-readback')).toHaveTextContent('unsupported');
  });
  expect(screen.queryByTestId('diagnostics-ptz-step-pos')).not.toBeInTheDocument();
});

it('warns on a freshly homed camera whose driver cannot read back', async () => {
  // The old all-zero heuristic went silent in exactly this state: nothing has moved
  // yet, so it could not tell "at origin" from "cannot measure".
  mockDiagnostics({
    ...baseDiag,
    ptz: { ...basePtz, position: [0, 0, 0], commands_completed: 0, step_readback: 'unsupported' },
  });
  renderWithProviders(<DiagnosticsPage />);
  await waitFor(() => {
    expect(screen.getByTestId('diagnostics-ptz-step-readback')).toHaveTextContent('unsupported');
  });
});

it('shows the step position when readback works', async () => {
  mockDiagnostics({
    ...baseDiag,
    ptz: { ...basePtz, step_readback: 'working', last_step_pos: { pan: 120, tilt: -40, age_ms: 3000 } },
  });
  renderWithProviders(<DiagnosticsPage />);
  await waitFor(() => {
    expect(screen.getByTestId('diagnostics-ptz-step-pos')).toHaveTextContent('120 / -40');
  });
});

it('renders the zoom axis from the tracked position tuple', async () => {
  mockDiagnostics({ ...baseDiag, ptz: { ...basePtz, position: [42, -8.5, 0.25] } });
  renderWithProviders(<DiagnosticsPage />);
  await waitFor(() => {
    expect(screen.getByTestId('diagnostics-ptz-zoom')).toHaveTextContent('0.25');
  });
});

it('labels the tracked axes as estimated', async () => {
  mockDiagnostics({ ...baseDiag, ptz: basePtz });
  renderWithProviders(<DiagnosticsPage />);
  await waitFor(() => {
    expect(screen.getByTestId('diagnostics-ptz-estimated-header')).toHaveTextContent(
      /dead-reckoned/i,
    );
  });
});
```

Reuse the file's existing `mockDiagnostics` / `baseDiag` fixtures; add a `basePtz` only if one is not already there.

**Step 2: Run tests to verify they fail**

```bash
cd cross-compile/www
npm test -- DiagnosticsPage
```

Expected: FAIL — testids `diagnostics-ptz-step-readback`, `diagnostics-ptz-zoom`, `diagnostics-ptz-estimated-header` do not exist.

**Step 3: Write the implementation**

Replace `PtzCard` (lines 182-290) with:

```tsx
const STEP_READBACK_LABEL: Record<string, string> = {
  working: 'working',
  unsupported: 'unsupported',
  unknown: '—',
};

function PtzCard({ ptz }: Readonly<{ ptz: NonNullable<Diagnostics['ptz']> }>) {
  const readback = ptz.step_readback ?? 'unknown';
  // Unsupported means the driver writes nothing into the status buffer, so every step
  // position it reports is a zero. Showing that zero as a value is the bug this fixes.
  const stepPosIsReal = readback !== 'unsupported';

  const measuredRows = [
    { label: 'Config', value: ptz.enabled ? 'enabled' : 'disabled', testId: 'diagnostics-ptz-config' },
    { label: 'Motors', value: ptz.opened ? 'open' : 'not opened', testId: 'diagnostics-ptz-motors' },
    { label: 'Self-check', value: ptz.self_check ?? '—', testId: 'diagnostics-ptz-self-check' },
    {
      label: 'Step readback',
      value: STEP_READBACK_LABEL[readback] ?? '—',
      testId: 'diagnostics-ptz-step-readback',
    },
  ];

  const motionRows = [
    ...(stepPosIsReal && ptz.last_step_pos
      ? [
          {
            label: 'Step pos',
            value: `${ptz.last_step_pos.pan ?? '—'} / ${ptz.last_step_pos.tilt ?? '—'}`,
            testId: 'diagnostics-ptz-step-pos',
          },
        ]
      : []),
    { label: 'Moving', value: ptz.moving ? 'yes' : 'no', testId: 'diagnostics-ptz-moving' },
    {
      label: 'Commands',
      value: String(ptz.commands_completed),
      testId: 'diagnostics-ptz-commands',
    },
  ];

  const axis = (i: number, suffix: string): string =>
    ptz.position ? `${ptz.position[i].toFixed(1)}${suffix}` : '—';

  const estimatedRows = [
    { label: 'Pan', value: axis(0, '°'), testId: 'diagnostics-ptz-pan' },
    { label: 'Tilt', value: axis(1, '°'), testId: 'diagnostics-ptz-tilt' },
    { label: 'Zoom', value: axis(2, ''), testId: 'diagnostics-ptz-zoom' },
    {
      // Always real even when the position beside it is not: this is when the last
      // turn finished, not where it finished.
      label: 'Last motion',
      value: ptz.last_step_pos
        ? `${formatDuration(ptz.last_step_pos.age_ms / 1000)} ago`
        : '—',
      testId: 'diagnostics-ptz-last-motion',
    },
  ];

  const row = ({ label, value, testId }: { label: string; value: string; testId: string }) => (
    <div key={testId} className="flex items-center justify-between">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="font-mono text-white" data-testid={testId}>
        {value}
      </dd>
    </div>
  );

  return (
    <Card className="border-border bg-card overflow-hidden">
      <CardHeader className="border-border border-b">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-cyan-500/10">
            <Move className="h-5 w-5 text-cyan-500" />
          </div>
          <div>
            <CardTitle
              className="text-foreground text-sm font-semibold"
              data-testid="diagnostics-ptz-title"
            >
              PTZ
            </CardTitle>
            <p className="text-muted-foreground text-xs">Motor bring-up and tracked motion</p>
          </div>
        </div>
      </CardHeader>
      <CardContent className="pt-4">
        <dl className="space-y-2 text-sm">
          <p
            className="text-muted-foreground text-[10px] font-semibold tracking-wide uppercase"
            data-testid="diagnostics-ptz-measured-header"
          >
            Measured
          </p>
          {measuredRows.map(row)}
          {ptz.init_error ? (
            <div className="flex items-center justify-between">
              <dt className="text-muted-foreground">Init error</dt>
              <dd className="font-mono text-red-500" data-testid="diagnostics-ptz-init-error">
                {ptz.init_error}
              </dd>
            </div>
          ) : null}
          {readback === 'unsupported' ? (
            <p
              className="text-xs text-yellow-500"
              data-testid="diagnostics-ptz-no-readback"
            >
              The motor driver accepts the status ioctl and writes nothing, so step
              positions are unavailable on this hardware.
            </p>
          ) : null}

          {!ptz.enabled ? (
            <p className="text-muted-foreground text-xs" data-testid="diagnostics-ptz-disabled-note">
              PTZ disabled in configuration
            </p>
          ) : (
            <>
              {motionRows.map(row)}
              <p
                className="text-muted-foreground border-border mt-3 border-t pt-3 text-[10px] font-semibold tracking-wide uppercase"
                data-testid="diagnostics-ptz-estimated-header"
              >
                Estimated · dead-reckoned, not measured
              </p>
              {estimatedRows.map(row)}
            </>
          )}
        </dl>
      </CardContent>
    </Card>
  );
}
```

Write the `·` and `°` escapes as literal `·` and `°` characters in the JSX text, matching how the rest of the file writes them.

**Step 4: Run tests to verify they pass**

```bash
npm test -- DiagnosticsPage
```

Expected: PASS. Pre-existing tests asserting `diagnostics-ptz-position` will now fail — that testid is intentionally replaced by `-pan`/`-tilt`/`-zoom`. Update those assertions; do not re-add the combined row.

**Step 5: Lint and type-check**

```bash
npm run lint && npm run type-check
```

Expected: clean.

**Step 6: Commit**

```bash
cd /home/kmk/dev/anyka-dev
rtk git add cross-compile/www/src/pages/DiagnosticsPage.tsx cross-compile/www/src/pages/DiagnosticsPage.test.tsx
rtk git commit -m "fix(webui): separate measured from dead-reckoned data in the PTZ card"
```

---

## Phase 3 — Frontend: the preset UI

### Task 7: Add the `GetStatus` SOAP client

**Files:**
- Modify: `cross-compile/www/src/services/soap/client.ts:194-232`
- Modify: `cross-compile/www/src/services/ptzService.ts`
- Test: `cross-compile/www/src/services/ptzService.test.ts`

**Step 1: Write the failing test**

```ts
describe('getPtzStatus', () => {
  it('parses the pan, tilt and zoom axes from GetStatusResponse', async () => {
    vi.mocked(soapRequest).mockResolvedValueOnce({
      PTZStatus: {
        Position: {
          PanTilt: { '@_x': '0.42', '@_y': '-0.085' },
          Zoom: { '@_x': '0.25' },
        },
      },
    });
    await expect(getPtzStatus('ProfileToken1')).resolves.toEqual({
      pan: 0.42,
      tilt: -0.085,
      zoom: 0.25,
    });
  });

  it('defaults missing axes to zero rather than NaN', async () => {
    vi.mocked(soapRequest).mockResolvedValueOnce({ PTZStatus: {} });
    await expect(getPtzStatus('ProfileToken1')).resolves.toEqual({ pan: 0, tilt: 0, zoom: 0 });
  });
});
```

Follow the file's existing `soapRequest` mocking style exactly.

**Step 2: Run test to verify it fails**

```bash
cd cross-compile/www
npm test -- ptzService
```

Expected: FAIL — `getPtzStatus is not a function`.

**Step 3: Write the implementation**

Add to `soapBodies` in `client.ts`, after `gotoHomePosition`:

```ts
  getStatus: (profileToken: string) =>
    `<tptz:GetStatus><tptz:ProfileToken>${escapeXml(profileToken)}</tptz:ProfileToken></tptz:GetStatus>`,
```

Add to `ptzService.ts`:

```ts
/** Current PTZ position in ONVIF normalised units. */
export interface PTZStatus {
  pan: number;
  tilt: number;
  zoom: number;
}

/** Read one attribute as a finite number, falling back to 0. */
function axis(node: unknown, attr: string): number {
  const raw = Number(safeString((node as Record<string, unknown>)?.[attr], ''));
  return Number.isFinite(raw) ? raw : 0;
}

/**
 * Read the camera's current position.
 *
 * Note this is the tracked position: on hardware without motor position readback it is
 * dead-reckoned from commanded moves, so callers presenting it must say so.
 */
export async function getPtzStatus(profileToken: string): Promise<PTZStatus> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.ptz,
    soapBodies.getStatus(profileToken),
    'GetStatusResponse',
  );
  const position = (data?.PTZStatus as Record<string, unknown>)?.Position as
    | Record<string, unknown>
    | undefined;
  return {
    pan: axis(position?.PanTilt, '@_x'),
    tilt: axis(position?.PanTilt, '@_y'),
    zoom: axis(position?.Zoom, '@_x'),
  };
}
```

**Step 4: Run tests to verify they pass**

```bash
npm test -- ptzService
```

Expected: PASS.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
rtk git add cross-compile/www/src/services/
rtk git commit -m "feat(webui): add a GetStatus client for the current PTZ position"
```

---

### Task 8: Render the real preset list

**Files:**
- Modify: `cross-compile/www/src/pages/LiveViewPage.tsx:227-245`, `:728-781`
- Test: `cross-compile/www/src/pages/LiveViewPage.test.tsx`

**Step 1: Write the failing tests**

```ts
it('renders every preset the device reports, not three slots', async () => {
  vi.mocked(getPresets).mockResolvedValueOnce([
    { token: 'p1', name: 'Front Door' },
    { token: 'p2', name: 'Back Yard' },
    { token: 'p3', name: 'Garage' },
    { token: 'p4', name: 'Driveway' },
  ]);
  renderWithProviders(<LiveViewPage />);
  await waitFor(() => {
    expect(screen.getByTestId('liveview-preset-p4-button')).toHaveTextContent('Driveway');
  });
});

it('shows an empty state instead of placeholder slots when no presets exist', async () => {
  vi.mocked(getPresets).mockResolvedValueOnce([]);
  renderWithProviders(<LiveViewPage />);
  await waitFor(() => {
    expect(screen.getByTestId('liveview-presets-empty')).toBeInTheDocument();
  });
  expect(screen.queryByTestId('liveview-preset-1-button')).not.toBeInTheDocument();
});
```

Note the testid moves from index-based to token-based — an index-keyed testid was only ever meaningful because the slots were fixed.

**Step 2: Run tests to verify they fail**

```bash
cd cross-compile/www
npm test -- LiveViewPage
```

Expected: FAIL — only three rows render and they use `liveview-preset-1-button`.

**Step 3: Write the implementation**

Delete `displayPresets` (lines 242-245). Replace the preset list body inside `SettingsCardContent`:

```tsx
              <div className="space-y-2">
                {(presets ?? []).map((preset) => (
                  <div key={preset.token} className="group flex items-center gap-2">
                    <Button
                      variant="outline"
                      className="border-border bg-muted text-foreground hover:border-ring hover:bg-muted/80 flex-1 justify-start"
                      data-testid={`liveview-preset-${preset.token}-button`}
                      onClick={() => handleGotoPreset(preset.token)}
                      disabled={pendingToken === preset.token}
                    >
                      <span className="truncate text-xs">{preset.name || preset.token}</span>
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="text-muted-foreground hover:bg-muted hover:text-foreground h-8 w-8"
                      aria-label={`Edit preset ${preset.name}`}
                      data-testid={`liveview-preset-${preset.token}-edit-button`}
                      onClick={() => setEditingPreset(preset)}
                      disabled={pendingToken === preset.token}
                    >
                      <Pencil className="h-3.5 w-3.5" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="text-muted-foreground hover:bg-muted hover:text-accent-red h-8 w-8"
                      aria-label={`Delete preset ${preset.name}`}
                      data-testid={`liveview-preset-${preset.token}-delete-button`}
                      onClick={() => setPresetToDelete(preset)}
                      disabled={pendingToken === preset.token}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                ))}

                {presets?.length === 0 && (
                  <p
                    className="text-muted-foreground py-4 text-center text-xs"
                    data-testid="liveview-presets-empty"
                  >
                    No presets saved.
                  </p>
                )}

                <Button
                  variant="outline"
                  className="border-border text-muted-foreground hover:bg-muted/50 hover:text-foreground mt-2 w-full border-dashed"
                  data-testid="liveview-add-preset-button"
                  onClick={() => setEditingPreset('new')}
                >
                  <span data-testid="liveview-add-preset-label">+ Save current position</span>
                </Button>
              </div>
```

Add the state and swap the icon imports (`Settings2` out; `Pencil`, `Trash2` in):

```tsx
  const [editingPreset, setEditingPreset] = useState<PTZPreset | 'new' | null>(null);
  const [presetToDelete, setPresetToDelete] = useState<PTZPreset | null>(null);
  // Which preset has a mutation in flight. A single page-wide boolean would freeze
  // every row while one of them is being deleted.
  const [pendingToken, setPendingToken] = useState<string | null>(null);
```

Delete `handleAddPreset` (lines 227-231) — the dialog replaces it in Task 10.

**Step 4: Run tests to verify they pass**

```bash
npm test -- LiveViewPage
```

Expected: the two new tests PASS. Pre-existing tests using `liveview-preset-1-button` and `liveview-preset-1-settings-button` will fail — update them to the token-based ids and to the split edit/delete buttons.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
rtk git add cross-compile/www/src/pages/LiveViewPage.tsx cross-compile/www/src/pages/LiveViewPage.test.tsx
rtk git commit -m "fix(webui): render every PTZ preset instead of three fixed slots"
```

---

### Task 9: Confirm before deleting a preset

**Files:**
- Modify: `cross-compile/www/src/pages/LiveViewPage.tsx`
- Test: `cross-compile/www/src/pages/LiveViewPage.test.tsx`

**Step 1: Write the failing tests**

```ts
it('does not delete until the confirm is accepted', async () => {
  const user = userEvent.setup();
  renderWithProviders(<LiveViewPage />);
  await waitFor(() => expect(screen.getByTestId('liveview-preset-preset1-delete-button')));

  await user.click(screen.getByTestId('liveview-preset-preset1-delete-button'));
  await testDialogOpen('liveview-delete-preset-dialog');
  expect(removePreset).not.toHaveBeenCalled();

  await user.click(screen.getByTestId('liveview-delete-preset-confirm'));
  await waitFor(() => expect(removePreset).toHaveBeenCalledWith('ProfileToken1', 'preset1'));
});

it('cancelling the confirm leaves the preset alone', async () => {
  const user = userEvent.setup();
  renderWithProviders(<LiveViewPage />);
  await waitFor(() => expect(screen.getByTestId('liveview-preset-preset1-delete-button')));

  await user.click(screen.getByTestId('liveview-preset-preset1-delete-button'));
  await user.click(screen.getByTestId('liveview-delete-preset-cancel'));
  expect(removePreset).not.toHaveBeenCalled();
});
```

Import `testDialogOpen` from `@/test/dialogTestHelpers`.

**Step 2: Run tests to verify they fail**

```bash
npm test -- LiveViewPage
```

Expected: FAIL — `removePreset` is called immediately, no dialog.

**Step 3: Write the implementation**

Import the `AlertDialog*` set from `@/components/ui/alert-dialog`. Render after the closing `</fieldset>`:

```tsx
      <AlertDialog open={!!presetToDelete} onOpenChange={() => setPresetToDelete(null)}>
        <AlertDialogContent
          className="border-border bg-card text-foreground"
          data-testid="liveview-delete-preset-dialog"
        >
          <AlertDialogHeader>
            <AlertDialogTitle>Delete preset?</AlertDialogTitle>
            <AlertDialogDescription className="text-muted-foreground">
              "{presetToDelete?.name}" will be removed from the camera. This cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel
              className="border-border bg-transparent"
              data-testid="liveview-delete-preset-cancel"
            >
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (presetToDelete) {
                  setPendingToken(presetToDelete.token);
                  removePresetMutation.mutate(presetToDelete.token);
                }
              }}
              className="bg-accent-red text-white hover:bg-red-700"
              data-testid="liveview-delete-preset-confirm"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
```

Clear `pendingToken` and `presetToDelete` in the mutation's `onSettled`:

```tsx
    onSettled: () => {
      setPendingToken(null);
      setPresetToDelete(null);
    },
```

Delete `handleRemovePreset` (lines 233-239); the confirm handler replaces it.

**Step 4: Run tests to verify they pass**

```bash
npm test -- LiveViewPage
```

Expected: PASS.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
rtk git add cross-compile/www/src/pages/LiveViewPage.tsx cross-compile/www/src/pages/LiveViewPage.test.tsx
rtk git commit -m "fix(webui): confirm before deleting a PTZ preset"
```

---

### Task 10: The create/update preset dialog

The one dialog serves both operations because the protocol has only one. `store.rs:374` inserts `PresetData { name, position }` wholesale, so a `SetPreset` carrying an existing token rewrites the position — the dialog must say so before the user commits.

**Files:**
- Create: `cross-compile/www/src/components/common/PresetDialog.tsx`
- Create: `cross-compile/www/src/components/common/PresetDialog.test.tsx`
- Modify: `cross-compile/www/src/pages/LiveViewPage.tsx`

**Step 1: Write the failing tests**

`PresetDialog.test.tsx`:

```tsx
const preset = { token: 'p1', name: 'Front Door' };

it('warns that updating also re-saves the position', async () => {
  vi.mocked(getPtzStatus).mockResolvedValue({ pan: 0.42, tilt: -0.085, zoom: 0 });
  renderWithProviders(
    <PresetDialog profileToken="ProfileToken1" preset={preset} onClose={vi.fn()} />,
  );
  await waitFor(() => {
    expect(screen.getByTestId('preset-dialog-position-warning')).toHaveTextContent(
      /also re-saves the position/i,
    );
  });
});

it('seeds the name field from the preset being edited', async () => {
  renderWithProviders(
    <PresetDialog profileToken="ProfileToken1" preset={preset} onClose={vi.fn()} />,
  );
  await waitFor(() => {
    expect(screen.getByTestId('preset-dialog-name-input')).toHaveValue('Front Door');
  });
});

it('suggests a non-colliding name after presets were deleted', async () => {
  // Two presets remain but their tokens run to Preset5: length+1 would collide.
  renderWithProviders(
    <PresetDialog
      profileToken="ProfileToken1"
      preset="new"
      existing={[
        { token: 'Preset5', name: 'Gate' },
        { token: 'Preset2', name: 'Yard' },
      ]}
      onClose={vi.fn()}
    />,
  );
  await waitFor(() => {
    expect(screen.getByTestId('preset-dialog-name-input')).toHaveValue('Preset 6');
  });
});

it('still saves when the position read fails', async () => {
  vi.mocked(getPtzStatus).mockRejectedValue(new Error('SOAP timeout'));
  const user = userEvent.setup();
  renderWithProviders(
    <PresetDialog profileToken="ProfileToken1" preset={preset} onClose={vi.fn()} />,
  );
  await user.click(await screen.findByTestId('preset-dialog-submit'));
  await waitFor(() =>
    expect(setPreset).toHaveBeenCalledWith('ProfileToken1', 'Front Door', 'p1'),
  );
});
```

**Step 2: Run tests to verify they fail**

```bash
cd cross-compile/www
npm test -- PresetDialog
```

Expected: FAIL — module does not exist.

**Step 3: Write the implementation**

```tsx
/**
 * Preset Dialog
 *
 * Creates or updates a PTZ preset. One dialog for both, because ONVIF has one
 * operation: SetPreset with an existing token replaces the stored position as well as
 * the name (see onvif/ptz/store.rs), and there is no RenamePreset. The position line is
 * the disclosure that keeps "edit the name" from silently re-aiming the preset.
 */
```

Component contract:

- Props: `{ profileToken: string; preset: PTZPreset | 'new'; existing?: PTZPreset[]; onClose: () => void }`
- `useQuery` for `getPtzStatus(profileToken)`, keyed `['ptzStatus', profileToken]`, `staleTime: 0` so opening refetches. No `refetchInterval` — this is read once per dialog, not polled.
- Name state seeded from `preset.name` when editing, or `Preset ${n}` when creating, where `n` is one more than the highest integer suffix across `existing` tokens matching `/^Preset(\d+)$/`, falling back to `existing.length + 1`. Using `length + 1` alone collides after any delete.
- Position line: on success, `Position: 0.42 / -0.085 (estimated)`; on error or while loading, `—`.
- Warning paragraph, testid `preset-dialog-position-warning`, shown only when editing: "This also re-saves the position as {…}." When the status query failed, drop the numbers: "This also re-saves the current position."
- Submit calls `setPreset(profileToken, name.trim(), preset === 'new' ? undefined : preset.token)`. Disabled when the trimmed name is empty. Submitting must not depend on the status query having resolved.
- Mirror the `Dialog`/`DialogContent`/`DialogHeader`/`DialogFooter` structure from `ProfilesPage.tsx:355-405`, but use design tokens (`border-border bg-card text-foreground`) rather than that file's hardcoded hex — LiveView is on tokens and the new component should match its neighbours.
- Testids: `preset-dialog`, `preset-dialog-name-input`, `preset-dialog-position`, `preset-dialog-position-warning`, `preset-dialog-submit`, `preset-dialog-cancel`.

In `LiveViewPage.tsx`, render it and let it own the mutation, invalidating `['ptzPresets', profileToken]` on success:

```tsx
      {editingPreset && (
        <PresetDialog
          profileToken={profileToken}
          preset={editingPreset}
          existing={presets ?? []}
          onClose={() => setEditingPreset(null)}
        />
      )}
```

Remove `setPresetMutation` from `LiveViewPage` once the dialog owns it.

**Step 4: Run tests to verify they pass**

```bash
npm test -- PresetDialog LiveViewPage
```

Expected: PASS.

**Step 5: Full frontend gate**

```bash
npm test
npm run lint
npm run type-check
```

Expected: all pass. `type-check` runs both TS7 and TS6 — both must be clean.

**Step 6: Commit**

```bash
cd /home/kmk/dev/anyka-dev
rtk git add cross-compile/www/src/
rtk git commit -m "feat(webui): add a preset dialog disclosing the position rewrite"
```

---

## Phase 4 — Verification

### Task 11: Full gate and hardware check

**Step 1: Backend**

```bash
cd /home/kmk/dev/anyka-dev && source ./setenv.sh
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
```

**Step 2: ARM build**

Must run from the crate directory, not the workspace root — from `cross-compile/` cargo silently links with the host toolchain.

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/onvif-rust
$CARGO build --release
```

**Step 3: Frontend**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/www
npm test && npm run lint && npm run type-check && npm run build
```

**Step 4: On-device check**

Deploy per @anyka-firmware-upgrade, then confirm the new field is actually served:

```bash
curl -s -u admin:<pw> http://192.168.2.198/api/diagnostics | grep -o '"step_readback":"[a-z]*"'
```

Expected on .198 (V500 board): `"step_readback":"unsupported"`. That board is the reason this work exists — `MOTOR_PARM`/`MOTOR_GET_STATUS` return success and write nothing there.

If it reports `unknown`, the calibration sweep did not complete; check the onvif-rust log for "PTZ calibration complete, step readback:". A `working` result on .198 would mean the sentinel is wrong — do not ship it, revisit the `steps_one_circle == 0` assumption in Task 2.

**Step 5: Code review**

Use @superpowers:requesting-code-review before merging.

---

## Notes for the implementer

- **Do not re-add a combined `Position` row** to the diagnostics card. Splitting pan/tilt/zoom and moving them under an "Estimated" header is the entire point of the change.
- **Do not add a standalone rename button.** ONVIF has no rename operation and `SetPreset` rewrites the position. If a pure rename is later required it needs a backend design of its own.
- **Do not add `dropdown-menu`.** Two icon buttons cost less than a new Radix dependency and a new primitive to test.
- The `pendingToken` state is deliberately a token, not a boolean. A page-wide boolean would disable every preset row while one of them is mid-delete.
