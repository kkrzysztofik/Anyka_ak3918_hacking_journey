# PR #55 Coverage Improvement — Design

Date: 2026-08-05
Status: approved; implementation plan at `docs/plans/2026-08-05-pr55-coverage-improvement.md`
Branch context: `feat/ir-led-support` (PR #55)
Sonar: [new coverage list](https://sonarcloud.io/component_measures?id=kkrzysztofik_Anyka_ak3918_hacking_journey&pullRequest=55&metric=new_coverage&view=list)

## Problem

SonarCloud quality gate on PR #55 fails on **new coverage**: **60.1%** (required ≥ **80%**).

| Metric | Value |
|---|---|
| New lines to cover | 315 |
| New uncovered lines | 131 |
| Approx. lines still needed for ≥80% | ~68 |

Worst files (new coverage):

| Uncovered | Lines | Cov% | File |
|---|---|---|---|
| 45 | 56 | 19.6% | `platform/anyka/imaging.rs` |
| 42 | 158 | 73.4% | `platform/anyka/night_mode.rs` |
| 25 | 35 | 28.6% | `onvif/ptz/service.rs` |
| 8 | 8 | 0% | `platform/common/traits.rs` (default NotSupported) |
| 4 | 4 | 0% | `platform/anyka/mod.rs` |
| 3+3+1 | — | — | WebUI / config / store leftovers |

Vendor-daemon C and HAL IPC are not driving this gate leaf list. The hole is Rust platform/ONVIF unit coverage.

## Goals

1. Clear the gate (`new_coverage ≥ 80%`).
2. Harden highest-risk new paths (imaging IR apply/lamps, PTZ aux dispatch) with asserts that fail if behavior regresses — not coverage padding.
3. Prefer injectable seams + host mocks; **no new `sonar.coverage.exclusions`** for these paths.

## Decisions

| # | Choice |
|---|---|
| D1 | Approach: extend existing tempfs + FFI mocks (reuse night_mode / imaging harness) |
| D2 | Priority order: `imaging.rs` → `ptz/service.rs` → conditional mop-up |
| D3 | No `NightMode` trait extraction |
| D4 | No Sonar coverage exclusions for platform GPIO/imaging |
| D5 | Night_mode / trait defaults / WebUI only if still &lt;80% after phases 1–2 |

Ponytail cuts applied: no new abstractions; no full `AnykaPlatform` boot test unless short; no unconditional night_mode rewrite; no optional lamp-failure matrix in the committed plan; avoid duplicating plan-ordering asserts already covered in `night_mode` tests.

## Architecture

```text
Imaging tests:
  tempfile NodePaths + seed GPIO nodes
  MockImagingHalTrait (ISP expectations only when apply() calls FFI)
  AnykaImagingControl::with_ffi_and_paths(...)
  drive set_settings / set_ir_lamp / set_white_light / enable_ir_auto
  assert auto flag, settings fields, GPIO node values

PTZ tests:
  existing MockImagingControl + service_with_imaging
  success paths for AuxCommand variants
  lamp_support via get_options Ok / Err

Conditional mop-up:
  night_mode uncovered risk branches
  → ImagingControl default NotSupported (stub)
  → config / WebUI leftovers
```

## Components

| Piece | Change |
|---|---|
| `platform/anyka/imaging.rs` `#[cfg(test)]` | Add Phase 1 tests using existing `with_ffi_and_paths` pattern |
| `onvif/ptz/service.rs` `#[cfg(test)]` | Add Phase 2 success + `lamp_support` cases |
| `night_mode.rs` / traits / WebUI | Phase 3 only if gate still red |
| `sonar-project.properties` | **No change** |

Production code changes only if a tiny testability fix is required to hit an otherwise unreachable branch without exclusions. Prefer zero production diff.

## Testing plan

### Phase 1 — imaging (committed)

| Case | Assert |
|---|---|
| `set_settings` ON | auto off; day apply; GPIO reflects day |
| `set_settings` OFF | auto off; night apply; GPIO reflects night |
| `set_settings` AUTO | auto on; no forced apply required |
| `set_ir_lamp` on/off | auto off; `IrLed` node; `settings.ir_led`; filter mode mirrors `current_mode` |
| `set_white_light` on/off | `WhiteLed` node |
| `enable_ir_auto` | auto on; settings `AUTO` |

### Phase 2 — PTZ (committed)

| Case | Assert |
|---|---|
| Aux IrLamp On/Off/Auto success | mock Ok → service Ok |
| Aux WhiteLight success | Ok |
| `lamp_support` from options | maps ir/white flags |
| `lamp_support` on options Err | → `LampSupport::default()` |

Do not re-test existing error-mapping cases already present.

### Phase 3 — conditional

Night_mode risk holes → trait defaults → config/WebUI — only if Sonar still &lt;80%.

### Verification

```bash
source ./setenv.sh
cd cross-compile
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust
$CARGO clippy --target x86_64-unknown-linux-gnu -p onvif-rust -- -D warnings
$CARGO fmt --check
```

Re-check Sonar PR measures after push: `new_coverage ≥ 80%`.

## Error handling / non-goals

- Out of scope: vendor-daemon C coverage import; `NightMode` trait; Sonar exclusions; coverage-only call-everything tests.
- Existing PTZ aux error maps stay as-is.
- `# ponytail:` if a production seam must be added, keep it one constructor/injection point and document the ceiling.

## Success criteria

- [ ] Phase 1–2 tests land with behavior asserts
- [ ] Host tests / clippy / fmt clean for touched crates
- [ ] Sonar PR quality gate coverage condition OK (≥80% new coverage)
- [ ] No new coverage exclusions for these files
