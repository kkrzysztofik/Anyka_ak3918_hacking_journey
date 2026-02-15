#!/bin/bash
# Build script for modern toolchain using crosstool-NG
# Supports: ARMv5TEJ (armv5te) and aarch64 architectures

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Architecture selection (default: armv5te)
ARCH="${ARCH:-armv5te}"

# Validate architecture
if [ "${ARCH}" != "armv5te" ] && [ "${ARCH}" != "aarch64" ]; then
    echo "Error: ARCH must be 'armv5te' or 'aarch64'"
    echo "Usage: ARCH=armv5te ./build_toolchain.sh  (default)"
    echo "       ARCH=aarch64 ./build_toolchain.sh"
    exit 1
fi

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}"

# Set installation directory based on architecture
if [ "${ARCH}" = "aarch64" ]; then
    INSTALL_DIR="${SCRIPT_DIR}/../aarch64-unknown-linux-gnu-toolchain"
    TARGET_TUPLE="aarch64-unknown-linux-gnu"
    CONFIG_FILE="crosstool-ng.config.aarch64"
else
    INSTALL_DIR="${SCRIPT_DIR}/../arm-anykav200-crosstool-ng"
    TARGET_TUPLE="arm-unknown-linux-uclibcgnueabi"
    CONFIG_FILE="crosstool-ng.config"
fi

CTNG_VERSION="1.28.0"
CTNG_DIR="${BUILD_DIR}/crosstool-ng-${CTNG_VERSION}"

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check dependencies
check_dependencies() {
    log_info "Checking build dependencies..."

    local deps=(
        "gcc" "make" "libncurses-dev" "gperf" "bison" "flex"
        "texinfo" "help2man" "gawk" "libtool-bin" "automake" "autoconf"
        "wget" "git" "file" "python3" "perl" "pkg-config"
    )

    # Check for libtool command specifically
    if ! command -v libtool &> /dev/null; then
        log_warn "libtool command not found in PATH"
        log_info "Installing libtool-bin (provides libtool command)..."
        log_info "Run: sudo apt-get install -y libtool-bin"
    fi

    local missing=()
    for dep in "${deps[@]}"; do
        # Special handling for libtool-bin - check for libtool command
        if [ "${dep}" = "libtool-bin" ]; then
            if ! command -v libtool &> /dev/null && ! dpkg -l | grep -q "^ii.*libtool-bin"; then
                missing+=("${dep}")
            fi
        elif ! command -v "${dep}" &> /dev/null && ! dpkg -l | grep -q "^ii.*${dep}"; then
            missing+=("${dep}")
        fi
    done

    if [ ${#missing[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing[*]}"
        log_info "Install with: sudo apt-get install -y ${missing[*]}"
        exit 1
    fi

    log_info "All dependencies satisfied"
}

# Download and build crosstool-NG
install_crosstool_ng() {
    log_info "Installing crosstool-NG ${CTNG_VERSION}..."

    if [ -d "${CTNG_DIR}" ]; then
        log_warn "crosstool-NG directory exists, skipping download"
    else
        log_info "Downloading crosstool-NG ${CTNG_VERSION}..."
        cd "${BUILD_DIR}"
        local archive_file="crosstool-ng-${CTNG_VERSION}.tar.xz"
        if ! wget "https://github.com/crosstool-ng/crosstool-ng/releases/download/crosstool-ng-${CTNG_VERSION}/${archive_file}"; then
            log_error "Failed to download crosstool-NG ${CTNG_VERSION}"
            log_error "Please check your network connection and try again"
            exit 1
        fi
        
        log_info "Extracting crosstool-NG ${CTNG_VERSION}..."
        if ! tar -xf "${archive_file}"; then
            log_error "Failed to extract crosstool-NG archive"
            exit 1
        fi
        
        rm "${archive_file}"
        log_info "crosstool-NG ${CTNG_VERSION} downloaded and extracted successfully"
    fi

    log_info "Building crosstool-NG..."
    cd "${CTNG_DIR}"

    if [ ! -f "configure" ]; then
        ./bootstrap
    fi

    # Clean up any previous failed configure attempts
    if [ -f "config.log" ] && ! grep -q "config.status: creating Makefile" "config.log" 2>/dev/null; then
        log_warn "Previous configure may have failed, cleaning up..."
        rm -f Makefile config.status config.cache
    fi

    if [ ! -f "Makefile" ]; then
        # Save original PATH and remove cross-compiler from it
        # crosstool-NG must be built with native compiler
        ORIGINAL_PATH="${PATH}"
        # Remove any cross-compiler paths from PATH temporarily
        CLEAN_PATH=$(echo "${PATH}" | tr ':' '\n' | grep -v "arm-anykav200-crosstool" | grep -v "arm-anykav200-crosstool-ng" | tr '\n' ':' | sed 's/:$//')

        # Explicitly use native gcc
        export PATH="${CLEAN_PATH}"
        export CC="gcc"
        export CXX="g++"

        log_info "Configuring crosstool-NG with native compiler..."
        log_info "Using CC=${CC} (native compiler)"
        ./configure --enable-local

        # Restore PATH
        export PATH="${ORIGINAL_PATH}"
        unset CC
        unset CXX
    fi

    # Apply patch to disable time64 for Linux < 5.1.0 (for ARMv5TE)
    if [ -f "${BUILD_DIR}/disable_time64.patch" ]; then
        log_info "Applying patch to disable time64 support for Linux 3.4.35..."
        if patch -p1 -d "${CTNG_DIR}" < "${BUILD_DIR}/disable_time64.patch" 2>&1 | grep -q "succeeded\|ignored"; then
            log_info "Patch applied successfully"
        else
            log_warn "Patch may have already been applied or failed (this is OK if already patched)"
        fi
    fi

    # Patch Linux build script to handle aarch64 architecture
    # Linux kernel uses "arm64" but crosstool-NG uses "aarch64" as CT_ARCH
    local linux_sh="${CTNG_DIR}/scripts/build/kernel/linux.sh"
    if [ -f "${linux_sh}" ] && ! grep -q "aarch64:\*)" "${linux_sh}"; then
        log_info "Patching Linux build script to support aarch64..."
        sed -i '/arm:64) kernel_arch="arm64";;/a\        # AArch64 architecture (when CT_ARCH is set to "aarch64")\n        aarch64:*) kernel_arch="arm64";;' "${linux_sh}"
        log_info "Linux build script patched for aarch64 support"
    fi

    make -j$(nproc)

    # Create symlink for aarch64 architecture support
    # aarch64 is handled by arm.sh in crosstool-NG, but arch.sh looks for aarch64.sh
    local arch_dir="${CTNG_DIR}/scripts/build/arch"
    if [ ! -f "${arch_dir}/aarch64.sh" ] && [ -f "${arch_dir}/arm.sh" ]; then
        log_info "Creating aarch64.sh symlink (aarch64 uses arm.sh)..."
        ln -sf arm.sh "${arch_dir}/aarch64.sh"
    fi

    log_info "crosstool-NG installed successfully"
}

# Configure toolchain
configure_toolchain() {
    log_info "Configuring toolchain for ${ARCH}..."

    cd "${BUILD_DIR}"

    # Use ct-ng from the built directory
    local CTNG_BIN="${CTNG_DIR}/ct-ng"

    # Always start from crosstool-NG sample to ensure complete config
    # The pre-made config files may be incomplete, so we regenerate from samples
    log_info "Creating configuration from crosstool-NG sample for ${ARCH}..."
    if [ "${ARCH}" = "aarch64" ]; then
        # Start with aarch64 sample configuration
        "${CTNG_BIN}" aarch64-unknown-linux-gnu
    else
        # Start with ARM uClibc sample configuration
        "${CTNG_BIN}" arm-unknown-linux-uclibcgnueabi
    fi
    log_info "Configuration template created. Customizing for ${ARCH}..."

    # Apply architecture-specific customizations
    log_info "Applying ${ARCH}-specific customizations..."
    create_config_file
}

# Create configuration file with all required settings
create_config_file() {
    log_info "Creating crosstool-NG configuration file for ${ARCH}..."

    cd "${BUILD_DIR}"

    # If .config doesn't exist, create from sample
    if [ ! -f ".config" ]; then
        if [ "${ARCH}" = "aarch64" ]; then
            log_info "Creating configuration from aarch64 sample..."
            "${CTNG_DIR}/ct-ng" aarch64-unknown-linux-gnu
        else
            log_info "Creating configuration from ARM uClibc sample..."
            "${CTNG_DIR}/ct-ng" arm-unknown-linux-uclibcgnueabi
        fi
    fi

    # Read current config and modify it
    local config_file=".config"

    log_info "Customizing configuration for ${ARCH}..."

    # Set common versions (same for both architectures)
    sed -i 's/^CT_GCC_VERSION=.*/CT_GCC_VERSION="15.2.0"/' "${config_file}"
    sed -i 's/^CT_BINUTILS_VERSION=.*/CT_BINUTILS_VERSION="2.45"/' "${config_file}"

    # GDB version
    if grep -q "CT_GDB_VERSION" "${config_file}"; then
        sed -i 's/^CT_GDB_VERSION=.*/CT_GDB_VERSION="16.3"/' "${config_file}" || \
        log_warn "Could not set GDB version to 16.3, using default"
    fi

    # Architecture-specific customizations
    if [ "${ARCH}" = "aarch64" ]; then
        log_info "Configuring for aarch64 (64-bit ARM, glibc)..."
        
        # Ensure aarch64 architecture is set
        sed -i 's/^CT_ARCH=.*/CT_ARCH="aarch64"/' "${config_file}"
        sed -i 's/^CT_ARCH_64=.*/CT_ARCH_64=y/' "${config_file}"
        
        # Fix Linux version to match the version flag (6.16, not 6.1.0)
        # The sample config has CT_LINUX_V_6_16=y but CT_LINUX_VERSION="6.1.0" which is wrong
        sed -i 's/^CT_LINUX_VERSION=.*/CT_LINUX_VERSION="6.16"/' "${config_file}"
        log_info "Set Linux kernel version to 6.16 (modern)"
        
        # Set glibc version
        sed -i 's/^CT_GLIBC_VERSION=.*/CT_GLIBC_VERSION="2.38"/' "${config_file}"
        sed -i '/^CT_UCLIBC_VERSION=/d' "${config_file}"
        
        # Enable glibc and disable uClibc-ng
        sed -i 's/^CT_LIBC_GLIBC=.*/CT_LIBC_GLIBC=y/' "${config_file}"
        sed -i '/^CT_LIBC_UCLIBC_NG=y/d' "${config_file}"
        echo "# CT_LIBC_UCLIBC_NG is not set" >> "${config_file}"
        
        # Set target tuple
        sed -i 's/^CT_TARGET_VENDOR=.*/CT_TARGET_VENDOR="unknown"/' "${config_file}"
        sed -i 's/^CT_TARGET_OS=.*/CT_TARGET_OS="linux"/' "${config_file}"
        sed -i 's/^CT_TARGET_SYS=.*/CT_TARGET_SYS="gnu"/' "${config_file}"
        
        log_info "Set glibc version to 2.38"
    else
        log_info "Configuring for ARMv5TE (32-bit ARM, uClibc-ng)..."
        
        # Set architecture to ARMv5TEJ
        sed -i 's/^CT_ARCH_ARM=.*/CT_ARCH_ARM=y/' "${config_file}"
        sed -i 's/^CT_ARCH_CPU=.*/CT_ARCH_CPU="arm926ej-s"/' "${config_file}"
        
        # Set float ABI to soft
        sed -i 's/^CT_ARCH_FLOAT=.*/CT_ARCH_FLOAT="soft"/' "${config_file}"
        sed -i 's/^CT_ARCH_FLOAT_CFLAGS=.*/CT_ARCH_FLOAT_CFLAGS="-mfloat-abi=soft"/' "${config_file}"
        
        # Set kernel version to 3.4.35
        sed -i 's/^CT_LINUX_VERSION=.*/CT_LINUX_VERSION="3.4.35"/' "${config_file}"
        sed -i 's/^CT_KERNEL_VERSION=.*/CT_KERNEL_VERSION="3.4.35"/' "${config_file}" 2>/dev/null || true
        sed -i 's/^CT_KERNEL=.*/CT_KERNEL="linux"/' "${config_file}"
        if ! grep -q "^CT_LINUX_VERSION=" "${config_file}"; then
            echo 'CT_LINUX_VERSION="3.4.35"' >> "${config_file}"
        fi
        
        # Set uClibc-ng version
        sed -i 's/^CT_UCLIBC_VERSION=.*/CT_UCLIBC_VERSION="1.0.54"/' "${config_file}"
        sed -i '/^CT_GLIBC_VERSION=/d' "${config_file}"
        
        # Enable uClibc-ng and disable glibc
        sed -i 's/^CT_LIBC_UCLIBC_NG=.*/CT_LIBC_UCLIBC_NG=y/' "${config_file}"
        sed -i '/^CT_LIBC_GLIBC=y/d' "${config_file}"
        echo "# CT_LIBC_GLIBC is not set" >> "${config_file}"
        
        # Set target tuple
        sed -i 's/^CT_TARGET_VENDOR=.*/CT_TARGET_VENDOR="unknown"/' "${config_file}"
        sed -i 's/^CT_TARGET_OS=.*/CT_TARGET_OS="linux"/' "${config_file}"
        sed -i 's/^CT_TARGET_SYS=.*/CT_TARGET_SYS="uclibcgnueabi"/' "${config_file}"
        
        # Additional optimizations for embedded
        if ! grep -q "CT_OPTIMIZE_FOR_SIZE" "${config_file}"; then
            echo "CT_OPTIMIZE_FOR_SIZE=y" >> "${config_file}"
        fi
        
        log_info "Set Linux kernel version to 3.4.35"
        log_info "Set uClibc-ng version to 1.0.54"
    fi

    # Set installation path
    sed -i "s|^CT_PREFIX_DIR=.*|CT_PREFIX_DIR=\"${INSTALL_DIR}\"|" "${config_file}"

    log_info "Configuration file created/updated"

    # Save a copy for future reference
    cp "${config_file}" "${CONFIG_FILE}"

    log_info "Configuration saved to ${CONFIG_FILE}"
    log_info "You can review/edit it and re-run this script, or run:"
    log_info "  ${CTNG_DIR}/ct-ng menuconfig"
}

# Build the toolchain
build_toolchain() {
    log_info "Building toolchain (this will take 1-3 hours)..."

    cd "${BUILD_DIR}"

    local CTNG_BIN="${CTNG_DIR}/ct-ng"

    # Verify configuration
    if [ ! -f ".config" ]; then
        log_error "Configuration file not found. Run configure_toolchain first."
        exit 1
    fi

    log_info "Starting build process..."
    "${CTNG_BIN}" build

    log_info "Toolchain build completed successfully!"
}

# Verify installation
verify_installation() {
    log_info "Verifying toolchain installation for ${ARCH}..."

    local gcc_path
    if [ "${ARCH}" = "aarch64" ]; then
        # aarch64 toolchain installs to bin/ directly (not usr/bin/)
        gcc_path="${INSTALL_DIR}/bin/aarch64-unknown-linux-gnu-gcc"
    else
        # ARMv5TE toolchain installs to usr/bin/
        gcc_path="${INSTALL_DIR}/usr/bin/arm-unknown-linux-uclibcgnueabi-gcc"
    fi

    if [ ! -f "${gcc_path}" ]; then
        log_error "Toolchain not found at expected location: ${gcc_path}"
        exit 1
    fi

    log_info "Testing GCC version..."
    "${gcc_path}" --version

    log_info "Testing target architecture..."
    "${gcc_path}" -dumpmachine

    if [ "${ARCH}" = "armv5te" ]; then
        log_info "Verifying soft-float configuration..."
        "${gcc_path}" -march=armv5te -mfloat-abi=soft -E -dM - < /dev/null | grep -i "float" || true
    else
        log_info "Verifying aarch64 configuration..."
        "${gcc_path}" -E -dM - < /dev/null | grep -i "aarch64" || true
    fi

    log_info "Toolchain verification completed"
}

# Main execution
main() {
    log_info "Starting toolchain build for ${ARCH}"
    log_info "Build directory: ${BUILD_DIR}"
    log_info "Install directory: ${INSTALL_DIR}"
    log_info "Target tuple: ${TARGET_TUPLE}"

    check_dependencies
    install_crosstool_ng
    configure_toolchain
    build_toolchain
    verify_installation

    log_info "=========================================="
    log_info "Toolchain build completed successfully!"
    log_info "Architecture: ${ARCH}"
    if [ "${ARCH}" = "aarch64" ]; then
        log_info "Installation location: ${INSTALL_DIR}"
        log_info "=========================================="
        log_info "To use the aarch64 toolchain:"
        log_info "  ${INSTALL_DIR}/bin/aarch64-unknown-linux-gnu-gcc"
    else
        log_info "Installation location: ${INSTALL_DIR}/usr"
        log_info "=========================================="
        log_info "To use the ARMv5TE toolchain, set:"
        log_info "  export ANYKA_TOOLCHAIN_VERSION=new"
        log_info "Or update your build scripts to use:"
        log_info "  ${INSTALL_DIR}/usr/bin/arm-unknown-linux-uclibcgnueabi-gcc"
    fi
}

# Run main function
main "$@"
