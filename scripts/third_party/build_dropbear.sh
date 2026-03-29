#!/usr/bin/env bash
# Build Dropbear for Anyka AK3918 (ARM/uClibc) and stage into SD overlay.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "$(cd "${SCRIPT_DIR}/../.." && pwd)/scripts/common.sh"
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

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      version="$2"
      shift 2
      ;;
    --sha256)
      sha256="$2"
      shift 2
      ;;
    --url)
      url="$2"
      shift 2
      ;;
    --archive)
      archive_override="$2"
      shift 2
      ;;
    --toolchain-dir)
      toolchain_dir="$2"
      shift 2
      ;;
    --target)
      target_triple="$2"
      shift 2
      ;;
    --link-mode)
      link_mode="$2"
      shift 2
      ;;
    --output-dir)
      output_dir="$2"
      shift 2
      ;;
    --output-lib-dir)
      output_lib_dir="$2"
      shift 2
      ;;
    --work-root)
      work_root="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [ -z "${sha256}" ]; then
  echo "ERROR: --sha256 is required" >&2
  exit 1
fi

if [ -z "${url}" ]; then
  url="https://matt.ucc.asn.au/dropbear/releases/dropbear-${version}.tar.bz2"
fi

if [ ! -d "${toolchain_dir}" ]; then
  echo "ERROR: toolchain not found: ${toolchain_dir}" >&2
  exit 1
fi

if [ "${link_mode}" != "static" ] && [ "${link_mode}" != "dynamic" ]; then
  echo "ERROR: --link-mode must be one of: static, dynamic" >&2
  exit 1
fi

mkdir -p "${work_root}" "${output_dir}" "${output_lib_dir}"

build_dir="${work_root}/dropbear-${version}"
archive_path="${build_dir}/dropbear-${version}.tar.bz2"
source_dir="${build_dir}/src/dropbear-${version}"

rm -rf "${build_dir}"
mkdir -p "${build_dir}/src"

fetch_archive() {
  local src_url="$1"
  echo "Downloading: ${src_url}"
  if curl -fL --retry 3 --connect-timeout 20 -o "${archive_path}" "${src_url}"; then
    return 0
  fi
  return 1
}

if [ -n "${archive_override}" ]; then
  if [ ! -f "${archive_override}" ]; then
    echo "ERROR: archive not found: ${archive_override}" >&2
    exit 1
  fi
  cp -f "${archive_override}" "${archive_path}"
else
  if ! fetch_archive "${url}"; then
    echo "Primary URL failed, trying fallback URL..."
    fetch_archive "${FALLBACK_URL/dropbear-${DEFAULT_VERSION}/dropbear-${version}}"
  fi
fi

echo "Verifying checksum..."
echo "${sha256}  ${archive_path}" | sha256sum -c -

echo "Extracting archive..."
tar -xjf "${archive_path}" -C "${build_dir}/src"

sysroot="${toolchain_dir}/${target_triple}/sysroot"
if [ ! -d "${sysroot}" ]; then
  echo "ERROR: sysroot not found: ${sysroot}" >&2
  exit 1
fi

export PATH="${toolchain_dir}/bin:${PATH}"
export CC="${target_triple}-gcc --sysroot=${sysroot}"
export CXX="${target_triple}-g++ --sysroot=${sysroot}"
export AR="${target_triple}-ar"
export RANLIB="${target_triple}-ranlib"
export STRIP="${target_triple}-strip"
export CPPFLAGS="--sysroot=${sysroot} -I${sysroot}/usr/include"
export CFLAGS="${CFLAGS:-} --sysroot=${sysroot} -fno-pie"
export CXXFLAGS="${CXXFLAGS:-} --sysroot=${sysroot} -fno-pie"
base_ldflags="--sysroot=${sysroot} -L${sysroot}/usr/lib -L${sysroot}/lib -no-pie"
if [ "${link_mode}" = "static" ]; then
  export LDFLAGS="${base_ldflags} -static"
else
  export LDFLAGS="${base_ldflags}"
fi

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

echo "Validating output binary..."
file "${output_dir}/dropbearmulti"
"${target_triple}-readelf" -h "${output_dir}/dropbearmulti" | sed -n '1,20p'

if "${target_triple}-readelf" -d "${output_dir}/dropbearmulti" 2>/dev/null | grep -q NEEDED; then
  if [ "${link_mode}" = "static" ]; then
    echo "ERROR: static link mode requested but dynamic dependencies are still present" >&2
    exit 1
  fi
  echo "Dynamic dependencies detected; bundling from sysroot into ${output_lib_dir}"
  needed_libs="$("${target_triple}-readelf" -d "${output_dir}/dropbearmulti" 2>/dev/null | awk '/NEEDED/ { gsub(/\[|\]/, "", $5); print $5 }')"
  for needed in ${needed_libs}; do
    src_path=""
    if [ -e "${sysroot}/lib/${needed}" ]; then
      src_path="${sysroot}/lib/${needed}"
    elif [ -e "${sysroot}/usr/lib/${needed}" ]; then
      src_path="${sysroot}/usr/lib/${needed}"
    fi

    if [ -z "${src_path}" ]; then
      echo "WARNING: could not find ${needed} in sysroot" >&2
      continue
    fi

    resolved_path="$(readlink -f "${src_path}")"
    resolved_name="$(basename "${resolved_path}")"
    install -m 0644 "${resolved_path}" "${output_lib_dir}/${resolved_name}"
    if [ "${needed}" != "${resolved_name}" ]; then
      install -m 0644 "${resolved_path}" "${output_lib_dir}/${needed}"
    fi
  done
else
  echo "dropbearmulti has no dynamic dependencies"
fi

echo "Dropbear build complete: ${output_dir}/dropbearmulti"
