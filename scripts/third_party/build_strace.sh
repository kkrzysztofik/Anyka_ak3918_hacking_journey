#!/usr/bin/env bash
# Build strace for Anyka AK3918 (ARM/uClibc) and stage into SD overlay.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "$(cd "${SCRIPT_DIR}/../.." && pwd)/scripts/common.sh"
# shellcheck source=scripts/third_party/build_lib.sh
source "${SCRIPT_DIR}/build_lib.sh"
REPO_ROOT="${ANYKA_REPO_ROOT}"

DEFAULT_VERSION="6.8"
DEFAULT_SHA256="ba6950a96824cdf93a584fa04f0a733896d2a6bc5f0ad9ffe505d9b41e970149"

version="${DEFAULT_VERSION}"
sha256="${DEFAULT_SHA256}"
url=""
archive_override=""
toolchain_dir="${REPO_ROOT}/toolchain/arm-anykav200-crosstool-ng"
target_triple="arm-unknown-linux-uclibcgnueabi"
link_mode="static"
output_bin_dir="${REPO_ROOT}/SD_card_contents/anyka_hack/bin"
output_lib_dir="${REPO_ROOT}/SD_card_contents/anyka_hack/lib"
work_root="${REPO_ROOT}/target/third_party/strace"
jobs="$(nproc)"

usage() {
  cat <<USAGE
Usage: $(basename "$0") [options]

Options:
  --version <v>         strace version (default: ${DEFAULT_VERSION})
  --sha256 <sum>        Source archive SHA256 (default: pinned for ${DEFAULT_VERSION})
  --url <url>           Explicit source URL
  --archive <path>      Use existing source archive instead of downloading
  --toolchain-dir <p>   Toolchain base path
  --target <triple>     Target triple (default: ${target_triple})
  --link-mode <mode>    Link mode: static|dynamic (default: ${link_mode})
  --output-bin-dir <p>  Binary output directory (default: ${output_bin_dir})
  --output-lib-dir <p>  Runtime lib output directory (default: ${output_lib_dir})
  --work-root <p>       Build working root (default: ${work_root})
  --jobs <n>            Parallel build jobs (default: nproc)
  -h, --help            Show this help

Examples:
  $(basename "$0")
  $(basename "$0") --link-mode static
USAGE
}

tp_parse_common_args "$@"
if [ "${#TP_EXTRA_ARGS[@]}" -gt 0 ]; then set -- "${TP_EXTRA_ARGS[@]}"; else set --; fi
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-bin-dir) output_bin_dir="$2"; shift 2 ;;
    --output-lib-dir) output_lib_dir="$2"; shift 2 ;;
    -h|--help)        usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
  esac
done

if [ -z "${sha256}" ]; then
  echo "ERROR: --sha256 is required" >&2
  exit 1
fi

if [ -z "${url}" ]; then
  url="https://github.com/strace/strace/releases/download/v${version}/strace-${version}.tar.xz"
fi

tp_validate_toolchain
tp_validate_link_mode
tp_validate_jobs

mkdir -p "${work_root}" "${output_bin_dir}" "${output_lib_dir}"

build_dir="${work_root}/strace-${version}"
rm -rf "${build_dir}"
tp_fetch_archive "strace-${version}" "tar.xz" "${url}" "${sha256}" "${archive_override}" "${build_dir}"
# source_dir set by tp_fetch_archive

tp_setup_cross_env
export CFLAGS="--sysroot=${sysroot} -Os -Wno-error=unterminated-string-initialization"
export LDFLAGS="$(tp_ldflags)"

cd "${source_dir}"

werror_opt=""
if ./configure --help | grep -q -- '--disable-gcc-Werror'; then
  werror_opt="--disable-gcc-Werror"
fi

echo "Configuring strace ${version} (link mode: ${link_mode})..."
./configure \
  --host="${target_triple}" \
  --prefix=/usr \
  --enable-mpers=no \
  --without-libunwind \
  --without-libdw \
  ${werror_opt}

# Newer compilers may promote warnings to errors in upstream defaults.
# Strip explicit -Werror occurrences from generated makefiles for toolchain compatibility.
find . -name 'Makefile' -type f -exec sed -i 's/[[:space:]]-Werror[[:space:]]/ /g' {} +

echo "Building strace..."
make -j"${jobs}"

strace_bin=""
if [ -f "${source_dir}/src/strace" ]; then
  strace_bin="${source_dir}/src/strace"
elif [ -f "${source_dir}/strace" ]; then
  strace_bin="${source_dir}/strace"
else
  echo "ERROR: strace binary not produced" >&2
  exit 1
fi

"${target_triple}-strip" "${strace_bin}" || true
install -m 0755 "${strace_bin}" "${output_bin_dir}/strace.bin"

cat > "${output_bin_dir}/strace" <<'WRAP'
#!/bin/sh
# Launcher for strace with bundled runtime libraries.

BIN_DIR="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
ANYKA_HACK_DIR="$(dirname "$BIN_DIR")"
STRACE_BIN="${BIN_DIR}/strace.bin"
LIB_DIR="${ANYKA_HACK_DIR}/lib"

if [ -d "${LIB_DIR}" ]; then
  if [ -n "${LD_LIBRARY_PATH:-}" ]; then
    export LD_LIBRARY_PATH="${LIB_DIR}:${LD_LIBRARY_PATH}"
  else
    export LD_LIBRARY_PATH="${LIB_DIR}"
  fi
fi

exec "${STRACE_BIN}" "$@"
WRAP
chmod 0755 "${output_bin_dir}/strace"

tp_show_binary_info "${output_bin_dir}/strace.bin"
tp_bundle_libs "${output_bin_dir}/strace.bin" "${output_lib_dir}"

echo "strace build complete: ${output_bin_dir}/strace"
