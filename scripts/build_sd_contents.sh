#!/usr/bin/env bash
# Assemble SD card payload into SD_card_contents/ (does not copy to media/device).
#
# Builds vendor-daemon, onvif-rust, anyka-init, snmp-agent, and the WebUI into:
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

Assemble vendor-daemon, onvif-rust, anyka-init, and WebUI into SD_card_contents/.

Options:
  --skip-www      Skip npm WebUI build
  --skip-vendor   Skip vendor-daemon build/install
  --debug         Build debug binaries (onvif-rust + vendor-daemon + anyka-init)
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

# ── anyka-init ───────────────────────────────────────────────────────────────
log_step "Building anyka-init (${BUILD_MODE})"
ANYKA_INIT_DIR="${REPO_ROOT}/cross-compile/anyka-init"
ANYKA_INIT_TARGET="armv5te-unknown-linux-uclibceabi"
# Ensure ARM cargo config exists (gitignored; generated from template).
if [[ ! -f "${ANYKA_INIT_DIR}/.cargo/config.toml" ]]; then
  if [[ ! -f "${ANYKA_INIT_DIR}/.cargo/config.toml.template" ]]; then
    log_error "anyka-init cargo config template missing"
    exit 1
  fi
  mkdir -p "${ANYKA_INIT_DIR}/.cargo"
  ANYKA_TOOLCHAIN_ROOT="$(cd "${ANYKA_TOOLCHAIN_BIN}/.." && pwd)"
  sed "s|@TOOLCHAIN_BASE@|${ANYKA_TOOLCHAIN_ROOT}|g" \
    "${ANYKA_INIT_DIR}/.cargo/config.toml.template" \
    > "${ANYKA_INIT_DIR}/.cargo/config.toml"
fi
(
  cd "${ANYKA_INIT_DIR}"
  if [[ "${BUILD_MODE}" = "debug" ]]; then
    "${CARGO}" build --target "${ANYKA_INIT_TARGET}"
    ANYKA_INIT_BIN="${REPO_ROOT}/cross-compile/target/${ANYKA_INIT_TARGET}/debug/anyka-init"
  else
    "${CARGO}" build --release --target "${ANYKA_INIT_TARGET}"
    ANYKA_INIT_BIN="${REPO_ROOT}/cross-compile/target/${ANYKA_INIT_TARGET}/release/anyka-init"
  fi
  install -m 0755 "${ANYKA_INIT_BIN}" "${ANYKA_HACK}/anyka-init.bin"
)
log_success "anyka-init installed to ${ANYKA_HACK}/anyka-init.bin"

# ── snmp-agent ───────────────────────────────────────────────────────────────
log_step "Building snmp-agent (${BUILD_MODE})"
# Build from onvif-rust so the (generated) .cargo/config.toml supplies the
# ARMv5TE linker — same pattern as other workspace members built for device.
ONVIF_DIR="${REPO_ROOT}/cross-compile/onvif-rust"
if [[ ! -f "${ONVIF_DIR}/.cargo/config.toml" ]]; then
  if [[ -x "${ONVIF_DIR}/scripts/setup-cargo-config.sh" ]]; then
    "${ONVIF_DIR}/scripts/setup-cargo-config.sh"
  else
    log_error "Missing ${ONVIF_DIR}/.cargo/config.toml — run onvif-rust setup-cargo-config.sh"
    exit 1
  fi
fi
(
  cd "${ONVIF_DIR}"
  if [[ "${BUILD_MODE}" = "debug" ]]; then
    "${CARGO}" build -p snmp-agent
    SNMP_BIN="${REPO_ROOT}/cross-compile/target/armv5te-unknown-linux-uclibceabi/debug/snmp-agent"
  else
    "${CARGO}" build --release -p snmp-agent
    SNMP_BIN="${REPO_ROOT}/cross-compile/target/armv5te-unknown-linux-uclibceabi/release/snmp-agent"
  fi
  mkdir -p "${ANYKA_HACK}/snmp"
  install -m 0755 "${SNMP_BIN}" "${ANYKA_HACK}/snmp/snmp-agent.bin"
)
log_success "snmp-agent installed to ${ANYKA_HACK}/snmp/snmp-agent.bin"

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
    lock_hash_file="node_modules/.anyka-lock-hash"
    # Empty when there is no lockfile to fingerprint. Caching on an empty hash
    # would match the empty stamp written by the previous run and skip installs
    # forever, so the no-lockfile path never reads or writes the stamp.
    current_hash="$(sha256sum package-lock.json 2>/dev/null | cut -d' ' -f1)"
    if [[ -z "${current_hash}" ]]; then
      npm install
    elif [[ -d node_modules && -f "${lock_hash_file}" && "$(cat "${lock_hash_file}")" == "${current_hash}" ]]; then
      log_info "Dependencies unchanged, skipping npm ci"
    else
      npm ci
      printf '%s' "${current_hash}" > "${lock_hash_file}"
    fi
    # `verify` is the single definition of the WebUI gate list, shared with
    # main-ci.yml so the two cannot drift — three TS7 errors merged green and
    # stopped a fleet rollout here because CI ran neither type-check nor build.
    # Tests and `npm audit` stay out of it on purpose: this script must work
    # offline and stay fast, and CI owns both.
    npm run verify
    npm run build
  )
  log_success "WebUI built to ${ANYKA_HACK}/onvif/www/"
fi

# ── Verify ───────────────────────────────────────────────────────────────────
# Clips are committed artifacts: regenerating them here would make the payload
# depend on whether the build host happens to have espeak-ng installed.
# Regenerate deliberately with scripts/make_speech.py and commit the result.
if [[ ! -f "${ANYKA_HACK}/onvif/sounds/boot.raw" ]]; then
  log_error "Missing sound clips under ${ANYKA_HACK}/onvif/sounds/ (run scripts/make_speech.py)"
  exit 1
fi

log_step "Verifying SD payload artifacts"
require_arm_elf "${ANYKA_HACK}/onvif/onvif-rust.bin"
require_arm_elf "${ANYKA_HACK}/vendor-daemon/vendor-daemon.bin"
require_arm_elf "${ANYKA_HACK}/anyka-init.bin"
require_arm_elf "${ANYKA_HACK}/snmp/snmp-agent.bin"

if [[ ! -f "${ANYKA_HACK}/lib/ld-uClibc.so.1" || ! -x "${ANYKA_HACK}/lib/ld-uClibc.so.1" ]]; then
  log_error "Missing dynamic loader required by anyka-init.bin: ${ANYKA_HACK}/lib/ld-uClibc.so.1"
  exit 1
fi

if [[ ! -x "${ANYKA_HACK}/onvif/onvif-rust" ]]; then
  log_error "Missing onvif-rust launcher: ${ANYKA_HACK}/onvif/onvif-rust"
  exit 1
fi

if [[ ! -f "${ANYKA_HACK}/anyka.toml" ]]; then
  log_error "Missing anyka.toml: ${ANYKA_HACK}/anyka.toml"
  exit 1
fi

if [[ ! -x "${FACTORY_DIR}/config.sh" ]]; then
  log_error "Missing or non-executable Factory/config.sh"
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
