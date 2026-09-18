---
name: anyka-embedded-build
description: Use when building, cross-compiling, linting, or testing Anyka ARM binaries for the AK3918 camera (ARM target, x86_64 host checks, armv5te, uclibc, setenv vendored toolchain, pre-commit quality gates). For getting a build onto a camera, use anyka-firmware-upgrade instead.
version: 2.0.0
---

# Anyka Embedded Build and Deployment

Cross-compile Rust binaries for ARM and deploy to Anyka cameras via SD card. Uses the project's **vendored toolchain** — never the system `cargo`.

## Mandatory Toolchain Setup

The project vendors a custom Rust toolchain at `toolchain/arm-anykav200-crosstool-ng/`. Always load it first:

```bash
# From repo root — exports $CARGO, $RUSTC, $RUSTDOC, sets CARGO_HOME, prepends toolchain bin/ to PATH
source ./setenv.sh
```

- **ALWAYS** use `$CARGO` (not bare `cargo`) for every Rust command.
- The default build target is `armv5te-unknown-linux-uclibceabi` (ARM camera).
- Host-side tests/linting/docs require explicit `--target x86_64-unknown-linux-gnu`.
- Do **not** use `rustup target add` or system rustup — the vendored toolchain is self-contained.

## Build Targets

### ARM Release Build (deployment)

```bash
source ./setenv.sh
cd cross-compile/onvif-rust
$CARGO build --release --target armv5te-unknown-linux-uclibceabi

# Output: target/armv5te-unknown-linux-uclibceabi/release/onvif-rust
```

### Host-Side Test / Lint / Doc

```bash
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO test --target x86_64-unknown-linux-gnu --lib        # unit tests only
$CARGO test --target x86_64-unknown-linux-gnu test_name -- --nocapture
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
$CARGO doc --target x86_64-unknown-linux-gnu --no-deps
```

`streaming-lib`, `vendor-daemon`, and `validation/rust` follow the same pattern (build ARM, test on x86_64).

## Pre-Commit Verification

Run all quality gates before committing:

```bash
source ./setenv.sh
cd cross-compile/onvif-rust
$CARGO fmt --check
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO doc --target x86_64-unknown-linux-gnu --no-deps
$CARGO build --release --target armv5te-unknown-linux-uclibceabi
```

## Deployment

This skill builds; it does not deploy. See **`anyka-firmware-upgrade`** for every
path onto a camera — A/B bundle upgrade, full SD payload, one-time slot
migration, and single-binary dev iteration.

The build output is staged under `SD_card_contents/anyka_hack/`, which is laid
out as A/B slots (`active`, `slots/{a,b}`, `spool/`, `state/`); that skill's
reference documents the tree.

## Device Runtime Facts

- Camera default IP: `192.168.2.198`. ONVIF is on port **80** (`http://<ip>/onvif/device_service`); 554 is RTSP and 8080 is HTTP-FLV.
- There is **no SSH** on the camera — remote shell is telnet port 24, and coredump collection is over FTP. See `anyka-remote-debugging` for both.

## Troubleshooting

- **"Linker not found" / wrong rustc version (E0514):** you used system `rustc` instead of the vendored one. Re-`source ./setenv.sh` and verify `$CARGO --version` points into `toolchain/arm-anykav200-crosstool-ng/bin/`.
- **`setenv.sh` clears `SYSROOT`** (crosstool exports it; it confuses clippy-driver).
- **Missing toolchain:** if `setenv.sh` fails, the toolchain dir is incomplete — do not fall back to system Rust.
