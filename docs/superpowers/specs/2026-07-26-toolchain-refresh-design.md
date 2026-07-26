# Toolchain Full Refresh Design (Hold uClibc)

**Date:** 2026-07-26  
**Status:** Approved (approach 2)  
**Branch:** `chore/toolchain-bump` (from current deps work tip)

## Goal

Rebuild the vendored Anyka ARMv5TE toolchain with the newest Rust, LLVM/Clang, and GDB releases while **holding uClibc-ng at 1.0.57** (camera Linux **3.4.35**; TIME64 must remain disabled). crosstool-NG stays at 1.28.0 (already current).

## Version targets

| Component | Current pin | Target |
|-----------|-------------|--------|
| crosstool-NG | 1.28.0 | 1.28.0 (no change) |
| uClibc-ng | 1.0.57 | **1.0.57 held** |
| LLVM/Clang | 22.1.2 | **22.1.8** |
| Rust | 1.94.1 | **1.97.1** |
| GDB | 17.1 | **17.2** |

Source of “newest”: `toolchain/toolchain-builder/scripts/check-versions.sh` against upstream (2026-07-26).

## Scope

### In scope

- Edit pins in `toolchain/toolchain-builder/scripts/common.sh`
- Fix stage scripts if bootstrap / target-spec APIs break (`scripts/stages/01_*.sh` … `05_gdb.sh`, inject helpers)
- Clean rebuild: `./build.sh --clean` → install into `toolchain/arm-anykav200-crosstool-ng/` (gitignored)
- Post-install: `source ./setenv.sh`; refresh cargo config via existing setup script if required
- Host + cross verification for `cross-compile/` and `validation/rust`
- Optionally re-bump crates previously MSRV-capped on 1.94 (e.g. `constant_time_eq` 0.5)

### Out of scope

- uClibc-ng 1.0.58 (tracked as follow-up after device/kernel validation)
- aarch64 toolchain rebuild unless explicitly requested
- Application features unrelated to toolchain/MSRV
- Committing the binary install tree (remains gitignored)

## Constraints

- Preserve ARMv5TE soft-float / `arm926ej-s` / `armv5te-unknown-linux-uclibceabi` custom target injection
- Preserve TIME64-disabled uClibc fragments and openssl-src app patch (orthogonal)
- Host bootstrap still needs `~/.cargo/bin/{rustc,cargo}` for stage 4
- Multi-hour build; use checkpoints / resume; monitor for hang

## Success criteria

- `rustc --version` / `cargo --version` report **1.97.x** via `setenv.sh`
- `rustc --print target-list` includes `armv5te-unknown-linux-uclibceabi`
- Clang reports **22.1.8**; GDB pin reflected in rebuilt gdb binary where applicable
- Host: `cargo clippy` + `test` for workspace and validation (x86_64 target)
- Cross: `cargo build --release` for workspace succeeds with vendored toolchain
- Pins in `common.sh` match the table above; uClibc remains 1.0.57

## Risks

| Risk | Mitigation |
|------|------------|
| Rust 1.97 bootstrap / `x.py` / target API drift | Fix `04_rust.sh` (and target JSON) iteratively; resume checkpoints |
| LLVM 22.1.8 build/cmake breakage | Fix `02_llvm.sh` / `03_compiler_rt.sh`; keep Rust in lockstep with working LLVM |
| Stage 1 still long despite unchanged ct-ng/uClibc pins | `--clean` rebuilds everything; acceptable for this refresh |
| Crate MSRV still above 1.97 | Unlikely for current pins; pin or wait if needed |

## Follow-up (not this change)

- Evaluate uClibc-ng **1.0.58** changelog vs Linux 3.4.35 / TIME64 policy, then bump in a dedicated change
