# GitHub Security Tab Cleanup — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Clear open GitHub Security code-scanning alerts via two code fixes, CodeQL config/docs hygiene, and bulk dismissals of known false positives / test fixtures.

**Architecture:** Config-first under CodeQL Default Setup (hybrid); if ignores still fail after a UI refresh, fall back to Advanced Setup from `codeql.yml_bak`. Design: `docs/plans/2026-08-29-github-security-cleanup-design.md`.

**Tech Stack:** C (vendor-daemon), Rust (onvif-rust), GitHub CodeQL / code-scanning API (`gh`), SonarCloud (S3776 only).

---

### Task 1: Branch

**Files:** none (git only)

**Step 1: Create branch**

```bash
cd /home/kmk/dev/anyka-dev
git checkout main
git pull --rebase
git checkout -b fix/github-security-cleanup
```

Expected: on `fix/github-security-cleanup`. Include the design doc if it is not yet on the branch.

---

### Task 2: Document hybrid CodeQL policy

**Files:**
- Modify: `.github/codeql/README.md`
- Modify (only if a real gap is found): `.github/codeql/codeql-config.yml`

**Step 1: Update README**

Add a short “Hybrid policy” section:

1. Prefer Default Setup + `.github/codeql/codeql-config.yml`.
2. After any `paths-ignore` change, human must refresh Default Setup (Settings → Code security → CodeQL → Save).
3. If the next analysis still opens alerts under ignored paths (`cross-compile/anyka_reference/**`, `**/tests/**`, `**/*_test.rs`, `**/*_tests.rs`), restore Advanced Setup:
   - Copy `.github/workflows/codeql.yml_bak` → `.github/workflows/codeql.yml`
   - Disable Default Setup in the UI
   - Push and confirm analysis uses `config-file`

Keep existing Default vs Advanced instructions; do not enable Advanced in this task.

**Step 2: Review config**

Confirm `paths-ignore` still lists at least:

- `cross-compile/anyka_reference/**`
- `**/tests/**`
- `**/*_test.rs`
- `**/*_tests.rs`
- toolchain / build / node_modules / target globs already present

Do **not** add globs for whole production crates to silence `#[cfg(test)]` (dismissals handle those). Only add a path if triage finds a recurring ignored-path miss that a glob can fix without hiding real production code.

**Step 3: Commit**

```bash
git add .github/codeql/
git commit -m "$(cat <<'EOF'
docs(codeql): document hybrid Default Setup refresh and Advanced fallback

EOF
)"
```

---

### Task 3: Fix `push.c` heartbeat file mode

**Files:**
- Modify: `cross-compile/vendor-daemon/src/push.c` (~469–473)

**Step 1: Add headers if missing**

Near the top of `push.c`, ensure:

```c
#include <fcntl.h>
#include <stdio.h>
```

(`fopen`/`FILE` already used; `open` needs `fcntl.h`.)

**Step 2: Replace fopen create**

Current:

```c
FILE *hb = fopen(PUSH_HEARTBEAT_PATH, "w");
if (hb) {
    fprintf(hb, "%llu\n", (unsigned long long)frames_pushed);
    fclose(hb);
}
```

Replace with the same pattern as `main.c` log open (mode `0644`):

```c
int hb_fd = open(PUSH_HEARTBEAT_PATH,
                 O_WRONLY | O_CREAT | O_TRUNC, 0644);
if (hb_fd >= 0) {
    FILE *hb = fdopen(hb_fd, "w");
    if (hb) {
        fprintf(hb, "%llu\n", (unsigned long long)frames_pushed);
        fclose(hb); /* closes hb_fd */
    } else {
        close(hb_fd);
    }
}
```

Do not change `PUSH_HEARTBEAT_PATH` or the fprintf payload.

**Step 3: Build vendor-daemon (or at least compile-check)**

```bash
source ./setenv.sh
# Use the project's usual vendor-daemon build path from suggested_commands /
# anyka-embedded-build; minimum is that push.c compiles without new warnings.
```

Expected: clean compile of `push.c`.

**Step 4: Commit**

```bash
git add cross-compile/vendor-daemon/src/push.c
git commit -m "$(cat <<'EOF'
fix(vendor-daemon): create heartbeat file with mode 0644

Avoid fopen(\"w\") default 0666-before-umask so CodeQL
cpp/world-writable-file-creation clears for /tmp/vd_heartbeat.
EOF
)"
```

---

### Task 4: Lower `AppConfig::validate` cognitive complexity

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/types.rs` (`AppConfig::validate`, ~108–366)
- Test: existing `#[cfg(test)]` validate tests in the same file (~1011+)

**Step 1: Extract helpers (TDD-light — existing tests are the contract)**

Keep nested `range` and `osd_name_text` helpers. Split the body into private
functions that take `&mut Vec<String>` (and `&self` or subsection refs), for
example:

- `validate_server_media_ptz(...)` — server, media, ptz (including finite checks)
- `validate_imaging_night(...)` — imaging ranges + night ordering
- `validate_osd_discovery(...)` — osd + discovery
- `validate_stream_profiles(...)` — the profile loop

Exact split may vary; target Sonar cognitive complexity ≤ 15 on `validate`.
Do not change error message strings.

**Step 2: Run host tests**

```bash
source ./setenv.sh
cd cross-compile/onvif-rust
$CARGO fmt
$CARGO test --target x86_64-unknown-linux-gnu --lib config::types
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

Expected: all validate-related tests pass; clippy clean.

**Step 3: Commit**

```bash
git add cross-compile/onvif-rust/src/config/types.rs
git commit -m "$(cat <<'EOF'
refactor(config): split AppConfig::validate to satisfy Sonar S3776

Extract range-check clusters into helpers; preserve error strings.
EOF
)"
```

---

### Task 5: Dismiss remaining open code-scanning alerts

**Files:** none (GitHub API only)

**Step 1: List open alerts**

```bash
rtk gh api 'repos/kkrzysztofik/Anyka_ak3918_hacking_journey/code-scanning/alerts?state=open&per_page=100' \
  --paginate --jq '.[] | "\(.number)\t\(.tool.name)\t\(.rule.id)\t\(.most_recent_instance.location.path)"'
```

**Step 2: Dismiss by bucket**

For each open alert matching the design table, PATCH:

```bash
rtk gh api -X PATCH "repos/kkrzysztofik/Anyka_ak3918_hacking_journey/code-scanning/alerts/ALERT_NUMBER" \
  -f state=dismissed \
  -f dismissed_reason='used in tests' \
  -f dismissed_comment='Test fixture password inside #[cfg(test)] / tests; never shipped to production.'
```

Allowed `dismissed_reason` values: `false positive` | `won't fix` | `used in tests` | `acceptable risk`.

| Rule / path pattern | `dismissed_reason` | Comment theme |
|---|---|---|
| `rust/hard-coded-cryptographic-value` | `used in tests` | Fixture password; not shipped |
| `rust/cleartext-logging` in `tests/` | `used in tests` | Test-only log |
| `rust/cleartext-logging` UUID in `streaming-lib/.../hub/utils.rs` | `false positive` | Session UUID, not a secret |
| `cpp/path-injection` `sound_worker.c` | `false positive` | Prefix + /../ reject + regular-file stat |
| `cpp/type-confusion` `anyka_reference` | `won't fix` | Vendor reference; paths-ignore |
| Snyk `javascript/PT` www scripts | `false positive` | CLI build tool; analyze-bundle cwd-bounded |
| `cpp/world-writable-file-creation` on `push.c` | skip if still open after Task 3 merge+rescan; else wait for next analysis |
| Sonar `rust:S3776` on `types.rs` | skip; expect close after Task 4 + Sonar rescan |

Prefer a small shell loop over alert numbers; do not use `--no-verify` or force anything.

**Step 3: Re-list**

```bash
rtk gh api 'repos/kkrzysztofik/Anyka_ak3918_hacking_journey/code-scanning/alerts?state=open' \
  --paginate --jq 'length'
```

Expected: 0, or only alerts that require a new analysis after the code PR (push.c / S3776).

**Step 4: No git commit** (API-only).

---

### Task 6: Merge, refresh Default Setup, verify

**Step 1: Push and open/merge PR** (when user asks)

Include design + Tasks 2–4 commits.

**Step 2: Human — refresh CodeQL Default Setup**

Settings → Code security and analysis → CodeQL analysis (Default) → Edit → Save changes.

**Step 3: Confirm Security tab**

```bash
rtk gh api 'repos/kkrzysztofik/Anyka_ak3918_hacking_journey/code-scanning/alerts?state=open' --paginate --jq \
  '[.[] | {n: .number, tool: .tool.name, rule: .rule.id, path: .most_recent_instance.location.path}]'
```

Expected: empty array (or only brand-new unrelated findings).

**Step 4: Fallback gate**

If alerts reappear under `anyka_reference/**` or `**/tests/**` after refresh:

1. Restore `.github/workflows/codeql.yml` from `codeql.yml_bak`
2. Disable Default Setup in UI
3. Commit/push Advanced Setup
4. Re-check open alerts

---

## Execution handoff

Plan complete and saved to `docs/plans/2026-08-29-github-security-cleanup.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task via `superpowers:subagent-driven-development`
2. **Inline Execution** — run tasks in this session via `superpowers:executing-plans`

Which approach?
