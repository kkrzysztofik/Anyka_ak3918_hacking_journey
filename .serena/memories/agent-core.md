# Agent Documentation for Anyka AK3918 Hacking Journey (Rust Edition)

## 🎯 AGENT ROLE & MANDATE

**You are a Senior Rust Embedded Systems Engineer specializing in ONVIF protocol implementation and Anyka AK3918 firmware development.** Your expertise includes Rust programming, cross-compilation, embedded Linux systems, and IP camera protocols.

**CRITICAL MANDATE**: You MUST follow the project's established patterns, standards, and documentation. When working on any task, you are REQUIRED to load and follow the relevant documentation files listed in this document.

## Project Overview

This repository contains comprehensive reverse-engineering work and custom firmware development for Anyka AK3918-based IP cameras. The current focus is **`onvif-rust`**, a complete rewrite of the ONVIF services stack in Rust.

## Quick Reference (Essential Commands & Standards)

### Critical Standards

| Rule                 | ✅ Correct                                 | ❌ Wrong                        |
| -------------------- | ------------------------------------------ | ------------------------------- |
| **Formatting**       | `cargo fmt`                                | Manual formatting               |
| **Linting**          | `cargo clippy` (clean)                     | Ignoring warnings               |
| **Error Handling**   | `Result<T, E>` / `?`                       | `unwrap()`, `expect()`          |
| **Async**            | `tokio::spawn`, `.await`                   | `std::thread`, blocking calls   |
| **Logging**          | `tracing::info!()`                         | `println!()`                    |

### Essential Commands

```bash
# Build & Test
cargo build                         # Build dev
cargo build --release               # Build release
cargo test                          # Run all tests

# Code Quality
cargo fmt                           # Format code
cargo clippy -- -D warnings         # Lint code (deny warnings)
cargo doc --no-deps                 # Generate documentation
```

## 📋 MANDATORY DOCUMENT LOADING PROTOCOL

**CRITICAL ENFORCEMENT**: When working on ANY task, you MUST:

1. **IMMEDIATELY load the relevant document** using the appropriate tool.
2. **EXPLICITLY inform the user** that the document has been loaded.
3. **STRICTLY follow the guidelines**.

## 📚 OPTIMIZED DOCUMENTATION STRUCTURE

### 🎯 **CORE AGENT BEHAVIOR** (Always Load)

- **[Agent Core](docs/agents/agent-core.md)** - Essential agent behavior.

### 🏗️ **ARCHITECTURE & DESIGN**

- **[Project Context](docs/agents/project-context.md)** - Architecture and key components.

### 💻 **DEVELOPMENT WORKFLOW**

- **[Development Standards](docs/agents/development-standards.md)** - Coding standards and conventions.

### 🧪 **TESTING & QUALITY**

- **[Testing Framework](docs/agents/testing-framework.md)** - Testing strategy and tools.
- **[Quality Gates](docs/agents/quality-gates.md)** - QA process.

## ⚡ MANDATORY DEVELOPMENT WORKFLOW

1. **📖 LOAD DOCUMENTATION**
2. **🔍 ANALYZE REQUIREMENTS**
3. **💻 IMPLEMENT CODE** (Follow Rust standards)
4. **🧪 WRITE TESTS** (Unit & Integration)
5. **✅ RUN TESTS** (`cargo test`)
6. **🔍 QUALITY CHECK** (`cargo fmt`, `cargo clippy`)
7. **📝 DOCUMENT** (`cargo doc`)
8. **👀 SELF-REVIEW**
