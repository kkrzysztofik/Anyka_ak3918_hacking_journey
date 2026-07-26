# Docker CI Image Toolchain Upgrade Design

**Date:** 2026-07-26  
**Status:** Approved (approach 1 — versioned tag)  
**Branch:** `integrate-video` (no feature branch)

## Goal

Rebuild and publish `kkrzysztofik/anyka-cross-compile` with the vendored ARMv5TE toolchain at Rust **1.97.1** / LLVM **22.1.8** / GDB **17.2**, and point main CI at the new versioned tag.

## Tags

| Tag | Action |
|-----|--------|
| `kkrzysztofik/anyka-cross-compile:rust-1.97.1` | New primary CI pin |
| `kkrzysztofik/anyka-cross-compile:latest` | Retarget to same image |
| `rust-1.91.1` | Leave published; unused |

## Scope

### In scope

- Rebuild image via `scripts/docker/docker-build.sh` (COPY local `toolchain/arm-anykav200-crosstool-ng`)
- Dockerfile: drop rustup `stable` install (vendored `cargo` is a real binary; rustup proxies caused clippy E0514)
- Keep `cargo-tarpaulin` + Snyk; install tarpaulin with vendored cargo
- Ensure `/opt/arm-anykav200-crosstool-ng/bin` stays ahead of any `~/.cargo/bin`
- Update `.github/workflows/main-ci.yml` → `rust-1.97.1`
- Leave `release.yml` on `:latest`
- Update Serena memory refs (`project-context`, `suggested_commands`)
- Smoke-test image; push tags to Docker Hub

### Out of scope

- Rebuilding the toolchain itself (already on integrate-video)
- Changing GH Actions job matrix beyond image tag
- Deleting old `rust-1.91.1` Hub tag

## Success criteria

- Image `rustc --version` reports 1.97.1-dev; `which rustc` is under `/opt/arm-anykav200-crosstool-ng/bin`
- `main-ci.yml` references `:rust-1.97.1`
- Hub has `rust-1.97.1` and updated `latest`
