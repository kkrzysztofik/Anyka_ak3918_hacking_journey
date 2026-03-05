#!/bin/bash
#
# Cross-compile v4l2-ctl for Anyka AK3918 (ARMv5TEJ, uClibc, soft-float)
# Produces a statically-linked v4l2-ctl binary
#
# v4l-utils uses meson, but v4l2-ctl can also be built with the legacy
# autotools build or even compiled directly from a handful of source files.
# We use the direct compilation approach for a minimal, static binary
# without pulling in meson, libudev, librt, etc.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TOOLCHAIN_DIR="/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng"
CROSS_PREFIX="${TOOLCHAIN_DIR}/bin/arm-unknown-linux-uclibcgnueabi-"
SYSROOT="${TOOLCHAIN_DIR}/arm-unknown-linux-uclibcgnueabi/sysroot"

CC="${CROSS_PREFIX}gcc"
CXX="${CROSS_PREFIX}g++"
STRIP="${CROSS_PREFIX}strip"

V4L_UTILS_VERSION="${V4L_UTILS_VERSION:-1.28.1}"
SRC_DIR="${SCRIPT_DIR}/v4l-utils-${V4L_UTILS_VERSION}"
BUILD_DIR="${SCRIPT_DIR}/build"
INSTALL_DIR="${SCRIPT_DIR}/install"
DEPLOY_DIR="/home/kmk/anyka-dev/SD_card_contents/anyka_hack/bin"

NPROC=$(nproc)

COMMON_CFLAGS="-march=armv5te -mtune=arm926ej-s -mfloat-abi=soft -Os --sysroot=${SYSROOT} -ffunction-sections -fdata-sections"
COMMON_LDFLAGS="-static -Wl,--gc-sections --sysroot=${SYSROOT}"

# ── Download source ──────────────────────────────────────────────────────────
download_source() {
    if [[ -d "${SRC_DIR}" ]]; then
        echo "Source directory already exists: ${SRC_DIR}"
        return
    fi

    local tarball="v4l-utils-${V4L_UTILS_VERSION}.tar.xz"
    if [[ ! -f "${SCRIPT_DIR}/${tarball}" ]]; then
        echo "Downloading v4l-utils ${V4L_UTILS_VERSION}..."
        wget --https-only --max-redirect=0 --secure-protocol=TLSv1_2 \
            -O "${SCRIPT_DIR}/${tarball}" \
            "https://linuxtv.org/downloads/v4l-utils/${tarball}"
    fi

    echo "Extracting..."
    tar xf "${SCRIPT_DIR}/${tarball}" -C "${SCRIPT_DIR}"
}

# ── Build v4l2-ctl directly (minimal approach) ──────────────────────────────
#
# v4l2-ctl's core source files are in utils/v4l2-ctl/ and depend on
# libv4l2 (lib/libv4l2/) and libv4lconvert. For a minimal static build
# we compile the v4l2-ctl sources directly with libv4l2 stubs.
#
# This avoids the full meson build system and all optional dependencies.
#
build_v4l2_ctl() {
    mkdir -p "${BUILD_DIR}" "${INSTALL_DIR}/bin"
    cd "${BUILD_DIR}"

    echo "Building v4l2-ctl for ARMv5TEJ (direct compilation)..."

    local v4l2_ctl_dir="${SRC_DIR}/utils/v4l2-ctl"

    if [[ ! -d "${v4l2_ctl_dir}" ]]; then
        echo "ERROR: v4l2-ctl source directory not found at ${v4l2_ctl_dir}" >&2
        exit 1
    fi

    # Generate media-bus-format-names.h (normally done by meson)
    echo "Generating media-bus-format-names.h..."
    local media_bus_hdr="${SRC_DIR}/include/linux/media-bus-format.h"
    if [[ ! -f "${media_bus_hdr}" ]]; then
        media_bus_hdr="${SYSROOT}/usr/include/linux/media-bus-format.h"
    fi
    if [[ ! -f "${media_bus_hdr}" ]]; then
        echo "ERROR: media-bus-format.h not found in source or sysroot" >&2
        exit 1
    fi
    bash "${SRC_DIR}/utils/gen_media_bus_format_names.sh" "${media_bus_hdr}" \
        > "${BUILD_DIR}/media-bus-format-names.h"

    # Include paths: common/ has the .h files, v4l2-ctl/ has symlinks to .cpp
    # Use gnu++17 (not c++17) for typeof() support in kernel-style macros
    local cxxflags="${COMMON_CFLAGS} -std=gnu++17"
    cxxflags+=" -I${BUILD_DIR}"
    cxxflags+=" -I${SRC_DIR}"
    cxxflags+=" -I${SRC_DIR}/include"
    cxxflags+=" -I${SRC_DIR}/lib/include"
    cxxflags+=" -I${SRC_DIR}/utils/common"
    cxxflags+=" -I${v4l2_ctl_dir}"
    cxxflags+=" -I${SYSROOT}/usr/include"
    # Disable libv4l2 wrapper (use direct kernel V4L2 ioctls)
    cxxflags+=" -DNO_LIBV4L2"
    # Define version macros normally set by meson
    cxxflags+=" -DPACKAGE_VERSION='\"${V4L_UTILS_VERSION}\"'"
    cxxflags+=" -DGIT_COMMIT_CNT=0"

    # C flags for the C source files (tpg, fwht codec, v4l-stream)
    local cflags="${COMMON_CFLAGS} -std=gnu99"
    cflags+=" -I${BUILD_DIR}"
    cflags+=" -I${SRC_DIR}"
    cflags+=" -I${SRC_DIR}/include"
    cflags+=" -I${SRC_DIR}/lib/include"
    cflags+=" -I${SRC_DIR}/utils/common"
    cflags+=" -I${v4l2_ctl_dir}"
    cflags+=" -I${SYSROOT}/usr/include"

    # Compile C support files from common/ (TPG, FWHT codec, v4l-stream)
    local c_sources=(
        "${SRC_DIR}/utils/common/v4l2-tpg-core.c"
        "${SRC_DIR}/utils/common/v4l2-tpg-colors.c"
        "${SRC_DIR}/utils/common/codec-fwht.c"
        "${SRC_DIR}/utils/common/codec-v4l2-fwht.c"
        "${SRC_DIR}/utils/common/v4l-stream.c"
    )

    local objects=()
    local failed=0

    echo "Compiling C support files..."
    for src in "${c_sources[@]}"; do
        if [[ ! -f "${src}" ]]; then
            echo "  SKIP: $(basename "${src}") (not found)"
            continue
        fi
        local obj="${BUILD_DIR}/$(basename "${src}" .c).o"
        echo "  CC  $(basename "${src}")"
        if ${CC} ${cflags} -c "${src}" -o "${obj}" 2>&1; then
            objects+=("${obj}")
        else
            echo "  FAILED: $(basename "${src}")"
            failed=$((failed + 1))
        fi
    done

    # Compile ONLY C++ files from v4l2-ctl/ directory (includes symlinks to common/)
    # Do NOT also compile from common/ - that causes duplicate symbols
    local sources=()
    for f in "${v4l2_ctl_dir}"/*.cpp; do
        sources+=("$f")
    done

    echo "Compiling ${#sources[@]} C++ source files..."
    for src in "${sources[@]}"; do
        local obj="${BUILD_DIR}/$(basename "${src}" .cpp).o"
        echo "  CXX $(basename "${src}")"
        if ${CXX} ${cxxflags} -c "${src}" -o "${obj}" 2>&1; then
            objects+=("${obj}")
        else
            echo "  FAILED: $(basename "${src}")"
            failed=$((failed + 1))
        fi
    done

    # Report compilation summary after both loops
    local total_sources=$((${#c_sources[@]} + ${#sources[@]}))
    echo ""
    echo "Compiled ${#objects[@]}/${total_sources} files (${failed} failed)"

    if [[ ${#objects[@]} -eq 0 ]]; then
        echo "ERROR: No objects compiled." >&2
        exit 1
    fi
    if [[ ${failed} -gt 0 ]]; then
        echo "ERROR: ${failed} file(s) failed to compile." >&2
        exit 1
    fi

    if [[ ${#objects[@]} -eq 0 ]]; then
        echo "ERROR: No objects compiled." >&2
        exit 1
    fi

    echo "Linking v4l2-ctl..."
    ${CXX} ${COMMON_LDFLAGS} -o "${INSTALL_DIR}/bin/v4l2-ctl" "${objects[@]}" \
        -lstdc++ -lm -lpthread -lrt

    ${STRIP} --strip-all "${INSTALL_DIR}/bin/v4l2-ctl"

    echo ""
    echo "Binary info:"
    file "${INSTALL_DIR}/bin/v4l2-ctl"
    ls -lh "${INSTALL_DIR}/bin/v4l2-ctl"
    return 0
}

# ── Fallback: meson-based build ─────────────────────────────────────────────
build_v4l2_ctl_meson() {
    echo ""
    echo "Building v4l2-ctl via meson cross-compilation..."

    # Check if meson and ninja are available
    if ! command -v meson &>/dev/null; then
        echo "ERROR: meson not found. Install with: pip install meson" >&2
        echo "Or: sudo apt install meson"
        exit 1
    fi
    if ! command -v ninja &>/dev/null; then
        echo "ERROR: ninja not found. Install with: pip install ninja" >&2
        echo "Or: sudo apt install ninja-build"
        exit 1
    fi

    # Create meson cross file
    local cross_file="${BUILD_DIR}/arm-cross.ini"
    cat > "${cross_file}" <<EOF
[binaries]
c = '${CC}'
cpp = '${CXX}'
ar = '${CROSS_PREFIX}ar'
strip = '${STRIP}'
pkg-config = 'pkg-config'

[built-in options]
c_args = ['-march=armv5te', '-mtune=arm926ej-s', '-mfloat-abi=soft', '-Os', '-ffunction-sections', '-fdata-sections']
c_link_args = ['-static', '-Wl,--gc-sections']
cpp_args = ['-march=armv5te', '-mtune=arm926ej-s', '-mfloat-abi=soft', '-Os', '-ffunction-sections', '-fdata-sections']
cpp_link_args = ['-static', '-Wl,--gc-sections']
default_library = 'static'

[properties]
sys_root = '${SYSROOT}'
pkg_config_libdir = '${SYSROOT}/usr/lib/pkgconfig'

[host_machine]
system = 'linux'
cpu_family = 'arm'
cpu = 'armv5te'
endian = 'little'
EOF

    local meson_build_dir="${BUILD_DIR}/meson-out"
    rm -rf "${meson_build_dir}"

    meson setup "${meson_build_dir}" "${SRC_DIR}" \
        --cross-file="${cross_file}" \
        --prefix="${INSTALL_DIR}" \
        --default-library=static \
        -Dbpf=disabled \
        -Dudevdir=/dev/null \
        -Dsystemdsystemunitdir=/dev/null \
        -Dqv4l2=disabled \
        -Dqvidcap=disabled \
        -Dgconv=disabled \
        -Djpeg=disabled \
        -Dlibdvbv5=disabled \
        -Dv4l2-tracer=disabled \
        -Dv4l-plugins=false \
        -Dv4l-wrappers=false \
        -Dv4l2-ctl-libv4l=false \
        -Dv4l2-ctl-stream-to=false \
        -Dv4l2-compliance-libv4l=false \
        -Ddoxygen-doc=disabled

    ninja -C "${meson_build_dir}" utils/v4l2-ctl/v4l2-ctl -j"${NPROC}"

    cp "${meson_build_dir}/utils/v4l2-ctl/v4l2-ctl" "${INSTALL_DIR}/bin/"
    ${STRIP} --strip-all "${INSTALL_DIR}/bin/v4l2-ctl"

    echo ""
    echo "Binary info:"
    file "${INSTALL_DIR}/bin/v4l2-ctl"
    ls -lh "${INSTALL_DIR}/bin/v4l2-ctl"
    return 0
}

# ── Deploy ───────────────────────────────────────────────────────────────────
deploy() {
    local binary="${INSTALL_DIR}/bin/v4l2-ctl"
    if [[ ! -f "${binary}" ]]; then
        echo "ERROR: v4l2-ctl binary not found. Run build first." >&2
        exit 1
    fi

    mkdir -p "${DEPLOY_DIR}"
    echo "Deploying to ${DEPLOY_DIR}..."
    cp "${binary}" "${DEPLOY_DIR}/v4l2-ctl"
    chmod +x "${DEPLOY_DIR}/v4l2-ctl"
    echo "Deployed."
    return 0
}

# ── Clean ────────────────────────────────────────────────────────────────────
clean() {
    echo "Cleaning build and install directories..."
    rm -rf "${BUILD_DIR}" "${INSTALL_DIR}"
    return 0
}

# ── Main ─────────────────────────────────────────────────────────────────────
usage() {
    echo "Usage: $0 [download|build|deploy|all|clean]"
    echo ""
    echo "Steps can be run individually or 'all' runs the full pipeline."
    echo ""
    echo "Environment variables:"
    echo "  V4L_UTILS_VERSION  v4l-utils version to build (default: 1.28.1)"
    return 0
}

case "${1:-all}" in
    download)   download_source ;;
    build)      build_v4l2_ctl ;;
    deploy)     deploy ;;
    all)
        download_source
        build_v4l2_ctl
        deploy
        echo ""
        echo "v4l2-ctl cross-compilation complete!"
        ;;
    clean)      clean ;;
    *)          usage ;;
esac
