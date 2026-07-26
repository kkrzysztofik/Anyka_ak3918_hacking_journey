#!/usr/bin/env bash
# scripts/third_party/build_lib.sh — Shared helpers for Anyka third-party cross-compilation scripts.
#
# Source this file from any build_*.sh script in the same directory:
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   source "${SCRIPT_DIR}/build_lib.sh"
#
# All helper functions read and write documented global variables.

[[ -n "${_ANYKA_BUILD_LIB_SH:-}" ]] && return 0
readonly _ANYKA_BUILD_LIB_SH=1

# ── Argument parsing ──────────────────────────────────────────────────────────
# tp_parse_common_args "$@"
#
# Processes the standard cross-build CLI options and sets the corresponding
# global variables.  Unrecognised options (including -h/--help) are collected
# in TP_EXTRA_ARGS so each caller script can handle its own flags.
#
# Variables set (only when the respective option is passed):
#   version, sha256, url, archive_override,
#   toolchain_dir, target_triple, link_mode, work_root, jobs
#
# Populates:
#   TP_EXTRA_ARGS  — array of arguments not consumed by this function
tp_parse_common_args() {
  TP_EXTRA_ARGS=()
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --version)        version="$2";           shift 2 ;;
      --sha256)         sha256="$2";            shift 2 ;;
      --url)            url="$2";               shift 2 ;;
      --archive)        archive_override="$2";  shift 2 ;;
      --toolchain-dir)  toolchain_dir="$2";     shift 2 ;;
      --target)         target_triple="$2";     shift 2 ;;
      --link-mode)      link_mode="$2";         shift 2 ;;
      --work-root)      work_root="$2";         shift 2 ;;
      --jobs)           jobs="$2";              shift 2 ;;
      *)                TP_EXTRA_ARGS+=("$1");  shift   ;;
    esac
  done
}

# ── Toolchain validation ──────────────────────────────────────────────────────
# tp_validate_toolchain
#
# Verifies that toolchain_dir and its sysroot exist, prepends the toolchain
# bin directory to PATH, and sets the global variable:
#   sysroot  — <toolchain_dir>/<target_triple>/sysroot
#
# Reads: toolchain_dir, target_triple
tp_validate_toolchain() {
  if [ ! -d "${toolchain_dir}" ]; then
    echo "ERROR: toolchain not found: ${toolchain_dir}" >&2
    exit 1
  fi
  sysroot="${toolchain_dir}/${target_triple}/sysroot"
  if [ ! -d "${sysroot}" ]; then
    echo "ERROR: sysroot not found: ${sysroot}" >&2
    exit 1
  fi
  export PATH="${toolchain_dir}/bin:${PATH}"
}

# tp_validate_link_mode
#
# Exits with an error if link_mode is not "static" or "dynamic".
tp_validate_link_mode() {
  if [ "${link_mode}" != "static" ] && [ "${link_mode}" != "dynamic" ]; then
    echo "ERROR: --link-mode must be one of: static, dynamic" >&2
    exit 1
  fi
}

# tp_validate_jobs
#
# Exits with an error if jobs is not a positive integer.
tp_validate_jobs() {
  if ! [ "${jobs}" -gt 0 ] 2>/dev/null; then
    echo "ERROR: --jobs must be a positive integer" >&2
    exit 1
  fi
}

# ── Archive fetch and extract ─────────────────────────────────────────────────
# tp_fetch_archive <basename> <ext> <url> <sha256> <archive_override> <dest_dir> [fallback_url]
#
# Downloads (or copies) a source archive, optionally verifies its SHA-256
# checksum, and extracts it.
#
# Parameters:
#   basename         — archive and source directory name (e.g. "strace-6.8")
#   ext              — archive extension without leading dot, e.g. "tar.xz" or "tar.bz2"
#   url              — primary download URL (ignored when archive_override is non-empty)
#   sha256           — expected hex digest, or "" to skip verification
#   archive_override — path to an existing local archive, or "" to download
#   dest_dir         — working directory; archive stored at <dest_dir>/<basename>.<ext>
#                      and extracted into <dest_dir>/src/
#   fallback_url     — (optional) secondary URL tried when the primary download fails
#
# Sets global:
#   source_dir  — <dest_dir>/src/<basename>
tp_fetch_archive() {
  local basename="$1"
  local ext="$2"
  local fetch_url="$3"
  local fetch_sha256="$4"
  local override="$5"
  local dest_dir="$6"
  local fallback_url="${7:-}"

  local archive_path="${dest_dir}/${basename}.${ext}"
  mkdir -p "${dest_dir}/src"

  if [ -n "${override}" ]; then
    if [ ! -f "${override}" ]; then
      echo "ERROR: archive not found: ${override}" >&2
      exit 1
    fi
    cp -f "${override}" "${archive_path}"
  else
    echo "Downloading: ${fetch_url}"
    if ! curl -fL --retry 3 --connect-timeout 20 -o "${archive_path}" "${fetch_url}"; then
      if [ -n "${fallback_url}" ]; then
        echo "Primary URL failed, trying fallback: ${fallback_url}"
        curl -fL --retry 3 --connect-timeout 20 -o "${archive_path}" "${fallback_url}"
      else
        echo "ERROR: download failed: ${fetch_url}" >&2
        exit 1
      fi
    fi
  fi

  if [ -n "${fetch_sha256}" ]; then
    echo "Verifying checksum..."
    echo "${fetch_sha256}  ${archive_path}" | sha256sum -c -
  else
    echo "WARNING: no --sha256 provided; skipping checksum verification" >&2
  fi

  echo "Extracting..."
  case "${ext}" in
    *bz2)  tar -xjf "${archive_path}" -C "${dest_dir}/src" ;;
    *gz)   tar -xzf "${archive_path}" -C "${dest_dir}/src" ;;
    *)     tar -xJf "${archive_path}" -C "${dest_dir}/src" ;;
  esac

  source_dir="${dest_dir}/src/${basename}"
  if [ ! -d "${source_dir}" ]; then
    echo "ERROR: source directory not found after extraction: ${source_dir}" >&2
    exit 1
  fi
}

# ── Cross-compilation environment ─────────────────────────────────────────────
# tp_setup_cross_env
#
# Exports the standard cross-compilation toolchain variables:
#   CC, CXX, AR, RANLIB, STRIP, CPPFLAGS
#
# CFLAGS and LDFLAGS are intentionally left to each caller since they vary
# per project (optimisation flags, -fno-pie, -Wno-error=..., etc.).
#
# Reads: target_triple, sysroot (set by tp_validate_toolchain)
tp_setup_cross_env() {
  export CC="${target_triple}-gcc"
  export CXX="${target_triple}-g++"
  export AR="${target_triple}-ar"
  export RANLIB="${target_triple}-ranlib"
  export STRIP="${target_triple}-strip"
  export CPPFLAGS="--sysroot=${sysroot} -I${sysroot}/usr/include"
}

# tp_ldflags
#
# Prints the base LDFLAGS string for the current link_mode.
# Append project-specific flags after capturing this output.
#
# Reads: sysroot, link_mode (defaults to "dynamic" when unset)
tp_ldflags() {
  local base="--sysroot=${sysroot} -L${sysroot}/usr/lib -L${sysroot}/lib"
  if [ "${link_mode:-dynamic}" = "static" ]; then
    printf '%s -static\n' "${base}"
  else
    printf '%s\n' "${base}"
  fi
}

# ── Binary info ───────────────────────────────────────────────────────────────
# tp_show_binary_info <binary>
#
# Prints file type, size, and ELF header summary for the given binary.
# Reads: target_triple
tp_show_binary_info() {
  local binary="$1"
  echo ""
  echo "Binary info:"
  file "${binary}"
  ls -lh "${binary}"
  "${target_triple}-readelf" -h "${binary}" | sed -n '1,20p'
}

# ── Dynamic library bundling ──────────────────────────────────────────────────
# tp_bundle_libs <binary> <lib_dir> [strict]
#
# Reads NEEDED entries from <binary> using readelf and copies the corresponding
# sysroot libraries (and their resolved symlink targets) into <lib_dir>.
#
# If the optional third argument is "strict", or if link_mode=static, and
# dynamic dependencies are found, the script exits with an error.
#
# Reads: target_triple, sysroot, link_mode (defaults to "dynamic" when unset)
tp_bundle_libs() {
  local binary="$1"
  local lib_dir="$2"
  local strict="${3:-}"

  local needed_libs
  needed_libs="$("${target_triple}-readelf" -d "${binary}" 2>/dev/null \
    | awk '/NEEDED/ { gsub(/\[|\]/, "", $5); print $5 }' || true)"

  if [ -z "${needed_libs}" ]; then
    echo "$(basename "${binary}") has no dynamic dependencies"
    return 0
  fi

  if [ "${strict}" = "strict" ] || [ "${link_mode:-dynamic}" = "static" ]; then
    echo "ERROR: static link mode requested but $(basename "${binary}") has dynamic dependencies:" >&2
    echo "${needed_libs}" | sed 's/^/  /' >&2
    echo "hint: check that static libs are present in sysroot, or use --link-mode dynamic" >&2
    exit 1
  fi

  echo "Dynamic dependencies detected; bundling from sysroot into ${lib_dir}"
  mkdir -p "${lib_dir}"

  local needed src_path resolved_path resolved_name
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
    install -m 0644 "${resolved_path}" "${lib_dir}/${resolved_name}"
    if [ "${needed}" != "${resolved_name}" ]; then
      install -m 0644 "${resolved_path}" "${lib_dir}/${needed}"
    fi
  done
}
