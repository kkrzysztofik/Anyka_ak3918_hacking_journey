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
  with ARM binaries, and `git log -- docs/plans/<file>` recovers any plan verbatim.
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

- `ls docs/plans/ | grep -v -- -design.md` → empty
- no markdown link resolves to a deleted plan. Two known hits to fix:
  `docs/reference/anyka-init-smoke-test.md:9` and `docs/archive/README.md:20`
- `scripts/ci/check_agent_config.py` still passes
- `git log --diff-filter=D --oneline -- docs/plans` shows the deletion is recoverable

## Flagged, not in scope

- **`flash-rootfs-cleanup` worktree** carries one unmerged commit: a +37/-10 edit to
  `2026-10-04-flash-rootfs-cleanup-design.md` plus its 719-line plan. The design edit
  needs a decision before that worktree goes anywhere.
- **`juan-board-support` worktree**: 12 unmerged commits, 5 dirty binaries, 102
  behind. Untouched.
- The three newest plans (`2026-09-28-juan-board-support`, `2026-10-04-anyka-init-…`,
  `2026-10-04-snmp-agent-…`) retire cleanly: the two cleanups shipped as PRs #131/#132,
  and the JUAN plan lives on in its own worktree.
