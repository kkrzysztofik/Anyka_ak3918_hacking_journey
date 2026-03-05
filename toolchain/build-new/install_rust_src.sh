#!/bin/bash
# Quick script to install rust-src component to existing custom Rust toolchain
# This is a lightweight alternative to rebuilding the entire toolchain

set -e  # Exit on error

# Script directory — must be set before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

# Check if Rust source directory exists
if [[ ! -d "${RUST_SRC_DIR}" ]]; then
    log_error "Rust source directory not found: ${RUST_SRC_DIR}"
    log_error "Please run bootstrap_rust.sh first to build the complete toolchain"
    exit 1
fi

# Check if custom toolchain exists
if [[ ! -d "${INSTALL_DIR}" ]]; then
    log_error "Custom toolchain not found: ${INSTALL_DIR}"
    log_error "Please run bootstrap_rust.sh first to build the toolchain"
    exit 1
fi

log_info "Installing rust-src component to custom toolchain..."
log_info "Rust source: ${RUST_SRC_DIR}"
log_info "Install directory: ${INSTALL_DIR}"

cd "${RUST_SRC_DIR}"

# Ensure cargo/rustc are in PATH
export PATH="${HOME}/.cargo/bin:${PATH}"

# Clear cross-compilation environment variables
unset CC CXX AR RANLIB CFLAGS CXXFLAGS LDFLAGS
unset TARGET_CC TARGET_CXX TARGET_AR TARGET_RANLIB
export CC="gcc"
export CXX="g++"
export AR="ar"
export RANLIB="ranlib"

log_info "Installing rust-src component (copying library sources)..."
log_info "Note: rust-src is just source code - no compilation needed, so memchr errors are not a problem"
RUST_SRC_PATH="${INSTALL_DIR}/lib/rustlib/src/rust"

# Create destination directory
mkdir -p "${RUST_SRC_PATH}" || {
    log_error "Failed to create rust-src destination directory: ${RUST_SRC_PATH}"
    exit 1
}

# Copy library source files (matches bootstrap dist.rs implementation)
if [[ -d "${RUST_SRC_DIR}/library" ]]; then
    log_info "Copying library sources from ${RUST_SRC_DIR}/library..."
    # Try rsync with exclusions first (matches bootstrap behavior), fallback to cp
    if command -v rsync >/dev/null 2>&1; then
        if rsync -a --exclude='backtrace/crates' \
              --exclude='stdarch/Cargo.toml' \
              --exclude='stdarch/crates/stdarch-verify' \
              --exclude='stdarch/crates/intrinsic-test' \
              "${RUST_SRC_DIR}/library/" "${RUST_SRC_PATH}/library/" \
              > >(tee -a "${SCRIPT_DIR}/rust_src_install.log") 2>&1; then
            log_info "Library sources copied successfully"
        else
            log_warn "rsync failed, falling back to cp"
            if cp -r "${RUST_SRC_DIR}/library" "${RUST_SRC_PATH}/" \
                > >(tee -a "${SCRIPT_DIR}/rust_src_install.log") 2>&1; then
                log_info "Library sources copied successfully"
            else
                log_error "Failed to copy library sources"
                exit 1
            fi
        fi
    else
        log_warn "rsync not found, falling back to cp"
        if cp -r "${RUST_SRC_DIR}/library" "${RUST_SRC_PATH}/" \
            > >(tee -a "${SCRIPT_DIR}/rust_src_install.log") 2>&1; then
            log_info "Library sources copied successfully"
        else
            log_error "Failed to copy library sources"
            exit 1
        fi
    fi
else
    log_error "Library source directory not found: ${RUST_SRC_DIR}/library"
    exit 1
fi

# Copy libunwind source (needed for some std library dependencies)
if [[ -d "${RUST_SRC_DIR}/src/llvm-project/libunwind" ]]; then
    log_info "Copying libunwind sources..."
    mkdir -p "${RUST_SRC_PATH}/src/llvm-project"
    cp -r "${RUST_SRC_DIR}/src/llvm-project/libunwind" "${RUST_SRC_PATH}/src/llvm-project/" 2>&1 | tee -a "${SCRIPT_DIR}/rust_src_install.log" || {
        log_warn "Failed to copy libunwind sources (non-critical)"
    }
fi

# Create version file
RUST_VERSION=$("${INSTALL_DIR}/bin/rustc" --version 2>/dev/null | cut -d' ' -f2 || echo "unknown")
echo "${RUST_VERSION}" > "${RUST_SRC_PATH}/version" 2>/dev/null || true

# Register rust-src in components file
COMPONENTS_FILE="${INSTALL_DIR}/lib/rustlib/components"
if [[ -f "${COMPONENTS_FILE}" ]] && ! grep -q "rust-src" "${COMPONENTS_FILE}"; then
    echo "rust-src" >> "${COMPONENTS_FILE}"
    log_info "Registered rust-src in components file"
fi

log_info "rust-src component installed successfully"

# Verify installation
log_info "Verifying rust-src installation..."

if [[ -d "${RUST_SRC_PATH}" ]]; then
    log_info "✓ rust-src component installed at: ${RUST_SRC_PATH}"
    
    # Check for key library directories
    if [[ -d "${RUST_SRC_PATH}/library/std" ]] && [[ -d "${RUST_SRC_PATH}/library/core" ]]; then
        log_info "✓ Standard library source code verified (std, core)"
    else
        log_warn "rust-src directory exists but library sources not found"
    fi
else
    log_error "rust-src component not found at expected location: ${RUST_SRC_PATH}"
    exit 1
fi

# Check components file
COMPONENTS_FILE="${INSTALL_DIR}/lib/rustlib/components"
if [[ -f "${COMPONENTS_FILE}" ]]; then
    if grep -q "rust-src" "${COMPONENTS_FILE}"; then
        log_info "✓ rust-src component registered in components file"
    else
        log_warn "rust-src not found in components file, but directory exists"
    fi
fi

log_info "=========================================="
log_info "rust-src installation completed successfully!"
log_info "=========================================="
log_info "Location: ${RUST_SRC_PATH}"
log_info ""
log_info "Next steps:"
log_info "1. Restart VS Code or reload the window"
log_info "2. rust-analyzer should now be able to load standard library sources"
log_info "3. Check rust-analyzer output panel for any remaining errors"
