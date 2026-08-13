#!/usr/bin/env bash
# Build Dropbear for Anyka AK3918 (ARM/uClibc) and stage into SD overlay.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "$(cd "${SCRIPT_DIR}/../.." && pwd)/scripts/common.sh"
# shellcheck source=scripts/third_party/build_lib.sh
source "${SCRIPT_DIR}/build_lib.sh"
REPO_ROOT="${ANYKA_REPO_ROOT}"

DEFAULT_VERSION="2024.86"
DEFAULT_SHA256="e78936dffc395f2e0db099321d6be659190966b99712b55c530dd0a1822e0a5e"
DEFAULT_URL="https://matt.ucc.asn.au/dropbear/releases/dropbear-${DEFAULT_VERSION}.tar.bz2"
FALLBACK_URL="https://launchpad.net/dropbear/trunk/${DEFAULT_VERSION}/+download/dropbear-${DEFAULT_VERSION}.tar.bz2"

version="${DEFAULT_VERSION}"
sha256="${DEFAULT_SHA256}"
url=""
archive_override=""
toolchain_dir="${REPO_ROOT}/toolchain/arm-anykav200-crosstool-ng"
target_triple="arm-unknown-linux-uclibcgnueabi"
link_mode="static"
output_dir="${REPO_ROOT}/SD_card_contents/anyka_hack/dropbear"
output_lib_dir="${REPO_ROOT}/SD_card_contents/anyka_hack/lib"
work_root="${REPO_ROOT}/target/third_party/dropbear"

usage() {
  cat <<USAGE
Usage: $(basename "$0") [options]

Options:
  --version <v>         Dropbear version (default: ${DEFAULT_VERSION})
  --sha256 <sum>        Source archive SHA256 (default: pinned for ${DEFAULT_VERSION})
  --url <url>           Explicit source URL
  --archive <path>      Use existing source archive instead of downloading
  --toolchain-dir <p>   Toolchain base path
  --target <triple>     Target triple (default: ${target_triple})
  --link-mode <mode>    Link mode: static|dynamic (default: ${link_mode})
  --output-dir <p>      Output directory (default: ${output_dir})
  --output-lib-dir <p>  Runtime lib output directory (default: ${output_lib_dir})
  --work-root <p>       Build working root (default: ${work_root})
  -h, --help            Show this help

Examples:
  $(basename "$0")
  $(basename "$0") --version 2024.86 --sha256 ${DEFAULT_SHA256}
USAGE
}

tp_parse_common_args "$@"
if [ "${#TP_EXTRA_ARGS[@]}" -gt 0 ]; then set -- "${TP_EXTRA_ARGS[@]}"; else set --; fi
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-dir)     output_dir="$2";     shift 2 ;;
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
  url="https://matt.ucc.asn.au/dropbear/releases/dropbear-${version}.tar.bz2"
fi

tp_validate_toolchain
tp_validate_link_mode

mkdir -p "${work_root}" "${output_dir}" "${output_lib_dir}"

build_dir="${work_root}/dropbear-${version}"
rm -rf "${build_dir}"

# Dropbear has a fallback URL; derive it from the template URL.
fallback_url="${FALLBACK_URL/dropbear-${DEFAULT_VERSION}/dropbear-${version}}"
tp_fetch_archive "dropbear-${version}" "tar.bz2" "${url}" "${sha256}" "${archive_override}" "${build_dir}" "${fallback_url}"
# source_dir set by tp_fetch_archive

# Dropbear requires --sysroot embedded in CC/CXX (configure does not honour
# CPPFLAGS/CFLAGS --sysroot reliably for all checks).
tp_setup_cross_env
export CC="${target_triple}-gcc --sysroot=${sysroot}"
export CXX="${target_triple}-g++ --sysroot=${sysroot}"
export CFLAGS="${CFLAGS:-} --sysroot=${sysroot} -fno-pie"
export CXXFLAGS="${CXXFLAGS:-} --sysroot=${sysroot} -fno-pie"
export LDFLAGS="$(tp_ldflags) -no-pie"

cd "${source_dir}"

echo "Configuring dropbear ${version} (link mode: ${link_mode})..."
./configure \
  --host="${target_triple}" \
  --prefix=/usr \
  --disable-harden \
  --disable-zlib \
  --disable-lastlog \
  --disable-utmp \
  --disable-utmpx \
  --disable-wtmp \
  --disable-wtmpx \
  --disable-pututline \
  --disable-loginfunc \
  --disable-shadow

echo "Building dropbearmulti..."
make -j"$(nproc)" PROGRAMS="dropbear dropbearkey scp" MULTI=1

if [ ! -f "${source_dir}/dropbearmulti" ]; then
  echo "ERROR: dropbearmulti not produced" >&2
  exit 1
fi

"${target_triple}-strip" "${source_dir}/dropbearmulti" || true

install -m 0755 "${source_dir}/dropbearmulti" "${output_dir}/dropbearmulti"

tp_show_binary_info "${output_dir}/dropbearmulti"
tp_bundle_libs "${output_dir}/dropbearmulti" "${output_lib_dir}"

echo "Dropbear build complete: ${output_dir}/dropbearmulti"
