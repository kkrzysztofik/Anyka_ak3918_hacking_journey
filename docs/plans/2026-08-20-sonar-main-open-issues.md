# Sonar Main Open Issues Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Clear all 47 open SonarCloud issues on `main` via code fixes and justified NOSONAR, without changing WS-Security SHA-1 or intentional leak semantics.

**Architecture:** One branch `fix/sonar-main-open-issues` from `main`. Risk-ordered hand edits: mechanical smells → complexity extracts → `ManuallyDrop` / NOSONAR. No `sonar-project.properties` changes. Design: `docs/plans/2026-08-20-sonar-main-open-issues-design.md`.

**Tech Stack:** Rust (onvif-rust, streaming-lib, anyka-init), TypeScript (www), vendored cargo via `source ./setenv.sh`, Vitest/ESLint for www.

---

### Task 1: Branch from main

**Files:** none (git only)

**Step 1: Create branch**

```bash
cd /home/kmk/dev/anyka-dev
git checkout main
git pull --rebase
git checkout -b fix/sonar-main-open-issues
```

Expected: on `fix/sonar-main-open-issues` tracking nothing yet. Design commit may already be on main ahead of origin — include it or rebase as needed so the branch contains the design doc.

**Step 2: Commit** (only if branch creation needs a marker — usually skip)

No code commit for this task.

---

### Task 2: Fix `rust:S1612` method refs (poison / into_inner)

**Files:**
- Modify: `cross-compile/anyka-init/src/sys.rs` (~179, ~282)
- Modify: `cross-compile/anyka-init/src/update.rs` (~432, ~461, ~525)
- Modify: `cross-compile/anyka-init/tests/supervision.rs` (~20)
- Modify: `cross-compile/onvif-rust/src/diagnostics/state.rs` (~140, ~183, ~234)
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs` (listed Sonar lines: 1126, 1130, 1161, 1213, 1298, 1305, 1309 — fix only open findings; leave identical patterns Sonar did not flag if you want minimal diff, or fix siblings in the same functions for consistency)
- Modify: `cross-compile/onvif-rust/src/security/brute_force.rs` (~119)
- Modify: `cross-compile/onvif-rust/src/security/rate_limit.rs` (~154)
- Modify: `cross-compile/streaming-lib/src/protocol/rtsp/session/server_session.rs` (~1605)

**Step 1: Apply replacements**

Replace poison-recovery closures with method refs:

```rust
// Before
.lock().unwrap_or_else(|e| e.into_inner())
.lock().unwrap_or_else(|poisoned| poisoned.into_inner())

// After
.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
```

Use the same `PoisonError::into_inner` form everywhere these are `std::sync::Mutex` poison paths. If a site already imports `PoisonError`, `PoisonError::into_inner` is fine.

**Step 2: Fix remaining S1612 sites (non-poison)**

| File | Approx line | Before → After |
|---|---|---|
| `onvif-rust/src/logging/mod.rs` | ~152 | closure → `OsStr::to_str` / method ref Sonar names (`to_str`) |
| `onvif-rust/src/main.rs` | ~160 | closure → method ref for `get` |
| `onvif-rust/src/onvif/common/validation.rs` | ~72 | `\|c\| c.is_whitespace()` → `char::is_whitespace` |
| `onvif-rust/src/onvif/device/user_types.rs` | ~151 | `\|c\| c.is_alphabetic()` → `char::is_alphabetic` |
| `streaming-lib/.../sdp/mod.rs` | ~174 | closure → `ToString::to_string` / `str::to_string` as Sonar indicates |

Open each site, apply the exact method reference Sonar message names, keep behavior identical.

**Step 3: Verify Rust packages**

```bash
source ./setenv.sh
cd cross-compile
$CARGO fmt
$CARGO clippy --target x86_64-unknown-linux-gnu -p onvif-rust -p streaming-lib -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust -p streaming-lib --lib
cd anyka-init
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu
```

Expected: all green.

**Step 4: Commit**

```bash
git add cross-compile/anyka-init cross-compile/onvif-rust cross-compile/streaming-lib
git commit -m "$(cat <<'EOF'
fix(sonar): use method references for S1612 closures

Replace poison-recovery and predicate closures with method refs so
Sonar S1612 findings on main clear without behavior change.
EOF
)"
```

---

### Task 3: Mechanical smells (`S7089`, `S8863`, `S8856`, `S2208`, `S9045`)

**Files:**
- Modify: `cross-compile/streaming-lib/src/io/bytes_writer.rs` (~1113)
- Modify: `cross-compile/streaming-lib/tests/httpflv_integration_test.rs` (~392, ~831)
- Modify: `cross-compile/streaming-lib/tests/streaming_service_test.rs` (~125)
- Modify: `cross-compile/streaming-lib/src/hub/mock_publisher.rs` (~48, ~50)
- Modify: `cross-compile/streaming-lib/src/validation/h264_file_reader.rs` (~105, ~106)
- Modify: `cross-compile/streaming-lib/src/protocol/rtsp/rtsp_transport.rs` (~5)
- Modify: `cross-compile/streaming-lib/src/protocol/rtsp/session/rtp_counters_tests.rs` (~6)
- Modify: `cross-compile/streaming-lib/src/protocol/rtsp/session/server_session_tests.rs` (~1)
- Modify: `cross-compile/onvif-rust/tests/fixtures/generate_test_aac.rs` (move `#[cfg(test)]` mod to end)
- Modify: `cross-compile/onvif-rust/tests/fixtures/generate_test_h264.rs` (same)

**Step 1: `vec![…]` for S7089**

At each site, replace `let mut v = Vec::new(); v.push(...); …` with a single `vec![...]` literal when values are known up front.

**Step 2: Drop redundant `'static` (S8863)**

In `mock_publisher.rs` and `h264_file_reader.rs`, remove `'static` lifetime annotations Sonar flags as redundant.

**Step 3: Empty line after attribute (S8856)**

In `rtsp_transport.rs`, delete the blank line immediately after the flagged attribute.

**Step 4: Explicit imports (S2208)**

Replace `use foo::*;` in the two test modules with explicit item imports actually used.

**Step 5: Test module order (S9045)**

In both fixture generators, move the `#[cfg(test)] mod tests { … }` block to after all other items in the file.

**Step 6: Verify + commit**

```bash
source ./setenv.sh
cd cross-compile
$CARGO fmt
$CARGO clippy --target x86_64-unknown-linux-gnu -p onvif-rust -p streaming-lib -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu -p streaming-lib
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --test generate_test_aac --test generate_test_h264 2>/dev/null || \
  $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib
```

```bash
git add cross-compile/streaming-lib cross-compile/onvif-rust/tests/fixtures
git commit -m "$(cat <<'EOF'
fix(sonar): clear mechanical streaming and fixture maintainability smells

Use vec literals, drop redundant 'static, fix imports/blank lines, and
move fixture test modules to file end.
EOF
)"
```

---

### Task 4: Cognitive complexity — `handle_play` (rust:S3776)

**Files:**
- Modify: `cross-compile/streaming-lib/src/protocol/rtsp/session/server_session.rs` (`handle_play` starting ~1662)
- Test: existing RTSP session tests under `server_session_tests.rs` / integration tests (no new test required if extracts are pure moves)

**Step 1: Identify extract seams**

`handle_play` is ~16 cognitive complexity (limit 15). Prefer extracting early-return gates already present as private helpers, mirroring prior DESCRIBE splits, for example:

- `authenticate_play_or_reject(...)` — auth block (~1663–1677)
- `reject_invalid_play_range(...)` — Range header validation (~1695–1707)
- `wait_for_play_tracks_or_reject(...)` — play-ready wait (~1709–1738)

Extract only enough to drop measured complexity ≤15; do not restructure the TCP/UDP play setup body unless still over limit.

**Step 2: Implement helpers on `ServerSession`**

Keep signatures borrowing `self` / `rtsp_request` as needed; preserve log fields and status codes.

**Step 3: Verify**

```bash
source ./setenv.sh
cd cross-compile
$CARGO test --target x86_64-unknown-linux-gnu -p streaming-lib
$CARGO clippy --target x86_64-unknown-linux-gnu -p streaming-lib -- -D warnings
```

**Step 4: Commit**

```bash
git add cross-compile/streaming-lib/src/protocol/rtsp/session/server_session.rs
git commit -m "$(cat <<'EOF'
fix(sonar): split handle_play to clear cognitive complexity

Extract play auth/range/track-wait gates so S3776 drops to the allowed
threshold without changing RTSP PLAY behavior.
EOF
)"
```

---

### Task 5: Cognitive complexity — `isDiagnostics` (typescript:S3776)

**Files:**
- Modify: `cross-compile/www/src/services/diagnosticsService.ts` (`isDiagnostics` ~160)
- Test: existing diagnostics service tests if present; otherwise `npm run test`

**Step 1: Extract field-group validators**

Split `isDiagnostics` so complexity ≤15, e.g.:

```typescript
function hasCoreDiagnosticsFields(value: Record<string, unknown>): boolean {
  return (
    typeof value.status === 'string' &&
    typeof value.firmware_version === 'string' &&
    isUptime(value.uptime) &&
    isNullOrNumber(value.cpu_percent) &&
    isNullOrRecord(value.memory) &&
    isNullOrRecord(value.storage) &&
    isNullOrRecord(value.network) &&
    isNullOrNumber(value.stream_frame_age_ms) &&
    Array.isArray(value.components) &&
    Array.isArray(value.degraded_services)
  );
}

function hasOptionalDiagnosticsFields(value: Record<string, unknown>): boolean {
  if (value.vision !== null && !isVision(value.vision)) return false;
  if (value.ptz !== null && value.ptz !== undefined && !isPtz(value.ptz)) return false;
  if (value.wifi !== null && value.wifi !== undefined && !isWifi(value.wifi)) return false;
  return true;
}

function isDiagnostics(value: unknown): value is Diagnostics {
  if (!isRecord(value)) return false;
  return hasCoreDiagnosticsFields(value) && hasOptionalDiagnosticsFields(value);
}
```

Adjust naming to match file style; preserve validation semantics (including “absent PTZ/wifi is fine”).

**Step 2: Verify www**

```bash
cd cross-compile/www
npm run lint && npm run type-check && npm run test
```

**Step 3: Commit**

```bash
git add cross-compile/www/src/services/diagnosticsService.ts
git commit -m "$(cat <<'EOF'
fix(sonar): split diagnostics type guard to clear S3776

Extract core vs optional field checks so isDiagnostics stays under the
cognitive complexity limit.
EOF
)"
```

---

### Task 6: Child reaper — `ManuallyDrop` (S9168 ×2)

**Files:**
- Modify: `cross-compile/anyka-init/src/sys.rs` (`spawn` ~168–172, `spawn_detached` ~304–308)

**Step 1: Replace forget with ManuallyDrop**

```rust
use std::mem::ManuallyDrop;

// In spawn / spawn_detached after successful spawn:
let child = ManuallyDrop::new(child);
let pid = child.id() as Pid;
// Do not into_inner / drop — reaper owns waitpid(-1).
Ok(pid)
```

Keep existing comments explaining reaper ownership; update “forget” wording to ManuallyDrop if needed.

**Step 2: Verify anyka-init**

```bash
source ./setenv.sh
cd cross-compile/anyka-init
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu
```

**Step 3: Commit**

```bash
git add cross-compile/anyka-init/src/sys.rs
git commit -m "$(cat <<'EOF'
fix(sonar): use ManuallyDrop for supervised Child handles

Avoid Drop/wait races with the reaper's waitpid(-1) without changing
ownership semantics Sonar flagged as mem::forget.
EOF
)"
```

---

### Task 7: NOSONAR intentional leaks + SHA-1

**Files:**
- Modify: `cross-compile/onvif-rust/src/app.rs` (~1775)
- Modify: `cross-compile/onvif-rust/src/hal/common/video.rs` (~122)
- Modify: `cross-compile/onvif-rust/src/platform/anyka/video_encoder.rs` (~1076, ~1079) — only Sonar-flagged sites; optional same comment on sibling hard-shutdown helpers if desired for consistency
- Modify: `cross-compile/onvif-rust/src/onvif/ws_security.rs` (~205, ~331)

**Step 1: Leak NOSONAR**

At each flagged `mem::forget`:

```rust
// NOSONAR rust:S9168 -- intentional leak: destructor would race/hang (hard shutdown or stuck FFI join)
std::mem::forget(handle);
```

Tailor the one-liner to the site (hard shutdown vs FFI timeout).

**Step 2: SHA-1 NOSONAR**

At both `Sha1::new()` call sites:

```rust
// NOSONAR rust:S4790 -- WS-Security UsernameToken PasswordDigest requires SHA-1(Nonce+Created+Password)
let mut hasher = Sha1::new();
```

Do **not** change the hash algorithm.

**Step 3: Verify onvif-rust**

```bash
source ./setenv.sh
cd cross-compile
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -p onvif-rust -- -D warnings
```

**Step 4: Commit**

```bash
git add cross-compile/onvif-rust/src/app.rs \
  cross-compile/onvif-rust/src/hal/common/video.rs \
  cross-compile/onvif-rust/src/platform/anyka/video_encoder.rs \
  cross-compile/onvif-rust/src/onvif/ws_security.rs
git commit -m "$(cat <<'EOF'
fix(sonar): NOSONAR intentional leaks and WS-Security SHA-1

Document hard-shutdown/FFI timeout leaks and protocol-mandated SHA-1
PasswordDigest without changing runtime behavior.
EOF
)"
```

---

### Task 8: Full quality gates + PR

**Files:** none new

**Step 1: Full gates**

```bash
source ./setenv.sh
cd cross-compile
$CARGO fmt --check
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu
cd anyka-init
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu
cd ../www
npm run lint && npm run type-check && npm run test
```

Expected: all green.

**Step 2: Push and open PR**

```bash
git push -u origin HEAD
gh pr create --title "fix(sonar): clear remaining main open issues" --body "$(cat <<'EOF'
## Summary
- Clear open SonarCloud findings on main via mechanical fixes, complexity splits, ManuallyDrop for Child/reaper, and NOSONAR for intentional leaks + WS-Security SHA-1.
- Design: `docs/plans/2026-08-20-sonar-main-open-issues-design.md`

## Test plan
- [ ] Workspace fmt/clippy/test (x86_64)
- [ ] anyka-init clippy/test
- [ ] www lint/type-check/test
- [ ] SonarCloud main/PR analysis shows open issues resolved (SHA-1/leaks via NOSONAR)

EOF
)"
```

---

## Reference: issue inventory (47)

See design doc and Sonar open-issues URL. Do not expand scope beyond listed rules/files unless a sibling line in the same function must change for compile consistency.
