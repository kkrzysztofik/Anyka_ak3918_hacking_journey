#!/usr/bin/env bash
# Build v4l2-ctl for Anyka AK3918 (ARMv5TEJ, uClibc, soft-float) and stage into SD overlay.
# Produces a statically-linked v4l2-ctl binary via direct source compilation
# (no meson required).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "$(cd "${SCRIPT_DIR}/../.." && pwd)/scripts/common.sh"
# shellcheck source=scripts/third_party/build_lib.sh
source "${SCRIPT_DIR}/build_lib.sh"
REPO_ROOT="${ANYKA_REPO_ROOT}"

DEFAULT_VERSION="1.28.1"
DEFAULT_SHA256=""

version="${DEFAULT_VERSION}"
sha256="${DEFAULT_SHA256}"
url=""
archive_override=""
toolchain_dir="${REPO_ROOT}/toolchain/arm-anykav200-crosstool-ng"
target_triple="arm-unknown-linux-uclibcgnueabi"
output_bin_dir="${REPO_ROOT}/SD_card_contents/anyka_hack/bin"
work_root="${REPO_ROOT}/target/third_party/v4l-utils"
jobs="$(nproc)"

usage() {
  cat <<USAGE
Usage: $(basename "$0") [options]

Options:
  --version <v>         v4l-utils version (default: ${DEFAULT_VERSION})
  --sha256 <sum>        Source archive SHA256 (recommended for reproducible builds)
  --url <url>           Explicit source URL
  --archive <path>      Use existing source archive instead of downloading
  --toolchain-dir <p>   Toolchain base path (default: repo-relative)
  --target <triple>     Target triple (default: ${target_triple})
  --output-bin-dir <p>  Binary output directory (default: SD_card_contents/anyka_hack/bin)
  --work-root <p>       Build working root (default: target/third_party/v4l-utils)
  --jobs <n>            Parallel build jobs (default: nproc; unused for direct compile)
  -h, --help            Show this help

Examples:
  $(basename "$0")
  $(basename "$0") --version ${DEFAULT_VERSION}
  $(basename "$0") --version 1.26.1 --sha256 <hex>
USAGE
}

tp_parse_common_args "$@"
if [ "${#TP_EXTRA_ARGS[@]}" -gt 0 ]; then set -- "${TP_EXTRA_ARGS[@]}"; else set --; fi
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-bin-dir) output_bin_dir="$2"; shift 2 ;;
    -h|--help)        usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
  esac
done

if [ -z "${url}" ]; then
  url="https://linuxtv.org/downloads/v4l-utils/v4l-utils-${version}.tar.xz"
fi

tp_validate_toolchain

mkdir -p "${work_root}" "${output_bin_dir}"

build_dir="${work_root}/v4l-utils-${version}"
build_out="${build_dir}/build-out"

rm -rf "${build_dir}"
tp_fetch_archive "v4l-utils-${version}" "tar.xz" "${url}" "${sha256}" "${archive_override}" "${build_dir}"
# source_dir set by tp_fetch_archive

cross_prefix="${toolchain_dir}/bin/${target_triple}-"
cc="${cross_prefix}gcc"
cxx="${cross_prefix}g++"
strip_bin="${cross_prefix}strip"

v4l2_ctl_dir="${source_dir}/utils/v4l2-ctl"
if [ ! -d "${v4l2_ctl_dir}" ]; then
  echo "ERROR: v4l2-ctl source directory not found at ${v4l2_ctl_dir}" >&2
  exit 1
fi

# ── Build (direct compilation — no meson required) ───────────────────────────
mkdir -p "${build_out}"

common_cflags="-march=armv5te -mtune=arm926ej-s -mfloat-abi=soft -Os --sysroot=${sysroot} -ffunction-sections -fdata-sections"
common_ldflags="-static -Wl,--gc-sections --sysroot=${sysroot}"

# Generate media-bus-format-names.h (normally produced by meson)
echo "Generating media-bus-format-names.h..."
media_bus_hdr="${source_dir}/include/linux/media-bus-format.h"
if [ ! -f "${media_bus_hdr}" ]; then
  media_bus_hdr="${sysroot}/usr/include/linux/media-bus-format.h"
fi
if [ ! -f "${media_bus_hdr}" ]; then
  echo "ERROR: media-bus-format.h not found in source or sysroot" >&2
  exit 1
fi
bash "${source_dir}/utils/gen_media_bus_format_names.sh" "${media_bus_hdr}" \
  > "${build_out}/media-bus-format-names.h"

# Include paths shared by both C and C++ compilations
common_includes=(
  "-I${build_out}"
  "-I${source_dir}"
  "-I${source_dir}/include"
  "-I${source_dir}/lib/include"
  "-I${source_dir}/utils/common"
  "-I${v4l2_ctl_dir}"
  "-I${sysroot}/usr/include"
)

# C++ flags: use gnu++17 (not c++17) for typeof() support in kernel-style macros
cxxflags="${common_cflags} -std=gnu++17 ${common_includes[*]}"
cxxflags+=" -DNO_LIBV4L2"
cxxflags+=" -DPACKAGE_VERSION=\"${version}\""
cxxflags+=" -DGIT_COMMIT_CNT=0"

# C flags for tpg/fwht/v4l-stream support files
cflags="${common_cflags} -std=gnu99 ${common_includes[*]}"

objects=()
failed=0

# Compile C support files from common/
c_sources=(
  "${source_dir}/utils/common/v4l2-tpg-core.c"
  "${source_dir}/utils/common/v4l2-tpg-colors.c"
  "${source_dir}/utils/common/codec-fwht.c"
  "${source_dir}/utils/common/codec-v4l2-fwht.c"
  "${source_dir}/utils/common/v4l-stream.c"
)

echo "Compiling C support files..."
for src in "${c_sources[@]}"; do
  if [ ! -f "${src}" ]; then
    echo "  SKIP: $(basename "${src}") (not found)"
    continue
  fi
  obj="${build_out}/$(basename "${src}" .c).o"
  echo "  CC  $(basename "${src}")"
  # shellcheck disable=SC2086
  if ${cc} ${cflags} -c "${src}" -o "${obj}" 2>&1; then
    objects+=("${obj}")
  else
    echo "  FAILED: $(basename "${src}")"
    failed=$((failed + 1))
  fi
done

# Compile C++ files from v4l2-ctl/ (includes symlinks to common/)
# Do NOT also compile from common/ — that causes duplicate symbols.
cpp_sources=()
for f in "${v4l2_ctl_dir}"/*.cpp; do
  cpp_sources+=("$f")
done

echo "Compiling ${#cpp_sources[@]} C++ source files..."
for src in "${cpp_sources[@]}"; do
  obj="${build_out}/$(basename "${src}" .cpp).o"
  echo "  CXX $(basename "${src}")"
  # shellcheck disable=SC2086
  if ${cxx} ${cxxflags} -c "${src}" -o "${obj}" 2>&1; then
    objects+=("${obj}")
  else
    echo "  FAILED: $(basename "${src}")"
    failed=$((failed + 1))
  fi
done

total_sources=$((${#c_sources[@]} + ${#cpp_sources[@]}))
echo ""
echo "Compiled ${#objects[@]}/${total_sources} files (${failed} failed)"

if [ ${#objects[@]} -eq 0 ]; then
  echo "ERROR: no objects compiled" >&2
  exit 1
fi
if [ "${failed}" -gt 0 ]; then
  echo "ERROR: ${failed} file(s) failed to compile" >&2
  exit 1
fi

echo "Linking v4l2-ctl..."
# shellcheck disable=SC2086
${cxx} ${common_ldflags} -o "${build_out}/v4l2-ctl" "${objects[@]}" \
  -lstdc++ -lm -lpthread -lrt

"${strip_bin}" --strip-all "${build_out}/v4l2-ctl"

echo ""
echo "Binary info:"
file "${build_out}/v4l2-ctl"
ls -lh "${build_out}/v4l2-ctl"

# ── Deploy ───────────────────────────────────────────────────────────────────
echo ""
echo "Installing to ${output_bin_dir}..."
install -m 0755 "${build_out}/v4l2-ctl" "${output_bin_dir}/v4l2-ctl"

echo "v4l2-ctl build complete: ${output_bin_dir}/v4l2-ctl"
