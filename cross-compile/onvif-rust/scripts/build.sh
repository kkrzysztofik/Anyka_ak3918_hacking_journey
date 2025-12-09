#!/bin/bash
# Build script for ONVIF Rust cross-compilation
# Builds the Rust ONVIF application for ARMv5TE target

set -e

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Go up 1 level: scripts -> onvif-rust
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

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
  echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
  echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $1"
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
      log_error "Unknown option: $1"
      echo "Use --help for usage information"
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

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
  log_error "cargo not found. Please ensure Rust toolchain is installed and in PATH."
  exit 1
fi

# Check if rustc is available
if ! command -v rustc &> /dev/null; then
  log_error "rustc not found. Please ensure Rust toolchain is installed and in PATH."
  exit 1
fi

log_info "Using cargo: $(which cargo)"
log_info "Using rustc: $(which rustc)"

# Clean if requested
if [ "${CLEAN}" = true ]; then
  log_info "Cleaning build artifacts..."
  cargo clean
fi

# Build the project
log_info "Building for target ${TARGET} in ${BUILD_MODE} mode..."

if [ "${BUILD_MODE}" = "release" ]; then
  cargo build --release --target "${TARGET}"
  BINARY_PATH="${PROJECT_DIR}/target/${TARGET}/release/onvif-rust"
else
  cargo build --target "${TARGET}"
  BINARY_PATH="${PROJECT_DIR}/target/${TARGET}/debug/onvif-rust"
fi

# Check if build succeeded
if [ ! -f "${BINARY_PATH}" ]; then
  log_error "Build failed - binary not found at expected location: ${BINARY_PATH}"
  exit 1
fi

log_success "Build completed successfully!"
log_info "Binary location: ${BINARY_PATH}"
log_info "Binary size: $(du -h "${BINARY_PATH}" | cut -f1)"

# Show binary information
if command -v file &> /dev/null; then
  log_info "Binary type: $(file "${BINARY_PATH}")"
fi

# Copy binary to deployment directory
DEPLOY_DIR="/home/kmk/anyka-dev/SD_card_contents/anyka_hack/onvif"
cp "${BINARY_PATH}" "${DEPLOY_DIR}/onvif-rust"
chmod 755 "${DEPLOY_DIR}/onvif-rust"
log_success "Binary copied to deployment directory: ${DEPLOY_DIR}/onvif-rust"

echo ""
log_info "To verify the binary, run:"
log_info "  ${SCRIPT_DIR}/verify_binary.sh"
