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
  VERSION="$(tr -d '[:space:]' < "${SRC}/onvif/.build-version")"
else
  VERSION="$(git -C "${ANYKA_REPO_ROOT}" describe --tags --always --dirty)"
  log_warn "no ${SRC}/onvif/.build-version (build onvif-rust first); falling back to git describe"
fi

# An empty VERSION would make every check below vacuous: the embedded-version
# grep matches any binary, and manifest.meta would ship `version=`.
if [[ -z "${VERSION}" ]]; then
  log_error "version stamp is empty (${SRC}/onvif/.build-version present but blank)"
  log_error "rebuild with ./scripts/build_bundle.sh; a blank stamp cannot be validated"
  exit 1
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
# on the concatenated output, so the match has to be a substring one.
#
# That is why the stamp is delimited. A bare `grep -F -- "${VERSION}"` is not
# merely collision-prone, it is systematically wrong in one direction: a clean
# version is a proper prefix of its own dirty form, so `a1660798` matches a
# binary stamped `a1660798-dirty` every time. That is the exact mislabelling
# described above. src/lib.rs stores the version only as
# `<<ANYKA_BUILD_VERSION:...>>`, so including the trailing delimiter makes the
# substring match exact at both ends.
STAMP="<<ANYKA_BUILD_VERSION:${VERSION}>>"
if ! strings -a "${SRC}/onvif/onvif-rust.bin" | grep -F -- "${STAMP}" >/dev/null; then
  log_error "onvif-rust.bin does not embed the build stamp '${STAMP}'"
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
# NUL-delimited: a bundled asset with whitespace in its name would otherwise be
# split by xargs and silently checksummed under the wrong path, or omitted.
( cd "${STAGE}" && find . -type f ! -name manifest.sha256 -printf '%P\0' \
    | sort -z | xargs -0 sha256sum > manifest.sha256 )

tar -cf "${OUT}" -C "${STAGE}" .

log_success "bundle ${VERSION} -> ${OUT} ($(du -h "${OUT}" | cut -f1))"
# /api/update requires Administrator Basic Auth. Use the upload wrapper (it
# reads the password from CAMERA_PASS/--pass-file, never from argv) so the
# credential stays out of shell history and `ps`.
log_info "push: CAMERA_PASS=\$CAMERA_PASS ./scripts/push_bundle.sh --host <camera> --user admin ${OUT}"
log_info "or drop it in /mnt/anyka_hack/spool/ over FTP, then touch bundle.trigger"
