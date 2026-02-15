#!/bin/bash
# Builds the Rust RTSP validator and copies the final binary into validation/.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VALIDATION_DIR="${REPO_ROOT}/validation"
RUST_DIR="${VALIDATION_DIR}/rust"

export CARGO="${CARGO:-${REPO_ROOT}/toolchain/arm-anykav200-crosstool-ng/bin/cargo}"

cd "$RUST_DIR"

echo "[INFO] Building rtsp_validation_tool (release)..."
"$CARGO" build --release

BIN_SRC="${VALIDATION_DIR}/target/x86_64-unknown-linux-gnu/release/rtsp_validation_tool"
BIN_DST="${VALIDATION_DIR}/rtsp_validation_tool"

if [ ! -f "$BIN_SRC" ]; then
  echo "[ERROR] Expected binary not found at: $BIN_SRC" >&2
  exit 1
fi

cp -f "$BIN_SRC" "$BIN_DST"
chmod +x "$BIN_DST" || true

echo "[INFO] Wrote: $BIN_DST"

