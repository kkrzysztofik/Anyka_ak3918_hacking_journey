#!/bin/bash
# Stage 1: Build GCC toolchain using crosstool-NG
# Installs to ${INSTALL_DIR}

set -euo pipefail

source "${SCRIPTS_DIR}/common.sh"

STAGE_NAME="toolchain"

stage_toolchain() {
    log_info "=========================================="
    log_info "Stage 1: Building GCC toolchain (crosstool-NG)"
    log_info "Architecture: ${ARCH}"
    log_info "Target: ${TARGET_TUPLE}"
    log_info "=========================================="

    if has_checkpoint "${STAGE_NAME}"; then
        log_info "Skipping - checkpoint exists"
        return 0
    fi

    ensure_dirs

    local ct_ng_src="${BUILD_DIR}/crosstool-ng-${CROSSTOOL_NG_VERSION}"
    local ct_ng_build="${BUILD_DIR}/.build/crosstool-ng-${CROSSTOOL_NG_VERSION}"

    # Download crosstool-NG if needed
    if [[ ! -d "${ct_ng_src}" ]]; then
        log_info "Downloading crosstool-NG ${CROSSTOOL_NG_VERSION}..."
        local tarball="crosstool-ng-${CROSSTOOL_NG_VERSION}.tar.xz"
        cd "${BUILD_DIR}"
        if [[ ! -f "${tarball}" ]]; then
            wget -q "https://github.com/crosstool-ng/crosstool-ng/releases/download/crosstool-ng-${CROSSTOOL_NG_VERSION}/${tarball}"
        fi
        tar -xf "${tarball}"
    fi

    # Build crosstool-NG (host tool)
    if [[ ! -f "${ct_ng_build}/ct-ng" ]]; then
        log_info "Building crosstool-NG host tool..."
        mkdir -p "${ct_ng_build}"
        cd "${ct_ng_src}"
        ./configure --prefix="${ct_ng_build}" --enable-silent-rules
        make -j"$(nproc)"
        make install
    fi

    export PATH="${ct_ng_build}/bin:${PATH}"

    # Prepare config
    local ct_config="${ROOT_DIR}/crosstool-ng.config"
    if [[ ! -f "${ct_config}" ]]; then
        log_error "crosstool-ng.config not found at ${ct_config}"
        exit 1
    fi

    # Inject version-specific config fragments
    log_info "Preparing crosstool-NG configuration..."
    local patched_config="${BUILD_DIR}/.build/crosstool-ng.config"
    cp "${ct_config}" "${patched_config}"

    if [[ -x "${SCRIPTS_DIR}/inject_ct_config_fragments.py" ]]; then
        python3 "${SCRIPTS_DIR}/inject_ct_config_fragments.py" "${patched_config}"
    fi

    # Configure for architecture
    if [[ "${ARCH}" == "armv5te" ]]; then
        sed -i 's|CT_PREFIX_DIR=.*|CT_PREFIX_DIR="${INSTALL_DIR}"|' "${patched_config}"
        sed -i 's|CT_TARGET_VENDOR=.*|CT_TARGET_VENDOR="unknown"|' "${patched_config}"
    elif [[ "${ARCH}" == "aarch64" ]]; then
        sed -i 's|CT_PREFIX_DIR=.*|CT_PREFIX_DIR="${INSTALL_DIR}"|' "${patched_config}"
        sed -i 's|CT_TARGET_VENDOR=.*|CT_TARGET_VENDOR="unknown"|' "${patched_config}"
    fi

    # Build toolchain
    log_info "Building cross-compiler toolchain (this may take 1-2 hours)..."
    cd "${CT_NG_WORK_DIR}"
    "${ct_ng_build}/bin/ct-ng" -f "${patched_config}" "${ARCH}"-unknown-linux-gnu
    "${ct_ng_build}/bin/ct-ng" build

    mark_checkpoint "${STAGE_NAME}"

    log_info "=========================================="
    log_info "GCC toolchain build completed!"
    log_info "Installation: ${INSTALL_DIR}"
    log_info "=========================================="
}
