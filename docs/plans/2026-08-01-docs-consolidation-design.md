# Design: Consolidate plans and docs into one convention

**Date:** 2026-08-01
**Status:** Approved
**Approach:** Adopt the superpowers-native layout (`docs/plans/`), freeze every other
convention under `docs/archive/`, and state the rule once in `AGENTS.md`.

## Problem

Five plan/spec conventions coexist in the repo, each left behind by a tool that was
later removed:

| Location | Convention | Newest | Files |
|---|---|---|---|
| `docs/plans/` | superpowers design+plan pairs | 2026-07-31 | 5 |
| `docs/superpowers/{plans,specs}/` | same superpowers output, split dirs | 2026-07-31 | 10 |
| `docs/specs/00{1,3,4}-*/` | spec-kit (spec/plan/tasks/contracts/checklists) | 2026-07-26 | ~35 |
| `docs/archive/{specs,steering}/` | kiro (requirements/design/tasks) | archived | ~25 |
| `claudedocs/` | SuperClaude ad-hoc plans and investigations | 2026-07-29 | 3 |

Three defects follow from this:

1. `docs/plans/` and `docs/superpowers/plans/` both hold superpowers output written on
   the same day. One tool, two homes.
2. The spec-kit engine was deleted on 2026-07-30 (see
   `2026-07-30-speckit-removal-design.md`) but its ~35 artifacts stayed in `docs/specs/`,
   where nothing distinguishes them from active work.
3. Nothing indexes any of it. `AGENTS.md` links `.serena/memories/**` and never mentions
   `docs/**`; `README.md` points only at the GitHub Wiki.

Removing a tool never removed its artifacts, so doc debt outlived tool debt.

## Goals

1. One place for new design and plan docs, chosen so the tools already write there.
2. Every superseded convention frozen and labelled, not deleted.
3. The rule stated once, where agents already read.

## Non-goals

- Rewriting historical statements inside archived or removal design docs.
- A CI guard or a new Serena memory. The fragmentation came from tools, not from people
  ignoring a rule, and the tools now agree with the rule.
- Touching `wiki/` (published to the GitHub Wiki) or `.serena/memories/**`.

## Why `docs/plans/` wins

The superpowers skills hardcode their output paths:

- `brainstorming/SKILL.md` → `docs/plans/YYYY-MM-DD-<topic>-design.md`
- `writing-plans/SKILL.md` → `docs/plans/YYYY-MM-DD-<feature-name>.md`

`docs/superpowers/**` is the deviation. Any other layout — lifecycle subdirectories,
per-feature folders — means correcting the tool by hand on every future plan. Flat and
dated costs nothing to maintain.

## Target tree

```
docs/
  README.md                        NEW - index + the rule
  plans/                           authoritative
    2026-07-25-async-hardware-separation.md           <- claudedocs/
    2026-07-26-deps-bump{,-design}.md                 <- docs/superpowers/
    2026-07-26-docker-ci-toolchain{,-design}.md       <- docs/superpowers/
    2026-07-26-toolchain-refresh{,-design}.md         <- docs/superpowers/
    2026-07-28-dashmap-removal.md                     <- claudedocs/
    2026-07-29-vendor-daemon-restart-resilience{,-design}.md
    2026-07-29-webui-build-improvements.md
    2026-07-29-webui-build-design.md
    2026-07-30-speckit-removal-design.md              <- docs/superpowers/
    2026-07-30-ubs-beads-distill-removal-design.md    <- docs/superpowers/
    2026-07-31-pr51-copilot-fixes{,-design}.md        <- docs/superpowers/
    2026-07-31-restart-resilience-hardware-fixes{,-design}.md
    2026-08-01-docs-consolidation{,-design}.md
  reference/                       NEW - durable, no end date
    architectural-complexity-analysis.md
    hack-process.md
    video-flow.md
    rtp-send-latency-investigation.md                 <- claudedocs/
  archive/                         frozen, never edited
    README.md                      NEW - provenance per convention
    steering/  specs/                                 (kiro, in place)
    speckit/001-rust-onvif-api/                       <- docs/specs/
    speckit/003-frontend-onvif-spec/
    speckit/004-hw-integration/
wiki/                              untouched
```

`docs/superpowers/`, `docs/specs/`, and `claudedocs/` disappear as directories. Every
file moves with `git mv`, so nothing is deleted and `git log --follow` still works.

## Classification rule for the loose files

A plan has an end date; reference does not. `dashmap-removal-plan.md` describes work that
concludes, so it becomes a plan. `rtp-send-latency-investigation.md` records how the
hardware behaves and stays true after the work ships, so it becomes reference.

## The rule

Stated in `AGENTS.md` as a `## Documentation Layout` section placed after the memories
list and before `## Key Development Areas`, and repeated in `docs/README.md`:

> New design and plan docs go in `docs/plans/` as `YYYY-MM-DD-<topic>-design.md` and
> `YYYY-MM-DD-<topic>.md` — the paths superpowers already writes. Durable analyses go in
> `docs/reference/`. `docs/archive/` is frozen history; never add to it, never edit it.
> User-facing docs go in `wiki/`.

## Reference rewrites

Four plan files cite their design doc by the old path and need a manual edit to point at
`docs/plans/`: `deps-bump`, `docker-ci-toolchain`, `toolchain-refresh`,
`pr51-copilot-fixes`.

Historical statements inside `2026-07-30-speckit-removal-design.md` ("keep
`docs/specs/**`") and `2026-07-30-ubs-beads-distill-removal-design.md` (naming
`docs/superpowers/plans/**`) are left alone — both documents explicitly scope out history
rewriting. `docs/archive/README.md` carries the forwarding note instead.

Per `AGENTS.md`, all edits are manual. No scripted transformations.

## Success criteria

| # | Check |
|---|---|
| P1 | `find docs -name '*.md' -not -path 'docs/archive/*'` returns only `docs/README.md`, `docs/plans/*`, `docs/reference/*` |
| P2 | `git log --follow docs/plans/2026-07-28-dashmap-removal.md` shows pre-move history |
| P3 | `grep -rn 'docs/superpowers\|claudedocs' docs/plans/` hits only the two removal designs |
| P4 | `git status` shows renames; no `D` without a matching `R` |
| P5 | `AGENTS.md` contains `## Documentation Layout` |

## Known warts, accepted

- `2026-07-29-webui-build-design.md` pairs with `2026-07-29-webui-build-improvements.md`.
  The stems differ. Renaming for symmetry is churn with no reader benefit; `docs/README.md`
  disambiguates.
- `docs/specs/004-hw-integration/tickets/` may describe unshipped hardware work. Freezing
  it under `archive/speckit/` preserves the content but marks the convention dead. If that
  work restarts, a fresh plan goes in `docs/plans/`.

## Out of scope

`.superpowers/sdd/phase-3-report.md` is session scratch that leaked into git — the rest of
that directory is ignored. Untracking it needs `git rm --cached`, an index deletion
requiring explicit permission under `AGENTS.md` RULE 1. Not part of this work.

## Authorization

User approved the design as presented on 2026-08-01 ("yes"), including freezing
`docs/specs/004-hw-integration` under `docs/archive/speckit/`.
