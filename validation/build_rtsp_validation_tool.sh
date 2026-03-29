#!/bin/bash
# Builds the Rust RTSP validator and copies the final binary into validation/.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/../scripts/common.sh"
REPO_ROOT="${ANYKA_REPO_ROOT}"
VALIDATION_DIR="${REPO_ROOT}/validation"
RUST_DIR="${VALIDATION_DIR}/rust"

export CARGO="${CARGO:-${ANYKA_CARGO}}"

cd "$RUST_DIR"

log_info "Building rtsp_validation_tool (release)..."
"$CARGO" build --release

BIN_SRC="${VALIDATION_DIR}/target/x86_64-unknown-linux-gnu/release/rtsp_validation_tool"
BIN_DST="${VALIDATION_DIR}/rtsp_validation_tool"

if [ ! -f "$BIN_SRC" ]; then
  log_error "Expected binary not found at: $BIN_SRC"
  exit 1
fi

cp -f "$BIN_SRC" "$BIN_DST"
chmod +x "$BIN_DST" || true

log_info "Wrote: $BIN_DST"

