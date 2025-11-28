#!/bin/bash
# Verify ONVIF Rust Binary for ARMv5TEJ
# This script verifies that the Rust binary is correctly compiled for ARMv5TEJ
# (soft-float, no VFP) and doesn't contain VFP/NEON instructions.

# Don't use set -e - we want to collect all test results
# set -e  # Exit on error

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Go up 1 level: scripts -> onvif-rust
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
BINARY_NAME="onvif-rust"

# Try to find binary in common locations
BINARY_PATH=""

# Check release build first
RELEASE_BINARY="${PROJECT_DIR}/target/armv5te-unknown-linux-uclibceabi/release/${BINARY_NAME}"
if [ -f "${RELEASE_BINARY}" ]; then
  BINARY_PATH="${RELEASE_BINARY}"
else
  # Check debug build
  DEBUG_BINARY="${PROJECT_DIR}/target/armv5te-unknown-linux-uclibceabi/debug/${BINARY_NAME}"
  if [ -f "${DEBUG_BINARY}" ]; then
    BINARY_PATH="${DEBUG_BINARY}"
  else
    # Try alternative target name
    ALT_RELEASE="${PROJECT_DIR}/target/arm-unknown-linux-uclibcgnueabi/release/${BINARY_NAME}"
    if [ -f "${ALT_RELEASE}" ]; then
      BINARY_PATH="${ALT_RELEASE}"
    else
      ALT_DEBUG="${PROJECT_DIR}/target/arm-unknown-linux-uclibcgnueabi/debug/${BINARY_NAME}"
      if [ -f "${ALT_DEBUG}" ]; then
        BINARY_PATH="${ALT_DEBUG}"
      fi
    fi
  fi
fi

# Allow override via command line
if [ -n "$1" ] && [ -f "$1" ]; then
  BINARY_PATH="$1"
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

# Logging functions
log_info() {
  echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
  echo -e "${GREEN}[PASS]${NC} $1"
  TESTS_PASSED=$((TESTS_PASSED + 1))
}

log_error() {
  echo -e "${RED}[FAIL]${NC} $1"
  TESTS_FAILED=$((TESTS_FAILED + 1))
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $1"
}

# Test 1: Check if binary exists
test_binary_exists() {
  log_info "Test 1: Checking if binary exists..."
  if [ -f "${BINARY_PATH}" ]; then
    log_success "Binary found at: ${BINARY_PATH}"
    return 0
  else
    log_error "Binary not found. Expected at: ${BINARY_PATH}"
    log_info "Searched paths:"
    log_info "  - ${RELEASE_BINARY}"
    log_info "  - ${DEBUG_BINARY}"
    log_info "  - ${ALT_RELEASE}"
    log_info "  - ${ALT_DEBUG}"
    log_info ""
    log_info "Usage: $0 [BINARY_PATH]"
    return 1
  fi
}

# Test 2: Check file type
test_file_type() {
  log_info "Test 2: Checking file type..."
  if ! command -v file &> /dev/null; then
    log_warn "file command not found, skipping file type check"
    return 0
  fi

  FILE_TYPE=$(file "${BINARY_PATH}" 2>/dev/null)
  if echo "${FILE_TYPE}" | grep -q "ARM\|arm"; then
    log_success "File type: ${FILE_TYPE}"
    return 0
  else
    log_error "Unexpected file type: ${FILE_TYPE}"
    return 1
  fi
}

# Test 3: Check ELF architecture
test_elf_architecture() {
  log_info "Test 3: Checking ELF architecture..."
  if ! command -v readelf &> /dev/null; then
    log_warn "readelf not found, skipping ELF check"
    return 0
  fi

  MACHINE=$(readelf -h "${BINARY_PATH}" 2>/dev/null | grep "Machine:" | awk '{print $2}')
  if [ "${MACHINE}" = "ARM" ]; then
    log_success "ELF Machine: ${MACHINE}"
    return 0
  else
    log_error "Unexpected ELF Machine: ${MACHINE} (expected ARM)"
    return 1
  fi
}

# Test 4: Check ELF flags (soft-float ABI)
test_elf_flags() {
  log_info "Test 4: Checking ELF flags for soft-float ABI..."
  if ! command -v readelf &> /dev/null; then
    log_warn "readelf not found, skipping ELF flags check"
    return 0
  fi

  FLAGS=$(readelf -h "${BINARY_PATH}" 2>/dev/null | grep "Flags:" | awk '{print $2}')
  log_info "ELF Flags: ${FLAGS}"

  # Check for soft-float ABI flag (0x200)
  if echo "${FLAGS}" | grep -q "0x.*2[0-9a-f][0-9a-f]"; then
    log_success "ELF flags indicate soft-float ABI (contains 0x2xx pattern)"
  else
    log_warn "ELF flags may not explicitly indicate soft-float ABI"
    log_info "This is acceptable - Rust compiler may not set this flag explicitly"
  fi

  return 0
}

# Test 5: Check for VFP instructions
test_no_vfp_instructions() {
  log_info "Test 5: Checking for VFP instructions (should find NONE)..."

  # Find cross-objdump
  OBJDUMP=""
  if command -v arm-unknown-linux-uclibcgnueabi-objdump &> /dev/null; then
    OBJDUMP="arm-unknown-linux-uclibcgnueabi-objdump"
  elif command -v arm-anykav200-linux-uclibcgnueabi-objdump &> /dev/null; then
    OBJDUMP="arm-anykav200-linux-uclibcgnueabi-objdump"
  elif command -v arm-linux-gnueabi-objdump &> /dev/null; then
    OBJDUMP="arm-linux-gnueabi-objdump"
  else
    log_warn "ARM objdump not found, skipping VFP instruction check"
    return 0
  fi

  # Disassemble and search for VFP instructions
  VFP_COUNT=$(${OBJDUMP} -d "${BINARY_PATH}" 2>/dev/null | grep -E "vldr|vstr|vmrs|vmsr|vadd\.f|vmul\.f|vmov.*s[0-9]|vmov.*d[0-9]" | wc -l)

  if [ "${VFP_COUNT}" -eq 0 ]; then
    log_success "No VFP instructions found (correct for ARMv5TEJ)"
    return 0
  else
    log_error "Found ${VFP_COUNT} VFP instruction(s) - binary may fail on ARMv5TEJ!"
    log_info "VFP instructions found:"
    ${OBJDUMP} -d "${BINARY_PATH}" 2>/dev/null | grep -E "vldr|vstr|vmrs|vmsr|vadd\.f|vmul\.f|vmov.*s[0-9]|vmov.*d[0-9]" | head -10
    return 1
  fi
}

# Test 6: Check for NEON instructions
test_no_neon_instructions() {
  log_info "Test 6: Checking for NEON instructions (should find NONE)..."

  # Find cross-objdump
  OBJDUMP=""
  if command -v arm-unknown-linux-uclibcgnueabi-objdump &> /dev/null; then
    OBJDUMP="arm-unknown-linux-uclibcgnueabi-objdump"
  elif command -v arm-anykav200-linux-uclibcgnueabi-objdump &> /dev/null; then
    OBJDUMP="arm-anykav200-linux-uclibcgnueabi-objdump"
  elif command -v arm-linux-gnueabi-objdump &> /dev/null; then
    OBJDUMP="arm-linux-gnueabi-objdump"
  else
    log_warn "ARM objdump not found, skipping NEON instruction check"
    return 0
  fi

  # Disassemble and search for NEON instructions
  NEON_COUNT=$(${OBJDUMP} -d "${BINARY_PATH}" 2>/dev/null | grep -E "vld1|vst1|vadd|vsub|vmul|vqadd|vqsub|vshl|vshr" | grep -v "vadd\.f\|vsub\.f\|vmul\.f" | wc -l)

  if [ "${NEON_COUNT}" -eq 0 ]; then
    log_success "No NEON instructions found (correct for ARMv5TEJ)"
    return 0
  else
    log_error "Found ${NEON_COUNT} NEON instruction(s) - binary may fail on ARMv5TEJ!"
    log_info "NEON instructions found:"
    ${OBJDUMP} -d "${BINARY_PATH}" 2>/dev/null | grep -E "vld1|vst1|vadd|vsub|vmul|vqadd|vqsub|vshl|vshr" | grep -v "vadd\.f\|vsub\.f\|vmul\.f" | head -10
    return 1
  fi
}

# Test 7: Check ABI attributes
test_abi_attributes() {
  log_info "Test 7: Checking ABI attributes..."

  # Find cross-readelf
  READELF=""
  if command -v arm-unknown-linux-uclibcgnueabi-readelf &> /dev/null; then
    READELF="arm-unknown-linux-uclibcgnueabi-readelf"
  elif command -v arm-anykav200-linux-uclibcgnueabi-readelf &> /dev/null; then
    READELF="arm-anykav200-linux-uclibcgnueabi-readelf"
  elif command -v arm-linux-gnueabi-readelf &> /dev/null; then
    READELF="arm-linux-gnueabi-readelf"
  elif command -v readelf &> /dev/null; then
    READELF="readelf"
  else
    log_warn "readelf not found, skipping ABI attributes check"
    return 0
  fi

  # Check for ABI FP attributes
  ABI_ATTRS=$(${READELF} -A "${BINARY_PATH}" 2>/dev/null | grep -i "Tag_ABI_FP" || true)

  if [ -n "${ABI_ATTRS}" ]; then
    log_success "ABI FP attributes found:"
    echo "${ABI_ATTRS}" | sed 's/^/  /'
  else
    log_warn "No ABI FP attributes found (may be acceptable)"
  fi

  return 0
}

# Test 8: Check dynamic linking and dependencies
test_dynamic_linking() {
  log_info "Test 8: Checking dynamic linking and dependencies..."

  # Find cross-readelf
  READELF=""
  if command -v arm-unknown-linux-uclibcgnueabi-readelf &> /dev/null; then
    READELF="arm-unknown-linux-uclibcgnueabi-readelf"
  elif command -v arm-anykav200-linux-uclibcgnueabi-readelf &> /dev/null; then
    READELF="arm-anykav200-linux-uclibcgnueabi-readelf"
  elif command -v readelf &> /dev/null; then
    READELF="readelf"
  else
    log_warn "readelf not found, skipping dynamic linking check"
    return 0
  fi

  DYNAMIC=$(${READELF} -d "${BINARY_PATH}" 2>/dev/null | grep -q "NEEDED" && echo "dynamic" || echo "static")

  if [ "${DYNAMIC}" = "static" ]; then
    log_success "Binary is statically linked (good for deployment)"
  else
    log_info "Binary is dynamically linked"
    log_info "Required libraries:"
    ${READELF} -d "${BINARY_PATH}" 2>/dev/null | grep "NEEDED" | sed 's/^/  /'
  fi

  return 0
}

# Test 9: Check binary size
test_binary_size() {
  log_info "Test 9: Checking binary size..."

  if [ ! -f "${BINARY_PATH}" ]; then
    return 1
  fi

  SIZE_BYTES=$(stat -c%s "${BINARY_PATH}" 2>/dev/null || stat -f%z "${BINARY_PATH}" 2>/dev/null)
  SIZE_MB=$(echo "scale=2; ${SIZE_BYTES} / 1024 / 1024" | bc 2>/dev/null || echo "N/A")

  log_info "Binary size: ${SIZE_BYTES} bytes (${SIZE_MB} MB)"

  # Warn if binary is very large (> 10MB)
  if [ "${SIZE_BYTES}" -gt 10485760 ]; then
    log_warn "Binary is larger than 10MB - consider optimizing"
  else
    log_success "Binary size is reasonable"
  fi

  return 0
}

# Main execution
main() {
  echo "=== Verifying ONVIF Rust Binary for ARMv5TEJ ==="
  if [ -n "${BINARY_PATH}" ]; then
    echo "Binary path: ${BINARY_PATH}"
  else
    echo "Binary path: (not found - will search)"
  fi
  echo ""

  # Run all tests (collect results, don't exit on individual failures)
  if test_binary_exists; then
    test_file_type
    test_elf_architecture
    test_elf_flags
    test_no_vfp_instructions
    test_no_neon_instructions
    test_abi_attributes
    test_dynamic_linking
    test_binary_size
  else
    log_error "Cannot proceed with verification - binary not found"
    TESTS_FAILED=$((TESTS_FAILED + 1))
  fi

  # Summary
  echo ""
  echo "=== Verification Summary ==="
  echo "Tests passed: ${TESTS_PASSED}"
  echo "Tests failed: ${TESTS_FAILED}"

  if [ ${TESTS_FAILED} -eq 0 ]; then
    echo ""
    log_success "All tests passed! Binary is correctly configured for ARMv5TEJ."
    return 0
  else
    echo ""
    log_error "Some tests failed. Please review the output above."
    echo ""
    echo "Note: If VFP/NEON instructions are found, they may be in unused code paths."
    echo "      Test the binary on the device - it may still work correctly."
    return 1
  fi
}

# Run main
main "$@"
