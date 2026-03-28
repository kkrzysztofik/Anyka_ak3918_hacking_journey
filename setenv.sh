#!/usr/bin/env bash

# Anyka project toolchain environment setup.
# Usage:
#   source ./setenv.sh

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  echo "This script must be sourced, not executed."
  echo "Run: source ./setenv.sh"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOOLCHAIN_BIN="${SCRIPT_DIR}/toolchain/arm-anykav200-crosstool-ng/bin"

export CARGO="${TOOLCHAIN_BIN}/cargo"
export RUSTC="${TOOLCHAIN_BIN}/rustc"
export RUSTDOC="${TOOLCHAIN_BIN}/rustdoc"

case ":${PATH}:" in
  *":${TOOLCHAIN_BIN}:"*) ;;
  *) export PATH="${TOOLCHAIN_BIN}:${PATH}" ;;
esac

echo "Anyka toolchain environment loaded"
echo "CARGO=${CARGO}"
echo "RUSTC=${RUSTC}"
echo "RUSTDOC=${RUSTDOC}"
