# Docs Plan Retirement — Design

Date: 2026-10-09
Status: Draft for review

## Problem

`docs/plans/` holds 128 files and 2.6 MB. Half of it is the wrong half:

| Kind | Files | Bytes |
|---|---|---|
| `*-design.md` | 64 | 676 KB |
| implementation plans | 64 | **2.0 MB** |

Three defects follow.

1. **Plans outlive their work.** Across the 64 plans there are 189 unchecked and 31
   checked task boxes. `2026-08-16-ptz-diagnostics-pane.md` shipped with all 53 boxes
   still unchecked. Nobody ticks them, so the checklist half of every plan is dead the
   day it is written, and it stays in the repo forever.
2. **The durable content is a few lines buried in a task list.**
   `2026-08-12-firmware-upgrade-path.md` is 64 KB. The part worth keeping is its
   *"Deviation from the design doc"* section: `manifest.json` became `manifest.sha256`
   because `anyka-init` has no serde_json and `sha256sum -c` verifies in one exit status.
   That is a durable decision, and it is filed under "task list nobody will reopen".
3. **The index already failed.** `docs/README.md` hand-maintains a plans table with 17
   rows while 63 of the 64 designs are missing from it.

## Goals

1. `docs/plans/` keeps only the durable designs.
2. Anything a plan learned that contradicted its design survives **in that design**.
3. A rule that stops the pile rebuilding, since the skills will keep writing plans.

## Non-goals

- **No git history rewrite.** The 2.0 MB is noise against a 525 MB `.git` filled
  with ARM binaries, and a deleted plan restores byte-for-byte with
  `git show "$(git log --diff-filter=D --format=%H -1 -- docs/plans/<file>)^:docs/plans/<file>"`.
- **No renaming or moving of designs.** 64 renames churns history to fix a name.
- `docs/reference/`, `docs/design/`, `docs/archive/` content is untouched except the
  two lines that link at a plan being deleted.
- The two remaining worktrees (`flash-rootfs-cleanup`, `juan-board-support`) stay.

## Decisions

| Topic | Choice | Why |
|---|---|---|
| Plan files | delete all 64 | git archives them; 251 commits touch `docs/plans/` |
| Durable residue | append `## Implementation outcome` to the owning design | the design is the durable artifact |
| Orphan plans (8, no design) | delete; promote to `docs/reference/` only if the finding stands alone | a design is not required to record a measurement |
| Directory name | keep `docs/plans/` | the superpowers skills write there and `docs/README.md` warns a new location will be ignored; the directory is the *workspace* — designs durable, plans transient |
| `docs/README.md` index | delete the plans table, keep the routing table | hand-maintained, drifted to 27% coverage, duplicates `ls` |
| CI enforcement | none | `Status:` lines are too inconsistent to assert on; add a check if the pile returns |

## The rule

> **Implementation plans are working artifacts, not documentation.** Commit them while
> the work is in flight. Delete the plan in the commit that ships the work, moving
> anything the design got wrong into that design's `## Implementation outcome`.

This is the actual cause of the pile: the plans were never deleted at ship time.

Lands in three places:

| File | Change |
|---|---|
| `AGENTS.md` | "Documentation Layout" row + workflow step 8 gains "delete the shipped plan" |
| `docs/README.md` | routing table row, new rule paragraph, plans table deleted |
| `docs/archive/README.md` | the pointer line that says "design and plan docs" |

## `## Implementation outcome` section

Appended to a design only when the plan taught it something. Cap ~10 lines.
Record only what the design does not already say:

- a deviation that shipped instead of the designed approach, and why
- a hardware measurement that contradicted the design
- a gate that failed, and what was done about it
- a known follow-up deliberately not taken

No findings → no section. Never summarise the task list; that is what git keeps.

## Work breakdown

1. **Merge pass** — read-only lanes, one per design+plan pair, each returning
   candidate outcome lines with a quote and the plan path. Parallel; no writes.
2. **Write pass** — parent applies the outcome sections. One writer per worktree.
3. **Delete** — `git rm` the 64 plans.
4. **Guidelines** — the three files above.
5. **Verify** — below.

## Verification

- `ls docs/plans/ | grep -v -- -design.md` → only plans whose work has not shipped; today
  that is `2026-09-28-juan-board-support.md`
- no markdown link resolves to a deleted plan, apart from deliberate historical prose
  (see Outcome)
- `scripts/ci/check_agent_config.py` still passes
- a deleted plan restores byte-for-byte via the `git show` command in Decision 1

## Flagged, not in scope

- **`flash-rootfs-cleanup` worktree** carries one unmerged commit: a +37/-10 edit to
  `2026-10-04-flash-rootfs-cleanup-design.md` plus its 719-line plan. The design edit
  needs a decision before that worktree goes anywhere.
- **`juan-board-support` worktree**: 12 unmerged commits, 5 dirty binaries, 102
  behind. Untouched.
- The three newest plans (`2026-09-28-juan-board-support`, `2026-10-04-anyka-init-…`,
  `2026-10-04-snmp-agent-…`) retire cleanly: the two cleanups shipped as PRs #131/#132,
  and the JUAN plan lives on in its own worktree.

## Implementation outcome

- **63 deleted, not 64.** `2026-09-28-juan-board-support.md` was kept because its work is
  still on the unmerged `feature/juan-board-support` branch and the rule deletes a plan when
  the work *ships*. Applying the rule on its first day amended this design.
- **Residue rate: 16 of 64 (25%).** 48 plans said nothing their design did not already say.
  Ten designs gained an `## Implementation outcome` section; three plans had no design to
  inherit their findings, becoming `docs/reference/wifi-bring-up-findings.md`,
  `docs/reference/camera-cutover-traps.md`, and a "Deferred decisions" section in
  `docs/reference/architectural-complexity-analysis.md`.
- **Two designs already recorded their own findings** and were dropped: `vendor-daemon-restart-resilience`
  already carried the R4 deferral, and `day-night-gaps` already carried the `.127` oscillation,
  the "all N samples agree on the same lie" conclusion, and both out-of-scope items. The
  "does the design already say it" filter, not the extraction, is where the value is.
- **Nine designs pointed at their own deleted plan**, mostly
  `Status: approved; implementation plan at <plan>`. `2026-08-12-firmware-upgrade-path-design.md`
  went further and deferred "the full reasoning" to a plan section. A future run must grep for
  self-references, not only for inbound links from elsewhere.
- **The marker scan is triage, not verdict.** It found 17 of the 31 real candidates but
  missed `wifi-findings-remediation` outright (its findings are headed `## F1 — BLOCKING`),
  while producing false positives from task titles containing "outcome" and "results".
- **Delegating the read pass cost more than it returned.** Lanes of 8 plans
  lost everything to a 30-minute per-child cap because output was bound rather than written as
  they went; writing each verdict to disk immediately salvaged 13 plans. The remaining 18 were
  faster done in-parent from a marker map than by another fan-out.
- **A rollout plan expires rather than ships.** Both fleet plans were deleted although
  neither rollout is recorded as complete. `a013f167` is explicitly superseded by the
  `6afa26f4` design, and a rollout plan targets one exact commit — once `main` moves past it,
  the plan cannot be executed as written, so it is dead rather than in flight. Unlike a
  feature plan, its expiry date is the next merge.
- **Review round corrections.** Three of the findings were right and are fixed: the documented
  recovery command was wrong (`--diff-filter=D` locates the deletion commit but never shows
  the file); the frozen `docs/archive/README.md` was reverted, accepting a historical
  reference over editing a frozen file; and `git add -A docs` swept an unrelated untracked
  reference doc into the PR, since removed. One finding was wrong: the `a013f167` plan was
  not pending but superseded, which is now recorded in that design as well as its successor.
