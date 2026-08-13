#!/usr/bin/env bash
# One-time migration of a flat anyka-init camera onto the A/B slot layout.
#
# The flat layout has no applier, so `PUT /api/update` has nothing to consume a
# bundle: a camera cannot upgrade itself onto the upgrade path. This script is
# that bootstrap, and it is needed exactly once per camera.
#
# End state:
#   slots/a/anyka-init.bin   the *current* supervisor, kept as the rollback target
#   slots/b/                 the new bundle, verified against its own manifest
#   active = b               plus state/trial-a, which arms the normal trial
#   /mnt/Factory/config.sh   slot-aware, previous copy kept as .preslots.bak
#
# The flat payload is copied, never moved: reverting to slots/a runs the old
# supervisor, which resolves services at their flat paths, so those must survive.
#
# After the reboot the camera runs the ordinary PR #74 path — trial, commit or
# self-revert — so a bad new build reverts to slots/a on its own.
#
# Usage:
#   ./scripts/migrate_to_slots.sh --host 192.168.3.127
#   ./scripts/migrate_to_slots.sh --host 192.168.3.127 --dry-run
#
# Behind the jumphost, forward telnet *and* the nc data port — the file push
# dials the camera on --nc-port, so tunnelling :24 alone gets as far as step 5
# and then hangs:
#   ssh -N -L 12346:192.168.30.146:24 -L 5557:192.168.30.146:5557 root@192.168.3.137
#   ./scripts/migrate_to_slots.sh --host 127.0.0.1 --port 12346

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"

HOST=""
PORT=""
BUNDLE="${ANYKA_REPO_ROOT}/bundle.tar"
ROOT="/mnt/anyka_hack"
FACTORY_CFG="/mnt/Factory/config.sh"
NC_PORT="5557"
DRY_RUN=false

usage() {
  sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host) HOST="${2:?--host needs a value}"; shift 2 ;;
    --port) PORT="${2:?--port needs a value}"; shift 2 ;;
    --bundle) BUNDLE="${2:?--bundle needs a value}"; shift 2 ;;
    --nc-port) NC_PORT="${2:?--nc-port needs a value}"; shift 2 ;;
    --dry-run) DRY_RUN=true; shift ;;
    -h | --help) usage; exit 0 ;;
    *) log_error "Unknown argument: $1"; usage >&2; exit 1 ;;
  esac
done

[[ -n "${HOST}" ]] || { log_error "--host is required"; exit 1; }
# NC_PORT is spliced into a shell command that runs on the camera (push_file);
# reject anything that is not a plain decimal port so a crafted value cannot
# terminate the `nc` command and run further commands on the device.
if ! [[ "${NC_PORT}" =~ ^[1-9][0-9]{0,4}$ ]] || (( 10#"${NC_PORT}" > 65535 )); then
  log_error "--nc-port must be an integer from 1 through 65535"
  exit 1
fi

CAM_ARGS=(--host "${HOST}")
[[ -n "${PORT}" ]] || PORT=""
[[ -z "${PORT}" ]] || CAM_ARGS+=(--port "${PORT}")

# Every device command goes through cam_exec's telnet channel. cam_exec puts the
# command's own output on stdout and its `[exit=N]` marker on stderr, so
# discarding stderr leaves exactly the device's bytes. Telnet still delivers CRLF.
cam() {
  ( cd "${ANYKA_REPO_ROOT}" \
      && uv run python3 scripts/debugging/cam_exec.py "${CAM_ARGS[@]}" "$1" ) \
    2>/dev/null | tr -d '\r'
}

run() {
  if [[ "${DRY_RUN}" = true ]]; then
    log_info "DRY-RUN would run on device: $1"
    return 0
  fi
  cam "$1"
}

# Push a local file to the device. The camera listens and we connect in: the
# dev box is a gateway hop away, so a camera-initiated connection back is not
# guaranteed to be routable, while inbound to the camera demonstrably is.
push_file() {
  local src="$1" dst="$2" want_md5 got_md5
  want_md5="$(md5sum "${src}" | cut -d' ' -f1)"
  log_info "push $(basename "${src}") ($(du -h "${src}" | cut -f1)) → ${dst}"
  if [[ "${DRY_RUN}" = true ]]; then
    log_info "DRY-RUN would push ${src} → ${dst} (md5 ${want_md5})"
    return 0
  fi

  cam "rm -f '${dst}'; (nc -l -p ${NC_PORT} > '${dst}' 2>/dev/null &); sleep 1; echo listening" \
    >/dev/null
  sleep 1
  nc -w 30 "${HOST}" "${NC_PORT}" < "${src}"
  sleep 2

  # md5 on the device, not a byte count: exFAT can absorb a short write as
  # NUL bytes, which no transfer-side check can see.
  got_md5="$(cam "md5sum '${dst}' 2>/dev/null | cut -d' ' -f1" | tr -d '[:space:]')"
  if [[ "${got_md5}" != "${want_md5}" ]]; then
    log_error "md5 mismatch for ${dst}: want ${want_md5}, device has ${got_md5:-<none>}"
    return 1
  fi
  log_success "verified ${dst} (${want_md5})"
}

log_info "=== Migrate ${HOST}${PORT:+:${PORT}} to the A/B slot layout ==="
[[ "${DRY_RUN}" = false ]] || log_warn "DRY RUN: no device state will change"

# ── 1. Preflight ─────────────────────────────────────────────────────────────
log_step "1/8 preflight"

[[ -f "${BUNDLE}" ]] || { log_error "bundle not found: ${BUNDLE}"; exit 1; }
for entry in ./manifest.meta ./manifest.sha256 ./anyka-init.bin; do
  tar tf "${BUNDLE}" "${entry}" >/dev/null 2>&1 \
    || { log_error "bundle is missing ${entry}"; exit 1; }
done
BUNDLE_VERSION="$(tar xOf "${BUNDLE}" ./manifest.meta | sed -n 's/^version=//p')"
BUNDLE_SCHEMA="$(tar xOf "${BUNDLE}" ./manifest.meta | sed -n 's/^requires_config_schema=//p')"
log_info "bundle ${BUNDLE_VERSION} (requires config schema ${BUNDLE_SCHEMA:-0})"

NEW_CONFIG_SH="${ANYKA_REPO_ROOT}/SD_card_contents/Factory/config.sh"
[[ -f "${NEW_CONFIG_SH}" ]] || { log_error "missing ${NEW_CONFIG_SH}"; exit 1; }
grep -q 'slots/\$SLOT' "${NEW_CONFIG_SH}" \
  || { log_error "${NEW_CONFIG_SH} is not slot-aware; refusing"; exit 1; }

probe="$(cam 'echo alive')" || true
[[ "${probe}" = *alive* ]] || { log_error "device did not answer over telnet"; exit 1; }

# Refuse a camera that is already migrated. Re-running would overwrite the
# rollback slot with the current build and re-arm a trial for an upgrade that
# already happened.
if [[ "$(cam "[ -d '${ROOT}/slots' ] && echo yes || echo no")" = *yes* ]]; then
  log_error "${ROOT}/slots already exists — this camera is already migrated."
  log_error "Use ./scripts/upload_upgrade_bundle.sh for a normal upgrade."
  exit 1
fi
[[ "$(cam "[ -x '${ROOT}/anyka-init.bin' ] && echo yes || echo no")" = *yes* ]] \
  || { log_error "no flat ${ROOT}/anyka-init.bin — not a flat anyka-init camera"; exit 1; }
log_success "flat anyka-init camera, not yet migrated"

# ── 2. Config schema ─────────────────────────────────────────────────────────
# The applier compares the bundle's requires_config_schema against the device's
# top-level `schema` key, which defaults to 0 when absent — so without this the
# bundle is rejected as "needs 1, has 0". Comparing the numeric value, not just
# key presence, also catches the case where the key exists but is lower than the
# bundle requires (e.g. schema = 0 with a schema-1 bundle), which previously
# skipped the update and let the trial self-revert. Edited in place with sed
# rather than by pushing a whole file: anyka.toml holds the live wifi PSK, and a
# bad push means Config::load fails, which parks the supervisor with no wifi and
# no deadman.
log_step "2/8 device config (schema + static_root)"

CFG_LOCAL="$(mktemp)"; trap 'rm -f "${CFG_LOCAL}" "${CFG_LOCAL}.after"' EXIT
cam "cat '${ROOT}/anyka.toml'" > "${CFG_LOCAL}"
[[ -s "${CFG_LOCAL}" ]] || { log_error "could not read ${ROOT}/anyka.toml"; exit 1; }

CHECKER="${ANYKA_REPO_ROOT}/cross-compile/target/x86_64-unknown-linux-gnu/debug/examples/config-check"
if [[ ! -x "${CHECKER}" ]]; then
  log_info "building the host config validator"
  ( cd "${ANYKA_REPO_ROOT}/cross-compile/anyka-init" \
      && PATH="${ANYKA_TOOLCHAIN_BIN}:${PATH}" \
         "${ANYKA_CARGO}" build --example config-check \
           --target x86_64-unknown-linux-gnu >/dev/null 2>&1 )
fi

TARGET_SCHEMA="${BUNDLE_SCHEMA:-1}"
current_schema="$(sed -n 's/^schema[[:space:]]*=[[:space:]]*\([0-9][0-9]*\).*/\1/p' "${CFG_LOCAL}" | head -1)"
if [[ -n "${current_schema}" ]] && (( 10#"${current_schema}" >= 10#"${TARGET_SCHEMA}" )); then
  log_info "schema already at or above required: schema = ${current_schema} (need ${TARGET_SCHEMA})"
else
  if [[ -n "${current_schema}" ]]; then
    log_info "schema ${current_schema} below required ${TARGET_SCHEMA}; rewriting the value"
    sed "s/^schema[[:space:]]*=.*/schema = ${TARGET_SCHEMA}/" "${CFG_LOCAL}" > "${CFG_LOCAL}.after"
    edit_expr="s/^schema[[:space:]]*=.*/schema = ${TARGET_SCHEMA}/"
  else
    log_info "no schema key; prepending schema = ${TARGET_SCHEMA}"
    { echo "schema = ${TARGET_SCHEMA}"; cat "${CFG_LOCAL}"; } > "${CFG_LOCAL}.after"
    edit_expr="1i schema = ${TARGET_SCHEMA}"
  fi
  # Validate the intended result before writing anything to the device.
  "${CHECKER}" "${CFG_LOCAL}.after" \
    || { log_error "device anyka.toml + schema change does not parse; refusing"; exit 1; }
  log_success "post-edit config parses with the new supervisor's parser"

  run "sed -i '${edit_expr}' '${ROOT}/anyka.toml'" >/dev/null
  if [[ "${DRY_RUN}" = false ]]; then
    cam "cat '${ROOT}/anyka.toml'" > "${CFG_LOCAL}.after"
    "${CHECKER}" "${CFG_LOCAL}.after" \
      || { log_error "anyka.toml no longer parses after the edit! restore it by hand"; exit 1; }
    got_schema="$(sed -n 's/^schema[[:space:]]*=[[:space:]]*\([0-9][0-9]*\).*/\1/p' "${CFG_LOCAL}.after" | head -1)"
    [[ -n "${got_schema}" ]] \
      || { log_error "schema key missing after the edit! restore it by hand"; exit 1; }
    log_success "schema = ${got_schema} in place, config still parses"
  fi
fi

# onvif-rust resolves a relative static_root against its own binary, so a slot
# build serves slots/<slot>/onvif/www. A device config still holding the flat
# absolute path keeps serving the *pre-upgrade* WebUI out of the flat tree —
# with HTTP 200 throughout, which is why nothing short of comparing the served
# asset against the build output catches it. Observed on .127, 2026-08-13.
ONVIF_CFG="${ROOT}/onvif/config.toml"
current_root="$(cam "sed -n 's/^static_root[[:space:]]*=[[:space:]]*//p' '${ONVIF_CFG}'" | tr -d '[:space:]')"
if [[ "${current_root}" = '"www"' ]]; then
  log_info "static_root is already relative (\"www\")"
elif [[ -z "${current_root}" ]]; then
  log_warn "no static_root in ${ONVIF_CFG}; leaving it alone"
else
  log_info "static_root is ${current_root} — rewriting to \"www\""
  run "cp '${ONVIF_CFG}' '${ONVIF_CFG}.preslots.bak' && sed -i 's|^static_root = .*|static_root = \"www\"|' '${ONVIF_CFG}' && sync" >/dev/null
  if [[ "${DRY_RUN}" = false ]]; then
    [[ "$(cam "sed -n 's/^static_root[[:space:]]*=[[:space:]]*//p' '${ONVIF_CFG}'" | tr -d '[:space:]')" = '"www"' ]] \
      || { log_error "static_root rewrite did not take; fix ${ONVIF_CFG} by hand"; exit 1; }
    log_success "static_root = \"www\" (previous kept at config.toml.preslots.bak)"
  fi
fi

# ── 3. Layout ────────────────────────────────────────────────────────────────
# slots/b is not created here: it arrives only via the verified staging dir
# promotion below, so a partial run never leaves a half-populated slot b that a
# re-run would have to delete.
log_step "3/8 create the slot layout"
run "mkdir -p '${ROOT}/slots/a' '${ROOT}/state' '${ROOT}/spool'" >/dev/null
log_success "slots/a, state/, spool/ present"

# ── 4. Rollback slot ─────────────────────────────────────────────────────────
# ponytail: slot a gets only anyka-init.bin, not the whole payload. The current
# supervisor predates slot_path(), so if it ever boots from slots/a it resolves
# services at their flat paths — which this script leaves in place. Copying the
# other ~19 MB onto a slow card would buy nothing. Populate it fully if the flat
# tree is ever removed.
log_step "4/8 seed the rollback slot"
run "cp '${ROOT}/anyka-init.bin' '${ROOT}/slots/a/anyka-init.bin' && chmod +x '${ROOT}/slots/a/anyka-init.bin'" >/dev/null
if [[ "${DRY_RUN}" = false ]]; then
  [[ "$(cam "[ -x '${ROOT}/slots/a/anyka-init.bin' ] && echo yes || echo no")" = *yes* ]] \
    || { log_error "rollback slot is not executable; refusing to continue"; exit 1; }
  a_md5="$(cam "md5sum '${ROOT}/anyka-init.bin' | cut -d' ' -f1" | tr -d '[:space:]')"
  b_md5="$(cam "md5sum '${ROOT}/slots/a/anyka-init.bin' | cut -d' ' -f1" | tr -d '[:space:]')"
  [[ "${a_md5}" = "${b_md5}" ]] \
    || { log_error "rollback copy md5 mismatch (${a_md5} vs ${b_md5})"; exit 1; }
fi
log_success "slots/a holds the current supervisor as the rollback target"

# ── 5. Stage the new bundle ──────────────────────────────────────────────────
log_step "5/8 stage the bundle into slots/b"
push_file "${BUNDLE}" "${ROOT}/spool/bundle.tar"
# mkdir without -p fails on an existing staging dir: a stale b.staging from a
# crashed earlier run must block rather than be deleted, so the operator
# decides what to do with it.
run "mkdir '${ROOT}/slots/b.staging' && busybox tar -xf '${ROOT}/spool/bundle.tar' -C '${ROOT}/slots/b.staging' && sync && echo untarred" >/dev/null

if [[ "${DRY_RUN}" = false ]]; then
  # The bundle's own manifest is the transfer check: it covers every file and
  # catches the exFAT NUL-byte write that a size comparison cannot.
  verdict="$(cam "cd '${ROOT}/slots/b.staging' && busybox sha256sum -c manifest.sha256 >/tmp/sha.log 2>&1 && echo VERIFIED || echo BAD")"
  if [[ "${verdict}" != *VERIFIED* ]]; then
    log_error "staged bundle failed its own manifest:"
    cam 'grep -v ": OK$" /tmp/sha.log | head -20' >&2
    log_error "leaving the camera on its flat layout (nothing flipped)"
    exit 1
  fi
  log_success "all $(cam "grep -c ': OK$' /tmp/sha.log" | tr -d '[:space:]') files match the manifest"
fi

run "mv '${ROOT}/slots/b.staging' '${ROOT}/slots/b' && sync && echo promoted" >/dev/null
run "rm -f '${ROOT}/spool/bundle.tar' '${ROOT}/spool/bundle.trigger'" >/dev/null
log_success "slots/b holds bundle ${BUNDLE_VERSION}"

# ── 6. Boot path ─────────────────────────────────────────────────────────────
# The one irreversible-looking step: /mnt/Factory/config.sh is what the vendor
# init sources, and the 240-second deadman lives inside it. Verified by md5
# before any reboot, with the previous copy kept alongside.
log_step "6/8 install the slot-aware config.sh"
run "[ -f '${FACTORY_CFG}.preslots.bak' ] || cp '${FACTORY_CFG}' '${FACTORY_CFG}.preslots.bak'" >/dev/null
push_file "${NEW_CONFIG_SH}" "${FACTORY_CFG}"
run "chmod +x '${FACTORY_CFG}'" >/dev/null
log_success "boot path is slot-aware (previous kept at ${FACTORY_CFG}.preslots.bak)"

# ── 7. Flip with the trial armed ─────────────────────────────────────────────
# Arm before flipping, matching update.rs:594-599: a power cut between the two
# leaves the old slot active with a stale marker, which the next boot resolves
# harmlessly. The reverse order could flip with no way back.
log_step "7/8 arm the trial and flip to slot b"
run "rm -f '${ROOT}/state/trial-a' '${ROOT}/state/trial-b' && : > '${ROOT}/state/trial-a' && sync" >/dev/null
run "printf b > '${ROOT}/active' && sync" >/dev/null
if [[ "${DRY_RUN}" = false ]]; then
  [[ "$(cam "cat '${ROOT}/active'")" = *b* ]] || { log_error "active pointer did not take"; exit 1; }
  [[ "$(cam "[ -f '${ROOT}/state/trial-a' ] && echo yes || echo no")" = *yes* ]] \
    || { log_error "trial marker missing; a bad build would not self-revert"; exit 1; }
fi
log_success "active=b, trial armed against slot a"

# ── 8. Reboot ────────────────────────────────────────────────────────────────
log_step "8/8 reboot"
if [[ "${DRY_RUN}" = true ]]; then
  log_info "DRY-RUN would reboot now"
  exit 0
fi
cam 'reboot' >/dev/null 2>&1 || true
log_success "reboot issued"
echo ""
log_info "Expect ~90-120s of downtime, then a 30s trial hold inside a 120s deadline."
log_info "Verify:  curl -su admin:PASS http://${HOST}/api/diagnostics | jq .firmware_version"
log_info "  expect ${BUNDLE_VERSION}, ports 80/554/8080 bound, no ${ROOT}/state/trial-*"
log_info "If the trial fails the camera returns on slot a by itself."
