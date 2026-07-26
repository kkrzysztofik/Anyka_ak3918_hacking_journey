#!/usr/bin/env bash

# Anyka project toolchain environment setup.
#
# This script exports portable paths derived from its location (works for any clone
# path, e.g. ~/dev/anyka-dev). Use ANYKA_REPO_ROOT in shell commands and scripts
# instead of hardcoding home-directory paths.
#
# rust-analyzer does not execute this file; use ${workspaceFolder} or repo-relative
# paths in .vscode/settings.json and rust-analyzer.toml. The Cursor/VS Code integrated
# terminal can load it via the "Anyka (setenv)" profile in .vscode/settings.json
# (.vscode/terminal-anyka.bashrc). Tasks in .vscode/tasks.json source this script.
#
# Usage:
#   source ./setenv.sh

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  echo "This script must be sourced, not executed."
  echo "Run: source ./setenv.sh"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export ANYKA_REPO_ROOT="${SCRIPT_DIR}"
export ANYKA_TOOLCHAIN_BIN="${ANYKA_REPO_ROOT}/toolchain/arm-anykav200-crosstool-ng/bin"

if [[ ! -d "${ANYKA_TOOLCHAIN_BIN}" ]]; then
  echo "Anyka setenv: ANYKA_TOOLCHAIN_BIN is not a directory: ${ANYKA_TOOLCHAIN_BIN} (ANYKA_REPO_ROOT=${ANYKA_REPO_ROOT})" >&2
  return 1
fi

export CARGO="${ANYKA_TOOLCHAIN_BIN}/cargo"
export RUSTC="${ANYKA_TOOLCHAIN_BIN}/rustc"
export RUSTDOC="${ANYKA_TOOLCHAIN_BIN}/rustdoc"

# crosstool/common.sh exports SYSROOT for the GCC uClibc tree; that confuses
# clippy-driver into using it as the Rust sysroot. Clear it for day-to-day use.
unset SYSROOT

_anyka_tool_missing=0
for _tool in CARGO RUSTC RUSTDOC; do
  _bin="${!_tool}"
  if [[ ! -x "${_bin}" ]]; then
    echo "Anyka setenv: missing or non-executable ${_tool}=${_bin}" >&2
    _anyka_tool_missing=1
  fi
done
if [[ "${_anyka_tool_missing}" -ne 0 ]]; then
  unset _anyka_tool_missing _tool _bin
  return 1
fi

unset _anyka_tool_missing _tool _bin

# Always put the vendored toolchain first so rustup shims in ~/.cargo/bin
# cannot win on `which rustc` / build-script discovery.
_anyka_path="${PATH}"
_anyka_path="${_anyka_path//:${ANYKA_TOOLCHAIN_BIN}/}"
_anyka_path="${_anyka_path//${ANYKA_TOOLCHAIN_BIN}:/}"
_anyka_path="${_anyka_path#${ANYKA_TOOLCHAIN_BIN}}"
export PATH="${ANYKA_TOOLCHAIN_BIN}:${_anyka_path}"
unset _anyka_path

# Cargo resolves subcommands from $CARGO_HOME/bin *before* PATH. rustup installs
# cargo-clippy / rustfmt proxies there; those pull rustup's clippy-driver and
# clash with this toolchain's rustc (1.97.1-dev vs rustup 1.97.1 → E0514).
# Use a project-local CARGO_HOME that reuses the user's crate cache but has an
# empty bin/ so vendored cargo-* on PATH wins.
_anyka_cargo_home="${ANYKA_REPO_ROOT}/toolchain/cargo-home"
mkdir -p "${_anyka_cargo_home}/bin"
if [[ -d "${HOME}/.cargo/registry" && ! -e "${_anyka_cargo_home}/registry" ]]; then
  ln -sfn "${HOME}/.cargo/registry" "${_anyka_cargo_home}/registry"
fi
if [[ -d "${HOME}/.cargo/git" && ! -e "${_anyka_cargo_home}/git" ]]; then
  ln -sfn "${HOME}/.cargo/git" "${_anyka_cargo_home}/git"
fi
if [[ -d "${HOME}/.cargo/cache" && ! -e "${_anyka_cargo_home}/cache" ]]; then
  ln -sfn "${HOME}/.cargo/cache" "${_anyka_cargo_home}/cache"
fi
export CARGO_HOME="${_anyka_cargo_home}"
unset _anyka_cargo_home

if [[ -t 1 && -z "${ANYKA_QUIET:-}" ]]; then
  echo "Anyka toolchain environment loaded"
  echo "ANYKA_REPO_ROOT=${ANYKA_REPO_ROOT}"
  echo "ANYKA_TOOLCHAIN_BIN=${ANYKA_TOOLCHAIN_BIN}"
  echo "CARGO=${CARGO}"
  echo "RUSTC=${RUSTC}"
  echo "RUSTDOC=${RUSTDOC}"
  echo "CARGO_HOME=${CARGO_HOME}"
fi

