# Agent Config Consolidation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `.claude/skills/` the single source of truth for Claude Code, opencode and pi, and remove the duplicated agent definitions and dead deployment content around it.

**Architecture:** No sync machinery. opencode reads `.claude/skills/` natively; pi is pointed at it with a three-line `.pi/settings.json`; Claude Code owns it. Duplicated agent bodies are removed rather than generated, because ~80% of their content already exists in skills. A CI guard asserts skill *count* and catches symlinks-committed-as-text, the exact failure that made `.opencode/skills/` silently dead for two months.

**Tech Stack:** Markdown skills (Agent Skills standard), JSON config, Python 3 (stdlib `unittest` — matches the existing `Test Build Scripts (Python)` CI step), GitHub Actions.

**Design:** `docs/plans/2026-09-18-agent-config-consolidation-design.md`

---

## Task 0: Deletion authorization gate (BLOCKING)

`AGENTS.md` RULE NUMBER 1 forbids deleting any file without express written permission. This plan deletes 30 tracked files. **Do not start Task 4, 8 or 10 until the user has approved this exact list in writing.**

Mitigating facts to present with the request:

- All 30 files are tracked in git. Deletion is `git rm` on a branch, fully recoverable via history — not `rm -rf` of untracked work.
- The 8 files under `.opencode/skills/` are broken stubs that have never resolved.

**Files to delete — `.opencode/skills/` (8, all broken stubs):**

```
.opencode/skills/anyka-embedded-build
.opencode/skills/anyka-rust-testing
.opencode/skills/anyka-webui-testing
.opencode/skills/camera-webui-components
.opencode/skills/onvif-service-impl
.opencode/skills/onvif-soap-client
.opencode/skills/protocol-debugging
.opencode/skills/rtsp-rtp-streaming
```

**Files to delete — `.claude/agents/` (12, all 12 after content is preserved by Tasks 5–7):**

```
architect.md  coder-c.md  coder-rust.md  coder-typescript.md
debugger.md  designer.md  devops.md  orchestrator.md
planner.md  qa-engineer-rust.md  qa-engineer-www.md  security.md
```

Plus the 5 reviewers move out of `.claude/agents/`, leaving the directory empty:
`reviewer-architecture.md  reviewer-consensus.md  reviewer-memory.md  reviewer-security.md  reviewer-testing.md`

**Files to delete — `.opencode/agents/` (12):** the same 12 names as above. The 5 `reviewer-*.md` files **stay**.

**Files to delete — skill assets (1):**

```
.claude/skills/anyka-embedded-build/scripts/deploy.sh
```

**Step 1:** Present the list above to the user verbatim and ask for explicit approval.

**Step 2:** Record the user's exact authorizing text in the commit body of the first deletion commit, per `AGENTS.md` "Document the confirmation".

**Do not proceed past this task without that approval.**

---

# Phase 1 — Skills plumbing

## Task 1: Point pi at `.claude/skills/`

**Files:**
- Create: `.pi/settings.json`

**Step 1: Create the file**

```json
{
  "skills": ["../.claude/skills"]
}
```

The path is relative to `.pi/`, per `earendil-works/pi` → `packages/coding-agent/docs/skills.md`.

**Step 2: Verify pi discovers the skills**

Run: `pi --help 2>&1 | head -5` first to confirm pi is on PATH, then from the repo root start pi interactively and accept the project-trust prompt (pi records it in `~/.pi/agent/trust.json`; it will not load project resources until you do).

Expected: all 11 skills listed, including `anyka-firmware-upgrade`, `anyka-remote-debugging` and `anyka-validation`.

If pi exposes a non-interactive listing in this version, prefer it. If it does not, record the interactive result in the commit message — an unverified claim here is worthless, since the whole bug class is silent empty discovery.

**Step 3: Commit**

```bash
rtk git add .pi/settings.json
rtk git commit -m "chore(pi): read project skills from .claude/skills"
```

---

## Task 2: Write the agent-config guard (TDD)

This guard exists to catch two things: skills silently dropping to zero, and symlinks committed as text.

**Files:**
- Create: `scripts/ci/check_agent_config.py`
- Test: `scripts/ci/test_check_agent_config.py`

**Step 1: Write the failing test**

```python
"""Tests for the agent-config guard."""

import tempfile
import unittest
from pathlib import Path

from check_agent_config import find_stub_symlinks, load_skills


class TestLoadSkills(unittest.TestCase):
    def test_load_skills_valid_dir_returns_name_and_description(self):
        with tempfile.TemporaryDirectory() as tmp:
            skill = Path(tmp) / "my-skill"
            skill.mkdir()
            (skill / "SKILL.md").write_text(
                "---\nname: my-skill\ndescription: Use when testing.\n---\n\nBody.\n"
            )
            skills = load_skills(Path(tmp))
            self.assertEqual(skills, {"my-skill": "Use when testing."})

    def test_load_skills_missing_description_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            skill = Path(tmp) / "bad-skill"
            skill.mkdir()
            (skill / "SKILL.md").write_text("---\nname: bad-skill\n---\n\nBody.\n")
            with self.assertRaises(ValueError):
                load_skills(Path(tmp))

    def test_load_skills_name_mismatch_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            skill = Path(tmp) / "dir-name"
            skill.mkdir()
            (skill / "SKILL.md").write_text(
                "---\nname: other-name\ndescription: x\n---\n"
            )
            with self.assertRaises(ValueError):
                load_skills(Path(tmp))


class TestFindStubSymlinks(unittest.TestCase):
    def test_find_stub_symlinks_detects_path_committed_as_text(self):
        with tempfile.TemporaryDirectory() as tmp:
            stub = Path(tmp) / "some-skill"
            stub.write_text("../../anyka-dev/.claude/skills/some-skill")
            self.assertEqual(find_stub_symlinks(Path(tmp)), [stub])

    def test_find_stub_symlinks_ignores_real_content(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "notes.md").write_text("# Real file\n\nWith prose.\n")
            self.assertEqual(find_stub_symlinks(Path(tmp)), [])


if __name__ == "__main__":
    unittest.main()
```

**Step 2: Run test to verify it fails**

```bash
cd scripts/ci && python3 -m unittest test_check_agent_config -v
```

Expected: FAIL with `ModuleNotFoundError: No module named 'check_agent_config'`.

**Step 3: Write minimal implementation**

```python
#!/usr/bin/env python3
"""Guard the shared agent configuration.

Two failure modes this catches, both of which are silent in every agent host:

1. Skill discovery dropping to zero. A host that finds no skills reports no
   error, it just stops offering them. Assert a count, never an error string.
2. Symlinks committed as regular files. Git stores a symlink as mode 120000;
   a checkout without symlink support writes mode 100644 with the target as
   the file's content. That is what killed .opencode/skills/ for two months.
"""

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SKILLS_DIR = REPO_ROOT / ".claude" / "skills"
PI_SETTINGS = REPO_ROOT / ".pi" / "settings.json"

# A stub is short, single-line, and looks like a path. Real prose does not.
STUB_MAX_BYTES = 256


def _frontmatter(text):
    """Return the frontmatter block as a dict of the keys we care about."""
    if not text.startswith("---"):
        return {}
    _, _, rest = text.partition("---")
    block, _, _ = rest.partition("---")
    fields = {}
    for line in block.splitlines():
        key, sep, value = line.partition(":")
        if sep:
            fields[key.strip()] = value.strip()
    return fields


def load_skills(skills_dir):
    """Map skill name -> description, validating each SKILL.md."""
    skills = {}
    for entry in sorted(skills_dir.iterdir()):
        if not entry.is_dir():
            continue
        manifest = entry / "SKILL.md"
        if not manifest.is_file():
            raise ValueError(f"{entry} has no SKILL.md")
        fields = _frontmatter(manifest.read_text(encoding="utf-8"))
        name = fields.get("name")
        description = fields.get("description")
        if not description:
            raise ValueError(f"{manifest} has no description; hosts filter it out")
        if name != entry.name:
            raise ValueError(f"{manifest} name '{name}' != directory '{entry.name}'")
        skills[name] = description
    return skills


def find_stub_symlinks(root):
    """Find regular files whose whole content is a filesystem path."""
    stubs = []
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.is_symlink():
            continue
        if path.stat().st_size > STUB_MAX_BYTES:
            continue
        try:
            text = path.read_text(encoding="utf-8").strip()
        except (UnicodeDecodeError, OSError):
            continue
        if not text or "\n" in text:
            continue
        if "/" in text and " " not in text:
            stubs.append(path)
    return stubs


def main():
    errors = []

    try:
        skills = load_skills(SKILLS_DIR)
    except ValueError as exc:
        print(f"FAIL: {exc}")
        return 1

    if len(skills) < 11:
        errors.append(f"expected >= 11 skills in {SKILLS_DIR}, found {len(skills)}")

    for directory in (REPO_ROOT / ".claude", REPO_ROOT / ".opencode", REPO_ROOT / ".pi"):
        if not directory.is_dir():
            continue
        for stub in find_stub_symlinks(directory):
            errors.append(f"{stub} looks like a symlink committed as text")

    if not PI_SETTINGS.is_file():
        errors.append(f"{PI_SETTINGS} is missing; pi will not see project skills")
    elif not (PI_SETTINGS.parent / "../.claude/skills").resolve().is_dir():
        errors.append("pi settings point at a directory that does not resolve")

    for error in errors:
        print(f"FAIL: {error}")
    if errors:
        return 1

    print(f"OK: {len(skills)} skills, no stub symlinks, pi wired up")
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

**Step 4: Run tests to verify they pass**

```bash
cd scripts/ci && python3 -m unittest test_check_agent_config -v
```

Expected: 5 tests, all PASS.

**Step 5: Run the guard against the live repo**

```bash
python3 scripts/ci/check_agent_config.py; echo "exit=$?"
```

Expected at this point: **exit=1**, reporting 8 stub symlinks under `.opencode/`. That is the bug, correctly detected. It goes green after Task 4.

**Step 6: Commit**

```bash
rtk git add scripts/ci/check_agent_config.py scripts/ci/test_check_agent_config.py
rtk git commit -m "test(ci): guard skill discovery count and stub symlinks"
```

---

## Task 3: Wire the guard into CI

`main-ci.yml` path filters cover `cross-compile/**`, `validation/**`, `scripts/**`, `SD_card_contents/**` and `.github/workflows/**` — **not** `.claude/**`, `.opencode/**` or `.pi/**`. A skills-only change triggers no workflow at all today. A separate small workflow is cleaner than widening the 45 KB pipeline's filters.

**Files:**
- Create: `.github/workflows/agent-config.yml`

**Step 1: Create the workflow**

```yaml
name: Agent Config

on:
  push:
    branches: [main]
    paths:
      - ".claude/**"
      - ".opencode/**"
      - ".pi/**"
      - "scripts/ci/check_agent_config.py"
      - ".github/workflows/agent-config.yml"
  pull_request:
    branches: [main]
    types: [opened, synchronize, reopened]
    paths:
      - ".claude/**"
      - ".opencode/**"
      - ".pi/**"
      - "scripts/ci/check_agent_config.py"
      - ".github/workflows/agent-config.yml"

jobs:
  check:
    name: Skills & agent config
    runs-on: ubuntu-latest
    steps:
      - name: Checkout Code
        uses: actions/checkout@v4

      - name: Unit tests
        working-directory: scripts/ci
        run: python3 -m unittest discover -p 'test_check_agent_config.py' -v

      - name: Check agent config
        run: python3 scripts/ci/check_agent_config.py
```

**Step 2: Commit**

```bash
rtk git add .github/workflows/agent-config.yml
rtk git commit -m "ci: check agent config on skill and agent changes"
```

---

## Task 4: Remove the broken opencode skill stubs

**Requires Task 0 approval.**

**Files:**
- Delete: the 8 stub files under `.opencode/skills/`

**Step 1: Confirm each one is a stub, not a real skill**

```bash
for f in .opencode/skills/*; do printf '%s -> ' "$f"; cat "$f"; echo; done
```

Expected: 8 single-line paths, no `SKILL.md` anywhere. If any file has real content, **stop** and re-check with the user.

**Step 2: Delete**

```bash
rtk git rm -r .opencode/skills
```

**Step 3: Verify opencode still sees every skill**

```bash
opencode --version   # expect >= 1.18.22
```

Then from the repo root, list skills in an opencode session. Expected: 11 skills, including the three that never had stubs. This is the load-bearing verification of the whole design — opencode must find them via its native `.claude/skills/` project scan.

**Step 4: Run the guard**

```bash
python3 scripts/ci/check_agent_config.py; echo "exit=$?"
```

Expected: **exit=0**, `OK: 11 skills, no stub symlinks, pi wired up`.

**Step 5: Commit**, quoting the Task 0 authorization in the body.

```bash
rtk git commit -m "chore(opencode): drop broken skill stubs

Authorized by: <paste the user's exact approval text>

The 8 entries were symlinks committed as regular files, pointing at
<repo>/anyka-dev/.claude/skills/* which has never existed. opencode reads
.claude/skills/ natively, so no replacement is needed."
```

---

# Phase 2 — Agents

Content is preserved **before** anything is deleted. Per `AGENTS.md` "No Script-Based Changes", move this prose by hand — no sed, no regex transforms.

## Task 5: Promote `coder-c` to a `vendor-daemon-ipc` skill

`coder-c` is the one agent holding content no skill covers: the IPC wire format, socket endpoints, frame-notification protocol and Anyka SDK APIs.

**Files:**
- Create: `.claude/skills/vendor-daemon-ipc/SKILL.md`
- Source: `.claude/agents/coder-c.md` (290 lines)

**Step 1: Create the skill with frontmatter**

```markdown
---
name: vendor-daemon-ipc
description: Use when writing or debugging the vendor-daemon C bridge — IPC wire format, control and frame sockets, poll() multiplexing, Anyka SDK calls, and ARMv5TE/uClibc cross-compilation.
version: 1.0.0
---
```

**Step 2: Move these sections from `coder-c.md` verbatim**

- `### What Is vendor-daemon?` and `### Source Layout`
- `## IPC Protocol Reference` in full — wire format, socket endpoints, frame notification protocol, client model
- `## Mandatory Coding Standards` — bounded buffer functions, SDK return-code checks, IPC input validation, logging macros, allocation rules, the `poll()` multiplexer pattern
- `## Anyka SDK Key APIs`

**Step 3: Drop what is now duplicated**

`## Toolchain and Build` → already in `anyka-embedded-build`. Replace with a one-line pointer.

**Step 4: Verify**

```bash
python3 scripts/ci/check_agent_config.py; echo "exit=$?"
```

Expected: exit=0, now reporting 12 skills.

**Step 5: Commit**

```bash
rtk git add .claude/skills/vendor-daemon-ipc
rtk git commit -m "docs(skills): add vendor-daemon-ipc from the coder-c agent"
```

---

## Task 6: Preserve the designer personas

**Files:**
- Create: `.claude/skills/camera-webui-components/references/personas.md`
- Source: `.claude/agents/designer.md` (230 lines)

**Step 1:** Move `## User Personas`, `## Design Constraints`, `## Job Statement` and `## Current Pain` into the new reference file, by hand.

**Step 2:** Add one line under an appropriate heading in `.claude/skills/camera-webui-components/SKILL.md` pointing at `references/personas.md`.

**Step 3: Commit**

```bash
rtk git add .claude/skills/camera-webui-components
rtk git commit -m "docs(skills): keep designer personas with the WebUI skill"
```

---

## Task 7: Move residual Rust rules into `AGENTS.md`

**Files:**
- Modify: `AGENTS.md`

**Step 1:** Check what is already there. `AGENTS.md` has a `### Critical Standards` block around line 84 — read it first and add only what is genuinely missing from `coder-rust.md`:

- no `unwrap()` / `expect()` outside tests
- `tracing::*`, never `println!` / `eprintln!`
- `tokio::sync::{Mutex, RwLock}`, never `std::sync` in async code
- `// SAFETY:` comment required on every `unsafe` block
- 24 MB memory budget: prefer `Box<T>` for multi-KB structs, `&str` over `String` in signatures

Add nothing that is already stated. This should be a handful of lines, not a section.

**Step 2: Commit**

```bash
rtk git add AGENTS.md
rtk git commit -m "docs: fold the coder-rust coding rules into AGENTS.md"
```

---

## Task 8: Remove the duplicated agents

**Requires Task 0 approval. Do not start before Tasks 5–7 are committed.**

**Step 1: Confirm the content is preserved**

```bash
rtk git log --oneline -3
ls .claude/skills/vendor-daemon-ipc/SKILL.md
ls .claude/skills/camera-webui-components/references/personas.md
```

All three must exist. If any is missing, stop.

**Step 2: Move the 5 reviewers out of `.claude/agents/`**

They stay only in `.opencode/agents/`, where the four models are genuinely different. Confirm each opencode copy is intact first:

```bash
for a in architecture consensus memory security testing; do
  printf '%-14s ' "$a"; awk '/^model:/{print $2; exit}' .opencode/agents/reviewer-$a.md
done
```

Expected: `openai/gpt-5.4`, `anthropic/claude-sonnet-4-5`, `anthropic/claude-sonnet-4-5`, `anthropic/claude-opus-4-6`, `google/gemini-3.1-pro-preview`.

**Step 3: Delete**

```bash
rtk git rm .claude/agents/architect.md .claude/agents/coder-c.md \
  .claude/agents/coder-rust.md .claude/agents/coder-typescript.md \
  .claude/agents/debugger.md .claude/agents/designer.md \
  .claude/agents/devops.md .claude/agents/orchestrator.md \
  .claude/agents/planner.md .claude/agents/qa-engineer-rust.md \
  .claude/agents/qa-engineer-www.md .claude/agents/security.md \
  .claude/agents/reviewer-architecture.md .claude/agents/reviewer-consensus.md \
  .claude/agents/reviewer-memory.md .claude/agents/reviewer-security.md \
  .claude/agents/reviewer-testing.md

rtk git rm .opencode/agents/architect.md .opencode/agents/coder-c.md \
  .opencode/agents/coder-rust.md .opencode/agents/coder-typescript.md \
  .opencode/agents/debugger.md .opencode/agents/designer.md \
  .opencode/agents/devops.md .opencode/agents/orchestrator.md \
  .opencode/agents/planner.md .opencode/agents/qa-engineer-rust.md \
  .opencode/agents/qa-engineer-www.md .opencode/agents/security.md
```

**Step 4: Verify what remains**

```bash
ls .opencode/agents/   # expect exactly 5 reviewer-*.md
ls .claude/agents/     # expect empty
```

**Step 5: Commit**, quoting the Task 0 authorization in the body.

---

## Task 9: Update the routing docs

**Files:**
- Modify: `CLAUDE.md` — the `### Prefer these subagents when delegation helps` list
- Modify: `AGENTS.md` — the `## Codex Instruction Mapping (from .github/)` paragraph, which claims Copilot profiles were removed "in favor of the `.opencode/agents/` + `.claude/agents/` sets"

**Step 1:** In `CLAUDE.md`, replace the subagent list with the current reality: consensus review runs from opencode (`reviewer-consensus`), and the per-language work is covered by skills, not agents. Keep only entries that still resolve.

**Step 2:** In `AGENTS.md`, state that skills live in `.claude/skills/` and are read by all three hosts, that `.opencode/agents/` holds the 5 consensus reviewers, and that `.claude/agents/` is intentionally empty.

**Step 3: Commit**

```bash
rtk git add CLAUDE.md AGENTS.md
rtk git commit -m "docs: point agent routing at skills and the opencode reviewers"
```

---

# Phase 3 — Deployment content merge

## Task 10: `anyka-embedded-build` becomes build-only

**Requires Task 0 approval for the `deploy.sh` deletion.**

**Files:**
- Modify: `.claude/skills/anyka-embedded-build/SKILL.md`
- Delete: `.claude/skills/anyka-embedded-build/scripts/deploy.sh`

**Step 1: Confirm the script is dead**

```bash
grep -c 'ssh\|scp' .claude/skills/anyka-embedded-build/scripts/deploy.sh
```

Expected: 8. The cameras run telnet on port 24 and FTP; there is no sshd. Every `ssh` in it fails on connect.

**Step 2: Remove `## SD Card Deployment`** — lines 65–86, including both `deploy.sh` invocations and the pre-slots layout block showing `config/` and `start.sh`.

**Step 3: Replace it with a pointer**

```markdown
## Deployment

See the `anyka-firmware-upgrade` skill. It covers all four paths: A/B bundle
upgrade, full SD payload, one-time slot migration, and single-binary dev
iteration.
```

**Step 4: Fix `## Device Runtime Facts`** — line 92 says the ONVIF endpoint is `http://<ip>:8080/onvif/device_service`. It is port **80**; 8080 is HTTP-FLV. Correct it, then replace the telnet and coredump bullets with a pointer to `anyka-remote-debugging`, which states them already.

**Step 5: Delete the script**

```bash
rtk git rm .claude/skills/anyka-embedded-build/scripts/deploy.sh
```

**Step 6: Verify no stale references remain**

```bash
grep -rn '8080\|deploy\.sh\|start\.sh' .claude/skills/anyka-embedded-build/
```

Expected: no hits.

**Step 7: Commit**, quoting the Task 0 authorization in the body.

---

## Task 11: `anyka-firmware-upgrade` becomes the single deployment skill

**Files:**
- Modify: `.claude/skills/anyka-firmware-upgrade/SKILL.md`

**Step 1: Add a routing table** immediately after the `## When to use` section:

```markdown
## Which path do I need?

| I want to… | Path |
|---|---|
| Ship a versioned change to a running camera | `build_upgrade_bundle.sh` → `upload_upgrade_bundle.sh` (A/B, auto-rollback) — this skill |
| Set up a fresh camera, or change `lib/` or Factory | `build_sd_contents.sh` → `copy_sd_contents.sh --sd PATH \| --ftp HOST` |
| Move a flat camera onto slots (once per camera) | `migrate_to_slots.sh` |
| Iterate on one binary while debugging | `deploy_onvif.sh` + `run_onvif.sh` — dev only, never for upgrades |
```

**Step 2: Add a short `## Full SD payload` section.** This procedure has no skill home today, which is part of why the dead `deploy.sh` looked like the answer. Cover: build then copy are separate steps, `--sd` vs `--ftp`, that it ships `lib/` and Factory unlike a bundle, and the md5-manifest verification requirement for `tar | nc` pushes.

**Step 3: Update the `## Do not` list** — the existing "Use per-binary `deploy_onvif.sh` for A/B upgrades" line stays; add that `.claude/skills/anyka-embedded-build/scripts/deploy.sh` is gone and was never functional.

**Step 4: Commit**

```bash
rtk git add .claude/skills/anyka-firmware-upgrade/SKILL.md
rtk git commit -m "docs(skills): make anyka-firmware-upgrade the single deploy entry point"
```

---

## Task 12: Final verification

**Step 1: Guard passes**

```bash
python3 scripts/ci/check_agent_config.py; echo "exit=$?"
```

Expected: exit=0, 12 skills.

**Step 2: Unit tests pass**

```bash
cd scripts/ci && python3 -m unittest discover -p 'test_*.py' -v
```

**Step 3: All three hosts discover 12 skills.** Check each one for real — Claude Code, opencode, and pi. Record the actual counts. Do not infer one host's result from another's; the entire bug being fixed was one host silently finding zero.

**Step 4: No dangling references**

```bash
rtk grep -rn 'coder-rust\|coder-c\|qa-engineer\|\.opencode/skills' --include='*.md' --include='*.yml' . | grep -v docs/plans
```

Expected: no hits outside the plan and design documents.

**Step 5: Open the PR.**

Note for the PR body: this branch touches no path in `main-ci.yml`'s filters except `scripts/**`, so the new `agent-config.yml` workflow is the meaningful gate. Check `gh pr view --json mergeable` if no runs appear at all.
