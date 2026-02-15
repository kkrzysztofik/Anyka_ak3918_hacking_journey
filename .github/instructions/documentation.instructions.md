---
applyTo: "**/*.rs,**/*.md"
description: "Documentation standards for Rust and Markdown files"
---

# Documentation Guidelines

## Rust Documentation

### Public API Documentation

All public items must have rustdoc comments (`///`):

```rust
/// Brief description of the function.
///
/// More detailed explanation if needed.
///
/// # Arguments
///
/// * `param1` - Description of first parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// When and why errors are returned
///
/// # Examples
///
/// ```
/// let result = function().await?;
/// ```
```

### Module Documentation

Use `//!` at top of modules for module-level docs.

### Documentation Rules

- First line is brief summary (one sentence)
- Use `# Panics` section if function can panic
- Use `# Safety` section for unsafe functions
- Examples should use `?` operator, not `unwrap()`
- Document error conditions clearly

## Markdown Documentation

### Structure

- Use descriptive headings
- Keep paragraphs focused
- Use code blocks with language specifiers
- Use tables for structured data

### File Organization

- README.md in each significant directory
- Keep documentation close to code
- Update docs when code changes

## Comments in Code

- Explain "why", not "what"
- Use comments for complex logic
- Mark TODOs with context: `// TODO(username): description`
- Use `// SAFETY:` before unsafe blocks
