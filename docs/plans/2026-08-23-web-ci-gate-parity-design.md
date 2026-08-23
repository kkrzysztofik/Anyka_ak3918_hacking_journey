# Web CI gate parity

**Date:** 2026-08-23
**Status:** approved, implementing

## Problem

Three lists of WebUI checks exist and nothing forces them to agree:

| Check | `build_sd_contents.sh` (deploy) | `main-ci.yml` (PR) | `release.yml` (tag) |
| --- | --- | --- | --- |
| `type-check` (ts7 + ts6) | yes | **no** | no |
| `lint` | yes | yes | no |
| `test` / coverage | no | yes | no |
| `build` (vite) | yes | **no** | yes |

PR CI is the only list that runs tests and the only one that runs neither
`type-check` nor `build`. A PR can therefore merge green while being
unbuildable, which is what happened: three TypeScript 7 errors reached `main`
and stopped a fleet rollout at the deploy gate (PR #93).

This is structurally the same failure as `armv5te` living only in
`release.yml`. The mechanism, not the missing step, is the bug: adding a fourth
entry to one list leaves it intact.

Two further findings from the audit:

- `sonarqube` is `continue-on-error: true`, so its quality gate cannot fail a
  PR. Every hard gate for the WebUI lives in `webui-quality-and-coverage` and
  nowhere else.
- Coverage is reported but not enforced — `vite.config.ts` sets no thresholds.

## Decisions

1. **Single source of truth.** Define the gate list once in `package.json` as
   `npm run verify`; both PR CI and `build_sd_contents.sh` call it.
2. **Tests stay CI-only.** `verify` excludes them so local deploys stay fast.
   The gap remains, but deliberately and documented.
3. **Coverage ratchet at current level.** Floors set below today's measurement,
   preventing regression without demanding more.
4. **npm audit gates all deps at `high`**, with the one existing dev-only
   advisory fixed in the same change.

Gates are placed by property, not taste:

- `verify` — offline and fast, so the deploy script can call it.
- `test:coverage` — needs a coverage run, stays in the quality job.
- `npm audit` — needs the network, joins the security job.

## Changes

### 1. `cross-compile/www/package.json`

```json
"format:check": "prettier --check .",
"verify": "npm run type-check && npm run lint && npm run format:check"
```

`prettier` previously existed only as `--write`. `eslint-config-prettier` only
disables conflicting ESLint rules; it verifies nothing.

This gate was approved on the premise that the tree was already
`prettier --check` clean. It was not — 15 files had drifted. The premise came
from `rtk prettier --check`, whose filter printed "All files formatted
correctly" while the underlying exit code was 1. **Check prettier with
`./node_modules/.bin/prettier --check .` and read the exit code; the RTK filter
masks failures.** The 15 files are reformatted in a separate commit so the
mechanical churn stays reviewable apart from the gate wiring.

### 2. `cross-compile/www/vite.config.ts`

Add `thresholds` to the existing `coverage` block:

| Metric | Measured 2026-08-23 | Floor |
| --- | --- | --- |
| Lines | 91.39% | 90 |
| Statements | 89.74% | 88 |
| Functions | 88.26% | 87 |
| Branches | 78.02% | 76 |

Floors sit 1–2 points below measurement. Pinned-exact ratchets fail on honest
changes — one added uncovered `catch` branch goes red — and are fragile against
local-vs-CI measurement drift. A gate that cries wolf gets disabled; that is
already what happened to `sonarqube` here.

### 3. `cross-compile/www/package-lock.json`

`brace-expansion` 5.0.8 -> 5.0.9 (GHSA-rgw5-rvv9-x895, DoS). Dev-only,
reached via `@trivago/prettier-plugin-sort-imports` -> `minimatch`.
Semver-compatible; `npm audit fix` without `--force`.

### 4. `scripts/build_sd_contents.sh`

```diff
-    npm run type-check
-    npm run lint
+    npm run verify   # tests + audit deliberately excluded; CI owns those
     npm run build
```

### 5. `.github/workflows/main-ci.yml`

`webui-quality-and-coverage`: `Lint` becomes `Verify`; a `Build` step follows
it. Final order:

```text
npm ci -> npm run verify -> npm run build -> npm run test:coverage -> upload
```

`build` precedes the tests deliberately: "does not bundle" should not wait on
the suite. `test:coverage` stays last before artifact upload so it still feeds
the `sonarqube` job unchanged.

One CI step, not two. Splitting `verify` into separate "Type check" / "Lint"
steps would give a nicer UI at the cost of CI once again owning its own copy of
the list. The log shows which half failed.

`security-scans`: add `setup-node` plus

```bash
npm audit --package-lock-only --audit-level=high
```

next to the two `cargo-audit` steps, folded into the existing `audit_status`
aggregation. `--package-lock-only` means no `npm ci`.

**Waiver convention.** npm audit has no native ignore list, unlike
`cargo-audit`'s `ignore:`. The escape hatch is an `overrides` entry in
`package.json` pinning a patched transitive. The rationale cannot live beside
it: `package.json` is strict JSON and rejects comments, so npm would refuse the
manifest. The advisory ID, reason and removal date go in the YAML comment above
the audit step instead — the same place `cargo-audit`'s `ignore:` records its
own waivers.
Gating all deps at `high` buys real supply-chain coverage and a recurring tax:
dev transitives will block unrelated PRs. That is survivable only because the
waiver path is written down up front.

### 6. Documentation

`cross-compile/www/AGENTS.md` and `.serena/memories/www-development-standards.md`
both spell out `npm run lint` / `npm run type-check` as the pre-commit ritual.
Both repoint at `npm run verify`. Docs are call sites too: a shared script half
the callers ignore is worse than none, because it looks solved while still
drifting.

## Verification

Revert one of PR #93's TypeScript 7 fixes on a scratch branch, push, confirm
`webui-quality-and-coverage` fails at `verify`, delete the branch. Without this
the change only proves the YAML parses.

## Deliberately out of scope

- `eslint-plugin-jsx-a11y` — new dep plus violation cleanup.
- Bundle-size budget. `npm run analyze` only reports; it exits non-zero on
  usage errors and missing sourcemaps, never on size. Real ceiling on a 36 MB
  camera, but needs a number someone has to pick.
- `release.yml` runs no lint, type-check or test at all.
- PR CI never cross-builds `armv5te`.

## Already covered — not added

- **SAST**: CodeQL default setup analyzes `javascript-typescript` (confirmed
  via the code-scanning API, alongside `rust`, `c-cpp`, `python`, `actions`).
- **Dependency updates**: `.github/dependabot.yml` covers `npm` at
  `/cross-compile/www`.
