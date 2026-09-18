#!/usr/bin/env bash
# STAGE:   build + package — THE DEFAULT for shipping a change.
# UNIT:    bundle.tar, built from fresh sources
# USE FOR: any change you want on a camera. Start here unless you know otherwise.
# NEXT:    push_bundle.sh --host <camera> --user admin bundle.tar
#
# Wrapper: build_payload.sh (compile) -> package_bundle.sh (tar + manifest).
# Use package_bundle.sh directly only when the payload is already fresh and you
# just want to re-tar it; it compiles nothing.
#
# Usage:
#   ./scripts/build_bundle.sh
#   ./scripts/build_bundle.sh --debug /tmp/bundle.tar
#   ./scripts/build_bundle.sh --skip-vendor bundle.tar

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"

SD_FLAGS=()
OUT=""

usage() {
  cat <<'EOF'
Usage: build_bundle.sh [OPTIONS] [OUT]

Cross-compile anyka-init / vendor-daemon / onvif-rust (+ WebUI), assemble into
SD_card_contents/anyka_hack/, then package a versioned upgrade bundle.tar.

Options (forwarded to build_payload.sh):
  --skip-vendor   Skip vendor-daemon build/install
  --debug         Build debug binaries
  -h, --help      Show this help

Arguments:
  OUT             Output path for the bundle (default: <repo>/bundle.tar)

Examples:
  ./scripts/build_bundle.sh
  ./scripts/build_bundle.sh --debug

Next step:
  CAMERA_PASS=\$CAMERA_PASS ./scripts/push_bundle.sh --host <camera> --user admin bundle.tar
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-www)
      log_error "--skip-www is not supported for upgrade bundles"
      log_error "build_bundle.sh stamps the bundle from current sources and must rebuild the WebUI to avoid packaging stale assets"
      log_info  "use ./scripts/build_payload.sh --skip-www only for local payload iteration, not release bundle packaging"
      exit 1
      ;;
    --skip-vendor | --debug)
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

# Capture the version stamp BEFORE any stage writes a tracked file.
#
# build_payload.sh installs freshly compiled binaries over tracked paths in
# SD_card_contents/, and onvif-rust stamps itself partway through that pipeline
# (onvif-rust/scripts/build.sh computes ANYKA_BUILD_VERSION with `git describe
# --dirty`). The vendor-daemon stage runs first and its output is not
# byte-reproducible, so by the time the stamp is taken the tree is already
# dirty -- and a build from a pristine checkout still produced "<hash>-dirty".
# That is why the whole camera fleet ran -dirty versions; it was never a stale
# working tree.
#
# The stamp describes the *source*, so the build's own output files are
# excluded from the dirty test -- but only those files. Excluding all of
# SD_card_contents/ used to let a dirty Factory/config.sh (or any other
# tracked payload input) stamp as clean while the bundle carried the
# uncommitted bytes. That also makes repeated builds in one checkout stamp
# identically, instead of requiring a `git checkout -- SD_card_contents/`
# ritual between them.
#
# Untracked files count too, but only where they ship: build_bundle.sh
# `cp -r`s vendor-daemon/ and onvif/sounds/ wholesale, so an untracked file
# dropped into either would ride into bundle.tar while the tree stamped
# clean. Untracked entries therefore dirty the stamp when (and only when)
# they sit under those two recursive archive paths, minus the generated
# vendor-daemon/lib/ (SDK copies; a toolchain change is a deliberate
# separate push). onvif/www/ is gitignored build output and never appears
# in `git status` at all; untracked files elsewhere in the repo never enter
# the bundle.
if [[ -z "${ANYKA_BUILD_VERSION:-}" ]]; then
  ANYKA_BUILD_VERSION="$(git -C "${ANYKA_REPO_ROOT}" describe --tags --always)"
  if [[ -n "$(git -C "${ANYKA_REPO_ROOT}" status --porcelain \
      -- ':!SD_card_contents/anyka_hack/anyka-init.bin' \
         ':!SD_card_contents/anyka_hack/onvif/onvif-rust.bin' \
         ':!SD_card_contents/anyka_hack/onvif/.build-version' \
         ':!SD_card_contents/anyka_hack/snmp/snmp-agent.bin' \
         ':!SD_card_contents/anyka_hack/vendor-daemon/vendor-daemon.bin' \
         ':!SD_card_contents/anyka_hack/vendor-daemon/lib/' \
     | awk '
          # Tracked changes (any line not starting with "??") always count.
          # Untracked lines count only if they ride into bundle.tar via the
          # wholesale cp -r of vendor-daemon/ or onvif/sounds/ -- but not the
          # generated vendor-daemon/lib/ (SDK copies; a toolchain change is a
          # deliberate separate push). onvif/www/ is gitignored and never
          # appears in `git status` at all.
          {
            if (substr($0, 1, 2) != "??" ||
                index($0, "?? SD_card_contents/anyka_hack/onvif/sounds/") == 1 ||
                (index($0, "?? SD_card_contents/anyka_hack/vendor-daemon/") == 1 &&
                 index($0, "?? SD_card_contents/anyka_hack/vendor-daemon/lib/") != 1))
              print
          }' \
     || true)" ]]; then
    ANYKA_BUILD_VERSION="${ANYKA_BUILD_VERSION}-dirty"
  fi
fi
export ANYKA_BUILD_VERSION

log_info "=== Build upgrade bundle ==="
log_info "Repo:    ${ANYKA_REPO_ROOT}"
log_info "Out:     ${OUT}"
log_info "Version: ${ANYKA_BUILD_VERSION}"
echo ""

log_step "1/2 assemble SD_card_contents/"
"${SCRIPT_DIR}/build_payload.sh" "${SD_FLAGS[@]+"${SD_FLAGS[@]}"}"

echo ""
log_step "2/2 package bundle.tar"
"${SCRIPT_DIR}/package_bundle.sh" "${OUT}"

echo ""
log_success "Upgrade bundle ready: ${OUT}"
log_info "Push: CAMERA_PASS=\$CAMERA_PASS ./scripts/push_bundle.sh --host <camera> --user admin ${OUT}"
