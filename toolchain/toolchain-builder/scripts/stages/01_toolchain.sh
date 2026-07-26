#!/bin/bash
# Stage 1: Build GCC toolchain using crosstool-NG
# Installs cross-compiler to ${INSTALL_DIR}

set -euo pipefail

source "${SCRIPTS_DIR}/common.sh"

STAGE_NAME="toolchain"

# CTNG_SRC_DIR is set here at module scope so it is visible to all helper
# functions (_install_ctng, _configure_ctng, _build_ctng) without needing
# to be passed as an argument or re-derived in each function.
CTNG_SRC_DIR="${BUILD_DIR}/crosstool-ng-${CROSSTOOL_NG_VERSION}"

# ---------------------------------------------------------------------------
# _install_ctng — download, vendor-patch, and build the ct-ng host tool
#
# Uses --enable-local so ct-ng runs in-place from ${CTNG_SRC_DIR}.
# A separate --prefix install is intentionally NOT used; all ct-ng commands
# must be invoked as "${CTNG_SRC_DIR}/ct-ng".
# ---------------------------------------------------------------------------
_install_ctng() {
    local tarball="${BUILD_DIR}/crosstool-ng-${CROSSTOOL_NG_VERSION}.tar.xz"

    # ── 1. Download source tarball ──────────────────────────────────────────
    if [[ ! -d "${CTNG_SRC_DIR}" ]]; then
        log_info "Downloading crosstool-NG ${CROSSTOOL_NG_VERSION}..."
        if [[ ! -f "${tarball}" ]]; then
            local ctng_url="https://github.com/crosstool-ng/crosstool-ng/releases/download/crosstool-ng-${CROSSTOOL_NG_VERSION}/crosstool-ng-${CROSSTOOL_NG_VERSION}.tar.xz"
            log_info "Fetching ${ctng_url}"
            wget "${ctng_url}" -O "${tarball}" || {
                rm -f "${tarball}"
                log_error "Failed to download crosstool-NG ${CROSSTOOL_NG_VERSION}"
                exit 1
            }
        fi
        tar -xf "${tarball}" -C "${BUILD_DIR}"
    fi

    # ── 2. Inject vendored uClibc-ng package metadata ───────────────────────
    # ct-ng 1.28.0 ships packages up to uClibc-ng 1.0.54.
    # We need 1.0.57, which is vendored under ${ROOT_DIR}/vendor/.
    local vendored_uclibc="${ROOT_DIR}/vendor/crosstool-ng/uClibc-ng/${UCLIBC_NG_VERSION}"
    if [[ ! -d "${vendored_uclibc}" ]]; then
        log_error "Vendored uClibc-ng metadata not found: ${vendored_uclibc}"
        log_error "Expected UCLIBC_NG_VERSION=${UCLIBC_NG_VERSION}"
        log_error "Run scripts/create-vendor-uclibc.sh to populate it, or see docs/toolchain-builder.md"
        exit 1
    fi

    local ctng_pkg_dir="${CTNG_SRC_DIR}/packages/uClibc-ng/${UCLIBC_NG_VERSION}"
    if [[ ! -d "${ctng_pkg_dir}" ]]; then
        log_info "Installing vendored uClibc-ng ${UCLIBC_NG_VERSION} package metadata..."
        mkdir -p "${ctng_pkg_dir}"
        cp -a "${vendored_uclibc}/." "${ctng_pkg_dir}/"
    fi

    # ── 3. Run ./bootstrap to regenerate Kconfig with new version ───────────
    local kconfig_token="UCLIBC_NG_V_${UCLIBC_NG_VERSION//./_}"
    local versions_in="${CTNG_SRC_DIR}/config/versions/uClibc-ng.in"
    if ! grep -q "${kconfig_token}" "${versions_in}" 2>/dev/null; then
        log_info "Running ./bootstrap in crosstool-NG (Kconfig regeneration for ${UCLIBC_NG_VERSION})..."
        (
            cd "${CTNG_SRC_DIR}"
            # bootstrap requires autoconf/automake but must NOT use the cross-compiler
            CC=gcc CXX=g++ ./bootstrap
        )
        # After bootstrap the Makefile/config.status are stale; clean them so
        # the configure step below runs fresh.
        (cd "${CTNG_SRC_DIR}" && make distclean 2>/dev/null || true)
        rm -f "${CTNG_SRC_DIR}/Makefile" \
              "${CTNG_SRC_DIR}/config.status" \
              "${CTNG_SRC_DIR}/config.cache"
    fi

    # ── 4. Configure and build ct-ng (--enable-local = in-place binary) ─────
    if [[ ! -f "${CTNG_SRC_DIR}/ct-ng" ]]; then
        log_info "Building crosstool-NG host tool (--enable-local)..."
        (
            cd "${CTNG_SRC_DIR}"
            # Strip any existing cross-compiler from PATH so ct-ng builds with
            # the native system compiler only.
            local clean_path
            clean_path="$(echo "${PATH}" | tr ':' '\n' \
                | grep -v "arm-anykav200" \
                | tr '\n' ':' \
                | sed 's/:$//')"
            PATH="${clean_path}" CC=gcc CXX=g++ \
                ./configure --enable-local
            make -j"$(nproc)"
        )
    fi
}

# ---------------------------------------------------------------------------
# _configure_ctng — lay down .config in the ct-ng work directory
# ---------------------------------------------------------------------------
_configure_ctng() {
    local ctng_bin="${CTNG_SRC_DIR}/ct-ng"

    # The work directory is where ct-ng reads/writes .config and .build/
    mkdir -p "${CT_NG_WORK_DIR}"

    # Start from the hand-maintained input config
    local src_config="${ROOT_DIR}/crosstool-ng.config"
    if [[ ! -f "${src_config}" ]]; then
        log_error "crosstool-ng.config not found at ${src_config}"
        exit 1
    fi

    local work_config="${CT_NG_WORK_DIR}/.config"
    cp "${src_config}" "${work_config}"

    # Patch CT_PREFIX_DIR to the actual install path at build time.
    # The value in crosstool-ng.config is a template placeholder that uses
    # CT_TOP_DIR, which is only valid when ct-ng is the one expanding it.
    # We force the absolute path here so the installed toolchain ends up
    # at the expected location regardless of where the build is run.
    local abs_install_dir
    abs_install_dir="$(cd "${ROOT_DIR}/.." && pwd)/arm-anykav200-crosstool-ng"
    sed -i "s|CT_PREFIX_DIR=.*|CT_PREFIX_DIR=\"${abs_install_dir}\"|" "${work_config}"

    # Inject version-pinned kconfig fragments (binutils 2.46.0, uClibc-ng version).
    log_info "Injecting kconfig fragments..."
    python3 "${SCRIPTS_DIR}/inject_ct_config_fragments.py" "${work_config}"

    # ── uClibc-ng configuration file ──────────────────────────────────────
    # Copy the seed config shipped with the vendored package, then disable
    # TIME64 (AK3918 runs Linux 3.4.35 < 5.1.0 required for 64-bit time_t
    # syscalls on 32-bit ARM).
    local uclibc_seed="${CTNG_SRC_DIR}/packages/uClibc-ng/config"
    if [[ ! -f "${uclibc_seed}" ]]; then
        log_error "Missing uClibc-ng seed config: ${uclibc_seed}"
        log_error "Ensure vendored package metadata was installed and ./bootstrap was run"
        exit 1
    fi

    local uclibc_config="${CT_NG_WORK_DIR}/uclibc-ng.config"
    cp "${uclibc_seed}" "${uclibc_config}"
    echo "# UCLIBC_USE_TIME64 is not set" >> "${uclibc_config}"

    # Tell ct-ng where the uClibc-ng config lives.
    sed -i "s|^CT_LIBC_UCLIBC_CONFIG_FILE=.*|CT_LIBC_UCLIBC_CONFIG_FILE=\"${uclibc_config}\"|" \
        "${work_config}"
    if ! grep -q "^CT_LIBC_UCLIBC_CONFIG_FILE=" "${work_config}"; then
        echo "CT_LIBC_UCLIBC_CONFIG_FILE=\"${uclibc_config}\"" >> "${work_config}"
    fi

    log_info "uClibc-ng ${UCLIBC_NG_VERSION}: TIME64 disabled (Linux 3.4.35 target)"

    # Enable step saving so that STOP/RESTART work correctly.
    if ! grep -q "^CT_DEBUG_CT_SAVE_STEPS=" "${work_config}"; then
        echo "CT_DEBUG_CT_SAVE_STEPS=y" >> "${work_config}"
    else
        sed -i 's/^CT_DEBUG_CT_SAVE_STEPS=.*/CT_DEBUG_CT_SAVE_STEPS=y/' "${work_config}"
    fi

    log_info "ct-ng configuration prepared at ${work_config}"
}

# ---------------------------------------------------------------------------
# _build_ctng — run ct-ng build with the unifdef.c compatibility patch
# ---------------------------------------------------------------------------
_build_ctng() {
    local ctng_bin="${CTNG_SRC_DIR}/ct-ng"

    cd "${CT_NG_WORK_DIR}"

    # ── Phase 1: build up to (but not including) kernel header extraction ──
    # We stop just before kernel headers so that Linux source has been
    # unpacked but no attempt has been made to compile unifdef.c yet.
    log_info "Phase 1: building up to libc_headers (stop before kernel headers)..."
    CT_DEBUG_CT_SAVE_STEPS=y STOP=libc_headers "${ctng_bin}" build 2>&1

    # ── Apply unifdef.c compatibility patch ───────────────────────────────
    # Linux 3.4.35 scripts/unifdef.c uses 'constexpr' as a plain identifier.
    # Modern GCC (C99 and later) treats 'constexpr' as a keyword, causing a
    # compile error.  Rename it to 'const_expr' to restore compatibility.
    local linux_ver
    linux_ver="$(grep '^CT_LINUX_VERSION=' "${CT_NG_WORK_DIR}/.config" \
                 | head -1 | cut -d= -f2- | tr -d '"')"
    linux_ver="${linux_ver:-3.4.35}"

    local unifdef_src="${CT_NG_WORK_DIR}/.build/src/linux-${linux_ver}/scripts/unifdef.c"
    if [[ -f "${unifdef_src}" ]]; then
        log_info "Patching Linux ${linux_ver} unifdef.c for modern GCC..."
        sed -i 's/\bconstexpr\b/const_expr/g' "${unifdef_src}"
    else
        log_warn "unifdef.c not found at expected path: ${unifdef_src}"
        log_warn "Continuing; kernel headers step may fail if source layout changed"
    fi

    # ── Phase 2: continue from kernel_headers through completion ──────────
    log_info "Phase 2: continuing build from kernel_headers..."
    CT_DEBUG_CT_SAVE_STEPS=y RESTART=kernel_headers "${ctng_bin}" build 2>&1

    log_info "ct-ng toolchain build completed!"
}

# ---------------------------------------------------------------------------
# _verify_toolchain — quick sanity check after the build
# ---------------------------------------------------------------------------
_verify_toolchain() {
    local gcc_bin="${INSTALL_DIR}/bin/${TARGET_TUPLE}-gcc"

    if [[ ! -f "${gcc_bin}" ]]; then
        log_error "Expected GCC not found: ${gcc_bin}"
        exit 1
    fi

    log_info "Toolchain verification:"
    "${gcc_bin}" --version
    log_info "Target machine: $("${gcc_bin}" -dumpmachine)"
    log_info "Float ABI check:"
    "${gcc_bin}" -march=armv5te -mfloat-abi=soft -E -dM - < /dev/null \
        | grep -i float || true
}

# ---------------------------------------------------------------------------
# stage_toolchain — main entry point (called by build.sh)
# ---------------------------------------------------------------------------
stage_toolchain() {
    log_info "=========================================="
    log_info "Stage 1: Building GCC toolchain (crosstool-NG ${CROSSTOOL_NG_VERSION})"
    log_info "Architecture : ${ARCH}"
    log_info "Target       : ${TARGET_TUPLE}"
    log_info "Install dir  : ${INSTALL_DIR}"
    log_info "=========================================="

    if has_checkpoint "${STAGE_NAME}"; then
        log_info "Skipping stage 1 — checkpoint exists (use --clean to rebuild)"
        return 0
    fi

    ensure_dirs

    _install_ctng        # sets CTNG_SRC_DIR
    _configure_ctng
    _build_ctng
    _verify_toolchain

    mark_checkpoint "${STAGE_NAME}"

    log_info "=========================================="
    log_info "Stage 1 complete — GCC toolchain installed"
    log_info "  ${INSTALL_DIR}/bin/${TARGET_TUPLE}-gcc"
    log_info "=========================================="
}
