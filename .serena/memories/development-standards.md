# Development Standards - Anyka AK3918 Project (Rust)

## Rust Code Standards

### Formatting & Linting (MANDATORY)

- **Formatting**: All code MUST be formatted using `rustfmt`.
  - Run: `cargo fmt`
- **Linting**: All code MUST pass `clippy` with no warnings.
  - Run: `cargo clippy -- -D warnings`

### Naming Conventions

- **Variables/Functions**: `snake_case`
- **Types/Traits**: `CamelCase`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Files**: `snake_case.rs`

### Error Handling

- **Application Code**: Use `anyhow::Result` for easy error propagation.
- **Library/Service Code**: Use `thiserror` to define custom, strongly-typed error enums.
- **Panic**: AVOID panicking (`unwrap()`, `expect()`) in production code. Use `?` operator or handle errors gracefully.

### Async/Await

- Use `tokio` as the async runtime.
- Prefer `.await` over blocking calls.
- Use `tokio::sync` primitives (Mutex, RwLock, channels) instead of `std::sync` in async contexts.

### Logging

- Use the `tracing` crate for all logging.
- Levels:
  - `error!`: Critical failures requiring immediate attention.
  - `warn!`: Unexpected but handled situations.
  - `info!`: High-level operational events (startup, shutdown, configuration changes).
  - `debug!`: Detailed flow information for debugging.
  - `trace!`: Extremely verbose low-level details.

### Dependencies

- Prefer standard, well-maintained crates (`tokio`, `serde`, `axum`, `uuid`, `chrono`).
- Avoid adding heavy dependencies for simple tasks.
- Keep `Cargo.toml` organized and sorted.

## Code Organization

- **Services**: Each ONVIF service (Device, Media, etc.) should be in its own module under `src/services/`.
- **Models**: XML serialization models should be separated from logic.
- **Utils**: Shared utilities go in `src/utils/`.
