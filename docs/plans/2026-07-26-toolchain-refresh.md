# Toolchain Full Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bump vendored toolchain pins (Rust 1.97.1, LLVM 22.1.8, GDB 17.2), hold uClibc 1.0.57, clean-rebuild ARMv5TE install, and verify host + cross builds.

**Architecture:** Version pins live in `toolchain/toolchain-builder/scripts/common.sh`. `./build.sh --clean` runs stages 01→05 into gitignored `toolchain/arm-anykav200-crosstool-ng/`. App code uses `source ./setenv.sh`.

**Tech Stack:** crosstool-NG 1.28.0, uClibc-ng 1.0.57, LLVM 22.1.8, Rust 1.97.1, GDB 17.2, Anyka ARMv5TE uClibc target.

## Global Constraints

- Spec: `docs/plans/2026-07-26-toolchain-refresh-design.md`
- Do **not** bump `UCLIBC_NG_VERSION` above 1.0.57
- Do **not** delete files without explicit user permission
- Pipe large build logs through `distill` with explicit PASS/FAIL prompts
- Build can take many hours — run with monitoring; resume via checkpoints if interrupted

---

## File map

| File | Role |
|------|------|
| `toolchain/toolchain-builder/scripts/common.sh` | Version pins |
| `toolchain/toolchain-builder/scripts/stages/01_toolchain.sh` … `05_gdb.sh` | Stage builds (edit only if broken) |
| `toolchain/toolchain-builder/scripts/armv5te-unknown-linux-uclibceabi.json` / inject in `04_rust.sh` | Custom Rust target |
| `toolchain/arm-anykav200-crosstool-ng/` | Install prefix (gitignored) |
| `setenv.sh` | Activation |
| `cross-compile/onvif-rust/Cargo.toml` (optional) | Re-raise MSRV pins after success |

---

### Task 1: Branch and pin updates

- [x] Create/switch to `chore/toolchain-bump` from current tip (`chore/deps-bump` or equivalent).
- [x] In `scripts/common.sh` set:
  - `LLVM_VERSION="22.1.8"`
  - `RUST_VERSION="1.97.1"`
  - `GDB_VERSION="17.2"`
  - Leave `CROSSTOOL_NG_VERSION="1.28.0"` and `UCLIBC_NG_VERSION="1.0.57"`.
- [x] Re-run `check_upstream_versions` and confirm expected “held / up to date / update applied” story.

**Verify:** `rg 'export (RUST|LLVM|GDB|UCLIBC|CROSSTOOL)_NG?_VERSION' scripts/common.sh` shows the target table.

---

### Task 2: Clean rebuild

- [x] Ensure host deps via builder’s dependency check (`build.sh` upfront check).
- [x] Ensure host `~/.cargo/bin/rustc` and `cargo` exist for Rust bootstrap.
- [x] From `toolchain/toolchain-builder`:

```bash
./build.sh --clean 2>&1 | tee /tmp/toolchain-build.log
```

- [x] If a stage fails, fix the specific stage script (no blind version rollback of uClibc), then `./build.sh --resume`.
- [x] On success, confirm install trees under `../arm-anykav200-crosstool-ng/`.

**Verify:**  
`source ../../setenv.sh && rustc --version && cargo --version && clang --version | head -1`  
Expect Rust **1.97.x**, Clang **22.1.8**.

---

### Task 3: Post-install wiring

- [x] `source ./setenv.sh` from repo root.
- [x] Regenerate cargo config if project scripts require it (`cross-compile/onvif-rust/scripts/setup-cargo-config.sh` or documented equivalent).
- [x] Confirm `rustc --print target-list | rg armv5te-unknown-linux-uclibceabi`.

**Verify:** target string present; `$CARGO`/`$RUSTC` point under `toolchain/arm-anykav200-crosstool-ng/bin/`.

---

### Task 4: Host quality gates

```bash
source ./setenv.sh
cd cross-compile
$CARGO fmt --check
$CARGO clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --workspace --target x86_64-unknown-linux-gnu
cd ../validation/rust
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu
```

- [x] Fix any rustc-1.97 breakages in app/tests (no shims).

**Verify:** all commands exit 0 (use `distill` on large logs).

**Note:** `setenv.sh` now uses project-local `toolchain/cargo-home` (registry/git symlinked from `~/.cargo`) so rustup’s `~/.cargo/bin/cargo-clippy` proxies cannot win and cause E0514. Host gates need `--config 'build.target="x86_64-unknown-linux-gnu"'` when `onvif-rust/.cargo/config.toml` defaults to ARM.

---

### Task 5: Cross release build

```bash
source ./setenv.sh
cd cross-compile
$CARGO build --release --workspace
```

- [x] Fix link/target issues if any (sysroot, clang wrapper, openssl-src patch).

**Verify:** release artifacts build for the default ARM target without error.

---

### Task 6: Optional MSRV crate re-bump

- [x] If 1.97 satisfies prior pins (e.g. `constant_time_eq` 0.5), bump those crates and re-run host tests.
- [x] If a crate still needs newer rustc, leave pinned and note in commit message.

**Verify:** host tests still PASS.

---

### Task 7: Commit builder/script/docs changes

- [x] Commit tracked changes (pins, stage fixes, specs/plans). **Do not** try to commit the binary install tree.
- [ ] Do not push unless user asks.

**Verify:** `git status` clean for intentional paths; install dir still gitignored.

---

## Execution notes

- Prefer Serena for script/source edits.
- Long build: background with output tee; poll only when blocked or for hang detection.
- Document any forced pin holds beyond uClibc in the final summary.
