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

case ":${PATH}:" in
  *":${ANYKA_TOOLCHAIN_BIN}:"*) ;;
  *) export PATH="${ANYKA_TOOLCHAIN_BIN}:${PATH}" ;;
esac

if [[ -t 1 && -z "${ANYKA_QUIET:-}" ]]; then
  echo "Anyka toolchain environment loaded"
  echo "ANYKA_REPO_ROOT=${ANYKA_REPO_ROOT}"
  echo "ANYKA_TOOLCHAIN_BIN=${ANYKA_TOOLCHAIN_BIN}"
  echo "CARGO=${CARGO}"
  echo "RUSTC=${RUSTC}"
  echo "RUSTDOC=${RUSTDOC}"
fi

