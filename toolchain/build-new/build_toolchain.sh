#!/bin/bash
# Build script for modern toolchain using crosstool-NG
# Supports: ARMv5TEJ (armv5te) for Anyka AK3918 cameras

set -e  # Exit on error

# Script directory — must be set before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

CONFIG_FILE="crosstool-ng.config"

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
    log_info "Installing crosstool-NG @ ${CTNG_GIT_REF}..."

    if [ -d "${CTNG_DIR}" ]; then
        if [ ! -d "${CTNG_DIR}/.git" ]; then
            log_warn "Removing non-git tree at ${CTNG_DIR} (expected git checkout)"
            rm -rf "${CTNG_DIR}"
        else
            cd "${CTNG_DIR}"
            local cur=""
            cur="$(git rev-parse HEAD 2>/dev/null || true)"
            local target_sha=""
            target_sha="$(git rev-parse --verify "${CTNG_GIT_REF}^{commit}" 2>/dev/null || true)"
            if [ -z "${target_sha}" ]; then
                target_sha="$(git ls-remote origin "${CTNG_GIT_REF}" 2>/dev/null | awk '{print $1; exit}')"
            fi
            if [ -z "${target_sha}" ]; then
                log_error "Could not resolve CTNG_GIT_REF=${CTNG_GIT_REF} to a commit SHA"
                exit 1
            fi
            if [ "${cur}" != "${target_sha}" ]; then
                log_info "Fetching crosstool-NG ${CTNG_GIT_REF} (${target_sha})..."
                git fetch --depth 1 origin "${CTNG_GIT_REF}"
                git checkout FETCH_HEAD
            fi
            cur="$(git rev-parse HEAD)"
            if [ "${cur}" != "${target_sha}" ]; then
                log_error "crosstool-NG HEAD mismatch: expected ${target_sha}, got ${cur}"
                exit 1
            fi
            cd "${BUILD_DIR}"
        fi
    fi

    if [ ! -d "${CTNG_DIR}" ]; then
        log_info "Cloning crosstool-NG (single-commit fetch)..."
        mkdir -p "${CTNG_DIR}"
        cd "${CTNG_DIR}"
        git init
        git remote add origin "${CTNG_GIT_URL}"
        local target_sha=""
        if [[ "${CTNG_GIT_REF}" =~ ^[0-9a-fA-F]{40}$ ]]; then
            target_sha="$(echo "${CTNG_GIT_REF}" | tr '[:upper:]' '[:lower:]')"
        else
            target_sha="$(git ls-remote "${CTNG_GIT_URL}" "${CTNG_GIT_REF}" 2>/dev/null | awk '{print $1; exit}')"
        fi
        if [ -z "${target_sha}" ]; then
            log_info "ls-remote did not resolve CTNG_GIT_REF=${CTNG_GIT_REF}; trying fetch by ref..."
            if ! git fetch origin "${CTNG_GIT_REF}"; then
                log_error "Could not resolve CTNG_GIT_REF=${CTNG_GIT_REF} to a commit SHA"
                exit 1
            fi
            target_sha="$(git rev-parse FETCH_HEAD)"
        fi
        if [ -z "${target_sha}" ]; then
            log_error "Could not resolve CTNG_GIT_REF=${CTNG_GIT_REF} to a commit SHA"
            exit 1
        fi
        if ! git fetch --depth 1 origin "${target_sha}"; then
            log_error "Failed to fetch crosstool-NG ${CTNG_GIT_REF}"
            exit 1
        fi
        git checkout FETCH_HEAD
        local cur=""
        cur="$(git rev-parse HEAD)"
        if [ "${cur}" != "${target_sha}" ]; then
            log_error "crosstool-NG clone checkout mismatch: expected ${target_sha}, got ${cur}"
            exit 1
        fi
        cd "${BUILD_DIR}"
    fi

    log_info "Installing vendored uClibc-ng ${UCLIBC_NG_VERSION} package metadata..."
    if [ -z "${UCLIBC_NG_VERSION}" ]; then
        log_error "UCLIBC_NG_VERSION is unset"
        exit 1
    fi
    local _vendored_uclibc="${SCRIPT_DIR}/vendor/crosstool-ng/uClibc-ng/${UCLIBC_NG_VERSION}"
    if [ ! -d "${_vendored_uclibc}" ]; then
        log_error "Vendored uClibc-ng metadata not found: ${_vendored_uclibc} (UCLIBC_NG_VERSION=${UCLIBC_NG_VERSION})"
        exit 1
    fi
    mkdir -p "${CTNG_DIR}/packages/uClibc-ng"
    rm -rf "${CTNG_DIR}/packages/uClibc-ng/${UCLIBC_NG_VERSION}"
    cp -a "${_vendored_uclibc}" "${CTNG_DIR}/packages/uClibc-ng/"

    local _uclibc_kconfig_token="UCLIBC_NG_V_${UCLIBC_NG_VERSION//./_}"
    if [ ! -f "${CTNG_DIR}/configure" ] || ! grep -q "${_uclibc_kconfig_token}" "${CTNG_DIR}/config/versions/uClibc-ng.in" 2>/dev/null; then
        log_info "Running ./bootstrap in crosstool-NG (Kconfig regeneration)..."
        (cd "${CTNG_DIR}" && ./bootstrap)
        if [ -f "${CTNG_DIR}/Makefile" ] || [ -f "${CTNG_DIR}/config.status" ]; then
            log_warn "Cleaning crosstool-NG host build after bootstrap..."
            (cd "${CTNG_DIR}" && make distclean 2>/dev/null || true)
            rm -f "${CTNG_DIR}/Makefile" "${CTNG_DIR}/config.status" "${CTNG_DIR}/config.cache"
        fi
    fi

    log_info "Building crosstool-NG..."
    cd "${CTNG_DIR}"

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

    make -j$(nproc)

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
    log_info "Creating configuration from crosstool-NG sample..."
    "${CTNG_BIN}" arm-unknown-linux-uclibcgnueabi
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
        log_info "Creating configuration from ARM uClibc sample..."
        "${CTNG_DIR}/ct-ng" arm-unknown-linux-uclibcgnueabi
    fi

    # Read current config and modify it
    local config_file=".config"

    log_info "Customizing configuration for ${ARCH}..."

    # Set common versions (same for both architectures)
    sed -i 's/^CT_GCC_VERSION=.*/CT_GCC_VERSION="15.2.0"/' "${config_file}"

    # GDB version
    if grep -q "CT_GDB_VERSION" "${config_file}"; then
        sed -i "s/^CT_GDB_VERSION=.*/CT_GDB_VERSION=\"${GDB_VERSION}\"/" "${config_file}" || \
        log_warn "Could not set GDB version to 17.1, using default"
    fi

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

    # Set installation path
    sed -i "s|^CT_PREFIX_DIR=.*|CT_PREFIX_DIR=\"${INSTALL_DIR}\"|" "${config_file}"

    # Pin binutils 2.46.0 + vendored uClibc-ng (see fragments/ and vendor/).
    if ! python3 "${SCRIPT_DIR}/inject_ct_config_fragments.py" "${config_file}"; then
        log_error "Failed to inject binutils / uClibc-ng config fragments"
        exit 1
    fi

    # Disable TIME64 in uClibc-ng: AK3918 runs Linux 3.4.35, which is older than
    # the Linux >= 5.1.0 requirement for 64-bit time_t syscalls on 32-bit ARM.
    # Without this, uClibc-ng fails to build with the 3.4.35 kernel headers.
    local uclibc_config="${BUILD_DIR}/uclibc-ng.config"
    local uclibc_seed="${CTNG_DIR}/packages/uClibc-ng/config"
    if [ ! -f "${uclibc_seed}" ]; then
        log_error "Missing uClibc-ng Kconfig seed at ${uclibc_seed} (install vendored uClibc-ng package and run bootstrap if needed)"
        exit 1
    fi
    cp "${uclibc_seed}" "${uclibc_config}"
    echo "# UCLIBC_USE_TIME64 is not set" >> "${uclibc_config}"
    sed -i "s|^CT_LIBC_UCLIBC_CONFIG_FILE=.*|CT_LIBC_UCLIBC_CONFIG_FILE=\"${uclibc_config}\"|" "${config_file}"
    if ! grep -q "^CT_LIBC_UCLIBC_CONFIG_FILE=" "${config_file}"; then
        echo "CT_LIBC_UCLIBC_CONFIG_FILE=\"${uclibc_config}\"" >> "${config_file}"
    fi
    log_info "uClibc-ng ${UCLIBC_NG_VERSION}: TIME64 disabled (required for Linux 3.4.35 target)"

    # Disable native GDB: we don't need a GDB binary running on the AK3918 target,
    # and it fails to build with the ARMv5TE sysroot on GDB 17.x.
    sed -i 's/^CT_GDB_NATIVE=.*/# CT_GDB_NATIVE is not set/' "${config_file}"
    log_info "GDB native: disabled (not needed on embedded target)"

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

    # Build up to libc headers first, so Linux sources are unpacked before we
    # patch legacy kernel host-tool sources for modern host compilers.
    # Persist step state so the follow-up build resumes instead of restarting from scratch
    # (otherwise the unifdef.c patch below can be lost).
    export CT_DEBUG_CT_SAVE_STEPS=y
    log_info "Building up to libc headers (pre-kernel-headers stage)..."
    STOP=libc_headers "${CTNG_BIN}" build

    # Legacy Linux scripts/unifdef.c uses 'constexpr' as an identifier, which
    # fails with modern GCC where constexpr is a reserved keyword in C mode.
    local CT_LINUX_VER="3.4.35"
    if [ -f "${BUILD_DIR}/.config" ]; then
        CT_LINUX_VER="$(grep '^CT_LINUX_VERSION=' "${BUILD_DIR}/.config" | head -1 | cut -d= -f2- | tr -d '\"')"
    fi
    local unifdef_path="${BUILD_DIR}/.build/src/linux-${CT_LINUX_VER}/scripts/unifdef.c"
    if [ -f "${unifdef_path}" ]; then
        log_info "Applying Linux ${CT_LINUX_VER} host-compiler compatibility fix (unifdef.c)..."
        sed -i 's/\<constexpr\>/const_expr/g' "${unifdef_path}"
    else
        log_warn "Expected file not found for patch: ${unifdef_path}"
        log_warn "Continuing build; kernel headers step may fail if source layout changed"
    fi

    # Resume after the patch; RESTART=kernel_headers skips re-unpacking Linux when step saves are on.
    log_info "Continuing full build after compatibility patch..."
    RESTART=kernel_headers "${CTNG_BIN}" build

    log_info "Toolchain build completed successfully!"
}

# Verify installation
verify_installation() {
    log_info "Verifying toolchain installation for ${ARCH}..."

    local gcc_path
    gcc_path="${INSTALL_DIR}/bin/arm-unknown-linux-uclibcgnueabi-gcc"

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
    log_info "Installation location: ${INSTALL_DIR}/usr"
    log_info "=========================================="
    log_info "To use the ARMv5TE toolchain, set:"
    log_info "  export ANYKA_TOOLCHAIN_VERSION=new"
    log_info "Or update your build scripts to use:"
    log_info "  ${INSTALL_DIR}/bin/arm-unknown-linux-uclibcgnueabi-gcc"
}

# Run main function
main "$@"
