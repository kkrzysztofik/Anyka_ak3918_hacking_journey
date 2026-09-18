#!/usr/bin/env bash
# STAGE:   package — compiles NOTHING. Tars whatever is already on disk.
# UNIT:    bundle.tar (three components plus a checksum manifest)
# USE FOR: repackaging an existing payload, e.g. a different OUT path or schema.
#
# PRECONDITION: SD_card_contents/anyka_hack/ must already hold a fresh build.
#   The version-embed check below catches a *mismatched* tree (manifest version
#   vs the stamp inside onvif-rust.bin), but it cannot catch a *uniformly stale*
#   one: an old .build-version agrees with the old binary it was built
#   alongside, so packaging succeeds and ships last week's work under last
#   week's label. Nothing fails; you just do not get your edits.
#   If you have touched any source since the last build, run
#   ./scripts/build_bundle.sh instead — it builds, then calls this script.
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

# The manifest must not be able to claim a version the binary does not report.
# `env!("ANYKA_BUILD_VERSION")` is baked into onvif-rust.bin at compile time, but
# a build that re-runs build.rs without recompiling lib.rs emits a binary that
# keeps an older stamp -- or none at all. Observed both ways: a bundle built
# 2026-08-29 carried manifest version=089b2dca over a binary with no version
# string whatsoever, and .198 was found in the field serving
# firmware_version=b38f8032-dirty from a slot whose manifest.meta said
# a1660798-dirty. Every downstream gate compares the two, so catch it here.
# Rust &str literals are length-prefixed (not NUL-terminated), so `strings`
# glues this to the next .rodata literal. `grep -Fx` (whole-line match) fails
# on the concatenated output; `grep -qF` (substring) finds the stamp. The old
# prefix-collision risk ("H" matching "H-dirty") is nil for 12+ char git
# hashes: a false match would require a coincidental 15-char prefix in .rodata.
if ! strings -a "${SRC}/onvif/onvif-rust.bin" | grep -F -- "${VERSION}" >/dev/null; then
  log_error "onvif-rust.bin does not embed the version string '${VERSION}'"
  log_error "the binary was not recompiled for this stamp; the bundle would be mislabelled"
  log_info  "force a rebuild with:"
  log_info  "  touch cross-compile/onvif-rust/src/lib.rs && ./scripts/build_payload.sh"
  exit 1
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

cp "${SRC}/anyka-init.bin"        "${STAGE}/"
cp -r "${SRC}/vendor-daemon"      "${STAGE}/"
mkdir -p "${STAGE}/onvif"
cp "${SRC}/onvif/onvif-rust.bin"  "${STAGE}/onvif/"
cp -r "${SRC}/onvif/www"          "${STAGE}/onvif/"
# Clips resolve beside the binary under slots/{a,b}/onvif/sounds/; omitting them
# from the bundle leaves A/B upgrades with silent event audio.
anyka_require_sound_clips "${SRC}/onvif"
cp -r "${SRC}/onvif/sounds"       "${STAGE}/onvif/"
cp "${SRC}/onvif/config.toml"     "${STAGE}/onvif/config.template.toml"
mkdir -p "${STAGE}/snmp"
cp "${SRC}/snmp/snmp-agent.bin"   "${STAGE}/snmp/"

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
log_info "push: CAMERA_PASS=\$CAMERA_PASS ./scripts/push_bundle.sh --host <camera> --user admin ${OUT}"
log_info "or drop it in /mnt/anyka_hack/spool/ over FTP, then touch bundle.trigger"
