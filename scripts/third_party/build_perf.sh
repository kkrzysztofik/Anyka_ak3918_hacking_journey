#!/usr/bin/env bash
# Build linux tools/perf for Anyka AK3918 (ARM/uClibc) and stage into SD overlay.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

DEFAULT_VERSION="3.4.35"
DEFAULT_SHA256="4421d79d5f2c75af9f9ed1fe206a8365d0371cc9551dc8f1e1a5ef95df07d4bf"

version="${DEFAULT_VERSION}"
sha256="${DEFAULT_SHA256}"
sha256_explicit="0"
url=""
archive_override=""
toolchain_dir="${REPO_ROOT}/toolchain/arm-anykav200-crosstool-ng"
target_triple="arm-unknown-linux-uclibcgnueabi"
link_mode="static"
output_bin_dir="${REPO_ROOT}/SD_card_contents/anyka_hack/bin"
output_lib_dir="${REPO_ROOT}/SD_card_contents/anyka_hack/lib"
work_root="${REPO_ROOT}/target/third_party/perf"
jobs="$(nproc)"

default_kernel_url_for_version() {
  ver="$1"
  major="$(printf '%s\n' "${ver}" | awk -F. '{print $1}')"
  case "${major}" in
    ''|*[!0-9]*)
      return 1
      ;;
  esac
  printf 'https://cdn.kernel.org/pub/linux/kernel/v%s.x/linux-%s.tar.xz\n' "${major}" "${ver}"
}

usage() {
  cat <<USAGE
Usage: $(basename "$0") [options]

Options:
  --version <v>         Linux kernel version for perf source (default: ${DEFAULT_VERSION})
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
  $(basename "$0") --version ${DEFAULT_VERSION} --sha256 ${DEFAULT_SHA256}
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
      sha256_explicit="1"
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

if [ "${version}" != "${DEFAULT_VERSION}" ] && [ "${sha256_explicit}" != "1" ]; then
  echo "ERROR: --sha256 is required when --version differs from default (${DEFAULT_VERSION})" >&2
  echo "hint: provide the checksum for linux-${version}.tar.xz from kernel.org" >&2
  exit 1
fi

if [ -z "${url}" ]; then
  if ! url="$(default_kernel_url_for_version "${version}")"; then
    echo "ERROR: unsupported --version format for automatic URL: ${version}" >&2
    echo "hint: use --url explicitly or pass a numeric version like 3.4.35 / 5.15.170" >&2
    exit 1
  fi
fi

if [ ! -d "${toolchain_dir}" ]; then
  echo "ERROR: toolchain not found: ${toolchain_dir}" >&2
  exit 1
fi

if ! [ "${jobs}" -gt 0 ] 2>/dev/null; then
  echo "ERROR: --jobs must be a positive integer" >&2
  exit 1
fi

if [ "${link_mode}" != "static" ] && [ "${link_mode}" != "dynamic" ]; then
  echo "ERROR: --link-mode must be one of: static, dynamic" >&2
  exit 1
fi

mkdir -p "${work_root}" "${output_bin_dir}" "${output_lib_dir}"

build_dir="${work_root}/linux-${version}"
archive_path="${build_dir}/linux-${version}.tar.xz"
source_root="${build_dir}/src/linux-${version}"
perf_dir="${source_root}/tools/perf"

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

if [ ! -d "${perf_dir}" ]; then
  echo "ERROR: perf source directory not found: ${perf_dir}" >&2
  exit 1
fi

sysroot="${toolchain_dir}/${target_triple}/sysroot"
if [ ! -d "${sysroot}" ]; then
  echo "ERROR: sysroot not found: ${sysroot}" >&2
  exit 1
fi

export PATH="${toolchain_dir}/bin:${PATH}"

cd "${perf_dir}"

echo "Building perf ${version} for ${target_triple}..."
make clean >/dev/null 2>&1 || true

gelf_compat_cflags=""
# Older libelf headers (like those in the Anyka toolchain sysroot) don't define
# GElf_Nhdr, but linux-3.4.x perf expects it in util/symbol.c.
if [ -f "${sysroot}/usr/include/libelf/gelf.h" ] && \
   ! grep -q "GElf_Nhdr" "${sysroot}/usr/include/libelf/gelf.h"; then
  echo "Enabling GElf_Nhdr compatibility header for old libelf..."
  compat_header="${perf_dir}/util/gelf_compat.h"
  cat > "${compat_header}" <<'EOF'
#ifndef PERF_GELF_NHDR_COMPAT_H
#define PERF_GELF_NHDR_COMPAT_H

#include <stdint.h>

typedef struct {
	uint32_t n_namesz;
	uint32_t n_descsz;
	uint32_t n_type;
} GElf_Nhdr;

#endif
EOF
  gelf_compat_cflags="-include ${compat_header}"
fi

base_ldflags="--sysroot=${sysroot} -L${sysroot}/usr/lib -L${sysroot}/lib"
if [ "${link_mode}" = "static" ]; then
  extra_ldflags="${base_ldflags} -static"
else
  extra_ldflags="${base_ldflags}"
fi

make -j"${jobs}" \
  ARCH=arm \
  CROSS_COMPILE="${target_triple}-" \
  PKG_CONFIG=false \
  LDFLAGS="${extra_ldflags}" \
  EXTRA_CFLAGS="--sysroot=${sysroot} ${gelf_compat_cflags}" \
  EXTRA_LDFLAGS="${extra_ldflags}" \
  NO_LIBPERL=1 \
  NO_LIBPYTHON=1 \
  NO_JVMTI=1 \
  NO_LIBNUMA=1 \
  NO_NEWT=1 \
  NO_SLANG=1 \
  NO_GTK2=1 \
  NO_LIBUNWIND=1 \
  NO_LIBAUDIT=1 \
  NO_LIBBIONIC=1 \
  NO_LIBBPF=1 \
  NO_LIBELF=1 \
  NO_ZSTD=1 \
  NO_LIBTRACEEVENT=1 \
  NO_LIBTRACEFS=1 \
  WERROR=0

if [ ! -f "${perf_dir}/perf" ]; then
  echo "ERROR: perf binary not produced" >&2
  exit 1
fi

"${target_triple}-strip" "${perf_dir}/perf" || true
install -m 0755 "${perf_dir}/perf" "${output_bin_dir}/perf.bin"

cat > "${output_bin_dir}/perf" <<'WRAP'
#!/bin/sh
# Launcher for perf with bundled runtime libraries.

BIN_DIR="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
ANYKA_HACK_DIR="$(dirname "$BIN_DIR")"
PERF_BIN="${BIN_DIR}/perf.bin"
LIB_DIR="${ANYKA_HACK_DIR}/lib"

if [ -d "${LIB_DIR}" ]; then
  if [ -n "${LD_LIBRARY_PATH:-}" ]; then
    export LD_LIBRARY_PATH="${LIB_DIR}:${LD_LIBRARY_PATH}"
  else
    export LD_LIBRARY_PATH="${LIB_DIR}"
  fi
fi

exec "${PERF_BIN}" "$@"
WRAP
chmod 0755 "${output_bin_dir}/perf"

echo "Validating output binary..."
file "${output_bin_dir}/perf.bin"
"${target_triple}-readelf" -h "${output_bin_dir}/perf.bin" | sed -n '1,20p'

needed_libs="$("${target_triple}-readelf" -d "${output_bin_dir}/perf.bin" 2>/dev/null | awk '/NEEDED/ { gsub(/\[|\]/, "", $5); print $5 }' || true)"
if [ "${link_mode}" = "static" ] && [ -n "${needed_libs}" ]; then
  echo "ERROR: static link requested but perf.bin still has dynamic dependencies:" >&2
  while IFS= read -r needed; do
    [ -n "${needed}" ] && echo "  - ${needed}" >&2
  done <<EOF
${needed_libs}
EOF
  echo "hint: static libs may be missing in sysroot; use --link-mode dynamic if needed." >&2
  exit 1
fi

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
  echo "perf has no dynamic dependencies"
fi

echo "perf build complete: ${output_bin_dir}/perf"
