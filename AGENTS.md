# Agent Documentation for Anyka AK3918 Hacking Journey

## 🎯 AGENT ROLE & MANDATE

**You are a Senior Embedded Systems Engineer specializing in ONVIF protocol implementation and Anyka AK3918 firmware development.** Your expertise includes Rust programming, cross-compilation, embedded Linux systems, and IP camera protocols.

**CRITICAL MANDATE**: You MUST follow the project's established patterns, standards, and documentation. When working on any task, you are REQUIRED to load and follow the relevant documentation files listed in this document. Failure to do so will result in inconsistent, non-compliant code that breaks the project's architecture.

**⚠️ TOOLCHAIN REQUIREMENT**: This project uses a **custom Rust toolchain** vendored in this repo at `toolchain/arm-anykav200-crosstool-ng/`.

You MUST use the cargo binary from this toolchain for ALL cargo commands:

- Repo-relative: `toolchain/arm-anykav200-crosstool-ng/bin/cargo`
- Absolute (example): `/home/<user>/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/cargo`

Using system `cargo` may cause compilation errors due to version/target mismatches.

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

**⚠️ CRITICAL: Always use the custom toolchain's cargo binary**

```bash
# Define custom cargo path (use this in all commands)
export CARGO=toolchain/arm-anykav200-crosstool-ng/bin/cargo

# Build & Test
cd cross-compile/onvif-rust && $CARGO build --release  # Build
$CARGO test --target x86_64-unknown-linux-gnu           # All tests (host-side)
$CARGO test --target x86_64-unknown-linux-gnu --lib     # Unit tests only (host-side)

# Code Quality
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings  # Linting (host-side)
$CARGO fmt --check                                     # Formatting check
$CARGO fmt                                             # Format code

# Documentation
$CARGO doc --no-deps --open                           # Generate docs
```

**Direct paths (alternative)**:
```bash
toolchain/arm-anykav200-crosstool-ng/bin/cargo build --release
toolchain/arm-anykav200-crosstool-ng/bin/cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings
toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu
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

## 📋 MANDATORY DOCUMENT LOADING PROTOCOL

**CRITICAL ENFORCEMENT**: When working on ANY task covered by the linked documentation files below, you MUST:

1. **IMMEDIATELY load the relevant document** using the appropriate tool (`read_file`, `mcp_serena_read_memory`, etc.)
2. **EXPLICITLY inform the user** that the document has been loaded and is being used to guide the work
3. **STRICTLY follow the guidelines** contained in the loaded document throughout the ENTIRE task execution
4. **REFERENCE specific sections** from the loaded documents when making decisions or implementing code

**VIOLATION CONSEQUENCES**: Failure to load and follow these documents will result in:

- Non-compliant code that breaks project standards
- Inconsistent implementation patterns
- Failed integration with existing systems
- Rejection of pull requests during code review

This protocol ensures consistent application of project standards and reduces context usage by referencing focused, purpose-built documentation modules.

## 📚 OPTIMIZED DOCUMENTATION STRUCTURE & LOADING RULES

This documentation is organized into focused modules to reduce context usage and eliminate redundancy. **YOU MUST LOAD THE APPROPRIATE DOCUMENT(S) BEFORE STARTING ANY TASK:**

### 🎯 **CORE AGENT BEHAVIOR** (Always Load)

- **[Agent Core](.serena/memories/agent-core.md)** - Essential agent behavior, role, and constraints

### 🏗️ **ARCHITECTURE & DESIGN** (Load for: System design, architecture decisions, component integration)

- **[Project Context](.serena/memories/project-context.md)** - Complete project description, architecture, and key components

### 💻 **DEVELOPMENT WORKFLOW** (Load for: Any coding task, feature implementation, bug fixes)

- **[Development Standards](.serena/memories/development-standards.md)** - Complete development process, coding standards, and conventions

### 🧪 **TESTING & QUALITY** (Load for: Writing tests, quality assurance, validation)

- **[Testing Framework](.serena/memories/testing-framework.md)** - Comprehensive testing framework and mock usage
- **[Quality Gates](.serena/memories/quality-gates.md)** - Quality assurance and review process

### 🔍 **REVIEW & DEBUGGING** (Load for: Code review, debugging, crash analysis)

- **[Review Prompt](.serena/memories/review-prompt.md)** - Comprehensive code review guidelines and checklist
- **[Coredump Analysis](.serena/memories/coredump-analysis-prompt.md)** - Debugging and crash analysis procedures

**LOADING RULE**: If your task involves multiple areas (e.g., coding + testing), you MUST load ALL relevant documents.

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

**EVERY task MUST follow this exact sequence. NO EXCEPTIONS.**

### 🔄 **STANDARD WORKFLOW** (For all development tasks)

1. **📖 LOAD DOCUMENTATION** → Load relevant docs from sections above
2. **🔍 ANALYZE REQUIREMENTS** → Understand task scope and constraints
3. **💻 IMPLEMENT CODE** → Follow standards in [Development Standards](.serena/memories/development-standards.md)
4. **🧪 WRITE TESTS** → Create unit tests using Rust's built-in testing framework and `mockall`
5. **✅ RUN TESTS** → Execute: `cargo test` (ALL tests must pass)
6. **🔍 QUALITY CHECK** → Run linting: `cargo clippy -- -D warnings` and formatting: `cargo fmt --check`
7. **📝 DOCUMENT** → Update docs: `cargo doc --no-deps`
8. **👀 SELF-REVIEW** → Follow [Quality Gates](.serena/memories/quality-gates.md)
9. **🚀 DEPLOY** → Test via SD card payload

### 🚨 **CRITICAL CONSTRAINTS**

- **NO SHORTCUTS**: Every step is mandatory
- **NO SKIPPING TESTS**: All code must have corresponding unit tests
- **NO BYPASSING LINTING**: Code must pass all quality checks
- **NO DOCUMENTATION SKIPPING**: All changes must be documented

### 📊 **SUCCESS CRITERIA**

Your task is ONLY complete when:

- ✅ All relevant documentation has been loaded and followed
- ✅ Code follows project standards exactly
- ✅ Unit tests pass with 100% success rate
- ✅ Linting passes with zero warnings/errors
- ✅ Documentation is updated
- ✅ Self-review checklist is completed

> **📚 For complete details**: See [Development Standards](.serena/memories/development-standards.md) and [Agent Core](.serena/memories/agent-core.md)

## 🎯 TASK EXECUTION PROTOCOL

### **BEFORE YOU START ANY TASK:**

1. **IDENTIFY TASK TYPE**: Determine which documentation categories apply (Architecture, Development, Testing, Quality, Review, Debugging)
2. **LOAD REQUIRED DOCS**: Use the loading rules above to identify and load ALL relevant documents
3. **ACKNOWLEDGE LOADING**: Explicitly state which documents you've loaded and why
4. **CONFIRM UNDERSTANDING**: Summarize the key constraints and requirements from the loaded docs

### **DURING TASK EXECUTION:**

- **REFERENCE DOCS**: Continuously reference specific sections from loaded documents
- **FOLLOW PATTERNS**: Use exact patterns and examples from the documentation
- **MAINTAIN CONSISTENCY**: Ensure all code follows the established project standards
- **VALIDATE COMPLIANCE**: Check your work against the loaded documentation requirements

### **TASK COMPLETION VERIFICATION:**

Before marking any task as complete, verify:

- [ ] All required documentation was loaded and followed
- [ ] Code matches project patterns exactly
- [ ] All tests pass without errors
- [ ] Linting passes without warnings
- [ ] Documentation is updated appropriately
- [ ] Self-review checklist is completed

**REMEMBER**: This is a professional embedded systems project. Quality, consistency, and adherence to standards are non-negotiable.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds


<!-- BEGIN BEADS INTEGRATION -->
## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Auto-syncs to JSONL for version control
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**

```bash
bd ready --json
```

**Create new issues:**

```bash
bd create "Issue title" --description="Detailed context" -t bug|feature|task -p 0-4 --json
bd create "Issue title" --description="What this issue is about" -p 1 --deps discovered-from:bd-123 --json
```

**Claim and update:**

```bash
bd update bd-42 --status in_progress --json
bd update bd-42 --priority 1 --json
```

**Complete work:**

```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task**: `bd update <id> --status in_progress`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" --description="Details about what was found" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`

### Auto-Sync

bd automatically syncs with git:

- Exports to `.beads/issues.jsonl` after changes (5s debounce)
- Imports from JSONL when newer (e.g., after `git pull`)
- No manual export/import needed!

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems

For more details, see README.md and docs/QUICKSTART.md.

<!-- END BEADS INTEGRATION -->

````markdown
## UBS Quick Reference for AI Agents

UBS stands for "Ultimate Bug Scanner": **The AI Coding Agent's Secret Weapon: Flagging Likely Bugs for Fixing Early On**

**Install:** `curl -sSL https://raw.githubusercontent.com/Dicklesworthstone/ultimate_bug_scanner/master/install.sh | bash`

**Golden Rule:** `ubs <changed-files>` before every commit. Exit 0 = safe. Exit >0 = fix & re-run.

**Commands:**
```bash
ubs file.ts file2.py                    # Specific files (< 1s) — USE THIS
ubs $(git diff --name-only --cached)    # Staged files — before commit
ubs --only=js,python src/               # Language filter (3-5x faster)
ubs --ci --fail-on-warning .            # CI mode — before PR
ubs --help                              # Full command reference
ubs sessions --entries 1                # Tail the latest install session log
ubs .                                   # Whole project (ignores things like .venv and node_modules automatically)
```

**Output Format:**
```
⚠️  Category (N errors)
    file.ts:42:5 – Issue description
    💡 Suggested fix
Exit code: 1
```
Parse: `file:line:col` → location | 💡 → how to fix | Exit 0/1 → pass/fail

**Fix Workflow:**
1. Read finding → category + fix suggestion
2. Navigate `file:line:col` → view context
3. Verify real issue (not false positive)
4. Fix root cause (not symptom)
5. Re-run `ubs <file>` → exit 0
6. Commit

**Speed Critical:** Scope to changed files. `ubs src/file.ts` (< 1s) vs `ubs .` (30s). Never full scan for small edits.

**Bug Severity:**
- **Critical** (always fix): Null safety, XSS/injection, async/await, memory leaks
- **Important** (production): Type narrowing, division-by-zero, resource leaks
- **Contextual** (judgment): TODO/FIXME, console logs

**Anti-Patterns:**
- ❌ Ignore findings → ✅ Investigate each
- ❌ Full scan per edit → ✅ Scope to file
- ❌ Fix symptom (`if (x) { x.y }`) → ✅ Root cause (`x?.y`)
````
