#!/bin/bash
# Build script for ONVIF Rust cross-compilation
# Builds the Rust ONVIF application for ARMv5TE target

set -e

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Go up 1 level: scripts -> onvif-rust
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
# Workspace root: onvif-rust -> cross-compile
WORKSPACE_DIR="$(cd "${PROJECT_DIR}/.." && pwd)"

# Default values
BUILD_MODE="release"
TARGET="armv5te-unknown-linux-uclibceabi"
CLEAN=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
  local message="$1"
  echo -e "${BLUE}[INFO]${NC} ${message}"
  return 0
}

log_success() {
  local message="$1"
  echo -e "${GREEN}[SUCCESS]${NC} ${message}"
  return 0
}

log_error() {
  local message="$1"
  echo -e "${RED}[ERROR]${NC} ${message}" >&2
  return 0
}

log_warn() {
  local message="$1"
  echo -e "${YELLOW}[WARN]${NC} ${message}"
  return 0
}

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
    -h|--help)
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --debug          Build in debug mode (default: release)"
      echo "  --release        Build in release mode (default)"
      echo "  --target TARGET  Specify target triple (default: armv5te-unknown-linux-uclibceabi)"
      echo "  --clean          Clean before building"
      echo "  -h, --help       Show this help message"
      echo ""
      exit 0
      ;;
    *)
      local unknown_arg="$1"
      log_error "Unknown option: ${unknown_arg}"
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

# Use vendored toolchain cargo by default (per project requirements)
REPO_ROOT="$(cd "${PROJECT_DIR}/../.." && pwd)"
DEFAULT_CARGO="${REPO_ROOT}/toolchain/arm-anykav200-crosstool-ng/bin/cargo"
export CARGO="${CARGO:-${DEFAULT_CARGO}}"

if [[ ! -x "${CARGO}" ]]; then
  log_error "cargo not found or not executable at: ${CARGO}"
  log_error "Set CARGO to the vendored toolchain cargo, e.g.:"
  log_error "  export CARGO=${DEFAULT_CARGO}"
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

if [[ "${BUILD_MODE}" = "release" ]]; then
  "${CARGO}" build --release --target "${TARGET}"
  WORKSPACE_BINARY_PATH="${WORKSPACE_DIR}/target/${TARGET}/release/onvif-rust"
  CRATE_BINARY_PATH="${PROJECT_DIR}/target/${TARGET}/release/onvif-rust"
else
  "${CARGO}" build --target "${TARGET}"
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
DEPLOY_DIR="${REPO_ROOT}/SD_card_contents/anyka_hack/onvif"
mkdir -p "${DEPLOY_DIR}"
cp "${BINARY_PATH}" "${DEPLOY_DIR}/onvif-rust"
chmod 755 "${DEPLOY_DIR}/onvif-rust"
log_success "Binary copied to deployment directory: ${DEPLOY_DIR}/onvif-rust"

echo ""
log_info "To verify the binary, run:"
log_info "  ${SCRIPT_DIR}/verify_binary.sh"
