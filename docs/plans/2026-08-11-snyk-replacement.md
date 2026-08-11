# Snyk Replacement Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove the Snyk integration from CI and the CI image, replacing it with `cargo audit` for Rust crate advisories and a `.github/dependabot.yml` for automated dependency updates.

**Architecture:** Snyk's SAST duplicated CodeQL and SonarCloud (both already running) and had been failing with `Forbidden (SNYK-CLI-0000)` on every scan, hidden by `continue-on-error: true`. Its SCA covered npm only, which Dependabot alerts already cover. The `security-scans` job in `main-ci.yml` survives but is re-purposed: Snyk steps out, two `cargo audit` steps in, one per lockfile. Design doc: `docs/plans/2026-08-11-snyk-replacement-design.md`.

**Tech Stack:** GitHub Actions, `rustsec/audit-check@v2.0.0` (ships prebuilt cargo-audit, has a `working-directory` input), Dependabot config v2, Docker.

---

## Background the executing engineer needs

**This repo pins every GitHub Action to a full commit SHA** with a trailing `# vX.Y.Z` comment. Follow that convention exactly. The SHA for `rustsec/audit-check@v2.0.0` is `69366f33c96575abad1ee0dba8212993eecbe998` (verified via `gh api repos/rustsec/audit-check/git/ref/tags/v2.0.0`).

**Two Cargo lockfiles exist**, and they are separate dependency graphs:

| Lockfile | Covers |
|---|---|
| `cross-compile/Cargo.lock` | workspace: `onvif-rust`, `streaming-lib`, `anyka-init` |
| `validation/rust/Cargo.lock` | `rtsp-validation-tool` (standalone package) |

**cargo-audit does not build anything** — it parses `Cargo.lock` against the RustSec advisory database. The vendored ARMv5TE toolchain at `toolchain/arm-anykav200-crosstool-ng/bin/cargo` is **not** involved and must not be used here. This job runs on stock `ubuntu-latest`.

**Gate semantics** (from the `rustsec/audit-check` README, and the reason it was chosen): "In case of any security advisories found, status check created by this Action will be marked as failed. Note that informational advisories are not affecting the check status." Vulnerabilities fail; `unmaintained` / `unsound` / `yanked` warn. Neither cargo-audit nor cargo-deny exposes a CVSS severity threshold — cargo-deny **removed** `severity-threshold` ("all vulnerability advisories now emit errors"). This vulnerability/informational split is the closest native equivalent, and is what the design approved.

**Known baseline** — verified locally on 2026-08-11 with `cargo-audit audit --no-fetch`:

- `cross-compile/Cargo.lock` — 368 crates, **0 vulnerabilities**, 1 warning (`proc-macro-error3` yanked)
- `validation/rust/Cargo.lock` — 312 crates, **0 vulnerabilities**, 1 warning (`proc-macro-error2` unmaintained, RUSTSEC-2026-0173)

Both findings are informational, so **CI must go green on the first run.** If it goes red, either the advisory DB moved or the wiring is wrong — investigate, do not paper over it with `continue-on-error`.

**Two pre-existing bugs you will see and should not be confused by:**

1. `main-ci.yml:332` declares the job output `snyk_issues: ${{ steps.scan-results.outputs.snyk_issues }}`, but `scan-results` (line 455) never sets it — `count-issues` (line 441) does. The output has always been empty. Both steps are being deleted, so this resolves itself.
2. `.github/workflows/security-scans.yml`, `reporting.yml` and `quality-gates.yml` are all `workflow_call` reusable workflows that **nothing calls** — `main-ci.yml` inlines its own copies. Verified: `rg 'uses:.*\.github/workflows'` returns zero hits repo-wide. Only `security-scans.yml` (Snyk-only) is deleted by this plan; the other two are out of scope.

**Line numbers below are as of `main` at commit `34ebefec`.** Re-verify with `rg -n` before each edit; do not edit by line number blindly.

---

## Task 1: Replace the Snyk steps with cargo audit

**Files:**
- Modify: `.github/workflows/main-ci.yml:318-487` (the whole `security-scans` job)

**Step 1: Establish the local baseline before changing anything**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile && cargo-audit audit
cd /home/kmk/dev/anyka-dev/validation/rust && cargo-audit audit
```

Expected: both end `warning: 1 allowed warning found` with **no** `error: N vulnerabilities found`. Exit code 0.

If `cargo-audit` is not on PATH, install it: `cargo install cargo-audit --locked`. Note this is the *host* rustup cargo (`~/.cargo/bin/cargo`), not the vendored toolchain — cargo-audit is a host developer tool and must not be built with the ARM toolchain.

**Step 2: Replace lines 318-487 of `main-ci.yml`**

Delete the entire existing `security-scans` job including its `# ===` banner, and put this in its place:

```yaml
  # =============================================================================
  # SECURITY SCANS - Rust dependency advisories (RustSec)
  # =============================================================================
  # cargo-audit parses Cargo.lock only; no build, no vendored toolchain.
  # Vulnerabilities fail the job. Informational advisories (unmaintained,
  # unsound, yanked) are reported as warnings and do not fail it.
  # SAST is covered by CodeQL default setup and SonarCloud, not here.
  security-scans:
    name: Security Scans
    runs-on: ubuntu-latest
    permissions:
      contents: read
      checks: write
      issues: write
    outputs:
      audit_status: ${{ steps.audit-status.outputs.audit_status }}
    steps:
      - name: Checkout Code
        uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd  # v6.0.0

      - name: Audit cross-compile workspace
        id: audit-workspace
        uses: rustsec/audit-check@69366f33c96575abad1ee0dba8212993eecbe998  # v2.0.0
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          working-directory: cross-compile

      - name: Audit validation tool
        id: audit-validation
        if: always()
        uses: rustsec/audit-check@69366f33c96575abad1ee0dba8212993eecbe998  # v2.0.0
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          working-directory: validation/rust

      - name: Record audit status
        id: audit-status
        if: always()
        run: |
          if [ "${{ steps.audit-workspace.outcome }}" = "success" ] && \
             [ "${{ steps.audit-validation.outcome }}" = "success" ]; then
            echo "audit_status=success" >> "$GITHUB_OUTPUT"
          else
            echo "audit_status=failure" >> "$GITHUB_OUTPUT"
          fi
```

Three deliberate changes beyond the tool swap, each of which matters:

- **`continue-on-error: true` is gone from the job.** That flag is why a persistent `Forbidden` error went unnoticed for so long. The audit is now a real gate.
- **`needs:` is gone.** The old job waited on all three quality jobs before running a scan that only reads a lockfile. It now starts immediately, in parallel.
- **`security-events: write` is replaced by `checks: write` + `issues: write`,** which is what `audit-check` needs to publish its check run. No SARIF is produced, so `security-events` is dead weight.

`if: always()` on the second audit ensures a failure in the workspace lockfile does not hide a separate failure in the validation lockfile.

**Step 3: Lint the workflow**

```bash
actionlint .github/workflows/main-ci.yml
```

Expected: no output (success). If `actionlint` is not installed:
`go install github.com/rhysd/actionlint/cmd/actionlint@latest`, or skip and rely on GitHub's own parse on push — but prefer installing it, since a YAML error here costs a full CI round-trip.

**Step 4: Commit**

```bash
rtk git add .github/workflows/main-ci.yml
rtk git commit -m "ci: replace Snyk scans with cargo audit

Snyk Code returned Forbidden (SNYK-CLI-0000) on every scan while
duplicating CodeQL and SonarCloud. Snyk OSS covered npm only, which
Dependabot alerts already cover. cargo audit closes the real gap:
Rust crate advisories were unscanned.

The job drops continue-on-error, so advisories now actually gate.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 2: Rewire the run summary and PR comment

**Files:**
- Modify: `.github/workflows/main-ci.yml:898-1006` (the `reporting` job)

The `reporting` job consumes three now-deleted outputs: `snyk_code_status`, `snyk_oss_status`, `snyk_issues`. Left as-is they render as empty strings.

**Step 1: Add `if: always()` to the reporting job**

At `main-ci.yml:900`, immediately after `runs-on: ubuntu-latest`, add:

```yaml
    if: always()
```

**Why this is required, not cosmetic:** `reporting` has `needs: [..., security-scans, ...]`. Now that `security-scans` can genuinely fail, a real advisory would skip `reporting` entirely and the PR would get no comment at all — the worst possible outcome for a security finding. `if: always()` keeps the report flowing.

**Step 2: Fix the summary step env block**

Replace lines 919-921:

```yaml
          SNYK_CODE_STATUS: ${{ needs.security-scans.outputs.snyk_code_status }}
          SNYK_OSS_STATUS: ${{ needs.security-scans.outputs.snyk_oss_status }}
          SNYK_ISSUES: ${{ needs.security-scans.outputs.snyk_issues }}
```

with:

```yaml
          AUDIT_STATUS: ${{ needs.security-scans.outputs.audit_status }}
```

**Step 3: Fix the summary body**

Replace lines 941-942:

```bash
          SNYK_CODE_STATUS=$(get_status "${SNYK_CODE_STATUS}")
          SNYK_OSS_STATUS=$(get_status "${SNYK_OSS_STATUS}")
```

with:

```bash
          AUDIT_STATUS=$(get_status "${AUDIT_STATUS}")
```

Replace the summary block at lines 956-959:

```markdown
          ## 🔒 Security Scans (Snyk)
          - **Code (SAST)**: ${SNYK_CODE_STATUS}
          - **Open Source (SCA)**: ${SNYK_OSS_STATUS}
          - **SCA Issues Found**: ${SNYK_ISSUES}
```

with:

```markdown
          ## 🔒 Dependency Advisories (cargo audit)
          - **Rust crates (RustSec)**: ${AUDIT_STATUS}
          - **SAST**: see CodeQL and SonarCloud below
```

**Step 4: Fix the PR comment script**

Replace lines 981-983:

```javascript
            const snykCodeStatus = '${{ needs.security-scans.outputs.snyk_code_status }}';
            const snykOssStatus = '${{ needs.security-scans.outputs.snyk_oss_status }}';
            const snykIssues = '${{ needs.security-scans.outputs.snyk_issues || 0 }}';
```

with:

```javascript
            const auditStatus = '${{ needs.security-scans.outputs.audit_status }}';
```

Replace the comment table at lines 1002-1006:

```javascript
            ### 🔒 Security Scans (Snyk)
            | Scan Type | Status | Details |
            |-----------|--------|---------|
            | **Code (SAST)** | ${formatStatus(snykCodeStatus)} | [View in Code Scanning](https://github.com/${context.repo.owner}/${context.repo.repo}/security/code-scanning) |
            | **Open Source (SCA)** | ${formatStatus(snykOssStatus)} | ${snykIssues !== '0' ? `⚠️ ${snykIssues} vulnerabilities found` : '✅ No issues'} |
```

with:

```javascript
            ### 🔒 Security
            | Scan Type | Status | Details |
            |-----------|--------|---------|
            | **Rust advisories (cargo audit)** | ${formatStatus(auditStatus)} | RustSec advisory database |
            | **SAST (CodeQL)** | — | [View in Code Scanning](https://github.com/${context.repo.owner}/${context.repo.repo}/security/code-scanning) |
```

**Step 5: Verify no Snyk references survive in main-ci.yml**

```bash
rg -in snyk .github/workflows/main-ci.yml
```

Expected: **no output.** Any hit is a missed edit.

**Step 6: Lint and commit**

```bash
actionlint .github/workflows/main-ci.yml
rtk git add .github/workflows/main-ci.yml
rtk git commit -m "ci: report cargo audit instead of Snyk in summary and PR comment

Adds if: always() to the reporting job so a failing audit still
produces a PR comment rather than silently skipping the report.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 3: Delete the dead Snyk-only reusable workflow

**Files:**
- Delete: `.github/workflows/security-scans.yml`
- Modify: `.github/workflows/reporting.yml:12-23, 42-48, 67-68, 79-82`

**Step 1: Confirm nothing calls it**

```bash
rg -n 'security-scans\.yml' --hidden . --glob '!node_modules/**'
```

Expected: **no output.** If this returns a hit, stop and reassess — the file is live and must be edited rather than deleted.

**Step 2: Delete it**

```bash
rtk git rm .github/workflows/security-scans.yml
```

**Step 3: Strip the Snyk inputs from `reporting.yml`**

This file is also uncalled, but it carries Snyk references that would otherwise survive the removal. Delete the three input declarations at lines 12-23 (`snyk_code_status`, `snyk_oss_status`, `snyk_issues`), the three `env:` entries at lines 42-43 and 48, the two `get_status` calls at lines 67-68, and replace the summary block at lines 79-82 with:

```markdown
          ## 🔒 Dependency Advisories (cargo audit)
          - **Rust crates (RustSec)**: see the security-scans job
```

**Step 4: Verify and commit**

```bash
rg -in snyk .github/workflows/
actionlint .github/workflows/reporting.yml
```

Expected: `rg` produces no output; `actionlint` is silent.

```bash
rtk git add -A .github/workflows/
rtk git commit -m "ci: drop the dead Snyk-only reusable workflow

security-scans.yml was a workflow_call workflow that nothing invoked;
main-ci.yml inlines its own copy. Also strips the Snyk inputs from
reporting.yml.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 4: Remove the Snyk CLI from the CI image

**Files:**
- Modify: `scripts/docker/Dockerfile:45, 51-57, 69`

**Step 1: Fix the comment at line 45**

`Preinstall tarpaulin + Snyk for CI.` → `Preinstall tarpaulin for CI.`

**Step 2: Delete the Snyk download block**

Lines 51-57 currently read:

```dockerfile
  rm -rf /root/.cargo/registry/index/* /root/.cargo/git/db/* && \
  curl --proto '=https' --tlsv1.2 --location --fail --show-error -Lo snyk-linux https://downloads.snyk.io/cli/stable/snyk-linux \
  && curl --proto '=https' --tlsv1.2 --location --fail --show-error -Lo snyk-linux.sha256 https://downloads.snyk.io/cli/stable/snyk-linux.sha256 \
  && sha256sum -c snyk-linux.sha256 \
  && chmod +x snyk-linux \
  && mv snyk-linux /usr/local/bin/snyk \
  && rm snyk-linux.sha256 && \
```

Replace the whole block with just:

```dockerfile
  rm -rf /root/.cargo/registry/index/* /root/.cargo/git/db/* && \
```

**Careful:** this is one long `RUN` with `&&` continuations. The line before must still end in `&& \` and the line after (`(arm-unknown-linux-uclibcgnueabi-gcc --version || ...`) must remain intact. Getting this wrong breaks the image build, not just the Snyk step.

**Step 3: Delete the version gate at line 69**

Remove the line `  snyk --version && \` entirely. The surrounding gates (`clang --version ... && \` before it, `ccache --version && \` after it) must remain chained.

**Step 4: Verify the Dockerfile still parses and builds**

```bash
rg -in snyk scripts/docker/Dockerfile
docker build -f scripts/docker/Dockerfile -t anyka-ci-test scripts/docker/
```

Expected: `rg` silent; build completes and reaches the final gate chain. The build is slow (cargo-tarpaulin compiles from source). If Docker is unavailable locally, at minimum run `docker build --check` or verify the `RUN` chain by eye — every line but the last must end `&& \`.

**Step 5: Commit**

```bash
rtk git add scripts/docker/Dockerfile
rtk git commit -m "ci: drop the Snyk CLI from the CI image

One less download and layer; nothing invokes snyk any more.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 5: Add the Dependabot configuration

**Files:**
- Create: `.github/dependabot.yml`

The repo has never had one — PR #57 arrived via GitHub's security-updates default, which needs no config. Adding this file enables *version* updates on top of the security ones.

**Step 1: Create `.github/dependabot.yml`**

```yaml
# Dependabot version updates. Security alerts are enabled separately in
# repo settings and need no configuration here.
#
# Grouping is deliberate: ungrouped, four ecosystems produce a PR per crate
# per week. Grouped minor+patch means roughly one PR per ecosystem.
version: 2
updates:
  # Rust workspace: onvif-rust, streaming-lib, anyka-init
  - package-ecosystem: cargo
    directory: /cross-compile
    schedule:
      interval: weekly
    open-pull-requests-limit: 3
    groups:
      cargo-minor-patch:
        update-types: [minor, patch]

  # Host-side RTSP validation tool
  - package-ecosystem: cargo
    directory: /validation/rust
    schedule:
      interval: weekly
    open-pull-requests-limit: 3
    groups:
      cargo-minor-patch:
        update-types: [minor, patch]

  # Camera WebUI
  - package-ecosystem: npm
    directory: /cross-compile/www
    schedule:
      interval: weekly
    open-pull-requests-limit: 3
    groups:
      npm-minor-patch:
        update-types: [minor, patch]

  # Every action in these workflows is hand-pinned to a SHA with a trailing
  # version comment. Dependabot maintains exactly that pattern.
  - package-ecosystem: github-actions
    directory: /
    schedule:
      interval: weekly
    open-pull-requests-limit: 5
    groups:
      actions-minor-patch:
        update-types: [minor, patch]

```

**Step 2: Sanity-check the paths**

```bash
ls cross-compile/Cargo.lock validation/rust/Cargo.lock \
   cross-compile/www/package-lock.json scripts/docker/Dockerfile
```

Expected: all four exist. `directory:` is the directory containing the manifest, not the manifest itself.

**Step 3: Commit**

```bash
rtk git add .github/dependabot.yml
rtk git commit -m "ci: add Dependabot version updates for cargo, npm, actions, docker

The repo had no dependabot.yml, so only GitHub's default security
updates ran. Grouped minor+patch keeps this to about one PR per
ecosystem per week.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

**Step 4: Validate after push**

Dependabot config errors surface only server-side. After pushing, check
**Insights → Dependency graph → Dependabot** for a parse error banner. A malformed file is silently ignored otherwise.

---

## Task 6: Remove the Snyk policy files and update the docs

**Files:**
- Delete: `.snyk`, `.dcignore`
- Modify: `docs/README.md:65`
- Modify: `wiki/Static-Analysis-Tools.md`

`.snyk` (6.7K) is a Snyk policy file listing exclusions; `.dcignore` is DeepCode's, the engine behind Snyk Code. Both are inert once Snyk is gone. Path exclusions for CodeQL live separately and correctly in `.github/codeql/codeql-config.yml` — **do not touch that file.**

**Step 1: Confirm nothing else reads them**

```bash
rg -n '\.snyk|\.dcignore' --hidden . --glob '!node_modules/**' --glob '!.git/**'
```

Expected: hits only in `docs/README.md` and `wiki/Static-Analysis-Tools.md`, both of which you are about to edit. Any hit in a script or workflow means something still consumes them — stop and reassess.

**Step 2: Delete them**

```bash
rtk git rm .snyk .dcignore
```

**Step 3: Fix `docs/README.md:65`**

```markdown
Excluded from Snyk and DeepCode analysis by `.snyk` and `.dcignore` — most of it is generated.
```

becomes:

```markdown
Excluded from CodeQL analysis by `.github/codeql/codeql-config.yml` — most of it is generated.
```

**Step 4: Rewrite the Snyk section of `wiki/Static-Analysis-Tools.md`**

The file documents Snyk as tool #3 with a token-setup walkthrough (lines ~19-24, 39-42, 68-69, 86-89, 93-125, 203-207). Replace all of it with a short `cargo audit` section:

````markdown
### 3. cargo audit (Rust dependency advisories)

Checks both `Cargo.lock` files against the [RustSec advisory database](https://rustsec.org).
Runs in CI on every push; no account or token required.

```bash
cargo install cargo-audit --locked
(cd cross-compile && cargo audit)
(cd validation/rust && cargo audit)
```

Vulnerabilities fail the build. Informational advisories — `unmaintained`,
`unsound`, `yanked` — are reported as warnings and do not.
````

Delete the "Snyk Authentication Setup" section entirely; there is no token any more.

**Step 5: Verify the repo is Snyk-free**

```bash
rg -in snyk --hidden . --glob '!node_modules/**' --glob '!.git/**' --glob '!docs/plans/**' --glob '!docs/archive/**'
```

Expected: **no output.** `docs/plans/` and `docs/archive/` are excluded on purpose — historical design docs, including this plan and its design doc, legitimately name Snyk and must not be rewritten.

**Step 6: Commit**

```bash
rtk git add -A
rtk git commit -m "docs: drop Snyk policy files and document cargo audit

.snyk and .dcignore are inert without Snyk. CodeQL path exclusions
live in .github/codeql/codeql-config.yml and are unaffected.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 7: Verify end to end

**Step 1: Push and open a PR**

```bash
rtk git push -u origin ci/replace-snyk
rtk gh pr create --title "ci: replace Snyk with cargo audit and Dependabot" --body "..."
```

**Step 2: Confirm the security-scans job passes**

```bash
rtk gh pr checks
```

Expected: **Security Scans** green. Per the baseline in the Background section, both lockfiles carry one informational advisory and zero vulnerabilities, so green is the correct result. Red means the advisory DB moved since 2026-08-11 — read the log and judge the finding on its merits before touching the workflow.

**Step 3: Confirm the check run and PR comment rendered**

Check the PR for the `rust-audit-check` check run, and confirm the unified analysis comment shows the **🔒 Security** table with the cargo audit row and no Snyk rows.

**Step 4: Confirm the job actually gates**

This is the step that proves the fix is real rather than cosmetic. Temporarily add a known-vulnerable crate to `validation/rust/Cargo.toml`, regenerate the lockfile, push, and confirm the job goes **red**:

```bash
cd validation/rust
cargo add time@0.1.44   # RUSTSEC-2020-0071, segfault in localtime_r
cargo update -p time
rtk git commit -am "test: temporary vulnerable dep, do not merge"
rtk git push
```

Expected: **Security Scans fails**, and the PR comment still appears (this is what `if: always()` on `reporting` buys). Then revert:

```bash
rtk git revert --no-edit HEAD
rtk git push
```

Do not skip this. Under the old `continue-on-error: true` the job could never fail, and verifying that it now can is the whole point of the change.

**Step 5: Confirm Dependabot parsed the config**

Visit **Insights → Dependency graph → Dependabot** and confirm four ecosystems are listed with no parse error. Expect a small burst of update PRs within a day.

**Step 6: Manual follow-up for the user**

`SNYK_TOKEN` is now an unused repository secret. Deleting it is a one-click action in **Settings → Secrets and variables → Actions** that this plan deliberately does not automate — the change must not touch repository secrets.

---

## Out of scope

- `.github/workflows/quality-gates.yml` and `reporting.yml` are dead reusable workflows (nothing calls them). Only the Snyk-specific parts are touched here; deleting them wholesale is a separate cleanup.
- A CVSS-based severity threshold. Neither cargo-audit nor cargo-deny supports one; implementing it means parsing CVSS vector strings in jq. Revisit only if the vulnerability/informational split proves too coarse in practice.
- Semgrep or any SAST backfill. CodeQL plus SonarCloud is the approved coverage.
