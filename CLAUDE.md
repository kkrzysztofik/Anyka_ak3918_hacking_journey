# Agent Documentation for Anyka AK3918 Hacking Journey

## Canonical Source

`AGENTS.md` is the canonical source for shared repository policy.

Use `AGENTS.md` for:
- agent role and project mandate
- mandatory document-loading protocol
- toolchain requirements
- coding, testing, and quality workflow
- branch/session completion rules

Keep this file focused on Claude-specific operational guidance so it does not drift from `AGENTS.md`.

## Quick Operational Rules

- Load the relevant Serena memories before starting work. For most coding tasks this means at least `agent-core`, `development-standards`, `testing-framework`, `quality-gates`, and `review-prompt`.
- Use the vendored Rust toolchain at `toolchain/arm-anykav200-crosstool-ng/bin/cargo` for all cargo commands.
- For host-side Rust operations, always use `--target x86_64-unknown-linux-gnu`.
- Follow the workflow and quality gates defined in `AGENTS.md`; do not skip tests, linting, or documentation updates when they apply.
- Before claiming completion, run verification and request code review using the matching project workflow.

## Claude-Specific Skill And Subagent Routing

Use the appropriate project skill or subagent before doing complex work.

### Prefer these skills first

Project skills live in `.claude/skills/` and are the single source of truth for
every agent host — Claude Code and opencode read that directory natively, pi is
pointed at it by `.pi/settings.json`. Add a skill once and all three see it.

- Rust implementation: `sc:implement`
- ONVIF service work: `onvif-service-impl`
- Rust testing: `anyka-rust-testing`
- RTSP/RTP streaming: `rtsp-rtp-streaming`
- vendor-daemon C / IPC work: `vendor-daemon-ipc`
- WebUI component work: `camera-webui-components`
- WebUI testing: `anyka-webui-testing`
- Cross-compilation and builds: `anyka-embedded-build`
- Deploying to a camera: `anyka-firmware-upgrade`
- On-device debugging and coredumps: `anyka-remote-debugging`
- Protocol conformance and performance runs: `anyka-validation`
- Follow the Skill & Complexity Policy in `AGENTS.md`: superpowers process skills auto-trigger on task type; ponytail (full) is the default for code and plans.

### Subagents

This repo defines no Claude Code subagents; `.claude/agents/` is intentionally
absent. The per-language "coder" and "qa-engineer" agents were removed because
the skills above already carried their content.

Consensus code review lives in **opencode**, at `.opencode/agents/`:
`reviewer-consensus` dispatches `reviewer-architecture` (gpt-5.4),
`reviewer-testing` (gemini-3.1-pro), `reviewer-security` (opus-4-6) and
`reviewer-memory` (sonnet-4-5). The value is four genuinely different models
disagreeing, so run it from opencode — a Claude-only run is not a consensus.

For delegation inside Claude Code, use the installed plugin agents
(`rust-engineer`, `typescript-pro`, `embedded-systems`, `performance-engineer`,
`explore`, …) or the superpowers process skills.

## Non-Negotiable Reminders

- Do not manually bypass a matching skill or subagent when one clearly applies.
- Prefer the Serena MCP tools for code search and edits.
- Keep changes minimal and consistent with surrounding style.
- Preserve behavior unless fixing a clear bug.
- When in doubt about the shared workflow, follow `AGENTS.md`.
