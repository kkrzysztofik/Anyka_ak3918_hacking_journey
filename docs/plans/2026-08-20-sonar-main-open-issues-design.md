# Sonar Main Open Issues — Design

Date: 2026-08-20
Status: approved
Branch: `fix/sonar-main-open-issues` (from `main`)
Sonar: [open issues](https://sonarcloud.io/project/issues?issueStatuses=OPEN&id=kkrzysztofik_Anyka_ak3918_hacking_journey)

## Problem

SonarCloud reports **47 open issues** on `main` for project
`kkrzysztofik_Anyka_ak3918_hacking_journey` after a prior clear-pass
(`fix(sonar): clear open project maintainability and security findings`).

| Severity | Count |
|---|---|
| CRITICAL | 6 |
| MAJOR | 12 |
| MINOR | 29 |

| Type | Count |
|---|---|
| CODE_SMELL | 39 |
| BUG | 6 |
| VULNERABILITY | 2 |

Hotspots by rule: `rust:S1612` (24), `rust:S9168` (6), `rust:S7089` (4),
`rust:S8863` (4), `rust:S4790` (2), plus smaller counts for imports, test-module
order, empty line after attribute, and cognitive complexity (+1 over limit) in
Rust and TypeScript.

Crates: `onvif-rust` (24), `streaming-lib` (14), `anyka-init` (8), `www` (1).

## Goals

1. Clear all actionable open findings on main via code fixes or justified
   in-line suppressions.
2. Keep WS-Security PasswordDigest on SHA-1 (protocol-required); do not change
   the algorithm.
3. Preserve intentional leak / ownership semantics for `mem::forget` sites;
   satisfy Sonar without changing behavior.
4. No drive-by refactors; no `sonar-project.properties` multicriteria additions
   for this pass.

## Decisions

| # | Choice |
|---|---|
| D1 | Approach: one PR, risk-ordered hand edits (mechanical → complexity → ownership/suppressions) |
| D2 | SHA-1 (`rust:S4790`): keep `Sha1`; inline `// NOSONAR` at the two `Sha1::new()` sites with UsernameToken justification |
| D3 | `mem::forget` Child/reaper (`anyka-init/src/sys.rs` ×2): replace with `ManuallyDrop` |
| D4 | `mem::forget` hard-shutdown / FFI-timeout (`app.rs`, `hal/common/video.rs`, `video_encoder.rs`): keep forget + `// NOSONAR` + one-line rationale |
| D5 | No `sonar-project.properties` e6 (or other) ignores for this pass |
| D6 | Complexity: extract helpers only enough to get ≤15 cognitive complexity |

## Why `mem::forget` is intentional

| Location | Forgotten value | Reason |
|---|---|---|
| `anyka-init/src/sys.rs` | `std::process::Child` | Reaper owns `waitpid(-1)`. Dropping `Child` would race that wait. |
| `onvif-rust/src/app.rs`, `platform/anyka/video_encoder.rs` | App / stream / encoder handles | Hard/unsafe shutdown: destructors would run SDK cleanup after a torn-down or stuck state. |
| `onvif-rust/src/hal/common/video.rs` | Timed-out FFI `JoinHandle` | Thread stuck in SDK; joining forever blocks shutdown. |

Sonar `S9168` correctly notes skipped destructors — that is the desired effect.
Removing forget without a replacement ownership model would reintroduce races or hangs.

## Fix map

| Rule | Count | Action |
|---|---|---|
| `rust:S1612` | 24 | Method refs (`into_inner`, `to_str`, `get`, `is_whitespace`, …) |
| `rust:S9168` | 6 | `ManuallyDrop` (Child ×2) or NOSONAR (leaks ×4) |
| `rust:S4790` | 2 | NOSONAR only (keep SHA-1) |
| `rust:S7089` | 4 | `vec![…]` literals |
| `rust:S2208` | 2 | Explicit imports in RTSP test modules |
| `rust:S9045` | 2 | Move test modules to end of fixture files |
| `rust:S8863` | 4 | Drop redundant `'static` |
| `rust:S8856` | 1 | Remove empty line after attribute |
| `rust:S3776` / `typescript:S3776` | 2 | Extract helpers (`server_session.rs`, `diagnosticsService.ts`) |

## Sequencing

1. Mechanical maintainability (`S1612`, `S7089`, `S8863`, `S8856`, `S2208`, `S9045`)
2. Complexity splits
3. Ownership / suppressions (`ManuallyDrop`, NOSONAR leaks, NOSONAR SHA-1)
4. Quality gates → PR

## Testing & success criteria

- Workspace: `$CARGO fmt --check`, clippy `-D warnings`, tests on
  `x86_64-unknown-linux-gnu` under `cross-compile/`
- `anyka-init`: host test + clippy for that package
- `www`: lint and Vitest after diagnosticsService change must pass
- **Blocked gate (pre-existing):** `www` `npm run type-check` may fail only with the
  documented NetworkPage TS2345 errors (identical on `main`; this work does not
  touch `NetworkPage*`). Treat that failure as expected until NetworkPage is fixed
  separately — do not expand this pass to clear it.
- Behavior: no digest algorithm change; Child reaping and hard-shutdown leaks unchanged
- Done when listed findings are fixed or NOSONAR’d as designed and applicable gates are green
  (except the documented NetworkPage type-check blocker)

## Non-goals

- Changing WS-Security hash algorithm
- Broad clippy cleanups beyond the open issue list
- Sonar property multicriteria for SHA-1 or forget
- Unrelated refactors or new features

## Implementation plan

Follow-up: `docs/plans/2026-08-20-sonar-main-open-issues.md`
