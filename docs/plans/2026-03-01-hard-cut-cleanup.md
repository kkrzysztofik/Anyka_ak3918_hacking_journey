# Hard-Cut Cleanup Plan: ONVIF + Dispatcher + Security

**Date**: 2026-03-01
**Branch**: hard-cut-cleanup
**Scope**: ONVIF services, dispatcher, security modules
**Strategy**: Hard-cut (no compatibility layers, no legacy retention)

---

## 1. Goals

1. **Remove duplicate WS-Security implementations** - Consolidate `src/auth/ws_security.rs` and `src/onvif/ws_security.rs` into a single canonical implementation
2. **Remove unused `src/auth/` module** - HTTP Basic, HTTP Digest, and credentials modules are not used by ONVIF services
3. **Remove roadmap stub handlers** - No-op success paths that return `Ok(Response{})` without actual implementation
4. **Remove dead code** - Functions marked with `#[allow(dead_code)]` that have no callers
5. **Clean up conditional stub code** - Reduce `#[cfg(use_stubs)]` conditional compilation complexity
6. **Maintain ONVIF compliance** - Keep all handlers that return proper ONVIF-compliant responses

---

## 2. Non-Goals

1. **No new features** - This is purely a cleanup operation
2. **No API changes** - External interfaces remain unchanged
3. **No test removal** - All existing tests must pass
4. **No HAL refactoring** - HAL stubs are needed for host-side testing
5. **No streaming-lib changes** - External dependency, out of scope

---

## 3. Breaking Changes

### 3.1 Module Removals

| Module | Impact | Migration |
|--------|--------|-----------|
| `src/auth/` | No external callers | None needed |
| `src/auth/ws_security.rs` | Unused duplicate | Use `src/onvif/ws_security.rs` |
| `src/auth/http_basic.rs` | Unused | None |
| `src/auth/http_digest.rs` | Unused | None |
| `src/auth/credentials.rs` | Unused | None |

### 3.2 Handler Changes

| Handler | Current Behavior | New Behavior |
|---------|------------------|--------------|
| `handle_set_system_factory_default` | Returns `Ok(())` stub | Return `Err(ActionNotSupported)` |
| `handle_create_certificate` | Returns `ActionNotSupported` | No change (already correct) |
| `handle_load_certificates` | Returns `ActionNotSupported` | No change (already correct) |
| `handle_delete_certificates` | Returns `ActionNotSupported` | No change (already correct) |
| Focus move/stop handlers | Returns `ActionNotSupported` | No change (already correct) |
| Imaging preset handlers | Returns `ActionNotSupported` | No change (already correct) |

### 3.3 Dead Code Removal

Functions currently marked `#[allow(dead_code)]` that will be removed if unused:

- `src/onvif/device/handlers.rs:72` - unused field in struct
- `src/onvif/ws_security.rs:150` - unused const
- `src/onvif/media/handlers.rs:72,75` - unused fields
- `src/onvif/imaging/handlers.rs:40` - unused field
- `src/onvif/ptz/handlers.rs:55,203,219` - unused fields
- `src/onvif/ptz/validation.rs:76` - unused validation function
- `src/hal/ptz_driver.rs:51,53,57` - kernel ABI reserved ioctls (KEEP)
- `src/hal/vendor_ipc.rs:214,1043-1077` - SDK struct fields (KEEP)
- `src/hal/ptz.rs:182,247` - HAL internals
- `src/hal/video.rs:43` - HAL internals
- `src/hal/audio.rs:38` - HAL internals
- `src/hal/shm_ring.rs:881` - HAL internals

---

## 4. Phased Task List

### Phase 1: Module Removal (Low Risk)

**Task 1.1: Remove unused `src/auth/` module**

Files to delete:
```
cross-compile/onvif-rust/src/auth/mod.rs
cross-compile/onvif-rust/src/auth/ws_security.rs
cross-compile/onvif-rust/src/auth/http_basic.rs
cross-compile/onvif-rust/src/auth/http_digest.rs
cross-compile/onvif-rust/src/auth/credentials.rs
```

Changes required:
- `src/lib.rs:71` - Remove `pub mod auth;`

Verification:
```bash
cd /home/kmk/anyka-dev/.worktrees/hard-cut-cleanup/cross-compile/onvif-rust
/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/cargo check --target x86_64-unknown-linux-gnu
```

---

### Phase 2: Stub Handler Cleanup (Medium Risk)

**Task 2.1: Convert `handle_set_system_factory_default` from stub to proper error**

File: `cross-compile/onvif-rust/src/onvif/device/handlers.rs`

Change (lines 414-433):
```rust
// BEFORE (stub - returns Ok with warning)
pub fn handle_set_system_factory_default(
    &self,
    request: SetSystemFactoryDefault,
) -> OnvifResult<SetSystemFactoryDefaultResponse> {
    tracing::warn!("Factory default reset requested but not implemented (stub)");
    Ok(SetSystemFactoryDefaultResponse {})
}

// AFTER (proper ONVIF error)
pub fn handle_set_system_factory_default(
    &self,
    request: SetSystemFactoryDefault,
) -> OnvifResult<SetSystemFactoryDefaultResponse> {
    tracing::warn!("Factory default reset requested - not supported");
    Err(OnvifError::ActionNotSupported(
        "SetSystemFactoryDefault".to_string(),
    ))
}
```

**Task 2.2: Remove multicast stub handlers (keep ONVIF-compliant behavior)**

File: `cross-compile/onvif-rust/src/onvif/media/handlers.rs`

Current behavior (lines 981-1006) is ONVIF-compliant - multicast operations return success without actual multicast. This is acceptable per ONVIF spec. **No changes needed.**

**Task 2.3: Clean up TODO comments**

Files with TODO comments to review:
- `src/onvif/device/handlers.rs:793,866,990` - Platform integration TODOs (keep for future work)
- `src/onvif/imaging/handlers.rs:299,330` - Focus TODOs (keep - hardware limitation documented)
- `src/onvif/ptz/state.rs:370,378` - Preset persistence TODOs (keep for future work)
- `src/onvif/ptz/handlers.rs:316` - Configuration persistence TODO (keep for future work)

Decision: Convert to `tracing::debug!` statements for runtime visibility instead of code comments.

---

### Phase 3: Dead Code Removal (Low Risk)

**Task 3.1: Remove unused struct fields in handlers**

Files to modify:
- `src/onvif/device/handlers.rs:72` - Remove `#[allow(dead_code)]` from `platform` field if unused
- `src/onvif/media/handlers.rs:72,75` - Remove unused fields
- `src/onvif/imaging/handlers.rs:40` - Remove unused field
- `src/onvif/ptz/handlers.rs:55,203,219` - Remove unused fields

**Task 3.2: Remove unused validation function**

File: `src/onvif/ptz/validation.rs:76`
- Function marked `#[allow(dead_code)]` with comment "Kept for potential future use"
- Decision: Remove if not used in tests, keep if tested

**Task 3.3: Keep HAL dead_code annotations**

Files to NOT modify:
- `src/hal/ptz_driver.rs` - Kernel ABI reserved ioctls
- `src/hal/vendor_ipc.rs` - SDK struct fields matching C ABI
- `src/hal/ptz.rs`, `src/hal/video.rs`, `src/hal/audio.rs` - HAL internals may be used by future platform implementations
- `src/hal/shm_ring.rs` - Shared memory ring buffer internals

---

### Phase 4: Conditional Code Cleanup (Medium Risk)

**Task 4.1: Audit `#[cfg(use_stubs)]` blocks**

Current count: 48 occurrences

Files with conditional stub code:
- `src/hal/vendor_ipc.rs` - Multiple `#[cfg(use_stubs)]` blocks
- `src/hal/imaging.rs` - 6 occurrences
- `src/hal/ptz.rs` - 8 occurrences
- `src/hal/mod.rs` - stub module definitions

Decision: Keep all HAL stub code - required for host-side testing. No changes.

---

### Phase 5: Documentation Updates (Low Risk)

**Task 5.1: Update module documentation**

Files to update:
- `src/lib.rs` - Remove `auth` module from documentation
- `src/security/mod.rs` - Update to clarify this is the canonical security module

**Task 5.2: Update AGENTS.md if needed**

Check if `cross-compile/onvif-rust/AGENTS.md` references removed modules.

---

## 5. Verification Commands

All commands use the vendored cargo:

```bash
# Set cargo path
export CARGO=/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/cargo
cd /home/kmk/anyka-dev/.worktrees/hard-cut-cleanup/cross-compile/onvif-rust

# After each phase:
$cargo check --target x86_64-unknown-linux-gnu
$cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings
$cargo test --target x86_64-unknown-linux-gnu
$cargo fmt --check

# Final verification:
$cargo build --release
$cargo doc --no-deps
```

---

## 6. Rollback Plan

If issues arise:

1. **Phase 1 rollback**: Re-add `src/auth/` module and `pub mod auth;` in lib.rs
2. **Phase 2 rollback**: Revert handler changes via git
3. **Phase 3 rollback**: Re-add removed dead code fields/functions
4. **Phase 4 rollback**: N/A (no changes planned)
5. **Phase 5 rollback**: Revert documentation changes

Git commands:
```bash
# View changes
git diff HEAD~N

# Revert specific commit
git revert <commit-sha>

# Full rollback to start
git reset --hard <starting-commit>
```

---

## 7. Estimated Impact

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Source files | ~120 | ~115 | -5 files |
| Lines of code | ~35,000 | ~34,000 | ~-1,000 LOC |
| Duplicate modules | 2 WS-Security | 1 | -1 |
| Dead code annotations | 25 | ~15 | -10 |
| `#[cfg(use_stubs)]` | 48 | 48 | 0 (kept for testing) |

---

## 8. Success Criteria

- [ ] All tests pass: `$cargo test --target x86_64-unknown-linux-gnu`
- [ ] No clippy warnings: `$cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings`
- [ ] Build succeeds: `$cargo build --release`
- [ ] No duplicate WS-Security implementations
- [ ] No stub handlers returning fake success
- [ ] Documentation updated
