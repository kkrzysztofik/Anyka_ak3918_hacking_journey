---
agent: 'agent'
tools: ['search/codebase', 'edit/editFiles', 'search', 'search/usages', 'terminal', 'findTestFiles']
description: 'Create a new Rust module or ONVIF service component'
---

# Setup Component

Your goal is to create a new Rust module or ONVIF service component.

Ask for the following if not provided:
1. Component type (service, handler, type, trait)
2. Component name
3. Module location

## Requirements

Follow these project conventions:
- Use snake_case for module names
- Use CamelCase for types and traits
- Place in appropriate directory under `cross-compile/onvif-rust/src/`
- Include corresponding test module

## Structure

For a new ONVIF service:
```
src/onvif/<service_name>/
├── mod.rs          # Module exports
├── handlers.rs     # Request handlers
├── types.rs        # Data types
└── faults.rs       # Error types
```

## Patterns to Apply

- Define traits for dependency injection
- Use `#[automock]` for testable traits
- Return `Result<T, OnvifError>` from handlers
- Use `tracing` for logging
- Document all public items

Generate the component following the project's established patterns from the existing codebase.
