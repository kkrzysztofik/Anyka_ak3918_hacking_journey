# GitHub Security Tab Cleanup — Design

**Date:** 2026-08-29
**Status:** Approved
**Security:** [repo Security tab](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/security)

## Problem

Dependabot and secret scanning are clean (0 open). Code scanning still has
open noise across three tools:

| Tool | Rule | Open | Nature |
|---|---|---:|---|
| CodeQL | `rust/hard-coded-cryptographic-value` | ~44 | Test / `#[cfg(test)]` fixture passwords |
| CodeQL | `rust/cleartext-logging` | 5 | Test password logs + session UUIDs |
| CodeQL | `cpp/path-injection` | 1 | `sound_worker.c` (already path-validated) |
| CodeQL | `cpp/world-writable-file-creation` | 1 | `push.c` heartbeat via `fopen("w")` |
| CodeQL | `cpp/type-confusion` | 1 | Vendor `anyka_reference` (already in `paths-ignore`) |
| SnykCode | `javascript/PT` | 3 | www CLI scripts (sanitizer / build-tool FPs) |
| SonarCloud | `rust:S3776` | 1 | `AppConfig::validate` complexity 18 > 15 |

CodeQL Default Setup is enabled (`extended` suite). Repo config at
`.github/codeql/codeql-config.yml` already ignores vendor and
`**/tests/**` / `*_test(s).rs`, but ignores are not fully sticking (vendor
and some test paths still alert). Inline `#[cfg(test)]` modules in
production `.rs` files are outside those path globs.

## Goals

1. Clear the Security tab of open alerts from this triage.
2. Fix only two real code issues: heartbeat file mode and Sonar complexity.
3. Reduce recurrence under Default Setup; document Advanced Setup as fallback.
4. No auth/password storage redesign; no moving `#[cfg(test)]` modules; no
   www script rewrites beyond dismissals.

## Decisions

| # | Choice |
|---|---|
| D1 | Approach 1 (config-first, then triage) |
| D2 | Stay on CodeQL Default Setup; expand/document config; refresh UI after merge |
| D3 | If next scan still alerts on ignored paths → enable Advanced from `codeql.yml_bak`, disable Default |
| D4 | Dismiss all hard-coded password alerts as `used in tests` (fixtures, not shipped) |
| D5 | Code fixes only: `push.c` heartbeat mode + `AppConfig::validate` helper split |
| D6 | Dismiss path-injection, type-confusion, cleartext-logging, Snyk PT with short reasons |

## Design

### 1. Scanning setup

- Keep / lightly extend `.github/codeql/codeql-config.yml` `paths-ignore`
  (vendor, tests, build artifacts already present).
- Update `.github/codeql/README.md` with the hybrid policy: after config
  edits, refresh Default Setup in GitHub Settings; if ignored paths still
  alert, switch to Advanced Setup (`codeql.yml` restored from bak) and
  disable Default.
- Manual step (human): Settings → Code security → CodeQL Default Setup →
  Save/refresh after the config PR lands.

### 2. Code fixes

**`cross-compile/vendor-daemon/src/push.c`** — heartbeat create mode.

Replace `fopen(PUSH_HEARTBEAT_PATH, "w")` with `open(..., O_WRONLY |
O_CREAT | O_TRUNC, 0644)` + `fdopen`, matching the log-file pattern in
`main.c`. Same path (`/tmp/vd_heartbeat`) and contents; only mode changes
so a permissive umask cannot leave a world-writable file.

**`cross-compile/onvif-rust/src/config/types.rs`** — Sonar `S3776`.

Extract existing check clusters inside `AppConfig::validate` into private
helpers (e.g. server/media/ptz/imaging/osd/discovery/stream-profile
ranges) until cognitive complexity ≤ 15. Preserve error strings and
semantics; no new public API.

### 3. Alert triage (dismiss)

| Bucket | Reason | Comment theme |
|---|---|---|
| hard-coded passwords (tests / `#[cfg(test)]`) | `used in tests` | Fixture; not shipped |
| cleartext-logging (test passwords) | `used in tests` | Test-only |
| cleartext-logging (UUID in hub utils) | `false positive` | Session id, not a secret |
| `sound_worker.c` path-injection | `false positive` | Prefix + `/../` reject + regular-file `stat` |
| `anyka_reference` type-confusion | `won't fix` | Vendor; `paths-ignore` |
| Snyk `javascript/PT` | `false positive` | CLI build tools; bundle path already cwd-bounded |

Use `gh api` to dismiss in bulk where practical.

### 4. Verification & done criteria

1. Local: vendor-daemon builds; onvif-rust host tests for config validation
   pass; fmt/clippy clean on touched Rust.
2. After merge + Default Setup refresh: open code-scanning alerts ≈ 0.
3. If ignored paths reappear → execute Advanced Setup fallback.

**Done when:** Security tab has no open alerts from this triage; both code
fixes merged; CodeQL README documents hybrid policy; fallback path written
(not necessarily executed).

## Out of scope

- Dependabot / secret scanning (already clean)
- Moving inline `#[cfg(test)]` into `*_test.rs`
- Hardening `precompress.mjs` / further www path work
- Changing `sound_worker.c` validation logic
- Editing vendor reference sources under `anyka_reference/`
