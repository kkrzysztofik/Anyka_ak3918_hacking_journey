# Agent Core - Anyka AK3918 Project

## Purpose

This memory is a compact operational summary. `AGENTS.md` is the canonical source for repository-wide policy and workflow.

## Always Load

Load this memory first, then add the task-specific memories from `AGENTS.md`.

For most implementation or cleanup tasks, also load:
- `development-standards`
- `testing-framework`
- `quality-gates`
- `review-prompt`
- `suggested_commands` (quick-reference command sheet)

Add `www-development-standards` for WebUI work and `security-guidelines` for auth, validation, or unsafe-code changes.

## Core Constraints

- Follow the project workflow defined in `AGENTS.md`.
- Load the vendored Rust toolchain with `source ./setenv.sh` from the repo root (exports `$CARGO`, `$RUSTC`, `$RUSTDOC`). Never use bare `cargo`, `rustup`, or hardcoded absolute paths.
- Use `--target x86_64-unknown-linux-gnu` for host-side Rust test, lint, and dev-build commands.
- Keep code aligned with local style: `snake_case` for Rust vars/functions, `CamelCase` for Rust types, and `data-testid` for WebUI tests.
- Avoid `unwrap()` and `expect()` in production code.
- Keep `unsafe` usage minimal, justified, and documented.
- Use `tracing` instead of `println!` in production Rust code.

## Execution Checklist

Before starting a task:
1. Identify which memories apply.
2. Load them explicitly.
3. Tell the user which guidance is shaping the work.
4. Follow the repo workflow without skipping required checks.

Before finishing a task:
1. Verify the changed files.
2. Run the required lint and test commands.
3. Review the result against `quality-gates`.
4. Treat `AGENTS.md` as the source of truth for any shared-policy ambiguity.
