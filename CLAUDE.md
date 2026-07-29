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

- Rust implementation: `sc:implement`
- ONVIF service work: `onvif-service-impl`
- Rust testing: `anyka-rust-testing`
- RTSP/RTP streaming: `rtsp-rtp-streaming`
- WebUI component work: `camera-webui-components`
- WebUI testing: `anyka-webui-testing`
- Cross-compilation and deploy: `anyka-embedded-build`
- Bug investigation: `superpowers:systematic-debugging`
- Pre-implementation workflow: `superpowers:test-driven-development`
- Verification before completion: `superpowers:verification-before-completion`
- Pre-merge review: `superpowers:requesting-code-review`

### Prefer these subagents when delegation helps

- Rust-heavy implementation: `rust-engineer`
- TypeScript/WebUI work: `typescript-pro`
- Embedded or hardware-facing work: `embedded-systems`
- Architecture review: `system-architect` or `backend-architect`
- Performance work: `performance-engineer`
- Refactoring: `refactoring-specialist`
- Debugging: `debugger`
- Broad codebase exploration: `explore`
- Security review: `security-engineer`
- Testing automation: `test-automator`

## Non-Negotiable Reminders

- Do not manually bypass a matching skill or subagent when one clearly applies.
- Prefer the Serena MCP tools for code search and edits.
- Keep changes minimal and consistent with surrounding style.
- Preserve behavior unless fixing a clear bug.
- When in doubt about the shared workflow, follow `AGENTS.md`.
