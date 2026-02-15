---
agent: 'agent'
tools: ['search/codebase', 'search', 'search/usages', 'terminal', 'read/problems', 'search/changes', 'web/fetch']
description: 'Debug issues in Rust code'
---

# Debug Issue

Your goal is to debug and resolve the specified issue.

## Debugging Process

1. **Understand the Problem**
   - What is the expected behavior?
   - What is the actual behavior?
   - When did it start occurring?

2. **Gather Information**
   - Check error messages and stack traces
   - Review recent changes
   - Check logs (tracing output)

3. **Reproduce the Issue**
   - Create minimal reproduction
   - Identify conditions that trigger it

4. **Analyze Root Cause**
   - Trace code execution path
   - Check for common issues (see below)

5. **Implement Fix**
   - Make minimal, focused changes
   - Add tests to prevent regression

## Common Rust Issues

### Compilation Errors
- Borrow checker violations
- Lifetime mismatches
- Type mismatches
- Missing trait implementations

### Runtime Errors
- Panics from `unwrap()`/`expect()`
- Stack overflow
- Memory allocation failures
- Deadlocks

### Async Issues
- Blocking in async context
- Missing `.await`
- Channel send/receive errors
- Timeout issues

### ONVIF-Specific
- XML serialization errors
- SOAP fault handling
- Authentication failures
- Profile/capability mismatches

## Debugging Commands

```bash
# Run tests with output
cargo test --target x86_64-unknown-linux-gnu -- --nocapture

# Check for warnings
cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings

# Run specific test
cargo test --target x86_64-unknown-linux-gnu test_name
```

Investigate the issue and provide a diagnosis with solution.
