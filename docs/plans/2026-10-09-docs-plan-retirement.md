# Docs Plan Retirement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the 2.0 MB of stale implementation plans from `docs/plans/`, carry each plan's durable findings into its design doc, and rewrite the three guideline files so the pile cannot rebuild.

**Architecture:** A read-only extraction pass classifies every plan and pulls candidate outcome lines into a scratch manifest; the parent then hand-writes `## Implementation outcome` sections into the designs, deletes the shipped plans in one reviewable commit, and lands the rule change. No scripted edits touch any markdown file — the repo forbids regex-driven code/doc transformation.

**Tech Stack:** git, ripgrep, bash verification one-liners. No build, no tests — this is a documentation change, so every task's check is a command with exact expected output.

**Spec:** `docs/plans/2026-10-09-docs-plan-retirement-design.md`

## Global Constraints

- Never script an edit to a markdown file. `## Implementation outcome` sections are hand-written `edit` calls. `git rm` of whole files is allowed.
- The only files deleted are `docs/plans/*.md` files that do **not** end in `-design.md`, plus the scratch manifest and this plan. Nothing else, ever.
- No `git push`, no history rewrite, no `git worktree` changes. The `flash-rootfs-cleanup` and `juan-board-support` worktrees are not touched.
- Design filenames never change. `docs/plans/` keeps its name.
- An outcome section is at most 10 lines and never summarises the task list.
- One writer (the parent) mutates files. Extraction lanes are read-only.

## Review Focus

The unrecoverable failure here is silent knowledge loss; everything else is recoverable from git.

1. **A shipped plan held a real deviation and got no outcome section.** The design then
   actively misdescribes the shipped code, and the plan is gone from the working tree.
   Expect: any plan section titled deviation / findings / measured / gate-failed appears
   in the manifest, even when the answer is "nothing worth keeping" — a `NONE` must be a
   decision, not a skip.
2. **An outcome section restates the task list.** That reintroduces the bloat wearing a
   design doc as a costume. Expect: sections contain decisions and measurements,
   no "Task 1..15" narrative, ≤10 lines.
3. **A markdown link resolves to a deleted plan.** Expect: zero dangling `docs/plans/*.md`
   links repo-wide after the delete commit.
4. **A guideline file still tells the next agent that design+plan pairs are the durable
   convention.** The rule then never takes effect and the pile returns in a month.
   Expect: `AGENTS.md` and `docs/README.md` both state the retirement rule.
5. **A plan for unshipped work gets deleted.** Its author loses the working artifact they
   still need. Expect: every kept plan is named with the evidence that its work is unmerged.

---

### Task 1: Classify every plan and extract candidate residue

**Files:**
- Create: `.docs-retirement-manifest.md` (repo root, untracked, scratch)

**Interfaces:**
- Consumes: `docs/plans/*.md`
- Produces: `.docs-retirement-manifest.md` — one pipe-delimited line per non-design plan:
  `plan-path | design-path-or-NONE | shipped|in-flight | NONE|<candidate outcome lines>`

- [ ] **Step 1: Enumerate the working set and confirm the count**

```bash
cd /home/kmk/dev/anyka-dev
git ls-files 'docs/plans/*.md' | grep -v -- '-design\.md$' | tee /tmp/plan-list.txt | wc -l
```
Expected: `64`. If it is not 64, stop — the spec's numbers are stale and the plan needs
revising before anything is deleted.

- [ ] **Step 2: Classify shipped vs in-flight before extracting anything**

For each plan, the work is **in-flight** if either the matching design's `Status:` line
says pending / not yet implemented / approved-implementing, **or** a symbol the plan tells
the implementer to create is absent from the tree. Spot-check the latter rather than
trusting the status line — they are known-unreliable (the spec documents 189 unchecked
boxes against 31 checked).

```bash
grep -n 'Status' docs/plans/2026-09-28-juan-board-support-design.md
```
Expected: a status indicating unmerged work. `2026-09-28-juan-board-support.md` is the
known in-flight case — its branch is 12 commits ahead of `main`.

- [ ] **Step 3: Run the extraction lanes**

Split the 64 plans into lanes of ~8, each lane read-only, each returning one manifest line
per plan. Per plan the lane reports: `shipped`/`in-flight`, and either `NONE` or the
verbatim candidate lines for a deviation that shipped instead of the designed approach, a
hardware measurement that contradicted the design, a gate that failed and its resolution,
or a deliberately-untaken follow-up. Lanes quote; they do not paraphrase or editorialise.

- [ ] **Step 4: Write the manifest and verify it covers the set**

```bash
wc -l < .docs-retirement-manifest.md
comm -3 <(sort /tmp/plan-list.txt) <(cut -d'|' -f1 .docs-retirement-manifest.md | sed 's/ //' | sort)
```
Expected: `64`, then no output (every plan present exactly once, no extras).

- [ ] **Step 5: Sanity-check the residue rate**

```bash
grep -c '| NONE$' .docs-retirement-manifest.md
grep -c '| in-flight' .docs-retirement-manifest.md
```
Expected: the `NONE` count is the large majority — most plans say nothing the design
does not already say. If almost every plan produced candidate lines, the lanes are
summarising task lists instead of extracting findings; re-run them with the four allowed
categories restated. Record both counts; Task 5 reports them.

_No commit — the manifest is scratch and stays untracked._

---

### Task 2: Write the outcome sections into the designs

**Files:**
- Modify: each `docs/plans/*-design.md` named in the manifest with residue
- Create: `docs/reference/<topic>.md` only for an orphan-plan finding that stands alone

**Interfaces:**
- Consumes: `.docs-retirement-manifest.md` (Task 1)
- Produces: designs carrying an optional `## Implementation outcome` section as their last section

- [ ] **Step 1: For each design with residue, hand-append the section**

Append as the final section, matching the house style of
`docs/plans/2026-08-11-diagnostics-vision-design.md` (ATX headings, `| Topic | Choice |`
tables, bold for the term being defined):

```markdown
## Implementation outcome

- **`manifest.sha256`, not `manifest.json`.** `anyka-init` has no serde_json and
  `sha256sum -c` verifies the whole bundle in one exit status.
```

One bullet per finding. Quote exact identifiers, filenames, and measured numbers from the
manifest. No bullet → no section, and no empty heading left behind.

- [ ] **Step 2: Handle the 8 orphan plans**

For each manifest line whose design is `NONE`, the residue either folds into an
existing `docs/reference/` document, becomes one new short one, or is dropped. Prefer
folding — `docs/reference/` already holds 6 documents and the index in
`docs/README.md` is a table to append a row to.

- [ ] **Step 3: Verify every design is well-formed and no design grew a task list**

```bash
git diff -- docs/plans | grep -E '^\+.*^- \['
git diff --name-only -- docs/plans | grep -v -- '-design\.md$'
git diff --stat -- docs/plans | tail -1
```
Expected: no output from the first two (no design gained checkbox syntax — two designs
already contain some, so check the diff, not the files; no non-design file touched), then
a stat line whose insertions are small relative to 64 files.

- [ ] **Step 4: Commit**

```bash
git add -A docs/plans docs/reference
git commit -m "docs(plans): carry implementation deviations into the design docs"
```

---

### Task 3: Delete the shipped plans and repair the links that pointed at them

**Files:**
- Delete: every `docs/plans/*.md` not ending in `-design.md`, except the in-flight keeps
- Modify: `docs/reference/anyka-init-smoke-test.md:9`
- Modify: `docs/archive/README.md:20`

**Interfaces:**
- Consumes: `.docs-retirement-manifest.md` (Task 1) for the delete list and keep list
- Produces: `docs/plans/` containing only designs plus any in-flight plans

- [ ] **Step 1: Stage the deletions and show them for review before committing**

```bash
git ls-files 'docs/plans/*.md' | grep -v -- '-design\.md$' \
  | grep -v -f <(grep '| in-flight' .docs-retirement-manifest.md | cut -d'|' -f1 | sed 's/^ *//') > /tmp/to-delete.txt
wc -l < /tmp/to-delete.txt
git rm --quiet $(cat /tmp/to-delete.txt) && git diff --cached --stat | tail -1
```
Expected: 64 minus the in-flight keeps. The staged stat is the deletion review — every
line must be a `docs/plans/` file, and the byte total should land near 2.0 MB.

- [ ] **Step 2: Repair the two known dangling links**

`docs/reference/anyka-init-smoke-test.md:9` reads
`Plan: \`docs/plans/2026-08-01-boot-runtime-rust.md\`` — drop the line, keeping the
`Design:` line above it. `docs/archive/README.md:20` cites
`docs/plans/2026-08-01-docs-consolidation.md` as a live file — reword to name the design
doc instead, which survives.

- [ ] **Step 3: Verify no link resolves to a deleted plan**

```bash
grep -rhoE 'docs/plans/[A-Za-z0-9._-]+\.md' --include='*.md' . \
  | grep -v node_modules | sort -u | while read -r p; do [ -f "$p" ] || echo "DANGLING: $p"; done
```
Expected: no output.

- [ ] **Step 4: Commit**

```bash
git commit -m "docs(plans): retire shipped implementation plans

Implementation plans are working artifacts, not documentation. Recover any of
these with: git log --diff-filter=D -- docs/plans/<name>.md"
```

---

### Task 4: Land the rule in the guideline files

**Files:**
- Modify: `AGENTS.md:167-173` (Documentation Layout) and workflow step 8
- Modify: `docs/README.md:7` (routing row), `:14` (skills note), `:22-40` (plans table)

**Interfaces:**
- Consumes: the rule text from the spec, verbatim
- Produces: guidelines that state plans are transient

- [ ] **Step 1: Rewrite the `docs/README.md` conventions**

Change the routing row to name `docs/plans/` as designs plus in-flight plans. Replace the
"Newest first. A `-design.md` file is the approved shape…" paragraph with the retirement
rule, keeping the existing and still-correct end-date test ("A plan has an end date —
it describes work that concludes. Reference does not."). Delete the 17-row plans table and
its trailing note about the WebUI irregular pair; `ls docs/plans/` is the index. Keep the
Reference and Design sections untouched.

- [ ] **Step 2: Update `AGENTS.md` to match**

The Documentation Layout block and its table row gain the same wording, and workflow step 8
gains the ship-time action: delete the plan, fold its deviations into the
design. Keep the existing "do not invent a new location" warning — this change depends on
the skills continuing to write to `docs/plans/`.

- [ ] **Step 3: Check `docs/archive/README.md` and leave it alone if it is already correct**

Its line 4 says new design and plan docs go in `docs/plans/`, which stays true — plans do
go there while in flight. Change it only if it asserts plans are kept permanently.

- [ ] **Step 4: Verify the rule is stated and the agent config still passes**

```bash
grep -rn 'working artifact\|delete the plan' AGENTS.md docs/README.md
python3 scripts/ci/check_agent_config.py
```
Expected: matches in both guideline files, and the config check exits clean.

- [ ] **Step 5: Commit**

```bash
git commit -m "docs: make implementation plans transient working artifacts"
```

---

### Task 5: Apply the new rule to this cleanup and verify end state

**Files:**
- Delete: `.docs-retirement-manifest.md`, `docs/plans/2026-10-09-docs-plan-retirement.md`
- Modify: `docs/plans/2026-10-09-docs-plan-retirement-design.md`

**Interfaces:**
- Consumes: counts recorded in Task 1 Step 5
- Produces: the spec's `## Implementation outcome`, and a clean tree

- [ ] **Step 1: Record this cleanup's own outcome in its design**

Append `## Implementation outcome` to
`docs/plans/2026-10-09-docs-plan-retirement-design.md`. Report the measured residue
rate from Task 1 Step 5, and any deviation from the spec — in particular whether plans were
kept as in-flight, which amends the spec's "delete all 64".

- [ ] **Step 2: Delete this plan and the manifest, dogfooding the rule**

```bash
git rm docs/plans/2026-10-09-docs-plan-retirement.md
rm .docs-retirement-manifest.md
```

- [ ] **Step 3: Verify the end state**

```bash
git ls-files 'docs/plans/*.md' | grep -v -- '-design\.md$'
grep -rhoE 'docs/plans/[A-Za-z0-9._-]+\.md' --include='*.md' . \
  | grep -v node_modules | sort -u | while read -r p; do [ -f "$p" ] || echo "DANGLING: $p"; done
python3 scripts/ci/check_agent_config.py
git status --porcelain
```
Expected: only the in-flight keeps on the first command; no `DANGLING:` lines; config check
clean; `git status --porcelain` shows nothing but the pre-existing untracked files
(`Firmwares X6E-WEQ/`, `outline.svg`, `scripts/serial_probe.sh`, `scripts/tftp_recv.py`,
`docs/reference/xm535-x6e-weq-firmware.md`).

- [ ] **Step 4: Commit**

```bash
git commit -m "docs(plans): retire this cleanup's implementation plan"
```

_Do not push. The spec's non-goals put branch and push strategy with the human partner._
