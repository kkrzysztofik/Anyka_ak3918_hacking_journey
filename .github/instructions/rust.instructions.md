---
applyTo: "**/*.rs"
description: "Rust coding conventions and best practices for embedded ONVIF implementation"
---

# Rust Development Guidelines

Follow idiomatic Rust practices and the project's established conventions.

## General Principles

- Prioritize readability, safety, and maintainability
- Use strong typing and leverage Rust's ownership system
- Break complex functions into smaller, manageable pieces
- Ensure code compiles without warnings

## Naming Conventions (RFC 430)

| Element | Convention | Example |
|---------|------------|---------|
| Variables/Functions | snake_case | `device_info`, `get_profile()` |
| Types/Structs/Enums | CamelCase | `DeviceService`, `StreamType` |
| Constants | SCREAMING_SNAKE | `MAX_CONNECTIONS` |
| Modules | snake_case | `device_service.rs` |

## Ownership and Borrowing

- Prefer borrowing (`&T`) over cloning unless ownership transfer is needed
- Use `&mut T` when modification is required
- Explicitly annotate lifetimes when compiler cannot infer
- Use `Arc<T>` for thread-safe reference counting
- Use `Mutex<T>` or `RwLock<T>` for multi-threaded interior mutability

## Error Handling

- Use `Result<T, E>` for recoverable errors
- Prefer `?` operator over `unwrap()` or `expect()`
- Create custom errors using `thiserror` for libraries
- Use `anyhow` with context for applications
- Provide meaningful error messages

## Async Patterns

- Use `tokio` runtime for async code
- Use `tokio::sync` primitives (never `std::sync` in async)
- Structure async code with `async/await`
- Use channels for async communication

## Patterns to Follow

- Use modules and `pub` to encapsulate logic
- Implement traits to abstract dependencies
- Prefer enums over flags for type safety
- Use builders for complex object creation
- Use iterators instead of index-based loops
- Prefer `&str` over `String` for function parameters

## Patterns to Avoid

- Don't use `unwrap()` or `expect()` in production
- Avoid panics in library code
- Don't rely on global mutable state
- Avoid deeply nested logic
- Don't ignore warnings
- Avoid `unsafe` unless required and documented
- Don't overuse `clone()`

## Code Style

- Use `rustfmt` for formatting
- Keep lines under 100 characters
- Use `cargo clippy` for linting
- Document with `///` before items

## Testing

- Write unit tests in `#[cfg(test)]` modules
- Use `mockall` with `#[automock]` for mocking
- Name tests: `test_<function>_<scenario>_<outcome>`
- Document error conditions and panic scenarios
