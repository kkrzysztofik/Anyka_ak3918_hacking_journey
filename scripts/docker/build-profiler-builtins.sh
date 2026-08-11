#!/bin/bash
# Build profiler_builtins with the vendored rustc and install into its sysroot.
# Required for cargo-llvm-cov (-C instrument-coverage). The stock Anyka
# toolchain is built without the profiler runtime (E0463 without this step).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/../common.sh"
PROJECT_ROOT="${ANYKA_REPO_ROOT}"

TOOLCHAIN="${PROJECT_ROOT}/toolchain/arm-anykav200-crosstool-ng"
RUST_SRC="${TOOLCHAIN}/lib/rustlib/src/rust/library/profiler_builtins"
SYSROOT_LIB="${TOOLCHAIN}/lib/rustlib/x86_64-unknown-linux-gnu/lib"
LLVM_TAG="${LLVM_PROFILER_LLVM_TAG:-llvmorg-22.1.8}"
WORK="${TMPDIR:-/tmp}/anyka-profiler-builtins-$$"

if [[ ! -x "${TOOLCHAIN}/bin/rustc" ]]; then
  log_error "Vendored rustc not found at ${TOOLCHAIN}/bin/rustc"
  exit 1
fi
if [[ ! -f "${RUST_SRC}/Cargo.toml" ]]; then
  log_error "profiler_builtins sources missing (rust-src): ${RUST_SRC}"
  exit 1
fi

# Already installed?
if compgen -G "${SYSROOT_LIB}/libprofiler_builtins-*.rlib" >/dev/null; then
  log_info "profiler_builtins already present in sysroot; skipping build"
  exit 0
fi

cleanup() {
  rm -rf "${WORK}"
}
trap cleanup EXIT

mkdir -p "${WORK}/extract" "${WORK}/target"
ARCHIVE="${WORK}/llvm.tgz"

if ! command -v gh >/dev/null 2>&1; then
  log_error "GitHub CLI (gh) is required to fetch compiler-rt sources"
  log_info "Install: https://cli.github.com/ — then re-run this script"
  exit 1
fi

log_info "Fetching ${LLVM_TAG} compiler-rt (profiler sources) via gh..."
gh api "repos/llvm/llvm-project/tarball/${LLVM_TAG}" \
  -H "Accept: application/vnd.github+json" >"${ARCHIVE}"

PREFIX="$(tar -tzf "${ARCHIVE}" | head -1 | cut -d/ -f1)"
tar -xzf "${ARCHIVE}" -C "${WORK}/extract" \
  "${PREFIX}/compiler-rt/lib/profile" \
  "${PREFIX}/compiler-rt/include"
CRT="${WORK}/extract/${PREFIX}/compiler-rt"
if [[ ! -f "${CRT}/lib/profile/InstrProfiling.c" ]]; then
  log_error "Failed to extract compiler-rt profile sources"
  exit 1
fi

log_info "Building profiler_builtins with vendored rustc..."
export PATH="${TOOLCHAIN}/bin:${PATH}"
export RUST_COMPILER_RT_FOR_PROFILER="${CRT}"
(
  cd "${RUST_SRC}"
  cargo rustc --release --target x86_64-unknown-linux-gnu --target-dir "${WORK}/target"
)

shopt -s nullglob
rlibs=("${WORK}/target/x86_64-unknown-linux-gnu/release/deps"/libprofiler_builtins-*.rlib)
rmetas=("${WORK}/target/x86_64-unknown-linux-gnu/release/deps"/libprofiler_builtins-*.rmeta)
if [[ ${#rlibs[@]} -eq 0 ]]; then
  log_error "profiler_builtins build produced no rlib"
  exit 1
fi
cp -a "${rlibs[@]}" "${rmetas[@]}" "${SYSROOT_LIB}/"
# Native static archive produced by build.rs (linked via the rlib)
native_a=("${WORK}/target/x86_64-unknown-linux-gnu/release/build"/profiler_builtins-*/out/libprofiler-rt.a)
if [[ ${#native_a[@]} -gt 0 ]]; then
  cp -a "${native_a[0]}" "${SYSROOT_LIB}/libprofiler-rt.a"
fi
shopt -u nullglob

# Smoke test
tmp_rs="${WORK}/covtest.rs"
tmp_bin="${WORK}/covtest"
printf 'fn main(){println!("ok");}\n' >"${tmp_rs}"
rustc -C instrument-coverage "${tmp_rs}" -o "${tmp_bin}"
"${tmp_bin}" >/dev/null

log_success "Installed profiler_builtins into ${SYSROOT_LIB}"
