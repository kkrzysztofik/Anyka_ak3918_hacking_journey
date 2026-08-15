#!/usr/bin/env bash
# Build ARM payloads into SD_card_contents/, then package an upgrade bundle.tar.
#
# Wrapper: build_sd_contents.sh → build_bundle.sh
#
# Usage:
#   ./scripts/build_upgrade_bundle.sh
#   ./scripts/build_upgrade_bundle.sh --skip-www
#   ./scripts/build_upgrade_bundle.sh --debug /tmp/bundle.tar
#   ./scripts/build_upgrade_bundle.sh --skip-vendor --skip-www bundle.tar

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"

SD_FLAGS=()
OUT=""

usage() {
  cat <<'EOF'
Usage: build_upgrade_bundle.sh [OPTIONS] [OUT]

Cross-compile anyka-init / vendor-daemon / onvif-rust (+ WebUI), assemble into
SD_card_contents/anyka_hack/, then package a versioned upgrade bundle.tar.

Options (forwarded to build_sd_contents.sh):
  --skip-www      Skip npm WebUI build
  --skip-vendor   Skip vendor-daemon build/install
  --debug         Build debug binaries
  -h, --help      Show this help

Arguments:
  OUT             Output path for the bundle (default: <repo>/bundle.tar)

Examples:
  ./scripts/build_upgrade_bundle.sh
  ./scripts/build_upgrade_bundle.sh --skip-www /tmp/cam-bundle.tar
  ./scripts/build_upgrade_bundle.sh --debug

Next step:
  CAMERA_PASS=\$CAMERA_PASS ./scripts/upload_upgrade_bundle.sh --host <camera> --user admin bundle.tar
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-www | --skip-vendor | --debug)
      SD_FLAGS+=("$1")
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    -*)
      log_error "Unknown option: $1"
      usage >&2
      exit 1
      ;;
    *)
      if [[ -n "${OUT}" ]]; then
        log_error "Unexpected argument: $1 (OUT already set to ${OUT})"
        usage >&2
        exit 1
      fi
      OUT="$1"
      shift
      ;;
  esac
done

OUT="${OUT:-${ANYKA_REPO_ROOT}/bundle.tar}"

log_info "=== Build upgrade bundle ==="
log_info "Repo: ${ANYKA_REPO_ROOT}"
log_info "Out:  ${OUT}"
echo ""

log_step "1/2 assemble SD_card_contents/"
"${SCRIPT_DIR}/build_sd_contents.sh" "${SD_FLAGS[@]+"${SD_FLAGS[@]}"}"

echo ""
log_step "2/2 package bundle.tar"
"${SCRIPT_DIR}/build_bundle.sh" "${OUT}"

echo ""
log_success "Upgrade bundle ready: ${OUT}"
log_info "Upload: CAMERA_PASS=\$CAMERA_PASS ./scripts/upload_upgrade_bundle.sh --host <camera> --user admin ${OUT}"
