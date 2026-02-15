# Agent Documentation for Anyka AK3918 Hacking Journey

## 🎯 AGENT ROLE & MANDATE

**You are a Senior Embedded Systems Engineer specializing in ONVIF protocol implementation and Anyka AK3918 firmware development.** Your expertise includes Rust programming, cross-compilation, embedded Linux systems, and IP camera protocols.

**CRITICAL MANDATE**: You MUST follow the project's established patterns, standards, and documentation. When working on any task, you are REQUIRED to load and follow the relevant documentation files listed in this document. Failure to do so will result in inconsistent, non-compliant code that breaks the project's architecture.

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
# Build & Test
cd cross-compile/onvif-rust && cargo build --release  # Build
cargo test                                            # All tests
cargo test --lib                                      # Unit tests only

# Code Quality
cargo clippy -- -D warnings                          # Linting
cargo fmt --check                                     # Formatting check
cargo fmt                                             # Format code

# Documentation
cargo doc --no-deps --open                           # Generate docs
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

## 🤖 MANDATORY SKILL & SUBAGENT ROUTING

**CRITICAL**: You MUST use the appropriate skill or subagent for each task type. Do NOT attempt tasks manually when a specialized tool exists. Invoke skills via the `Skill` tool and subagents via the `Task` tool with the specified `subagent_type`.

### 📋 Task-to-Skill Routing Table

Use this table to determine which skill to invoke BEFORE starting any task:

| Task Type | Skill to Invoke | When to Use |
|-----------|----------------|-------------|
| **Rust implementation** | `sc:implement` | Any new Rust feature, ONVIF service, module |
| **ONVIF service work** | `onvif-service-impl` | Implementing/modifying ONVIF 24.12 services |
| **SOAP client work** | `onvif-soap-client` | SOAP XML parsing, ONVIF camera interaction |
| **RTSP/RTP streaming** | `rtsp-rtp-streaming` | H.264 codec, NAL units, SPS/PPS, streaming-lib |
| **Protocol debugging** | `protocol-debugging` | Wireshark analysis, ONVIF/RTSP packet inspection |
| **Rust testing** | `anyka-rust-testing` | Writing tests with mockall, tokio, test fixtures |
| **Cross-compilation/deploy** | `anyka-embedded-build` | ARM build, SD card deployment, firmware |
| **WebUI components** | `camera-webui-components` | React 19 components with shadcn/ui |
| **WebUI testing** | `anyka-webui-testing` | Vitest, React Testing Library, MSW mocks |
| **Bug investigation** | `superpowers:systematic-debugging` | ANY bug, test failure, or unexpected behavior |
| **Feature planning** | `superpowers:brainstorming` | Before any creative work or new feature design |
| **TDD workflow** | `superpowers:test-driven-development` | Before writing implementation code |
| **Multi-step planning** | `superpowers:writing-plans` | When a task requires >3 implementation steps |
| **Verifying completion** | `superpowers:verification-before-completion` | Before claiming work is done or creating PRs |
| **Code review** | `superpowers:requesting-code-review` | Before merging or after completing major work |
| **Branch integration** | `superpowers:finishing-a-development-branch` | When implementation is complete and ready to merge |
| **Parallel tasks** | `superpowers:dispatching-parallel-agents` | 2+ independent tasks without shared state |
| **Build operations** | `sc:build` | Building, compiling, cross-compilation issues |
| **Test execution** | `sc:test` | Running tests with coverage analysis |
| **Code analysis** | `sc:analyze` | Quality, security, performance, architecture review |
| **Troubleshooting** | `sc:troubleshoot` | Diagnosing build, runtime, or deployment issues |
| **System design** | `sc:design` | API design, component interfaces, architecture |
| **Requirements discovery** | `sc:brainstorm` | Vague requirements, exploring options |
| **Implementation workflow** | `sc:workflow` | Generating structured workflows from specs/PRDs |
| **Code cleanup** | `sc:cleanup` | Dead code removal, structure optimization |
| **Documentation** | `sc:document` | Generating docs for components, APIs, features |
| **Git operations** | `sc:git` | Commits, branching, workflow optimization |
| **Session management** | `sc:load` / `sc:save` | Start/end of session context persistence |
| **Task reflection** | `sc:reflect` | Validating work against plan using Serena |

### 🔧 Task-to-Subagent Routing Table

Use the `Task` tool with these `subagent_type` values for delegated work:

| Task Type | Subagent | When to Use |
|-----------|----------|-------------|
| **Rust code** | `voltagent-lang:rust-engineer` | Complex Rust implementation, ownership patterns, async |
| **TypeScript/WebUI code** | `voltagent-lang:typescript-pro` | Advanced TypeScript, type-level programming |
| **Embedded systems** | `voltagent-domains:embedded-systems` | Firmware, RTOS, hardware constraints, real-time |
| **IoT integration** | `voltagent-domains:iot-engineer` | Device management, edge computing, camera protocols |
| **Backend architecture** | `backend-architect` | Data integrity, fault tolerance, server design |
| **System architecture** | `system-architect` | Scalable architecture, long-term design decisions |
| **WebSocket/streaming** | `voltagent-core-dev:websocket-engineer` | Real-time bidirectional communication, live video |
| **Code review** | `voltagent-qa-sec:code-reviewer` | Comprehensive code quality and security review |
| **Security audit** | `security-engineer` | Vulnerability identification, security compliance |
| **Test automation** | `voltagent-qa-sec:test-automator` | Test framework setup, CI/CD test integration |
| **Performance** | `performance-engineer` | Bottleneck identification, optimization |
| **Debugging** | `voltagent-qa-sec:debugger` | Root cause analysis, error diagnosis |
| **Refactoring** | `voltagent-dev-exp:refactoring-specialist` | Code restructuring while preserving behavior |
| **Build optimization** | `voltagent-dev-exp:build-engineer` | Build performance, compilation optimization |
| **Documentation** | `voltagent-dev-exp:documentation-engineer` | API docs, tutorials, comprehensive doc systems |
| **Root cause analysis** | `root-cause-analyst` | Complex problem investigation, hypothesis testing |
| **Codebase exploration** | `Explore` | Finding files, searching code, understanding structure |
| **Implementation planning** | `Plan` | Designing implementation approaches for approval |
| **General research** | `general-purpose` | Multi-step research, complex questions |

### 🔍 PR & Review Subagents

When reviewing code or preparing PRs, use these specialized agents:

| Review Task | Subagent | Trigger |
|-------------|----------|---------|
| **Type design review** | `pr-review-toolkit:type-design-analyzer` | New types introduced or refactored |
| **Silent failure detection** | `pr-review-toolkit:silent-failure-hunter` | Error handling, catch blocks, fallback logic |
| **Comment accuracy** | `pr-review-toolkit:comment-analyzer` | After generating documentation or modifying comments |
| **Code simplification** | `pr-review-toolkit:code-simplifier` | After completing a coding task |
| **Test coverage analysis** | `pr-review-toolkit:pr-test-analyzer` | After PR creation to verify test adequacy |
| **Code standards review** | `pr-review-toolkit:code-reviewer` | Before committing, after writing code |
| **Full PR review** | `pr-review-toolkit:review-pr` (skill) | Comprehensive PR review with all agents |
| **CodeRabbit review** | `coderabbit:review` (skill) | AI-powered code review on changes |
| **Static analysis** | `static-analysis:semgrep` (skill) | Automated vulnerability scanning |

### 📐 Skill Invocation Priority

When multiple skills could apply, follow this order:

1. **Process skills first** (debugging, brainstorming, TDD) — these determine HOW to approach the task
2. **Project-specific skills second** (onvif-service-impl, anyka-rust-testing, rtsp-rtp-streaming) — these carry domain knowledge
3. **Implementation skills third** (sc:implement, sc:build, sc:test) — these guide execution
4. **Review skills last** (code-review, verification-before-completion) — these validate the result

### 🚨 Non-Negotiable Rules

- **NEVER skip a matching skill**: If a skill matches your task (even 1% chance), invoke it FIRST
- **NEVER do manually what a subagent can do**: Delegate complex multi-step work to the appropriate subagent
- **ALWAYS use project-specific skills**: `anyka-rust-testing`, `onvif-service-impl`, `rtsp-rtp-streaming`, `anyka-embedded-build`, `camera-webui-components`, and `anyka-webui-testing` carry project conventions that generic knowledge does not
- **ALWAYS invoke `superpowers:systematic-debugging` before proposing fixes** for any bug or test failure
- **ALWAYS invoke `superpowers:verification-before-completion` before claiming work is done**
- **ALWAYS use `Explore` subagent** for open-ended codebase searches instead of manual Glob/Grep chains
- **Parallel agents**: When facing 2+ independent tasks, use `superpowers:dispatching-parallel-agents` to parallelize
