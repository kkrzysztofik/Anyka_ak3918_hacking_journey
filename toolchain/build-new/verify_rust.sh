#!/bin/bash
# Verification script for Rust toolchain
# Tests Rust installation and cross-compilation capability for armv5te-unknown-linux-uclibceabi

set -e

# Script directory — must be set before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

# Rust target triple (distinct from the GCC TARGET_TUPLE in common.sh)
TARGET_NAME="armv5te-unknown-linux-uclibceabi"

RUSTC="${INSTALL_DIR}/bin/rustc"
CARGO="${INSTALL_DIR}/bin/cargo"

if [ ! -f "${RUSTC}" ]; then
    log_error "rustc not found: ${RUSTC}"
    exit 1
fi

log_info "Rust toolchain verification"
log_info "Installation location: ${INSTALL_DIR}"
log_info "Target: ${TARGET_NAME}"
if [ -d "${SYSROOT}" ]; then
    log_info "Sysroot: ${SYSROOT}"
else
    log_warn "Sysroot not found: ${SYSROOT}"
fi

# Check for OpenSSL build dependencies (needed for xiu project with native-tls-vendored)
log_info "Checking OpenSSL build dependencies (for xiu project)..."
OPENSSL_DEPS=("perl" "pkg-config" "make" "gcc")
MISSING_OPENSSL_DEPS=()
for dep in "${OPENSSL_DEPS[@]}"; do
    if ! command -v "${dep}" &> /dev/null; then
        MISSING_OPENSSL_DEPS+=("${dep}")
    fi
done
if [ ${#MISSING_OPENSSL_DEPS[@]} -ne 0 ]; then
    log_warn "Missing OpenSSL build dependencies: ${MISSING_OPENSSL_DEPS[*]}"
    log_warn "These are required for building xiu with native-tls-vendored feature"
    log_warn "Install with: sudo apt-get install -y ${MISSING_OPENSSL_DEPS[*]}"
else
    log_info "OpenSSL build dependencies satisfied"
fi

# Test 1: Check rustc version
log_info "Test 1: Checking rustc version..."
"${RUSTC}" --version

# Test 2: Check target availability
log_info "Test 2: Checking target availability..."
TARGET_LIST=$("${RUSTC}" --print target-list 2>/dev/null || echo "")
if echo "${TARGET_LIST}" | grep -q "${TARGET_NAME}"; then
    log_info "Target ${TARGET_NAME} is available!"
else
    log_error "Target ${TARGET_NAME} not found in target list"
    log_info "Available targets:"
    echo "${TARGET_LIST}" | head -20
    exit 1
fi

# Test 3: Check target info
log_info "Test 3: Checking target specification..."
"${RUSTC}" --target "${TARGET_NAME}" --print cfg 2>&1 | head -10 || {
    log_warn "Could not print target cfg (may need std library built)"
}

# Test 4: Compile a simple Rust program
log_info "Test 4: Compiling test Rust program for ${TARGET_NAME}..."
TEST_DIR=$(mktemp -d)
TEST_SRC="${TEST_DIR}/main.rs"
TEST_BIN="${TEST_DIR}/main"

cat > "${TEST_SRC}" << 'EOF'
fn main() {
    println!("Hello from Rust!");
    let x: i32 = 42;
    let y: i32 = 10;
    let z = x + y;
    println!("Result: {}", z);
}
EOF

    # Use clang-wrapper.sh which handles the triple translation:
    #   arm-unknown-linux-uclibcgnueabi -> armv5te-unknown-linux-gnueabi
    # The wrapper also provides sysroot, -march, -mfloat-abi, and GCC crt/lib paths.
    "${RUSTC}" \
    --target "${TARGET_NAME}" \
    --crate-type bin \
    -C linker="${SCRIPT_DIR}/clang-wrapper.sh" \
    -o "${TEST_BIN}" \
    "${TEST_SRC}" 2>&1 || {
    log_error "Failed to compile test Rust program"
    log_info "This may be expected if std library is not yet built for the target"
    log_info "Run: ./bootstrap_rust.sh to build the std library"
    rm -rf "${TEST_DIR}"
    exit 1
}

if [ -f "${TEST_BIN}" ]; then
    log_info "Test binary compiled successfully"

    # Check file type
    if command -v file &> /dev/null; then
        FILE_TYPE=$(file "${TEST_BIN}")
        log_info "Binary type: ${FILE_TYPE}"

        if echo "${FILE_TYPE}" | grep -qi "arm"; then
            log_info "Binary is ARM architecture - OK"
        else
            log_warn "Binary architecture may be incorrect"
        fi

        if echo "${FILE_TYPE}" | grep -qi "statically linked"; then
            log_info "Binary is statically linked - OK"
        fi
    fi

    # Check for VFP instructions (should not have any)
    if command -v "${INSTALL_DIR}/bin/arm-unknown-linux-uclibcgnueabi-objdump" &> /dev/null; then
        OBJDUMP="${INSTALL_DIR}/bin/arm-unknown-linux-uclibcgnueabi-objdump"
        # Check for actual VFP/NEON instruction mnemonics in disassembly
        # Look for instruction patterns: vadd, vsub, vmul, vldr, vstr, vldm, vstm, flds, fsts, etc.
        # Exclude function names and addresses by looking for tab/space before instruction
        VFP_FOUND=$("${OBJDUMP}" -d "${TEST_BIN}" 2>/dev/null | \
            grep -iE "^[[:space:]]*[0-9a-f]+:[[:space:]]+[0-9a-f]+[[:space:]]+(vadd|vsub|vmul|vdiv|vldr|vstr|vldm|vstm|flds|fsts|fldd|fstd|fmrx|fmxr|vmov|vabs|vneg)" | \
            head -1)
        if [ -n "${VFP_FOUND}" ]; then
            log_error "VFP/NEON instructions found in binary! This is not compatible with ARMv5TEJ"
            log_error "Found: ${VFP_FOUND}"
            rm -rf "${TEST_DIR}"
            exit 1
        else
            log_info "No VFP/NEON instructions found - OK"
        fi
    fi
else
    log_error "Test binary was not created"
    rm -rf "${TEST_DIR}"
    exit 1
fi

# Test 5: Check if cargo is available
if [ -f "${CARGO}" ]; then
    log_info "Test 5: Checking cargo..."
    "${CARGO}" --version

    # Try to create a simple project
    log_info "Test 6: Testing cargo project creation..."
    CARGO_PROJECT="${TEST_DIR}/cargo_test"
    "${CARGO}" new --bin "${CARGO_PROJECT}" 2>&1 | head -5 || log_warn "Cargo project creation had issues"

    if [ -d "${CARGO_PROJECT}" ]; then
        log_info "Cargo project created successfully"
        # Configure linker for cross-compilation via .cargo/config.toml
        mkdir -p "${CARGO_PROJECT}/.cargo"
        cat > "${CARGO_PROJECT}/.cargo/config.toml" << CARGO_EOF
[target.${TARGET_NAME}]
linker = "${SCRIPT_DIR}/clang-wrapper.sh"
CARGO_EOF
        # Try to build (may fail if std not available)
        cd "${CARGO_PROJECT}"
        "${CARGO}" build --target "${TARGET_NAME}" 2>&1 | head -10 || {
            log_warn "Cargo build failed (may be expected if std library not built)"
        }
    fi
else
    log_warn "cargo not found at: ${CARGO}"
    log_warn "cargo is required for building Rust projects (like xiu)"

    # Check if system cargo is available
    SYSTEM_CARGO=$(which cargo 2>/dev/null || echo "")
    if [ -n "${SYSTEM_CARGO}" ] && [ -f "${SYSTEM_CARGO}" ]; then
        log_info "Found system cargo at: ${SYSTEM_CARGO}"
        log_info "Creating symlink to use system cargo..."
        mkdir -p "$(dirname "${CARGO}")"
        ln -sf "${SYSTEM_CARGO}" "${CARGO}" 2>/dev/null && {
            log_info "Symlink created successfully"
            log_info "Testing cargo..."
            "${CARGO}" --version || log_warn "cargo symlink created but version check failed"
        } || {
            log_warn "Failed to create symlink"
            log_warn ""
            log_warn "To fix this manually, run:"
            log_warn "  mkdir -p $(dirname ${CARGO})"
            log_warn "  ln -s ${SYSTEM_CARGO} ${CARGO}"
        }
    else
        log_warn ""
        log_warn "To fix this, you have a few options:"
        log_warn "1. Re-run bootstrap_rust.sh - it should install cargo automatically"
        log_warn "2. If cargo exists elsewhere, create a symlink:"
        log_warn "   ln -s \$(which cargo) ${CARGO}"
        log_warn "3. Check if cargo was built but not installed:"
        log_warn "   find ${INSTALL_DIR}/.. -name cargo -type f 2>/dev/null"
        log_warn ""
        log_warn "Note: rustc is available and working, but cargo is needed for project builds"
    fi
fi

# Test 7: Check std library availability
log_info "Test 7: Checking std library availability..."
STD_LIB="${INSTALL_DIR}/lib/rustlib/${TARGET_NAME}/lib/libstd-*.rlib"
if ls ${STD_LIB} 1> /dev/null 2>&1; then
    log_info "Std library found for target"
    ls -1 ${STD_LIB} | head -3
else
    log_warn "Std library not found for target"
    log_info "This is expected if std library has not been built yet"
    log_info "The bootstrap process should build it"
fi

# Test 8: Check rust-analyzer-proc-macro-srv binary
log_info "Test 8: Checking rust-analyzer-proc-macro-srv..."
PROC_MACRO_SRV=""
if [[ -f "${INSTALL_DIR}/libexec/rust-analyzer-proc-macro-srv" ]]; then
    PROC_MACRO_SRV="${INSTALL_DIR}/libexec/rust-analyzer-proc-macro-srv"
elif [[ -f "${INSTALL_DIR}/bin/rust-analyzer-proc-macro-srv" ]]; then
    PROC_MACRO_SRV="${INSTALL_DIR}/bin/rust-analyzer-proc-macro-srv"
fi

if [[ -n "${PROC_MACRO_SRV}" ]]; then
    log_info "rust-analyzer-proc-macro-srv found: ${PROC_MACRO_SRV}"
else
    log_warn "rust-analyzer-proc-macro-srv not found in libexec/ or bin/"
    log_warn "Proc-macro expansion in rust-analyzer will be disabled"
    log_warn "To rebuild with proc-macro-srv, run: ./bootstrap_rust.sh"
    log_warn "Expected locations:"
    log_warn "  ${INSTALL_DIR}/libexec/rust-analyzer-proc-macro-srv"
    log_warn "  ${INSTALL_DIR}/bin/rust-analyzer-proc-macro-srv"
fi

# Test 9: Check rust-analyzer LSP binary
log_info "Test 9: Checking rust-analyzer..."
RUST_ANALYZER="${INSTALL_DIR}/bin/rust-analyzer"
if [[ -f "${RUST_ANALYZER}" ]]; then
    log_info "rust-analyzer found: ${RUST_ANALYZER}"
    "${RUST_ANALYZER}" --version 2>/dev/null && log_info "rust-analyzer version OK" || log_warn "rust-analyzer binary exists but --version failed"
else
    log_warn "rust-analyzer not found at: ${RUST_ANALYZER}"
    log_warn "IDE language server features (hover, goto-def, completion) will be unavailable"
    log_warn "To rebuild with rust-analyzer, run: ./bootstrap_rust.sh"
fi

# Cleanup
rm -rf "${TEST_DIR}"

log_info "=========================================="
log_info "Rust toolchain verification completed!"
log_info "=========================================="
log_info "Target ${TARGET_NAME} is ready to use!"
log_info "To use in your projects, add to .cargo/config.toml:"
log_info "  [build]"
log_info "  target = \"${TARGET_NAME}\""
log_info ""
log_info "  [target.${TARGET_NAME}]"
log_info "  linker = \"${SCRIPT_DIR}/clang-wrapper.sh\""
log_info ""
log_info "  The clang-wrapper.sh script handles the ARMv5TE triple translation,"
log_info "  sysroot, and GCC crt/lib paths automatically."
