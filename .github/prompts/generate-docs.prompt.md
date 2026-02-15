---
agent: 'agent'
tools: ['search/codebase', 'edit/editFiles', 'search', 'search/usages', 'terminal', 'web/fetch']
description: 'Generate rustdoc documentation for Rust code'
---

# Generate Documentation

Your goal is to generate or improve documentation for the specified code.

## Rust Documentation Format

```rust
/// Brief one-line description.
///
/// Detailed explanation of functionality, behavior, and usage.
///
/// # Arguments
///
/// * `param_name` - Description of the parameter
///
/// # Returns
///
/// Description of the return value
///
/// # Errors
///
/// * `ErrorType::Variant` - When this error occurs
///
/// # Panics
///
/// Describe when the function panics (if applicable)
///
/// # Safety
///
/// Explain safety requirements (for unsafe functions)
///
/// # Examples
///
/// ```
/// use crate::module::function;
///
/// let result = function(arg)?;
/// assert_eq!(result, expected);
/// ```
```

## Documentation Rules

1. First line is a brief summary (imperative mood)
2. Document all public items
3. Use `# Errors` for Result-returning functions
4. Use `# Panics` if function can panic
5. Use `# Safety` for unsafe functions
6. Examples should use `?`, not `unwrap()`

## Module Documentation

Use `//!` at module top for module-level docs:

```rust
//! Module description
//!
//! This module provides...
```

Generate documentation following these patterns.
