# Rust CI Speedup Design (Approach 1)

Date: 2026-08-10

## Problem

The monolithic `Rust - Lint, Test & Coverage` job spent ~12 minutes on a representative run, dominated by `cargo tarpaulin` (~435s), restoring a ~3GB Actions cache including `cross-compile/target` (~135s), and pulling the full cross-compile image (~65s / ~10GB).

## Goals

- Keep coverage on the Sonar critical path (`sonar.rust.cobertura.reportPaths`).
- Target ~3 minutes wall clock for the Rust coverage job on a warm registry cache (stretch).
- Software first; larger runners only after measuring the new baseline.

## Design

1. Split into parallel `rust-lint` and `rust-coverage` jobs.
2. Replace tarpaulin with `cargo-llvm-cov` (Cobertura + Lcov + HTML).
3. Cache only `~/.cargo/registry` and `~/.cargo/git` under key prefix `cargo-linux-rust-reg-`.
4. Use slim host-CI image `kkrzysztofik/anyka-cross-compile:rust-1.97.1-ci` (~2.3GB locally vs ~10.4GB fat image), built via `scripts/docker/Dockerfile.ci` + `prepare-ci-toolchain.sh`. Keep fat image for `release.yml`.
5. `fetch-depth: 1` on rust jobs; Sonar keeps full history.

## Build / publish host-CI image

Requires local vendored toolchain + `gh` (for compiler-rt fetch when building `profiler_builtins`).

```bash
./scripts/docker/docker-build.sh --ci -t kkrzysztofik/anyka-cross-compile:rust-1.97.1-ci
docker push kkrzysztofik/anyka-cross-compile:rust-1.97.1-ci
```

`prepare-ci-toolchain.sh` calls `build-profiler-builtins.sh` so the slim image can run `cargo-llvm-cov` (vendored rustc ships without profiler runtime).

## Local measurement (2026-08-10)

| Step | Before | After (local) |
|------|--------|----------------|
| Image size | ~10.4GB (`rust-1.97.1`) | ~2.3GB (`rust-1.97.1-ci`) |
| Cold image pull | ~65s | ~22s |
| Fat `target/` cache restore | ~135s | removed (registry-only cache) |
| Coverage run | tarpaulin ~435s | llvm-cov cold ~176s + report ~5s |

Estimated GitHub `rust-coverage` wall clock (cold instrumented build, warm crates.io cache): ~4–5 minutes.

Follow-up: `sccache` is baked into `:rust-1.97.1-ci` and wired via `RUSTC_WRAPPER` + Actions cache of `/tmp/sccache` on both rust jobs (llvm-cov chains the pre-existing wrapper). Warm-cache runs should improve; cold runs still pay full instrumented compile.

## Sonar / Cobertura path mapping

`cargo llvm-cov` runs from `cross-compile/` and emits Cobertura `<class filename="onvif-rust/src/...">` with `<source>/workspace/cross-compile</source>`. Sonar is configured with `sonar.sources=cross-compile`, so filenames must be repo-root-relative (`cross-compile/onvif-rust/src/...`).

CI rewrites Cobertura XML via `scripts/ci/rewrite_cobertura_paths.py` (ElementTree): set `<source>.</source>` and prepend `cross-compile/` to each class filename. Lcov/HTML artifacts are for humans only; Sonar consumes the rewritten Cobertura.

Image tag `:rust-1.97.1-ci` is intentionally floating (not digest-pinned) so rebuild/push picks up the next CI run without editing the workflow.

## Success criteria

Warm-cache `rust-coverage` job ≤ ~3 minutes. If not, re-measure step breakdown before considering larger runners.
