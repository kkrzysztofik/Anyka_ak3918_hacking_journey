#!/usr/bin/env bash
# Build an upgrade bundle: the three components plus a checksum manifest.
#
# Deliberately not the whole anyka_hack tree. lib/ is 31 MB of uClibc runtime
# that changes only on a toolchain bump; the manifest does not cover it, so a
# toolchain change is a separate deliberate push, not an accidental one.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"

SRC="${ANYKA_REPO_ROOT}/SD_card_contents/anyka_hack"
OUT="${1:-${ANYKA_REPO_ROOT}/bundle.tar}"
SCHEMA="${ANYKA_CONFIG_SCHEMA:-1}"
# The manifest's requires_config_schema must be a decimal u32: anyka-init
# rejects a present-but-unparseable value, and silently writing garbage would
# ship a bundle that can never apply.
if ! [[ "${SCHEMA}" =~ ^[0-9]+$ ]] || (( SCHEMA > 4294967295 )); then
  log_error "ANYKA_CONFIG_SCHEMA='${SCHEMA}' is not a decimal u32"
  exit 1
fi
# One captured version for both manifest.meta and the binary's FirmwareVersion:
# the onvif-rust build writes `.build-version` next to the deployed binary.
# Falling back to a fresh `git describe` would risk a bundle that claims a
# version the binary does not report.
if [[ -f "${SRC}/onvif/.build-version" ]]; then
  VERSION="$(cat "${SRC}/onvif/.build-version")"
else
  VERSION="$(git -C "${ANYKA_REPO_ROOT}" describe --tags --always --dirty)"
  log_warn "no ${SRC}/onvif/.build-version (build onvif-rust first); falling back to git describe"
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

cp "${SRC}/anyka-init.bin"        "${STAGE}/"
cp -r "${SRC}/vendor-daemon"      "${STAGE}/"
mkdir -p "${STAGE}/onvif"
cp "${SRC}/onvif/onvif-rust.bin"  "${STAGE}/onvif/"
cp -r "${SRC}/onvif/www"          "${STAGE}/onvif/"
cp "${SRC}/onvif/config.toml"     "${STAGE}/onvif/config.template.toml"

cat > "${STAGE}/manifest.meta" <<EOF
version=${VERSION}
requires_config_schema=${SCHEMA}
EOF

# sha256sum format, so `busybox sha256sum -c manifest.sha256` verifies it on the
# device and a human can verify it by hand over telnet.
( cd "${STAGE}" && find . -type f ! -name manifest.sha256 -printf '%P\n' \
    | sort | xargs sha256sum > manifest.sha256 )

tar -cf "${OUT}" -C "${STAGE}" .

log_success "bundle ${VERSION} -> ${OUT} ($(du -h "${OUT}" | cut -f1))"
# /api/update requires Administrator Basic Auth. Use the upload wrapper (it
# reads the password from CAMERA_PASS/--pass-file, never from argv) so the
# credential stays out of shell history and `ps`.
log_info "deploy: CAMERA_PASS=\$CAMERA_PASS ./scripts/upload_upgrade_bundle.sh --host <camera> --user admin ${OUT}"
log_info "or drop it in /mnt/anyka_hack/spool/ over FTP, then touch bundle.trigger"
