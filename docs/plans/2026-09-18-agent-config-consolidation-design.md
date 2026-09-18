# Agent Config Consolidation — Design

**Date:** 2026-09-18
**Status:** Approved, pending implementation plan

Make `.claude/skills/` the single source of truth for every coding agent used on
this repo (Claude Code, opencode, pi), and delete the duplicated agent
definitions and dead deployment content that accumulated alongside it.

## Problem

Three separate failures, all silent:

1. **`.opencode/skills/` never worked.** Its 8 entries are 51-byte *regular
   files* whose content is a relative path — symlinks committed as text (git
   mode `100644` where `120000` was intended). The path is wrong regardless:
   from `.opencode/skills/`, `../../anyka-dev/.claude/skills/X` resolves to
   `<repo>/anyka-dev/.claude/skills/X`, which has never existed. opencode has
   discovered zero project skills since 2026-07-26. The three newest skills
   (`anyka-firmware-upgrade`, `anyka-remote-debugging`, `anyka-validation`)
   never received even a broken stub.

2. **`.opencode/agents/` duplicates `.claude/agents/`.** 17 files each, byte-identical
   bodies, differing only in frontmatter. ~4,000 lines with nothing keeping
   them in sync.

3. **Deployment knowledge is spread across 8 surfaces** and includes code that
   cannot work. `.claude/skills/anyka-embedded-build/scripts/deploy.sh` is
   8.5 KB of `ssh`/`scp` against cameras that run only telnet (port 24) and
   FTP. The same skill documents the ONVIF endpoint as `:8080` (it is `:80`)
   and an SD layout that predates the A/B slots migration.

## Discovery matrix (verified)

Checked against installed binaries and published docs, 2026-09-18.

| Host | Project `.claude/skills/` | Project `.agents/skills/` | Own dir |
|---|---|---|---|
| Claude Code | native | — | `.claude/skills/` |
| opencode 1.18.22 | **native**, walks cwd up to git worktree root | native | `.opencode/skill(s)/` |
| pi | opt-in via settings | native | `.pi/skills/`, `~/.pi/agent/skills/` |

Sources: <https://opencode.ai/docs/skills/>,
`earendil-works/pi` → `packages/coding-agent/docs/skills.md`.

Agents do **not** converge. opencode scans only `.opencode/agent(s)/<name>.md`
and never `.claude/agents/`. The frontmatter conflicts irreconcilably: Claude
wants `name` + `description` + `tools: Read, Grep`; opencode wants
`mode: subagent`, `model: <provider>/<id>`, `tools: {write: false}`,
`permission: {bash: {...}}`. No single file satisfies both.

## Part A — Skills plumbing

```
.claude/skills/      single source of truth, unchanged
.pi/settings.json    new: {"skills": ["../.claude/skills"]}
.opencode/skills/    deleted
```

No generation, no symlinks, no sync step. Adding a skill reaches all three
hosts with no extra work.

**CI guard:** assert each host discovers at least the expected skill count.
This failure mode produces an empty list rather than an error, so a check for
"no errors" would not have caught the last two months. Assert the count.

**Note:** pi prompts once, interactively, to trust a project before loading
project-local resources (recorded in `~/.pi/agent/trust.json`).

## Part B — Agents

### B1. Delete: content already lives in skills (6 files, ~1,640 lines)

| Agent | Already covered by |
|---|---|
| `coder-rust` | `anyka-embedded-build`, `anyka-rust-testing`, `onvif-service-impl` |
| `coder-typescript` | `camera-webui-components`, `onvif-soap-client`, `anyka-webui-testing` |
| `qa-engineer-rust` | `anyka-rust-testing` |
| `qa-engineer-www` | `anyka-webui-testing` |
| `devops` | `anyka-embedded-build`, `anyka-firmware-upgrade` |

`coder-rust` was compared heading by heading: "Run Quality Gates", "Write Tests
(Mandatory)" and "ONVIF Service Implementation Patterns" restate three existing
skills. The residue worth keeping — no `unwrap()`, `tracing` not `println!`,
`tokio::sync` not `std::sync`, the 24 MB budget — is a handful of lines that
belong in `AGENTS.md`, which every host already reads.

### B2. Promote: real content with no skill home (1 new skill)

`coder-c` documents the vendor-daemon IPC wire format, socket endpoints, frame
notification protocol and Anyka SDK APIs. None of this exists elsewhere. It
becomes a new skill, `vendor-daemon-ipc`. This is the only place the work adds
rather than removes.

### B3. Keep as agents: the model assignment is the point (5 files)

| Reviewer | opencode model | Claude model |
|---|---|---|
| `reviewer-architecture` | `openai/gpt-5.4` | `opus` |
| `reviewer-testing` | `google/gemini-3.1-pro-preview` | `sonnet` |
| `reviewer-security` | `anthropic/claude-opus-4-6` | `opus` |
| `reviewer-memory` | `anthropic/claude-sonnet-4-5` | `sonnet` |

Plus `reviewer-consensus`, which dispatches the four.

The consensus review's value is four *different* models disagreeing, which only
happens in opencode. The Claude-side copies dispatch four prompts to two
Anthropic models and report the result as multi-model consensus — the structure
survives the port, the meaning does not. These five live in `.opencode/agents/`
only; `.claude/agents/` loses them.

### B4. Delete: restates installed superpowers skills (5 files)

`architect` → `superpowers:brainstorming`; `planner` → `superpowers:writing-plans`;
`debugger` → `superpowers:systematic-debugging` + `anyka-remote-debugging`;
`orchestrator` → `superpowers:dispatching-parallel-agents`; `security` →
`code-security-audit` skill + `security-guidelines` Serena memory.

`designer` is kept — its user personas and design constraints are not recorded
anywhere else.

**Net:** 34 agent files (~7,400 lines) → 5 agents in one location, plus 1 new skill.

## Part C — Deployment content merge

### `anyka-embedded-build` becomes build-only

Remove `## SD Card Deployment`, `scripts/deploy.sh` (SSH against a host with no
sshd), and `## Device Runtime Facts` (duplicates `anyka-remote-debugging §
Access Model`). Keep toolchain setup, build targets, pre-commit gates,
troubleshooting. Correct ONVIF `:8080` → `:80` and replace the pre-slots SD
layout with `slots/{a,b}`.

### `anyka-firmware-upgrade` becomes the single deployment skill

Add a routing table at the top:

| I want to… | Path |
|---|---|
| Ship a versioned change to a running camera | `build_upgrade_bundle.sh` → `upload_upgrade_bundle.sh` (A/B, auto-rollback) |
| Set up a fresh camera, or change `lib/` or Factory | `build_sd_contents.sh` → `copy_sd_contents.sh --sd\|--ftp` |
| Move a flat camera onto slots (once per camera) | `migrate_to_slots.sh` |
| Iterate on one binary while debugging | `deploy_onvif.sh` + `run_onvif.sh` — dev only, never for upgrades |

The SD-payload procedure currently has no skill home, which is part of why the
dead `deploy.sh` looked like the answer.

## Out of scope

`.cursor/rules/`, `.github/instructions/`, `.github/prompts/` and
`.serena/memories/` keep their current layout. Folding them in was considered
and rejected as too large a blast radius for this pass.

## Risks

- Deleting `.claude/agents/reviewer-*` removes the review path Claude Code
  sessions currently invoke. `reviewer-consensus` must be run from opencode
  afterwards, and `AGENTS.md` needs to say so.
- pi's trust prompt is interactive; a headless pi run against a fresh clone
  loads no project skills until someone accepts it once.
- opencode discovers project skills by walking up to the git worktree root, so
  behaviour inside `.worktrees/` should be confirmed during implementation.
