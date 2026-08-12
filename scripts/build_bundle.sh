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
VERSION="$(git -C "${ANYKA_REPO_ROOT}" describe --tags --always --dirty)"

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
log_info "deploy: curl -T ${OUT} http://<camera>/api/update   # increment 2"
log_info "or drop it in /mnt/anyka_hack/spool/ over FTP, then touch bundle.trigger"
