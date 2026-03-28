#!/bin/bash
# build.sh — Orchestrates the complete toolchain build for Anyka AK3918 cameras.
#
# Runs all stages in sequence:
#   1. build_toolchain.sh            GCC cross-compiler via crosstool-NG
#   2. build_llvm.sh                 LLVM/Clang (native, needed by Rust)
#   3. build_compiler_rt_builtins.sh compiler-rt builtins (cross-compiled)
#   4. bootstrap_rust.sh             Rust compiler from source
#   5. install_rust_src.sh           rust-src component (IDE support)
#   6. verify_rust.sh                Rust installation verification
#   7. rebuild_gdb.sh                GDB with embedded dynamic linker
#
# USAGE:
#   ./build.sh [OPTIONS]
#
# OPTIONS:
#   --no-rust    Skip Rust stages (4, 5, 6) — saves ~6 hours
#   --no-gdb     Skip GDB rebuild (stage 7)
#   --resume     Skip stages with an existing checkpoint (.build/checkpoints/.done_*)
#   --dry-run    Print what would run without executing anything
#   -h, --help   Show this help message

set -euo pipefail

# Script directory — must be set before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "${SCRIPT_DIR}/common.sh"

# ── Option parsing ────────────────────────────────────────────────────────────
OPT_NO_RUST=false
OPT_NO_GDB=false
OPT_RESUME=false
OPT_DRY_RUN=false

usage() {
    sed -n '/^# USAGE:/,/^[^#]/{ /^[^#]/d; p }' "$0" | sed 's/^# //; s/^#$//'
    exit 0
}

for arg in "$@"; do
    case "${arg}" in
        --no-rust)  OPT_NO_RUST=true ;;
        --no-gdb)   OPT_NO_GDB=true ;;
        --resume)   OPT_RESUME=true ;;
        --dry-run)  OPT_DRY_RUN=true ;;
        -h|--help)  usage ;;
        *)
            log_error "Unknown option: ${arg}"
            log_info  "Run with --help for usage."
            exit 1
            ;;
    esac
done

# ── Log file ──────────────────────────────────────────────────────────────────
if [[ "${OPT_DRY_RUN}" != "true" ]]; then
    mkdir -p "${BUILD_DIR}/.build"
    LOG_FILE="${BUILD_DIR}/.build/build_$(date +%Y%m%d_%H%M%S).log"
    exec > >(tee -a "${LOG_FILE}") 2>&1
fi

# ── Checkpoint helpers ────────────────────────────────────────────────────────
CHECKPOINT_DIR="${BUILD_DIR}/.build/checkpoints"
mkdir -p "${CHECKPOINT_DIR}"

checkpoint_done() { [[ -f "${CHECKPOINT_DIR}/.done_${1}" ]]; }
checkpoint_save() { touch "${CHECKPOINT_DIR}/.done_${1}"; }

# ── Stage count ───────────────────────────────────────────────────────────────
stage_current=0
stage_total=3  # always: toolchain + llvm + compiler_rt
[[ "${OPT_NO_RUST}" != "true" ]] && stage_total=$(( stage_total + 3 ))
[[ "${OPT_NO_GDB}" != "true" ]]  && stage_total=$(( stage_total + 1 ))

trap 'log_error "Build failed at stage ${stage_current}/${stage_total}. Use --resume to skip completed stages."' ERR

# ── Stage runner ──────────────────────────────────────────────────────────────
# run_stage <checkpoint-name> <script-filename> <description>
run_stage() {
    local name="$1"
    local script="$2"
    local desc="$3"

    stage_current=$(( stage_current + 1 ))
    echo ""
    log_info "=========================================="
    log_info "Stage ${stage_current}/${stage_total}: ${desc}"
    log_info "=========================================="

    if [[ "${OPT_RESUME}" == "true" ]] && checkpoint_done "${name}"; then
        log_warn "  Skipping — checkpoint .done_${name} exists."
        log_warn "  Delete ${CHECKPOINT_DIR}/.done_${name} to force a re-run."
        return 0
    fi

    if [[ "${OPT_DRY_RUN}" == "true" ]]; then
        log_info "  [DRY-RUN] Would execute: ${SCRIPT_DIR}/${script}"
        return 0
    fi

    "${SCRIPT_DIR}/${script}"
    checkpoint_save "${name}"
    log_info "Stage '${name}' complete."
}

# ── Banner ────────────────────────────────────────────────────────────────────
echo ""
log_info "=========================================="
log_info "  Anyka AK3918 — Full Toolchain Build"
log_info "=========================================="
echo ""
log_info "Target:      ${TARGET_TUPLE}"
log_info "Install dir: ${INSTALL_DIR}"
log_info "Stages:      ${stage_total}"
echo ""
[[ "${OPT_NO_RUST}" == "true" ]]  && log_info "  --no-rust  (skipping Rust stages 4-6)"
[[ "${OPT_NO_GDB}" == "true" ]]   && log_info "  --no-gdb   (skipping GDB rebuild)"
[[ "${OPT_RESUME}" == "true" ]]   && log_info "  --resume   (completed stages will be skipped)"
[[ "${OPT_DRY_RUN}" == "true" ]]  && log_info "  --dry-run  (no commands will be executed)"

est_hours=6  # toolchain 2-3h + llvm 3-4h + compiler-rt ~1h
[[ "${OPT_NO_RUST}" != "true" ]] && est_hours=$(( est_hours + 7 ))
[[ "${OPT_NO_GDB}" != "true" ]]  && est_hours=$(( est_hours + 1 ))
echo ""
log_warn "Estimated build time: ${est_hours}+ hours (varies by CPU and network speed)"
log_warn "Tip: run inside 'tmux' or 'screen' to survive disconnects."
echo ""

if [[ "${OPT_DRY_RUN}" != "true" ]]; then
    log_info "Starting in 5 seconds — press Ctrl+C to abort."
    sleep 5
fi

# ── Stages ────────────────────────────────────────────────────────────────────
run_stage "toolchain"   "build_toolchain.sh"            "GCC cross-compiler (crosstool-NG git ${CTNG_GIT_REF:0:12})"
run_stage "llvm"        "build_llvm.sh"                 "LLVM/Clang ${LLVM_VERSION}"
run_stage "compiler_rt" "build_compiler_rt_builtins.sh" "compiler-rt builtins (cross-compiled)"

if [[ "${OPT_NO_RUST}" != "true" ]]; then
    run_stage "rust"        "bootstrap_rust.sh"   "Rust ${RUST_VERSION} from source"
    run_stage "rust_src"    "install_rust_src.sh" "rust-src component"
    run_stage "verify_rust" "verify_rust.sh"      "Rust installation verification"
fi

if [[ "${OPT_NO_GDB}" != "true" ]]; then
    run_stage "gdb" "rebuild_gdb.sh" "GDB ${GDB_VERSION} with embedded dynamic linker"
fi

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
log_info "=========================================="
log_info "  Build complete!"
log_info "=========================================="
echo ""
log_info "Toolchain installed to: ${INSTALL_DIR}"
echo ""
log_info "Key binaries:"
log_info "  ${INSTALL_DIR}/bin/${TARGET_TUPLE}-gcc"
log_info "  ${INSTALL_DIR}/bin/${TARGET_TUPLE}-g++"
log_info "  ${INSTALL_DIR}/bin/clang"
if [[ "${OPT_NO_RUST}" != "true" ]]; then
    log_info "  ${INSTALL_DIR}/bin/rustc"
    log_info "  ${INSTALL_DIR}/bin/cargo"
fi
echo ""
log_info "To use in a Cargo project, add to .cargo/config.toml:"
log_info "  [target.armv5te-unknown-linux-uclibceabi]"
log_info "  linker = \"${INSTALL_DIR}/bin/clang\""
echo ""
if [[ "${OPT_DRY_RUN}" != "true" ]]; then
    log_info "Full build log: ${LOG_FILE}"
fi
