#!/bin/bash
# Stage a host-only Rust 1.97.1-dev toolchain for Dockerfile.ci.
# Omits ARM crosstool/sysroot and heavy clang/LLVM tooling unused by host CI.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/../common.sh"
PROJECT_ROOT="${ANYKA_REPO_ROOT}"

SRC="${PROJECT_ROOT}/toolchain/arm-anykav200-crosstool-ng"
DST="${SCRIPT_DIR}/.ci-toolchain"

if [[ ! -x "${SRC}/bin/rustc" || ! -x "${SRC}/bin/cargo" ]]; then
  log_error "Vendored toolchain not found at ${SRC}"
  log_info "Build or install toolchain/arm-anykav200-crosstool-ng before preparing the CI image."
  exit 1
fi

# Ensure llvm-cov can link (-C instrument-coverage)
bash "${SCRIPT_DIR}/build-profiler-builtins.sh"

log_info "Staging host-CI toolchain: ${SRC} -> ${DST}"
rm -rf "${DST}"
mkdir -p "${DST}/bin" "${DST}/lib/rustlib"

BINS=(
  cargo
  rustc
  rustdoc
  clippy-driver
  rustfmt
  cargo-clippy
  cargo-fmt
  llvm-cov
  llvm-profdata
)

for bin in "${BINS[@]}"; do
  if [[ ! -e "${SRC}/bin/${bin}" ]]; then
    log_error "Missing required toolchain binary: ${SRC}/bin/${bin}"
    exit 1
  fi
  cp -a "${SRC}/bin/${bin}" "${DST}/bin/"
done

# rustc loads librustc_driver via $ORIGIN/../lib
shopt -s nullglob
rust_driver_libs=("${SRC}/lib"/librustc_driver*.so)
if [[ ${#rust_driver_libs[@]} -eq 0 ]]; then
  log_error "No librustc_driver*.so found under ${SRC}/lib"
  exit 1
fi
cp -a "${rust_driver_libs[@]}" "${DST}/lib/"
shopt -u nullglob

cp -a "${SRC}/lib/rustlib/x86_64-unknown-linux-gnu" "${DST}/lib/rustlib/"
if [[ -d "${SRC}/lib/rustlib/etc" ]]; then
  cp -a "${SRC}/lib/rustlib/etc" "${DST}/lib/rustlib/"
fi

# Drop ARM std and unused source tree from the staged rustlib copy
rm -rf "${DST}/lib/rustlib/armv5te-unknown-linux-uclibceabi" \
  "${DST}/lib/rustlib/src" 2>/dev/null || true

if ! compgen -G "${DST}/lib/rustlib/x86_64-unknown-linux-gnu/lib/libprofiler_builtins-*.rlib" >/dev/null; then
  log_error "Staged toolchain is missing profiler_builtins (required for cargo-llvm-cov)"
  exit 1
fi

log_success "Staged host-CI toolchain ($(du -sh "${DST}" | cut -f1))"
echo "${DST}"
