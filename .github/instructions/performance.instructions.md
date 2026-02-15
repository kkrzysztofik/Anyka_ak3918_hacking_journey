---
applyTo: "**/*.rs"
description: "Performance optimization guidelines for embedded systems"
---

# Performance Guidelines

## Memory Constraints

This project targets embedded hardware with 24MB memory limit.

### Memory Optimization

- Prefer borrowing over cloning
- Use `&str` instead of `String` when possible
- Avoid unnecessary allocations
- Use `Cow<str>` for conditionally owned strings
- Consider `SmallVec` for small, fixed-size collections
- Release resources as early as possible

### Stack vs Heap

- Prefer stack allocation for small, fixed-size data
- Box large types to prevent stack overflow
- Use arenas for many small allocations

## CPU Efficiency

- Use iterators instead of index-based loops
- Avoid premature `collect()` - keep iterators lazy
- Use parallel iteration sparingly (Rayon)
- Profile before optimizing

## Async Performance

- Use `tokio::sync` primitives (not std::sync in async)
- Avoid blocking operations in async context
- Use channels for async communication
- Consider buffer sizes for channels

## Serialization

- Use `quick-xml` for efficient XML parsing
- Avoid unnecessary allocations during parsing
- Consider zero-copy deserialization where possible

## Networking

- Reuse connections where possible
- Set appropriate timeouts
- Use streaming for large payloads

## Profiling

- Use `perf` for CPU profiling
- Use `heaptrack` for memory profiling
- Benchmark critical paths with `criterion`
- Test on target hardware

## Build Optimization

- Use `--release` for production builds
- Enable LTO for size optimization
- Consider `opt-level = "s"` for size
- Strip debug symbols in release
