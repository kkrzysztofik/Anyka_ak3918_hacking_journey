---
agent: 'agent'
tools: ['search/codebase', 'search', 'search/usages', 'search/changes', 'read/problems', 'web/githubRepo', 'terminal', 'findTestFiles']
description: 'Review code against project standards and best practices'
---

# Code Review

Your goal is to review the specified code against project standards.

## Mandatory Pre-Review (Run First)

```bash
cd cross-compile/onvif-rust
cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings
cargo build --release
cargo test --target x86_64-unknown-linux-gnu
cargo fmt --check
```

## Review Checklist

### Critical Standards (REQUIRED)
- [ ] **Naming**: snake_case for vars/functions, CamelCase for types, SCREAMING_SNAKE for constants
- [ ] **Error Handling**: NO `unwrap()` or `expect()` in production - use `Result<T, E>` with `?`
- [ ] **Unsafe Code**: `unsafe` blocks minimal, justified, and have `// SAFETY:` comments
- [ ] **Module Organization**: Code properly organized, no circular dependencies
- [ ] **Test Coverage**: All new functionality has corresponding tests
- [ ] **Documentation**: Public APIs have doc comments (`///`) with examples

### Security (REQUIRED)
- [ ] Input validation on all user inputs
- [ ] No unnecessary panics in production paths
- [ ] Proper `Result` usage, no error swallowing
- [ ] Authentication gaps addressed (WS-Security, HTTP Digest)
- [ ] XML security (XXE protection, XML bomb prevention)
- [ ] Memory safety (proper ownership, no data races)

### Performance
- [ ] Memory-efficient (24MB target constraint)
- [ ] Prefers borrowing over cloning
- [ ] No blocking in async context
- [ ] Appropriate buffer sizes

### Testing
- [ ] Tests follow naming: `test_<function>_<scenario>_<outcome>`
- [ ] Mocks use `mockall` properly with `#[automock]`
- [ ] Both success and error paths tested
- [ ] Edge cases covered
- [ ] WebUI uses `data-testid` selectors

## Output Format

### Executive Summary (Required)
```markdown
## ONVIF Code Review Summary

**Build Status**: [✅ Success / ❌ Failed]
**Critical Issues**: [X] found
**Security Vulnerabilities**: [X] high, [X] medium
**Standards Compliance**: [✅ Compliant / ❌ X violations]

**Recommendation**: [APPROVE / REJECT / CONDITIONAL APPROVAL]
```

### For Each Critical Issue
```markdown
## 🚨 **CRITICAL ISSUE**: [Brief Description]

**File**: `path/to/file.rs:line`
**Severity**: [Critical/High]
**Rule Violated**: [Specific standard]
**Impact**: [Security/Functionality/Compliance impact]

**Current Code**:
```rust
[Code snippet]
```

**Required Fix**:
```rust
[Corrected code]
```
```

### Standards Compliance Table
| Standard | Status | Violations |
|----------|--------|------------|
| Naming Conventions | [✅/❌] | [X] |
| Error Handling | [✅/❌] | [X] |
| Unsafe Code Usage | [✅/❌] | [X] |
| Test Coverage | [✅/❌] | [X] |
| Documentation | [✅/❌] | [X] |

Rate severity: 🔴 Critical | 🟡 Warning | 🟢 Info

## Response Limits

- **Total Response**: 2,000-3,000 words maximum
- **Focus on**: Critical issues that block merge
- **Ignore**: Minor style issues (handled by rustfmt)
