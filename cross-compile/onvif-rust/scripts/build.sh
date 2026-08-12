#!/bin/bash
# Build script for ONVIF Rust cross-compilation
# Builds the Rust ONVIF application for ARMv5TE target

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "$(cd "${SCRIPT_DIR}/../../.." && pwd)/scripts/common.sh"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORKSPACE_DIR="$(cd "${PROJECT_DIR}/.." && pwd)"

# Default values
BUILD_MODE="release"
TARGET="armv5te-unknown-linux-uclibceabi"
CLEAN=false
EXTRA_FEATURES=""
NO_IPC=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --debug)
      BUILD_MODE="debug"
      shift
      ;;
    --release)
      BUILD_MODE="release"
      shift
      ;;
    --target)
      TARGET="$2"
      shift 2
      ;;
    --clean)
      CLEAN=true
      shift
      ;;
    --features)
      EXTRA_FEATURES="$2"
      shift 2
      ;;
    --no-ipc|--direct-ffi)
      NO_IPC=true
      shift
      ;;
    -h|--help)
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --debug               Build in debug mode (default: release)"
      echo "  --release             Build in release mode (default)"
      echo "  --target TARGET       Specify target triple (default: armv5te-unknown-linux-uclibceabi)"
      echo "  --clean               Clean before building"
      echo "  --clean               Clean before building"
      echo "  --features FEATURES   Additional cargo features"
      echo "  --no-ipc              Disable IPC mode (use direct vendor FFI linking)"
      echo "  -h, --help            Show this help message"
      echo ""
      exit 0
      ;;
    *)
      log_error "Unknown option: $1"
      echo "Use --help for usage information" >&2
      exit 1
      ;;
  esac
done

# Change to project directory
cd "${PROJECT_DIR}"

# Setup cargo config for current environment
log_info "Setting up cargo configuration..."
"${SCRIPT_DIR}/setup-cargo-config.sh"

log_info "Building ONVIF Rust application"
log_info "Project directory: ${PROJECT_DIR}"
log_info "Target: ${TARGET}"
log_info "Build mode: ${BUILD_MODE}"
[[ -n "${EXTRA_FEATURES}" ]] && log_info "Extra features: ${EXTRA_FEATURES}"

# Use vendored toolchain cargo by default (per project requirements)
export CARGO="${CARGO:-${ANYKA_CARGO}}"

if [[ ! -x "${CARGO}" ]]; then
  log_error "cargo not found or not executable at: ${CARGO}"
  log_error "Set CARGO to the vendored toolchain cargo, e.g.:"
  log_error "  export CARGO=${ANYKA_CARGO}"
  exit 1
fi

log_info "Using cargo: ${CARGO}"

# Clean if requested
if [[ "${CLEAN}" = true ]]; then
  log_info "Cleaning build artifacts..."
  "${CARGO}" clean
fi

# Build the project
log_info "Building for target ${TARGET} in ${BUILD_MODE} mode..."

FEATURES_ARGS=()
# No default features - use --features to add any needed features
DEFAULT_FEATURES=""

# Combine default + extra features
ALL_FEATURES="${DEFAULT_FEATURES}"
if [[ -n "${EXTRA_FEATURES}" ]]; then
  if [[ -n "${ALL_FEATURES}" ]]; then
    ALL_FEATURES="${ALL_FEATURES},${EXTRA_FEATURES}"
  else
    ALL_FEATURES="${EXTRA_FEATURES}"
  fi
fi

if [[ -n "${ALL_FEATURES}" ]]; then
  FEATURES_ARGS=(--features "${ALL_FEATURES}")
fi

[[ -n "${ALL_FEATURES}" ]] && log_info "Features: ${ALL_FEATURES}"

if [[ "${BUILD_MODE}" = "release" ]]; then
  "${CARGO}" build --release --target "${TARGET}" "${FEATURES_ARGS[@]}"
  WORKSPACE_BINARY_PATH="${WORKSPACE_DIR}/target/${TARGET}/release/onvif-rust"
  CRATE_BINARY_PATH="${PROJECT_DIR}/target/${TARGET}/release/onvif-rust"
else
  "${CARGO}" build --target "${TARGET}" "${FEATURES_ARGS[@]}"
  WORKSPACE_BINARY_PATH="${WORKSPACE_DIR}/target/${TARGET}/debug/onvif-rust"
  CRATE_BINARY_PATH="${PROJECT_DIR}/target/${TARGET}/debug/onvif-rust"
fi

# Resolve actual binary location.
# When building as part of the `cross-compile` workspace, Cargo places artifacts in `${WORKSPACE_DIR}/target`.
# Older standalone builds may place artifacts in `${PROJECT_DIR}/target`.
if [[ -f "${WORKSPACE_BINARY_PATH}" ]]; then
  BINARY_PATH="${WORKSPACE_BINARY_PATH}"
elif [[ -f "${CRATE_BINARY_PATH}" ]]; then
  BINARY_PATH="${CRATE_BINARY_PATH}"
else
  BINARY_PATH=""
fi

# Check if build succeeded
if [[ ! -f "${BINARY_PATH}" ]]; then
  log_error "Build failed - binary not found at expected location: ${BINARY_PATH}"
  log_error "Searched:"
  log_error "  - ${WORKSPACE_BINARY_PATH}"
  log_error "  - ${CRATE_BINARY_PATH}"
  exit 1
fi

# Fix program header ordering for uClibc-ng compatibility.
# LLD places PT_TLS before PT_DYNAMIC, but uClibc-ng 1.0.54's linker has a
# `break` in the PT_TLS handler that prevents PT_DYNAMIC from being processed,
# causing SIGSEGV at _dl_get_ready_to_run. Swap the entries so PT_DYNAMIC
# is processed first.
FIX_PHDR="${SCRIPT_DIR}/fix_phdr_order.py"
if [[ -f "${FIX_PHDR}" ]]; then
  log_info "Fixing program header order (uClibc-ng PT_TLS/PT_DYNAMIC workaround)..."
  uv run "${FIX_PHDR}" "${BINARY_PATH}" --verbose
fi

log_success "Build completed successfully!"
log_info "Binary location: ${BINARY_PATH}"
log_info "Binary size: $(du -h "${BINARY_PATH}" | cut -f1)"

# Show binary information
if command -v file &> /dev/null; then
  log_info "Binary type: $(file "${BINARY_PATH}")"
  if ! file "${BINARY_PATH}" | grep -q "ELF 32-bit.*ARM"; then
    log_error "Refusing to deploy: produced binary does not look like an ARMv5 32-bit ELF."
    log_error "This often happens if you built for the host (x86_64) and then copied it to the camera."
    exit 1
  fi
fi

# Copy binary to deployment directory
DEPLOY_DIR="${ANYKA_REPO_ROOT}/SD_card_contents/anyka_hack/onvif"
mkdir -p "${DEPLOY_DIR}"
cp "${BINARY_PATH}" "${DEPLOY_DIR}/onvif-rust.bin"
chmod 755 "${DEPLOY_DIR}/onvif-rust.bin"

# Record the version this binary reports as FirmwareVersion so the bundle
# script can reuse it for manifest.meta. build.rs honors ANYKA_BUILD_VERSION
# (falling back to git describe); capture whichever value landed.
VERSION="${ANYKA_BUILD_VERSION:-$(git -C "${ANYKA_REPO_ROOT}" describe --tags --always --dirty)}"
printf '%s\n' "${VERSION}" > "${DEPLOY_DIR}/.build-version"
log_info "build version: ${VERSION}"

cat > "${DEPLOY_DIR}/onvif-rust" <<'EOF'
#!/bin/sh
# Launcher for ONVIF Rust server.
# The binary has RPATH embedded — no LD_LIBRARY_PATH manipulation needed.
# Do NOT export LD_LIBRARY_PATH here; it poisons child processes
# (busybox/date linked against stock uClibc 0.9.33.2).
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
exec "${SCRIPT_DIR}/onvif-rust.bin" "$@"
EOF
chmod 755 "${DEPLOY_DIR}/onvif-rust"
log_success "Binary and launcher copied to deployment directory:"
log_info "  - ${DEPLOY_DIR}/onvif-rust.bin"
log_info "  - ${DEPLOY_DIR}/onvif-rust"

# Copy vendor-daemon if built
VENDOR_DAEMON_BIN="${WORKSPACE_DIR}/vendor-daemon/build/vendor-daemon.bin"
VENDOR_DAEMON_DEPLOY="${ANYKA_REPO_ROOT}/SD_card_contents/anyka_hack/vendor-daemon"
if [[ -f "${VENDOR_DAEMON_BIN}" ]]; then
  mkdir -p "${VENDOR_DAEMON_DEPLOY}"
  cp "${VENDOR_DAEMON_BIN}" "${VENDOR_DAEMON_DEPLOY}/vendor-daemon.bin"
  chmod 755 "${VENDOR_DAEMON_DEPLOY}/vendor-daemon.bin"
  log_success "Vendor daemon copied to deployment directory"
  log_info "  - ${VENDOR_DAEMON_DEPLOY}/vendor-daemon.bin"
fi

echo ""
log_info "To verify the binary, run:"
log_info "  ${SCRIPT_DIR}/verify_binary.sh"
