# Docs Consolidation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Collapse five competing plan/spec conventions into one — `docs/plans/` — with everything superseded frozen under `docs/archive/` and the rule stated once in `AGENTS.md`.

**Architecture:** Every file moves with `git mv`, never delete-and-recreate, so `git log --follow` survives and no file leaves the tree (`AGENTS.md` RULE 1 forbids deletion without written permission). Four plan files cite their design doc by an old path and get manual edits. Two new README files (`docs/README.md`, `docs/archive/README.md`) index the result. No scripted transformations — `AGENTS.md` forbids them.

**Tech Stack:** git, plain Markdown. No build, no tests to run, no toolchain involved. "Tests" here are `find`/`grep`/`git status` assertions run after each move.

**Design doc:** `docs/plans/2026-08-01-docs-consolidation-design.md`

**Branch:** `docs/consolidate-plans` (already created; design already committed at `4e0734d`)

---

## Background for someone with zero context

This repo accumulated one documentation directory per AI tooling generation. Each tool was
eventually removed, but its artifacts stayed:

| Directory | Left behind by | Still has an engine? |
|---|---|---|
| `docs/plans/` | superpowers (current) | yes — this is the live one |
| `docs/superpowers/{plans,specs}/` | superpowers, misfiled | no — same tool, wrong path |
| `docs/specs/00{1,3,4}-*/` | spec-kit | no — engine deleted 2026-07-30 |
| `docs/archive/{specs,steering}/` | kiro | no |
| `docs/archive/config.example.toml` | Spec Workflow MCP | no |
| `claudedocs/` | SuperClaude ad-hoc | no |

The superpowers skills hardcode `docs/plans/YYYY-MM-DD-<topic>-design.md` and
`docs/plans/YYYY-MM-DD-<feature>.md`. That is why `docs/plans/` wins: it needs no
enforcement, because the tools already write there.

**Terminology used below.** A *plan* has an end date — it describes work that concludes.
*Reference* does not — it stays true after the work ships. That is the only test used to
sort the loose files.

**Two rules that constrain every task:**

1. `AGENTS.md` RULE 1 — never delete a file. Use `git mv`. If a step seems to need a
   deletion, stop and ask.
2. `AGENTS.md` "No Script-Based Changes" — never run sed/awk/perl over repo files. The
   four path rewrites in Task 3 are hand edits.

---

## Task 1: Create `docs/reference/` and move the loose analyses

Three analysis documents sit at `docs/` root with no home. None has an end date, so all
three are reference.

**Files:**
- Move: `docs/architectural-complexity-analysis.md` → `docs/reference/architectural-complexity-analysis.md`
- Move: `docs/hack-process.md` → `docs/reference/hack-process.md`
- Move: `docs/video-flow.md` → `docs/reference/video-flow.md`

**Step 1: Confirm nothing links to these paths**

```bash
grep -rn "docs/architectural-complexity-analysis\|docs/hack-process\|docs/video-flow" \
  --include="*.md" --include="*.rs" --include="*.ts" --include="*.tsx" \
  --include="*.yml" --include="*.yaml" --include="*.toml" --include="*.json" . \
  | grep -v node_modules
```

Expected: no output. If there ARE hits, note each one — they need rewriting in this task
before committing, and the plan's assumption was wrong.

**Step 2: Create the directory and move**

```bash
mkdir -p docs/reference
git mv docs/architectural-complexity-analysis.md docs/reference/
git mv docs/hack-process.md docs/reference/
git mv docs/video-flow.md docs/reference/
```

**Step 3: Verify git recorded renames, not delete+add**

```bash
git status --short
```

Expected: three lines beginning `R ` (rename). If you see `D ` and `A ` pairs instead,
something used the wrong command — undo with `git reset` and redo with `git mv`.

**Step 4: Verify `docs/` root is now clean of stray markdown**

```bash
ls docs/*.md 2>/dev/null
```

Expected: no output (`docs/README.md` does not exist yet — it arrives in Task 6).

**Step 5: Commit**

```bash
git add -A docs/
git commit -m "docs: move loose analyses into docs/reference/"
```

---

## Task 2: Fold `claudedocs/` into `docs/`

Three files. Two are plans (they conclude), one is an investigation record (it does not).
The plans get date prefixes taken from their git history, matching the `docs/plans/`
convention.

| Source | Git date | Destination |
|---|---|---|
| `claudedocs/onvif-rust-async-hardware-fix-plan.md` | 2026-07-25 | `docs/plans/2026-07-25-async-hardware-separation.md` |
| `claudedocs/dashmap-removal-plan.md` | 2026-07-28 | `docs/plans/2026-07-28-dashmap-removal.md` |
| `claudedocs/rtp-send-latency-investigation.md` | 2026-07-29 | `docs/reference/rtp-send-latency-investigation.md` |

**Files:**
- Move + rename: the three above
- Directory `claudedocs/` disappears (git removes empty directories automatically)

**Step 1: Re-confirm the dates before renaming**

```bash
for f in claudedocs/*.md; do echo -n "$f "; git log -1 --format=%ad --date=short -- "$f"; done
```

Expected:
```
claudedocs/dashmap-removal-plan.md 2026-07-28
claudedocs/onvif-rust-async-hardware-fix-plan.md 2026-07-25
claudedocs/rtp-send-latency-investigation.md 2026-07-29
```

If a date differs, use the date git reports, not the one in the table.

**Step 2: Move**

```bash
git mv claudedocs/onvif-rust-async-hardware-fix-plan.md \
       docs/plans/2026-07-25-async-hardware-separation.md
git mv claudedocs/dashmap-removal-plan.md \
       docs/plans/2026-07-28-dashmap-removal.md
git mv claudedocs/rtp-send-latency-investigation.md \
       docs/reference/rtp-send-latency-investigation.md
```

**Step 3: Verify the directory is gone and history followed**

```bash
ls claudedocs 2>&1
git log --follow --format="%h %s" docs/plans/2026-07-28-dashmap-removal.md | tail -3
```

Expected: `ls: cannot access 'claudedocs': No such file or directory`, and the `git log`
shows commits from before the rename. An empty log means the rename broke history — stop
and investigate.

**Step 4: Commit**

```bash
git add -A
git commit -m "docs: fold claudedocs into docs/plans and docs/reference"
```

---

## Task 3: Merge `docs/superpowers/**` into `docs/plans/`

Ten files, same tool as `docs/plans/`, wrong directory. Filenames already match the
convention, so this is a straight move with no renames. Verified: no name collisions with
existing `docs/plans/` contents.

**Files:**
- Move: 4 files from `docs/superpowers/plans/` → `docs/plans/`
- Move: 6 files from `docs/superpowers/specs/` → `docs/plans/`
- Modify: 4 of those files, one line each (Step 3)

**Step 1: Move everything**

```bash
git mv docs/superpowers/plans/2026-07-26-deps-bump.md docs/plans/
git mv docs/superpowers/plans/2026-07-26-docker-ci-toolchain.md docs/plans/
git mv docs/superpowers/plans/2026-07-26-toolchain-refresh.md docs/plans/
git mv docs/superpowers/plans/2026-07-31-pr51-copilot-fixes.md docs/plans/
git mv docs/superpowers/specs/2026-07-26-deps-bump-design.md docs/plans/
git mv docs/superpowers/specs/2026-07-26-docker-ci-toolchain-design.md docs/plans/
git mv docs/superpowers/specs/2026-07-26-toolchain-refresh-design.md docs/plans/
git mv docs/superpowers/specs/2026-07-30-speckit-removal-design.md docs/plans/
git mv docs/superpowers/specs/2026-07-30-ubs-beads-distill-removal-design.md docs/plans/
git mv docs/superpowers/specs/2026-07-31-pr51-copilot-fixes-design.md docs/plans/
```

**Step 2: Verify the directory is empty and gone**

```bash
ls -R docs/superpowers 2>&1
```

Expected: `No such file or directory`.

**Step 3: Hand-edit the four stale cross-references**

Each of these four files names its design doc by the old path. Use the `Edit` tool, one
call per file. Do NOT use sed — `AGENTS.md` forbids scripted edits to repo files.

| File | Find | Replace with |
|---|---|---|
| `docs/plans/2026-07-26-docker-ci-toolchain.md` | `` `docs/superpowers/specs/2026-07-26-docker-ci-toolchain-design.md` `` | `` `docs/plans/2026-07-26-docker-ci-toolchain-design.md` `` |
| `docs/plans/2026-07-26-toolchain-refresh.md` | `` `docs/superpowers/specs/2026-07-26-toolchain-refresh-design.md` `` | `` `docs/plans/2026-07-26-toolchain-refresh-design.md` `` |
| `docs/plans/2026-07-26-deps-bump.md` | `` `docs/superpowers/specs/2026-07-26-deps-bump-design.md` `` | `` `docs/plans/2026-07-26-deps-bump-design.md` `` |
| `docs/plans/2026-07-31-pr51-copilot-fixes.md` | `` `docs/superpowers/specs/2026-07-31-pr51-copilot-fixes-design.md` `` | `` `docs/plans/2026-07-31-pr51-copilot-fixes-design.md` `` |

**Step 4: Verify only the intended references remain**

```bash
grep -rn "docs/superpowers" docs/plans/
```

Expected: exactly two hits, both historical statements that the design doc says to leave
alone:
- `2026-07-30-speckit-removal-design.md` — line mentioning `docs/superpowers/specs/**`
- `2026-07-30-ubs-beads-distill-removal-design.md` — line naming `docs/superpowers/plans/**`

Any other hit means Step 3 missed a file.

**Step 5: Commit**

```bash
git add -A docs/
git commit -m "docs: merge docs/superpowers into docs/plans"
```

---

## Task 4: Freeze the spec-kit artifacts under `docs/archive/`

Three directories, ~35 files. The spec-kit engine was deleted on 2026-07-30; these
artifacts are orphaned and nothing in the repo references them. They move whole, so
relative links inside them keep working.

**Files:**
- Move: `docs/specs/001-rust-onvif-api/` → `docs/archive/speckit/001-rust-onvif-api/`
- Move: `docs/specs/003-frontend-onvif-spec/` → `docs/archive/speckit/003-frontend-onvif-spec/`
- Move: `docs/specs/004-hw-integration/` → `docs/archive/speckit/004-hw-integration/`

**Step 1: Move the three directories**

```bash
mkdir -p docs/archive/speckit
git mv docs/specs/001-rust-onvif-api docs/archive/speckit/
git mv docs/specs/003-frontend-onvif-spec docs/archive/speckit/
git mv docs/specs/004-hw-integration docs/archive/speckit/
```

Note: some filenames under `004-hw-integration/tickets/` contain `&`, spaces, and square
brackets (e.g. `[SPLIT]_Platform_Layer_-_Media_Encoder_&_Frame_Callbacks_→_T8a_+_T8b.md`).
Moving the parent directory avoids having to quote any of them.

**Step 2: Verify `docs/specs/` is gone and the count survived**

```bash
ls docs/specs 2>&1
find docs/archive/speckit -type f | wc -l
```

Expected: `No such file or directory`, then `35`. A lower count means files were lost —
stop, `git reset --mixed` is NOT allowed here without asking; report the discrepancy.

**Step 3: Commit**

```bash
git add -A docs/
git commit -m "docs: freeze spec-kit artifacts under docs/archive/speckit"
```

---

## Task 5: Write `docs/archive/README.md`

The archive now holds four dead conventions. Without a note, a future reader cannot tell
which is which or why they are frozen.

**Files:**
- Create: `docs/archive/README.md`

**Step 1: Create the file**

```markdown
# Archived documentation

Frozen. Nothing here is active work, and nothing here should be edited or added to.
New design and plan docs go in `docs/plans/` — see `docs/README.md`.

Each subdirectory is the residue of an AI tooling generation that was later removed. The
artifacts were kept; only the engines were deleted.

| Path | Convention | Engine removed | Notes |
|---|---|---|---|
| `steering/` | kiro steering docs | before 2026-07-26 | product/structure/tech context |
| `specs/` | kiro (requirements/design/tasks per feature) | before 2026-07-26 | 8 features |
| `speckit/` | spec-kit (spec/plan/tasks/contracts/checklists) | 2026-07-30 | 3 features; see `docs/plans/2026-07-30-speckit-removal-design.md` |
| `config.example.toml`, `.gitignore` | Spec Workflow MCP server | unknown | leftover config |

## Forwarding note

`docs/plans/2026-07-30-speckit-removal-design.md` says the spec-kit artifacts were kept at
`docs/specs/**`. They were moved here on 2026-08-01 by
`docs/plans/2026-08-01-docs-consolidation.md`. That design doc was deliberately not
rewritten — it records what was true when it was written.
```

**Step 2: Verify the referenced paths exist**

```bash
ls docs/archive/steering docs/archive/specs docs/archive/speckit docs/archive/config.example.toml
```

Expected: all resolve. Any missing path is a broken link in a brand-new file — fix before
committing.

**Step 3: Commit**

```bash
git add docs/archive/README.md
git commit -m "docs: document archived conventions and their provenance"
```

---

## Task 6: Write `docs/README.md` — the index

**Files:**
- Create: `docs/README.md`

**Step 1: Create the file**

```markdown
# Documentation

## Where things go

| Kind | Location | Naming |
|---|---|---|
| Design + implementation plans | `docs/plans/` | `YYYY-MM-DD-<topic>-design.md` and `YYYY-MM-DD-<topic>.md` |
| Durable analyses and investigations | `docs/reference/` | free-form |
| Superseded conventions | `docs/archive/` | frozen — never add, never edit |
| User-facing documentation | `wiki/` | published to the GitHub Wiki |
| Agent standards loaded by `AGENTS.md` | `.serena/memories/` | see `AGENTS.md` |

`docs/plans/` is the paths the superpowers `brainstorming` and `writing-plans` skills
already write to. Do not invent a new location; the tools will not follow you there.

A plan has an end date — it describes work that concludes. Reference does not — it stays
true after the work ships. Sort by that test.

## Plans

Newest first. A `-design.md` file is the approved shape; the matching plain file is the
task-by-task implementation plan. Some entries have only one of the two.

| Date | Topic | Design | Plan |
|---|---|---|---|
| 2026-08-01 | Docs consolidation | ✅ | ✅ |
| 2026-07-31 | Restart-resilience hardware fixes | ✅ | ✅ |
| 2026-07-31 | PR51 Copilot fixes | ✅ | ✅ |
| 2026-07-30 | UBS / beads / distill removal | ✅ | — |
| 2026-07-30 | SpecKit removal | ✅ | — |
| 2026-07-29 | Vendor-daemon restart resilience | ✅ | ✅ |
| 2026-07-29 | WebUI build improvements | ✅ (`-webui-build-design.md`) | ✅ (`-webui-build-improvements.md`) |
| 2026-07-28 | dashmap removal | — | ✅ |
| 2026-07-26 | Toolchain refresh | ✅ | ✅ |
| 2026-07-26 | Docker CI toolchain | ✅ | ✅ |
| 2026-07-26 | Dependency bump | ✅ | ✅ |
| 2026-07-25 | Async / hardware layer separation | — | ✅ |

## Reference

| Document | Subject |
|---|---|
| `reference/architectural-complexity-analysis.md` | onvif-rust RTSP/video pipeline complexity and simplification roadmap |
| `reference/rtp-send-latency-investigation.md` | Why RTP sends stall on the AK3918 |
| `reference/video-flow.md` | Video path from sensor to client |
| `reference/hack-process.md` | Reverse-engineering narrative for the camera |

## Archive

See `archive/README.md`.
```

**Step 2: Verify every file the index names actually exists**

```bash
ls docs/plans/ docs/reference/
```

Cross-check each row of both tables against the listing. The WebUI row is the one to watch
— its design and plan stems differ (`webui-build-design` vs `webui-build-improvements`),
which is a known and accepted wart.

**Step 3: Commit**

```bash
git add docs/README.md
git commit -m "docs: add documentation index"
```

---

## Task 7: State the rule in `AGENTS.md`

`AGENTS.md` is the canonical policy file and currently links only `.serena/memories/**`.
It never mentions `docs/**`, which is the reason nothing stopped the fragmentation.

**Files:**
- Modify: `AGENTS.md` — insert a section between the `**LOADING RULE**:` line and
  `## Key Development Areas` (around line 213)

**Step 1: Insert the section**

Use the `Edit` tool. Anchor on the existing text:

```
**LOADING RULE**: If your task involves multiple areas (e.g., coding + testing), you MUST load ALL relevant documents.

## Key Development Areas
```

Replace with:

```
**LOADING RULE**: If your task involves multiple areas (e.g., coding + testing), you MUST load ALL relevant documents.

## Documentation Layout

**New design and plan docs go in `docs/plans/`** as `YYYY-MM-DD-<topic>-design.md` and
`YYYY-MM-DD-<topic>.md`. These are the paths the superpowers `brainstorming` and
`writing-plans` skills already write to — do not invent a new location.

| Kind | Location |
|---|---|
| Designs and implementation plans | `docs/plans/` |
| Durable analyses and investigations | `docs/reference/` |
| Superseded conventions | `docs/archive/` — frozen, never add, never edit |
| User-facing documentation | `wiki/` — published to the GitHub Wiki |

A plan has an end date; reference does not. See `docs/README.md` for the index.

## Key Development Areas
```

**Step 2: Verify**

```bash
grep -n "## Documentation Layout" AGENTS.md
grep -c "## Key Development Areas" AGENTS.md
```

Expected: one line number for the first, and `1` for the second (the anchor was not
duplicated).

**Step 3: Commit**

```bash
git add AGENTS.md
git commit -m "docs: declare docs/plans the single home for designs and plans"
```

---

## Task 8: Final verification

Run every success criterion from the design doc. This task produces no commit unless a
check fails and needs a fix.

**Step 1: P1 — only three markdown locations survive outside the archive**

```bash
find docs -name '*.md' -not -path 'docs/archive/*' | sort
```

Expected: `docs/README.md`, then only `docs/plans/*.md` and `docs/reference/*.md`. Any
other path is a miss.

**Step 2: P2 — renames preserved history**

```bash
git log --follow --format="%h %s" docs/plans/2026-07-28-dashmap-removal.md | wc -l
```

Expected: greater than 1. A value of 1 means only the rename commit is visible and history
was severed.

**Step 3: P3 — no live references to the old locations**

```bash
grep -rn "docs/superpowers\|claudedocs\|docs/specs/" docs/plans/ docs/reference/ docs/README.md AGENTS.md
```

Expected: exactly the two historical mentions inside `2026-07-30-speckit-removal-design.md`
and `2026-07-30-ubs-beads-distill-removal-design.md`, plus nothing else.

**Step 4: P4 — nothing was deleted**

```bash
git diff --stat main...HEAD -- docs/ claudedocs/ | tail -1
git diff --name-status main...HEAD | grep -c '^D' || echo "0 deletions"
```

Expected: `0 deletions`. Any `D` line without a matching `R` violates `AGENTS.md` RULE 1 —
stop and report it rather than proceeding.

**Step 5: P5 — the rule is in `AGENTS.md`**

```bash
grep -n "## Documentation Layout" AGENTS.md
```

Expected: one hit.

**Step 6: Whole-tree sanity — the old directories are gone**

```bash
ls docs/superpowers docs/specs claudedocs 2>&1
```

Expected: three `No such file or directory` errors.

**Step 7: Report**

Summarise: files moved, files deleted (must be zero), which checks passed, which failed.
Do not claim completion unless P1–P5 all pass with output shown.

---

## Not in this plan

`.superpowers/sdd/phase-3-report.md` is session scratch that leaked into git — the rest of
that directory is gitignored. Untracking it needs `git rm --cached`, which removes a file
from the index and therefore requires explicit written permission under `AGENTS.md` RULE 1.
Ask separately; do not do it as part of this work.
