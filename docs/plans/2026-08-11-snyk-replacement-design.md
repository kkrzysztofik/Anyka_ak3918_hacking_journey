# Design: Replace the Snyk integration

**Date:** 2026-08-11
**Status:** Approved
**Approach:** Delete Snyk outright. CodeQL and SonarCloud already cover the SAST
surface; Dependabot alerts already cover npm. Add `cargo audit` for the one thing
nothing covers today — Rust crate advisories — and a `.github/dependabot.yml` the
repo has never had.

## Problem

Snyk runs in `main-ci.yml` as the `security-scans` job and produces three things.
Evidence below is from CI run `31474879853` (2026-08-11) and the GitHub
code-scanning API, not from the workflow YAML's description of itself.

**Snyk Code (SAST)** — four scans: `onvif-rust`, `streaming-lib`, `validation/rust`,
`www`. Every one of them currently ends:

```
 ERROR   Forbidden (SNYK-CLI-0000)
##[error]Process completed with exit code 1.
```

The job is `continue-on-error: true`, so the failure is swallowed into a PR-comment
status line nobody acts on. This has been the steady state, not a one-off.

**Snyk Open Source (SCA)** — npm only: `✔ Tested 81 dependencies for known issues,
no vulnerable paths found`. The Rust crates get no dependency scan at all.

**Snyk Monitor** — pushes npm snapshots to the Snyk dashboard.

### The coverage is already bought twice

Open code-scanning alerts by tool:

| Tool | Open alerts |
|---|---|
| SonarCloud | 5 |
| SnykCode | 3 |
| CodeQL | 1 |

CodeQL default setup analyses `c-cpp`, `javascript-typescript`, `python` and `rust`
(193 results in one `main` analysis). SonarCloud runs a quality gate on top. Snyk
Code is the third SAST reading the same files.

Dependabot vulnerability alerts are enabled (`GET /vulnerability-alerts` → `204`)
with **0 open alerts** across every ecosystem — matching Snyk OSS's own zero.

### The findings it does produce are structural false positives

Before erroring out, Snyk Code reported: `TEST_PASSWORD` constants in unit tests,
`PASSWORD_CHARSET` in the password generator, and `Sha1::new` in
`src/onvif/ws_security.rs`. The last is not a choice — SHA-1 is **mandated by the
ONVIF/WS-Security UsernameToken profile**. A scanner that cannot be taught this
costs more in triage than it returns.

### The real gap

**Rust crates have no advisory scan in CI.** Dependabot alerts watch the lockfiles,
but RustSec carries `unmaintained`, `unsound` and `yanked` advisories that never
become GHSAs. That is worth *adding*, and it is the only thing Snyk's removal
leaves to replace.

## Alternatives considered

**OSV-Scanner** — one Google binary scans `Cargo.lock` and `package-lock.json`
together and emits SARIF, reusing the existing upload step. Rejected: it duplicates
Dependabot alerts for npm while missing RustSec's unmaintained-crate class, so it is
a new dependency that covers strictly less than the chosen option.

**Semgrep OSS as a SAST backfill** — rules you can write yourself, so ONVIF-mandated
SHA-1 and test fixtures get silenced in-repo rather than in a vendor UI. Rejected for
now: CodeQL and SonarCloud already produce 6 open alerts between them, and a third
scanner mostly adds triage. Revisit if CodeQL alone proves thin.

**cargo-deny instead of cargo-audit** — investigated because its `[advisories]`
section historically offered a CVSS `severity-threshold`. That option has since been
**removed**: "all vulnerability advisories now emit errors." It offers no advantage
over cargo-audit for advisory checking, and its licence/ban checks are not wanted.

## Design

### 1. Removal

| Location | Action |
|---|---|
| `main-ci.yml:406-573` | Delete the Snyk steps; the `security-scans` job survives, re-purposed for `cargo audit` |
| `.github/workflows/security-scans.yml` | Delete the file — Snyk-only, and **nothing calls it** (verified: zero references repo-wide) |
| `scripts/docker/Dockerfile:52-69` | Drop the Snyk CLI download, checksum verification and `snyk --version` gate |
| `main-ci.yml:1006-1070`, `reporting.yml:12-81` | Rewire the three `snyk_*` outputs to `cargo_audit_*` |
| `wiki/Static-Analysis-Tools.md`, `docs/README.md:65` | Update the Snyk and `.snyk` references |

`SNYK_TOKEN` becomes an unused repo secret. Flagged for manual deletion; the change
does not touch repository secrets.

### 2. Replacement

**SAST: nothing.** CodeQL default setup plus the SonarCloud gate. Removing Snyk Code
drops the third scanner on identical files.

**Rust SCA: `cargo audit`,** two invocations, one per lockfile:

- `cross-compile/Cargo.lock` — covers `onvif-rust`, `streaming-lib`, `anyka-init`
- `validation/rust/Cargo.lock`

Runs on stock `ubuntu-latest` with stable Rust. cargo-audit only parses the lockfile,
so the vendored ARMv5TE toolchain is never involved and there is no build step. This
is net-new coverage: Snyk OSS scanned npm only.

**npm SCA: Dependabot alerts,** already enabled, already at zero.

**New `.github/dependabot.yml`** — the repo has never had one, so PR #57 arrived via
GitHub's security-updates default rather than config. Four ecosystems: `cargo` (both
lockfile directories), `npm`, `github-actions`, `docker`. The `github-actions` entry
earns its place independently: every action in these workflows is hand-pinned to a
SHA with a hand-written `# v6.0.0` comment, which is exactly the pattern Dependabot
maintains automatically.

### 3. Gate behaviour

`cargo audit` **fails the job** on a vulnerability advisory. Informational advisories
(`unmaintained`, `unsound`, `yanked`) print as warnings and pass.

This is a deliberate substitution for the requested "fail on high/critical only".
Neither cargo-audit nor cargo-deny exposes a CVSS threshold — cargo-deny removed
`severity-threshold`, and cargo-audit never had one. Implementing it literally means
parsing the CVSS vector string out of `cargo audit --json` and computing a base score
in jq, to gate lockfiles that currently carry zero advisories. RustSec's own
vulnerability/informational split is the same distinction without the parser: block
on the exploitable class, warn on the noise.

Contrast with today, where the entire Snyk job is `continue-on-error: true` — which
is why a persistent `Forbidden` error went unnoticed. Dependabot version-update PRs
never block anything.

### 4. Reporting

The PR comment and run summary keep their shape. The "🔒 Security Scans (Snyk)" block
becomes a `cargo audit` line reporting advisory counts per lockfile.

No SARIF plumbing. Pushing cargo-audit findings into code scanning would mean writing
and maintaining a SARIF converter for a tool whose output is already one line per
advisory, and whose failures now fail the job outright.

### 5. Verification

- `actionlint` on every changed workflow
- A grep proving zero `snyk` / `SNYK_TOKEN` references remain in `.github/` and `scripts/`
- Local `cargo audit` against both lockfiles, confirming the current clean state
- Docker image rebuild, confirming that removing the Snyk layer does not break the
  build gates surrounding it
- The dependabot.yml validated by GitHub on push (malformed config surfaces in the
  repo's Insights → Dependency graph → Dependabot tab)

## Consequences

The security-scans job no longer pulls a `snyk/snyk:node` Docker image or makes
four network-bound Snyk Code scan calls plus a Snyk Monitor upload, but
`rustsec/audit-check` compiles `cargo-audit` from source on every run (the
runner has no persistent tool cache), a ~2-3 minute cost in its place. That job
runs in parallel with the container-based Rust jobs, so it is off the critical
path and wall-clock impact on total CI time is likely nil. The CI image loses
a layer and a download.

Security coverage increases: Rust crate advisories go from unscanned to gated, and
dependency updates go from security-only to all four ecosystems.

The repo drops a vendor account, an API token, and a dashboard from its supply chain.
