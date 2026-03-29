#!/usr/bin/env bash
# Build strace for Anyka AK3918 (ARM/uClibc) and stage into SD overlay.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "$(cd "${SCRIPT_DIR}/../.." && pwd)/scripts/common.sh"
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
    --output-bin-dir)
      output_bin_dir="$2"
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
    --jobs)
      jobs="$2"
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
  url="https://github.com/strace/strace/releases/download/v${version}/strace-${version}.tar.xz"
fi

if [ ! -d "${toolchain_dir}" ]; then
  echo "ERROR: toolchain not found: ${toolchain_dir}" >&2
  exit 1
fi

if [ "${link_mode}" != "static" ] && [ "${link_mode}" != "dynamic" ]; then
  echo "ERROR: --link-mode must be one of: static, dynamic" >&2
  exit 1
fi

if ! [ "${jobs}" -gt 0 ] 2>/dev/null; then
  echo "ERROR: --jobs must be a positive integer" >&2
  exit 1
fi

mkdir -p "${work_root}" "${output_bin_dir}" "${output_lib_dir}"

build_dir="${work_root}/strace-${version}"
archive_path="${build_dir}/strace-${version}.tar.xz"
source_dir="${build_dir}/src/strace-${version}"

rm -rf "${build_dir}"
mkdir -p "${build_dir}/src"

if [ -n "${archive_override}" ]; then
  if [ ! -f "${archive_override}" ]; then
    echo "ERROR: archive not found: ${archive_override}" >&2
    exit 1
  fi
  cp -f "${archive_override}" "${archive_path}"
else
  echo "Downloading: ${url}"
  curl -fL --retry 3 --connect-timeout 20 -o "${archive_path}" "${url}"
fi

echo "Verifying checksum..."
echo "${sha256}  ${archive_path}" | sha256sum -c -

echo "Extracting archive..."
tar -xJf "${archive_path}" -C "${build_dir}/src"

if [ ! -d "${source_dir}" ]; then
  echo "ERROR: source directory not found: ${source_dir}" >&2
  exit 1
fi

sysroot="${toolchain_dir}/${target_triple}/sysroot"
if [ ! -d "${sysroot}" ]; then
  echo "ERROR: sysroot not found: ${sysroot}" >&2
  exit 1
fi

export PATH="${toolchain_dir}/bin:${PATH}"
export CC="${target_triple}-gcc"
export CXX="${target_triple}-g++"
export AR="${target_triple}-ar"
export RANLIB="${target_triple}-ranlib"
export STRIP="${target_triple}-strip"
export CPPFLAGS="--sysroot=${sysroot} -I${sysroot}/usr/include"
export CFLAGS="--sysroot=${sysroot} -Os -Wno-error=unterminated-string-initialization"

base_ldflags="--sysroot=${sysroot} -L${sysroot}/usr/lib -L${sysroot}/lib"
if [ "${link_mode}" = "static" ]; then
  export LDFLAGS="${base_ldflags} -static"
else
  export LDFLAGS="${base_ldflags}"
fi

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

echo "Validating output binary..."
file "${output_bin_dir}/strace.bin"
"${target_triple}-readelf" -h "${output_bin_dir}/strace.bin" | sed -n '1,20p'

needed_libs="$("${target_triple}-readelf" -d "${output_bin_dir}/strace.bin" 2>/dev/null | awk '/NEEDED/ { gsub(/\[|\]/, "", $5); print $5 }' || true)"
if [ -n "${needed_libs}" ]; then
  echo "Dynamic dependencies detected; bundling from sysroot into ${output_lib_dir}"
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
  echo "strace has no dynamic dependencies"
fi

echo "strace build complete: ${output_bin_dir}/strace"
