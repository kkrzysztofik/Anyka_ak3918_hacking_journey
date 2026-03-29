#!/usr/bin/env bash
# Build htop for Anyka AK3918 (ARM/uClibc) and stage into SD overlay.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "$(cd "${SCRIPT_DIR}/../.." && pwd)/scripts/common.sh"
REPO_ROOT="${ANYKA_REPO_ROOT}"

DEFAULT_VERSION="3.3.0"
DEFAULT_SHA256="a69acf9b42ff592c4861010fce7d8006805f0d6ef0e8ee647a6ee6e59b743d5c"
DEFAULT_URL="https://github.com/htop-dev/htop/releases/download/${DEFAULT_VERSION}/htop-${DEFAULT_VERSION}.tar.xz"

version="${DEFAULT_VERSION}"
sha256="${DEFAULT_SHA256}"
url=""
archive_override=""
toolchain_dir="${REPO_ROOT}/toolchain/arm-anykav200-crosstool-ng"
target_triple="arm-unknown-linux-uclibcgnueabi"
output_bin_dir="${REPO_ROOT}/SD_card_contents/anyka_hack/bin"
output_lib_dir="${REPO_ROOT}/SD_card_contents/anyka_hack/lib"
output_terminfo_dir="${REPO_ROOT}/SD_card_contents/anyka_hack/share/terminfo"
work_root="${REPO_ROOT}/target/third_party/htop"

usage() {
  cat <<USAGE
Usage: $(basename "$0") [options]

Options:
  --version <v>         htop version (default: ${DEFAULT_VERSION})
  --sha256 <sum>        Source archive SHA256 (default: pinned for ${DEFAULT_VERSION})
  --url <url>           Explicit source URL
  --archive <path>      Use existing source archive instead of downloading
  --toolchain-dir <p>   Toolchain base path
  --target <triple>     Target triple (default: ${target_triple})
  --output-bin-dir <p>  Binary output directory (default: ${output_bin_dir})
  --output-lib-dir <p>  Lib output directory (default: ${output_lib_dir})
  --output-terminfo-dir <p>  Terminfo output directory (default: ${output_terminfo_dir})
  --work-root <p>       Build working root (default: ${work_root})
  -h, --help            Show this help

Examples:
  $(basename "$0")
  $(basename "$0") --version 3.3.0 --sha256 ${DEFAULT_SHA256}
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
    --output-bin-dir)
      output_bin_dir="$2"
      shift 2
      ;;
    --output-lib-dir)
      output_lib_dir="$2"
      shift 2
      ;;
    --output-terminfo-dir)
      output_terminfo_dir="$2"
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
  url="https://github.com/htop-dev/htop/releases/download/${version}/htop-${version}.tar.xz"
fi

if [ ! -d "${toolchain_dir}" ]; then
  echo "ERROR: toolchain not found: ${toolchain_dir}" >&2
  exit 1
fi

mkdir -p "${work_root}" "${output_bin_dir}" "${output_lib_dir}" "${output_terminfo_dir}"

build_dir="${work_root}/htop-${version}"
archive_path="${build_dir}/htop-${version}.tar.xz"
source_dir="${build_dir}/src/htop-${version}"

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
export CFLAGS="--sysroot=${sysroot} -Os"
export LDFLAGS="--sysroot=${sysroot} -L${sysroot}/usr/lib -L${sysroot}/lib"

cd "${source_dir}"

build_mode="dynamic"

configure_and_build() {
  local extra_ldflags="$1"

  make distclean >/dev/null 2>&1 || true

  LDFLAGS="${LDFLAGS} ${extra_ldflags}" ./configure \
    --host="${target_triple}" \
    --enable-unicode=no \
    --prefix=/usr

  make -j"$(nproc)"
}

echo "Attempting static htop build..."
if configure_and_build "-static"; then
  build_mode="static"
else
  echo "Static build failed, retrying with dynamic linking..."
  configure_and_build ""
  build_mode="dynamic"
fi

if [ ! -f "${source_dir}/htop" ]; then
  echo "ERROR: htop binary not produced" >&2
  exit 1
fi

"${target_triple}-strip" "${source_dir}/htop" || true
install -m 0755 "${source_dir}/htop" "${output_bin_dir}/htop.bin"

cat > "${output_bin_dir}/htop" <<'EOF'
#!/bin/sh
# Launcher for Anyka htop with bundled terminfo fallback for SSH sessions.

BIN_DIR="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
ANYKA_HACK_DIR="$(dirname "$BIN_DIR")"
TERMINFO_DIR="${ANYKA_HACK_DIR}/share/terminfo"
HTOP_BIN="${BIN_DIR}/htop.bin"

if [ -d "${TERMINFO_DIR}" ]; then
  export TERMINFO="${TERMINFO_DIR}"
fi

if [ -z "${TERM:-}" ]; then
  TERM="xterm"
fi

if [ -d "${TERMINFO_DIR}" ]; then
  term_subdir="$(printf '%s' "${TERM}" | cut -c1)"
  if [ ! -f "${TERMINFO_DIR}/${term_subdir}/${TERM}" ]; then
    case "${TERM}" in
      *256color*)
        TERM="xterm"
        ;;
      tmux*|screen*)
        TERM="screen"
        ;;
      *)
        TERM="vt100"
        ;;
    esac
  fi
fi

export TERM
exec "${HTOP_BIN}" "$@"
EOF
chmod 0755 "${output_bin_dir}/htop"

echo "Validating output binary..."
file "${output_bin_dir}/htop.bin"
"${target_triple}-readelf" -h "${output_bin_dir}/htop.bin" | sed -n '1,20p'

needed_libs="$("${target_triple}-readelf" -d "${output_bin_dir}/htop.bin" 2>/dev/null | awk '/NEEDED/ { gsub(/\[|\]/, "", $5); print $5 }' || true)"
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
fi

install_terminfo_entry() {
  local term_name="$1"
  local term_subdir="${term_name:0:1}"
  local src_path=""

  for base in /usr/share/terminfo /lib/terminfo /etc/terminfo; do
    if [ -f "${base}/${term_subdir}/${term_name}" ]; then
      src_path="${base}/${term_subdir}/${term_name}"
      break
    fi
  done

  if [ -z "${src_path}" ]; then
    echo "WARNING: terminfo entry not found on build host: ${term_name}" >&2
    return 0
  fi

  mkdir -p "${output_terminfo_dir}/${term_subdir}"
  install -m 0644 "${src_path}" "${output_terminfo_dir}/${term_subdir}/${term_name}"
}

for term_name in xterm xterm-256color screen screen-256color tmux tmux-256color vt100 linux ansi; do
  install_terminfo_entry "${term_name}"
done

echo "htop build complete (${build_mode}): ${output_bin_dir}/htop (wrapper), ${output_bin_dir}/htop.bin"
