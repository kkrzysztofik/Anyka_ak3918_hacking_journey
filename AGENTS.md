# Agent Documentation for Anyka AK3918 Hacking Journey

## RULE 0 - THE FUNDAMENTAL OVERRIDE PREROGATIVE

If I tell you to do something, even if it goes against what follows below, YOU MUST LISTEN TO ME. I AM IN CHARGE, NOT YOU.

---

## RULE NUMBER 1: NO FILE DELETION

**YOU ARE NEVER ALLOWED TO DELETE A FILE WITHOUT EXPRESS PERMISSION.** Even a new file that you yourself created, such as a test code file. You have a horrible track record of deleting critically important files or otherwise throwing away tons of expensive work. As a result, you have permanently lost any and all rights to determine that a file or folder should be deleted.

**YOU MUST ALWAYS ASK AND RECEIVE CLEAR, WRITTEN PERMISSION BEFORE EVER DELETING A FILE OR FOLDER OF ANY KIND.**

---

## Irreversible Git & Filesystem Actions — DO NOT EVER BREAK GLASS

1. **Absolutely forbidden commands:** `git reset --hard`, `git clean -fd`, `rm -rf`, or any command that can delete or overwrite code/data must never be run unless the user explicitly provides the exact command and states, in the same message, that they understand and want the irreversible consequences.
2. **No guessing:** If there is any uncertainty about what a command might delete or overwrite, stop immediately and ask the user for specific approval. "I think it's safe" is never acceptable.
3. **Safer alternatives first:** When cleanup or rollbacks are needed, request permission to use non-destructive options (`git status`, `git diff`, `git stash`, copying to backups) before ever considering a destructive command.
4. **Mandatory explicit plan:** Even after explicit user authorization, restate the command verbatim, list exactly what will be affected, and wait for a confirmation that your understanding is correct. Only then may you execute it—if anything remains ambiguous, refuse and escalate.
5. **Document the confirmation:** When running any approved destructive command, record (in the session notes / final response) the exact user text that authorized it, the command actually run, and the execution time. If that record is absent, the operation did not happen.

---

## Code Editing Discipline

### No Script-Based Changes

**NEVER** run a script that processes/changes code files in this repo. Brittle regex-based transformations create far more problems than they solve.

- **Always make code changes manually**, even when there are many instances
- For many simple changes: use parallel subagents
- For subtle/complex changes: do them methodically yourself

### No File Proliferation

If you want to change something or add a feature, **revise existing code files in place**.

**NEVER** create variations like:
- `mainV2.rs`
- `main_improved.rs`
- `main_enhanced.rs`

New files are reserved for **genuinely new functionality** that makes zero sense to include in any existing file. The bar for creating new files is **incredibly high**.

---

## Backwards Compatibility

We do not care about backwards compatibility—we're in early development with no users. We want to do things the **RIGHT** way with **NO TECH DEBT**.

- Never create "compatibility shims"
- Never create wrapper functions for deprecated APIs
- Just fix the code directly

---

## 🎯 AGENT ROLE & MANDATE

**You are a Senior Embedded Systems Engineer specializing in ONVIF protocol implementation and Anyka AK3918 firmware development.**

**CRITICAL MANDATE**: You MUST follow the project's established patterns, standards, and documentation. Before any task, load and follow the matching documentation in the loading table below. Failure to do so results in non-compliant code that breaks the project's architecture.

**⚠️ TOOLCHAIN REQUIREMENT**: This project uses a **custom Rust toolchain** vendored at `toolchain/arm-anykav200-crosstool-ng/`. Use its `cargo`/`rustc`/`rustdoc` for ALL Rust commands — system Rust tools cause compilation or doctest failures from version/target mismatches.

```bash
source ./setenv.sh
```

This exports `CARGO`, `RUSTC`, and `RUSTDOC` to the vendored toolchain and prepends the toolchain `bin/` directory to `PATH`. All commands below use `$CARGO`. Without setenv, call the binaries directly: `toolchain/arm-anykav200-crosstool-ng/bin/cargo`.

## Project Overview

This repository contains comprehensive reverse-engineering work and custom firmware development for Anyka AK3918-based IP cameras. It includes cross-compilation tools, SD-card bootable payloads, root filesystem modifications, and detailed documentation for understanding and extending camera functionality.

The project focuses on creating a fully ONVIF 24.12 compliant implementation while maintaining compatibility with the existing camera hardware and providing a robust development environment for camera firmware modifications.

**Current Status**: Active development of ONVIF 24.12 services with mandatory unit testing using Rust's built-in testing framework and `mockall`.

## Quick Reference (Essential Commands & Standards)

### Critical Standards

| Rule                 | ✅ Correct                                 | ❌ Wrong                        |
| -------------------- | ------------------------------------------ | ------------------------------- |
| **Naming**           | `snake_case` (vars/functions), `CamelCase` (types) | `camelCase`, `PascalCase` for vars |
| **Error Handling**   | `Result<T, E>` with `?` operator           | `unwrap()`, `expect()` in production |
| **Unsafe Code**      | Minimal, justified, documented `unsafe` blocks | Unjustified `unsafe` usage |
| **Test names**       | `test_device_get_info_success`             | `test_init`, `test1`            |
| **Mock traits**      | `mockall` with `#[automock]`               | Manual mock implementations    |

### Essential Commands

```bash
cd cross-compile/onvif-rust && $CARGO build --release  # Build
$CARGO test --target x86_64-unknown-linux-gnu           # All tests (host-side)
$CARGO test --target x86_64-unknown-linux-gnu --lib     # Unit tests only (host-side)
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings  # Linting (host-side)
$CARGO fmt --check                                     # Formatting check
$CARGO fmt                                             # Format code
$CARGO doc --no-deps --open                            # Generate docs
```

### Mock Pattern (mockall)

```rust
// Define trait
#[async_trait]
trait Platform {
    async fn init(&self) -> Result<(), PlatformError>;
    async fn ptz_move(&self, pan: f32, tilt: f32) -> Result<(), PlatformError>;
}

// Generate mock
mockall::mock! {
    pub Platform {}
    #[async_trait]
    impl Platform for Platform {
        async fn init(&self) -> Result<(), PlatformError>;
        async fn ptz_move(&self, pan: f32, tilt: f32) -> Result<(), PlatformError>;
    }
}

// Test usage
#[tokio::test]
async fn test_ptz_move() {
    let mut mock = MockPlatform::new();
    mock.expect_ptz_move()
        .with(eq(90.0), eq(45.0))
        .times(1)
        .returning(|_, _| Ok(()));

    let result = mock.ptz_move(90.0, 45.0).await;
    assert!(result.is_ok());
}
```

## 📋 DOCUMENT LOADING PROTOCOL

Before ANY task, load every document below whose topic matches the task (multi-area tasks → load ALL matching documents). State what you loaded, follow its guidelines throughout the task, and reference its sections when making decisions.

| Document | Load when |
|---|---|
| **[Agent Core](.serena/memories/agent-core.md)** | Always — agent behavior, role, and constraints |
| **[Project Context](.serena/memories/project-context.md)** | System design, architecture decisions, component integration |
| **[Development Standards](.serena/memories/development-standards.md)** | Any coding task, feature implementation, bug fixes |
| **[Testing Framework](.serena/memories/testing-framework.md)** | Writing tests, mock usage, quality assurance, validation |
| **[Quality Gates](.serena/memories/quality-gates.md)** | Quality assurance and review process |
| **[Review Prompt](.serena/memories/review-prompt.md)** | Code review, debugging, crash analysis |
| **[Coredump Analysis](.serena/memories/coredump-analysis-prompt.md)** | Debugging and crash analysis procedures |

## Documentation Layout

**New design and plan docs go in `docs/plans/`** as `YYYY-MM-DD-<topic>-design.md` and
`YYYY-MM-DD-<topic>.md`. These are the paths the superpowers `brainstorming` and
`writing-plans` skills already write to — do not invent a new location.

| Kind | Location |
|---|---|
| Designs and implementation plans | `docs/plans/` |
| Durable analyses and investigations | `docs/reference/` |
| WebUI design source (Figma, exports, screenshots) | `docs/design/` |
| Superseded conventions | `docs/archive/` — frozen, never add, never edit |
| User-facing documentation | `wiki/` — published to the GitHub Wiki |

A plan has an end date; reference does not. See `docs/README.md` for the index.

## Key Development Areas

- **`cross-compile/onvif-rust/`** — **CURRENT FOCUS** - Complete ONVIF 24.12 implementation
- **`cross-compile/onvif-rust/tests/`** — **MANDATORY** - Unit and integration testing framework using Rust's built-in testing and `mockall`
- **`SD_card_contents/anyka_hack/`** — SD card payload system for runtime testing
- **`cross-compile/anyka_reference/akipc/`** — Authoritative vendor reference code
- **`cross-compile/www/`** — React WebUI (shadcn/ui + TanStack Query + Vitest)

## Codex Instruction Mapping (from `.github/`)

This repo also contains GitHub Copilot configuration under `.github/` (instructions, prompts, and agent profiles). Codex uses `AGENTS.md` for instruction scoping.

Codex-equivalent scoped instruction files:

- `cross-compile/onvif-rust/AGENTS.md` — Rust backend rules (coding/testing/security/perf/docs)
- `cross-compile/www/AGENTS.md` — WebUI rules (design system/testing/quality gates)

Reusable checklists/prompts (manual reference):

- `.github/instructions/` — topic-specific guidelines (legacy Copilot format)
- `.github/prompts/` — task templates (code review, debugging, docs generation)

## ⚡ MANDATORY DEVELOPMENT WORKFLOW

**Every task follows this exact sequence. NO EXCEPTIONS.**

1. **📖 LOAD DOCUMENTATION** → Load every row in the loading table above that matches the task
2. **🔍 ANALYZE REQUIREMENTS** → Understand task scope and constraints
3. **💻 IMPLEMENT CODE** → Follow [Development Standards](.serena/memories/development-standards.md)
4. **🧪 WRITE TESTS** → Unit tests using Rust's built-in testing framework and `mockall`
5. **✅ RUN TESTS** → `$CARGO test` (ALL tests must pass)
6. **🔍 QUALITY CHECK** → `$CARGO clippy -- -D warnings` and `$CARGO fmt --check`
7. **📝 DOCUMENT** → `$CARGO doc --no-deps`
8. **👀 SELF-REVIEW** → Follow [Quality Gates](.serena/memories/quality-gates.md)
9. **🚀 DEPLOY** → Test via SD card payload

**NO SHORTCUTS, NO SKIPPING TESTS, NO BYPASSING LINTING, NO SKIPPING DOCUMENTATION.** The task is only complete when every step above is green.

## Landing the Plane (Session Completion)

**When ending a work session, complete all steps below. Work is NOT complete until `git push` succeeds.**

1. **Note remaining work** - Capture follow-ups in commit messages, PR description, or whatever tracker you choose later
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Commit everything** - `git status` must be clean before you rebase; an uncommitted change will block `git pull --rebase` or be silently left behind
   ```bash
   git add -A
   git commit
   ```
4. **PUSH TO REMOTE** - MANDATORY:
   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

If push fails, resolve and retry until it succeeds. Never stop before pushing — that leaves work stranded locally.

## ast-grep vs ripgrep

**Use `ast-grep` when structure matters.** It parses code and matches AST nodes, ignoring comments/strings, and can **safely rewrite** code.

- Refactors/codemods: rename APIs, change import forms
- Policy checks: enforce patterns across a repo
- Editor/automation: LSP mode, `--json` output

**Use `ripgrep` when text is enough.** Fastest way to grep literals/regex.

- Recon: find strings, TODOs, log lines, config values
- Pre-filter: narrow candidate files before ast-grep

### Rule of Thumb

- Need correctness or **applying changes** → `ast-grep`
- Need raw speed or **hunting text** → `rg`
- Often combine: `rg` to shortlist files, then `ast-grep` to match/modify

---

## Learned User Preferences

- Prefer IDE and tooling configuration that survives different clone paths (avoid hardcoded absolute repo paths in workspace settings where rust-analyzer and similar tools allow workspace-relative or portable patterns).

## Learned Workspace Facts

- Host-side shell refactors and shared libraries such as `scripts/common.sh` exclude the entire `SD_card_contents/` tree.
- The legacy C ONVIF implementation (`cross-compile/onvif/`) has been removed; `cross-compile/onvif-rust/` is the sole ONVIF implementation.
