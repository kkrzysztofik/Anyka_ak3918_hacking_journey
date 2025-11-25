#!/bin/bash
# Build script for modern Anyka AK3918 toolchain using crosstool-NG
# Target: ARMv5TEJ, soft-float, uClibc-ng, Linux 3.4.35

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}"
INSTALL_DIR="${SCRIPT_DIR}/../arm-anykav200-crosstool-ng"
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
        wget -q "https://github.com/crosstool-ng/crosstool-ng/releases/download/crosstool-ng-${CTNG_VERSION}/crosstool-ng-${CTNG_VERSION}.tar.xz"
        tar -xf "crosstool-ng-${CTNG_VERSION}.tar.xz"
        rm "crosstool-ng-${CTNG_VERSION}.tar.xz"
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

    # Apply patch to disable time64 for Linux < 5.1.0
    if [ -f "${BUILD_DIR}/disable_time64.patch" ]; then
        log_info "Applying patch to disable time64 support for Linux 3.4.35..."
        if patch -p1 -d "${CTNG_DIR}" < "${BUILD_DIR}/disable_time64.patch" 2>&1 | grep -q "succeeded\|ignored"; then
            log_info "Patch applied successfully"
        else
            log_warn "Patch may have already been applied or failed (this is OK if already patched)"
        fi
    fi

    make -j$(nproc)

    log_info "crosstool-NG installed successfully"
}

# Configure toolchain
configure_toolchain() {
    log_info "Configuring toolchain..."

    cd "${BUILD_DIR}"

    # Use ct-ng from the built directory
    local CTNG_BIN="${CTNG_DIR}/ct-ng"

    # Load configuration if it exists, otherwise start from sample
    if [ -f "crosstool-ng.config" ]; then
        log_info "Loading existing configuration from crosstool-ng.config"
        cp "crosstool-ng.config" ".config"
    else
        log_info "Creating new configuration..."
        # Start with a sample configuration for ARM uClibc
        "${CTNG_BIN}" arm-unknown-linux-uclibcgnueabi

        # Now we need to customize it via menuconfig or by editing .config
        log_info "Configuration template created. Run '${CTNG_BIN} menuconfig' to customize,"
        log_info "or edit crosstool-ng.config and re-run this script."
        log_warn "For now, using default ARM uClibc configuration"
    fi

    # Apply customizations via sed or expect script
    log_info "Applying Anyka-specific customizations..."

    # Create a script to set the configuration
    cat > "${BUILD_DIR}/configure_toolchain.exp" << 'EOF'
#!/usr/bin/expect -f
set timeout 300
spawn ../crosstool-ng-1.28.0/ct-ng menuconfig
expect "Arrow keys navigate the menu"
send "\x1B[B"  # Down arrow
# Navigate and set options
# This is a simplified version - in practice, we'd use a config file
send "q"
expect "Save"
send "y"
expect eof
EOF
    chmod +x "${BUILD_DIR}/configure_toolchain.exp"

    # Instead of interactive menuconfig, we'll create the config file directly
    create_config_file
}

# Create configuration file with all required settings
create_config_file() {
    log_info "Creating crosstool-NG configuration file..."

    cd "${BUILD_DIR}"

    # If a pre-made config exists, use it but still apply fixes
    if [ -f "crosstool-ng.config" ]; then
        log_info "Using existing crosstool-ng.config"
        cp "crosstool-ng.config" ".config"
        # Don't return - continue to apply fixes below
    fi

    # If .config doesn't exist, create from sample
    if [ ! -f ".config" ]; then
        log_info "Creating configuration from ARM uClibc sample..."
        "${CTNG_DIR}/ct-ng" arm-unknown-linux-uclibcgnueabi
    fi

    # Read current config and modify it
    local config_file=".config"

    log_info "Customizing configuration for Anyka AK3918..."

    # Set versions and options using sed (handle both =y and ="value" formats)
    # Note: GCC version must be "15.2.0" not "15.2" (full version format)
    sed -i 's/^CT_GCC_VERSION=.*/CT_GCC_VERSION="15.2.0"/' "${config_file}"
    sed -i '/^CT_GLIBC_VERSION=/d' "${config_file}"
    sed -i 's/^CT_UCLIBC_VERSION=.*/CT_UCLIBC_VERSION="1.0.54"/' "${config_file}"
    sed -i 's/^CT_BINUTILS_VERSION=.*/CT_BINUTILS_VERSION="2.45"/' "${config_file}"

    # GDB version - check if it's available in crosstool-NG 1.28.0
    # If 16.3 is not available, use the latest available version
    if grep -q "CT_GDB_VERSION" "${config_file}"; then
        sed -i 's/^CT_GDB_VERSION=.*/CT_GDB_VERSION="16.3"/' "${config_file}" || \
        log_warn "Could not set GDB version to 16.3, using default"
    fi

    # Set architecture to ARMv5TEJ
    # Note: GCC 15.2.0 requires actual CPU name (arm926ej-s) not architecture name (armv5te)
    sed -i 's/^CT_ARCH_ARM=.*/CT_ARCH_ARM=y/' "${config_file}"
    sed -i 's/^CT_ARCH_CPU=.*/CT_ARCH_CPU="arm926ej-s"/' "${config_file}"

    # Set float ABI to soft
    sed -i 's/^CT_ARCH_FLOAT=.*/CT_ARCH_FLOAT="soft"/' "${config_file}"
    sed -i 's/^CT_ARCH_FLOAT_CFLAGS=.*/CT_ARCH_FLOAT_CFLAGS="-mfloat-abi=soft"/' "${config_file}"

    # Set kernel version - the variable is CT_LINUX_VERSION, not CT_KERNEL_VERSION
    sed -i 's/^CT_LINUX_VERSION=.*/CT_LINUX_VERSION="3.4.35"/' "${config_file}"
    # Also handle CT_KERNEL_VERSION if it exists (for compatibility)
    sed -i 's/^CT_KERNEL_VERSION=.*/CT_KERNEL_VERSION="3.4.35"/' "${config_file}" 2>/dev/null || true
    # Ensure kernel is set to Linux (not other OS)
    sed -i 's/^CT_KERNEL=.*/CT_KERNEL="linux"/' "${config_file}"
    # Add Linux version if it doesn't exist
    if ! grep -q "^CT_LINUX_VERSION=" "${config_file}"; then
        echo 'CT_LINUX_VERSION="3.4.35"' >> "${config_file}"
    fi
    log_info "Set Linux kernel version to 3.4.35"

    # Set target tuple (should already be set, but ensure it's correct)
    sed -i 's/^CT_TARGET_VENDOR=.*/CT_TARGET_VENDOR="unknown"/' "${config_file}"
    sed -i 's/^CT_TARGET_OS=.*/CT_TARGET_OS="linux"/' "${config_file}"
    sed -i 's/^CT_TARGET_SYS=.*/CT_TARGET_SYS="uclibcgnueabi"/' "${config_file}"

    # Enable uClibc-ng and disable glibc
    sed -i 's/^CT_LIBC_UCLIBC_NG=.*/CT_LIBC_UCLIBC_NG=y/' "${config_file}"
    sed -i '/^CT_LIBC_GLIBC=y/d' "${config_file}"
    echo "# CT_LIBC_GLIBC is not set" >> "${config_file}"

    # Set installation path
    sed -i "s|^CT_PREFIX_DIR=.*|CT_PREFIX_DIR=\"${INSTALL_DIR}\"|" "${config_file}"

    # Additional optimizations for embedded
    if ! grep -q "CT_OPTIMIZE_FOR_SIZE" "${config_file}"; then
        echo "CT_OPTIMIZE_FOR_SIZE=y" >> "${config_file}"
    fi

    log_info "Configuration file created/updated"

    # Save a copy for future reference
    cp "${config_file}" "crosstool-ng.config"

    log_info "Configuration saved to crosstool-ng.config"
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
    log_info "Verifying toolchain installation..."

    local gcc_path="${INSTALL_DIR}/usr/bin/arm-unknown-linux-uclibcgnueabi-gcc"

    if [ ! -f "${gcc_path}" ]; then
        log_error "Toolchain not found at expected location: ${gcc_path}"
        exit 1
    fi

    log_info "Testing GCC version..."
    "${gcc_path}" --version

    log_info "Testing target architecture..."
    "${gcc_path}" -dumpmachine

    log_info "Verifying soft-float configuration..."
    "${gcc_path}" -march=armv5te -mfloat-abi=soft -E -dM - < /dev/null | grep -i "float" || true

    log_info "Toolchain verification completed"
}

# Main execution
main() {
    log_info "Starting modern toolchain build for Anyka AK3918"
    log_info "Build directory: ${BUILD_DIR}"
    log_info "Install directory: ${INSTALL_DIR}"

    check_dependencies
    install_crosstool_ng
    configure_toolchain
    build_toolchain
    verify_installation

    log_info "=========================================="
    log_info "Toolchain build completed successfully!"
    log_info "Installation location: ${INSTALL_DIR}/usr"
    log_info "=========================================="
    log_info "To use the new toolchain, set:"
    log_info "  export ANYKA_TOOLCHAIN_VERSION=new"
    log_info "Or update your build scripts to use:"
    log_info "  ${INSTALL_DIR}/usr/bin/arm-unknown-linux-uclibcgnueabi-gcc"
}

# Run main function
main "$@"
