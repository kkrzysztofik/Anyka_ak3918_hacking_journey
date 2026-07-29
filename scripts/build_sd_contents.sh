#!/usr/bin/env bash
# Assemble SD card payload into SD_card_contents/ (does not copy to media/device).
#
# Builds vendor-daemon, onvif-rust, and the WebUI into:
#   SD_card_contents/anyka_hack/
# Factory scripts are already tracked under SD_card_contents/Factory/.
#
# Usage:
#   ./scripts/build_sd_contents.sh
#   ./scripts/build_sd_contents.sh --skip-www
#   ./scripts/build_sd_contents.sh --debug
#
# After this succeeds, copy to a card/device with:
#   ./scripts/copy_sd_contents.sh --sd /path/to/mount
#   ./scripts/copy_sd_contents.sh --ftp 192.168.1.100

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"

SKIP_WWW=false
SKIP_VENDOR=false
BUILD_MODE="release"

usage() {
  cat <<'EOF'
Usage: build_sd_contents.sh [OPTIONS]

Assemble vendor-daemon, onvif-rust, and WebUI into SD_card_contents/.

Options:
  --skip-www      Skip npm WebUI build
  --skip-vendor   Skip vendor-daemon build/install
  --debug         Build debug binaries (onvif-rust + vendor-daemon)
  -h, --help      Show this help

Examples:
  ./scripts/build_sd_contents.sh
  ./scripts/build_sd_contents.sh --skip-www
  ./scripts/build_sd_contents.sh --debug
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-www)
      SKIP_WWW=true
      shift
      ;;
    --skip-vendor)
      SKIP_VENDOR=true
      shift
      ;;
    --debug)
      BUILD_MODE="debug"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      log_error "Unknown option: $1"
      usage >&2
      exit 1
      ;;
  esac
done

REPO_ROOT="${ANYKA_REPO_ROOT}"
VENDOR_DIR="${REPO_ROOT}/cross-compile/vendor-daemon"
ONVIF_BUILD="${REPO_ROOT}/cross-compile/onvif-rust/scripts/build.sh"
WWW_DIR="${REPO_ROOT}/cross-compile/www"
SD_ROOT="${REPO_ROOT}/SD_card_contents"
ANYKA_HACK="${SD_ROOT}/anyka_hack"
FACTORY_DIR="${SD_ROOT}/Factory"

require_arm_elf() {
  local path="$1"
  if [[ ! -f "${path}" ]]; then
    log_error "Missing artifact: ${path}"
    exit 1
  fi
  if command -v file &>/dev/null; then
    if ! file "${path}" | grep -q "ELF 32-bit.*ARM"; then
      log_error "Refusing to continue: ${path} is not an ARMv5 32-bit ELF"
      log_error "Got: $(file "${path}")"
      exit 1
    fi
  fi
}

log_info "=== Build SD card contents ==="
log_info "Repo: ${REPO_ROOT}"
log_info "Mode: ${BUILD_MODE}"
echo ""

anyka_require_vendored_cargo
export CARGO="${ANYKA_CARGO}"
export RUSTC="${ANYKA_RUSTC}"
export RUSTDOC="${ANYKA_RUSTDOC}"
export PATH="${ANYKA_TOOLCHAIN_BIN}:${PATH}"

# ── vendor-daemon ────────────────────────────────────────────────────────────
if [[ "${SKIP_VENDOR}" = true ]]; then
  log_warn "Skipping vendor-daemon build (--skip-vendor)"
else
  log_step "Building vendor-daemon (${BUILD_MODE})"
  if [[ ! -d "${VENDOR_DIR}" ]]; then
    log_error "vendor-daemon directory not found: ${VENDOR_DIR}"
    exit 1
  fi

  if [[ "${BUILD_MODE}" = "debug" ]]; then
    make -C "${VENDOR_DIR}" debug
    mkdir -p "${ANYKA_HACK}/vendor-daemon/lib"
    install -m 755 "${VENDOR_DIR}/build/vendor-daemon-debug.bin" \
      "${ANYKA_HACK}/vendor-daemon/vendor-daemon.bin"
    # Keep SDK libs in sync with source tree (same as make install).
    cp -f "${VENDOR_DIR}"/lib/*.so "${ANYKA_HACK}/vendor-daemon/lib/"
  else
    make -C "${VENDOR_DIR}" release install
  fi
  log_success "vendor-daemon installed to ${ANYKA_HACK}/vendor-daemon/"
fi

# ── onvif-rust ───────────────────────────────────────────────────────────────
log_step "Building onvif-rust (${BUILD_MODE})"
if [[ ! -x "${ONVIF_BUILD}" ]]; then
  log_error "onvif-rust build script not found or not executable: ${ONVIF_BUILD}"
  exit 1
fi

if [[ "${BUILD_MODE}" = "debug" ]]; then
  "${ONVIF_BUILD}" --debug
else
  "${ONVIF_BUILD}" --release
fi
log_success "onvif-rust deployed to ${ANYKA_HACK}/onvif/"

# ── WebUI ────────────────────────────────────────────────────────────────────
if [[ "${SKIP_WWW}" = true ]]; then
  log_warn "Skipping WebUI build (--skip-www)"
else
  log_step "Building WebUI"
  anyka_check_commands npm
  if [[ ! -d "${WWW_DIR}" ]]; then
    log_error "WebUI directory not found: ${WWW_DIR}"
    exit 1
  fi
  (
    cd "${WWW_DIR}"
    if [[ -f package-lock.json ]]; then
      npm ci
    else
      npm install
    fi
    npm run build
  )
  log_success "WebUI built to ${ANYKA_HACK}/onvif/www/"
fi

# ── Verify ───────────────────────────────────────────────────────────────────
log_step "Verifying SD payload artifacts"
require_arm_elf "${ANYKA_HACK}/onvif/onvif-rust.bin"
require_arm_elf "${ANYKA_HACK}/vendor-daemon/vendor-daemon.bin"

if [[ ! -x "${ANYKA_HACK}/onvif/onvif-rust" ]]; then
  log_error "Missing onvif-rust launcher: ${ANYKA_HACK}/onvif/onvif-rust"
  exit 1
fi

if [[ "${SKIP_WWW}" = false && ! -f "${ANYKA_HACK}/onvif/www/index.html" ]]; then
  log_error "WebUI index.html missing under ${ANYKA_HACK}/onvif/www/"
  exit 1
fi

if [[ ! -d "${FACTORY_DIR}" ]]; then
  log_error "Factory directory missing: ${FACTORY_DIR}"
  exit 1
fi

echo ""
log_success "SD_card_contents assembly complete"
log_info "Payload roots:"
log_info "  ${ANYKA_HACK}"
log_info "  ${FACTORY_DIR}"
echo ""
log_info "Next: copy to SD card or camera:"
log_info "  ./scripts/copy_sd_contents.sh --sd /path/to/mounted/sd"
log_info "  ./scripts/copy_sd_contents.sh --ftp <camera-ip>"
