#!/usr/bin/env bash
# STAGE:   push — sends to a card or camera. Compiles nothing.
# UNIT:    the whole payload tree, including lib/ and Factory/
# USE FOR: a fresh camera, or a change to lib/ or Factory that a bundle cannot
#          carry. For an ordinary code change use build_bundle.sh + push_bundle.sh
#          instead — this path has no versioning, no trial and no rollback.
#
# PRECONDITION: run ./scripts/build_payload.sh first. This script only copies.
#
# Modes (exactly one required):
#   --sd PATH     Sync anyka_hack/ + Factory/ onto a mounted SD card
#   --ftp HOST    Upload trees to /mnt/anyka_hack and /mnt/Factory on the camera
#
# Usage:
#   ./scripts/push_payload.sh --sd /media/$USER/SDCARD
#   ./scripts/push_payload.sh --ftp 192.168.1.100
#   ./scripts/push_payload.sh --ftp 192.168.1.100 --user root --pass ''
#   ./scripts/push_payload.sh --ftp 192.168.1.100 --dry-run

set -euo pipefail

# The wifi-preservation scratch dir (created in fetch_live_wifi) holds the
# camera's live wifi password. Remove it on every exit path — clean exit,
# failure, or Ctrl-C — so a credentials file never lingers on the build box.
WIFI_WORKDIR=""
WIFI_FINAL=""
wifi_scratch_cleanup() {
  if [[ -n "${WIFI_WORKDIR}" ]]; then
    rm -rf "${WIFI_WORKDIR}" 2>/dev/null || true
  fi
  return 0
}
trap wifi_scratch_cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"

MODE=""
SD_MOUNT=""
FTP_HOST=""
# The camera's FTP account is root, not admin. The password is a secret: pass it
# with --pass or export ANYKA_FTP_PASS. Never hardcode it here.
FTP_USER="root"
FTP_PASS="${ANYKA_FTP_PASS:-}"
REMOTE_ROOT="/mnt"
DRY_RUN=false
DO_DELETE=false

usage() {
  cat <<'EOF'
Usage: push_payload.sh (--sd PATH | --ftp HOST) [OPTIONS]

Copy SD_card_contents/anyka_hack and SD_card_contents/Factory to a mounted
SD card or to the camera filesystem over FTP (/mnt/...).

Modes (exactly one):
  --sd PATH           Local mount point of the SD card
  --ftp HOST          Camera IP/hostname (uploads under /mnt)

Options:
  --user NAME         FTP username (default: root)
  --pass PASS         FTP password (default: $ANYKA_FTP_PASS env var)
  --remote-root PATH  Remote SD mount root (default: /mnt)
  --delete            Remove destination files not present in source
  --dry-run           Show what would be copied without writing
  -h, --help          Show this help

Examples:
  ./scripts/push_payload.sh --sd /media/kmk/SDCARD
  ./scripts/push_payload.sh --ftp 192.168.1.100
  ./scripts/push_payload.sh --ftp 192.168.1.100 --user root --pass mypass --delete
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --sd)
      [[ $# -ge 2 ]] || { log_error "--sd requires a path"; exit 1; }
      MODE="sd"
      SD_MOUNT="$2"
      shift 2
      ;;
    --ftp)
      [[ $# -ge 2 ]] || { log_error "--ftp requires a host"; exit 1; }
      MODE="ftp"
      FTP_HOST="$2"
      shift 2
      ;;
    --user)
      [[ $# -ge 2 ]] || { log_error "--user requires a name"; exit 1; }
      FTP_USER="$2"
      shift 2
      ;;
    --pass)
      [[ $# -ge 2 ]] || { log_error "--pass requires a value"; exit 1; }
      FTP_PASS="$2"
      shift 2
      ;;
    --remote-root)
      [[ $# -ge 2 ]] || { log_error "--remote-root requires a path"; exit 1; }
      REMOTE_ROOT="$2"
      shift 2
      ;;
    --delete)
      DO_DELETE=true
      shift
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      log_error "Unknown option: $1"
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "${MODE}" ]]; then
  log_error "Exactly one of --sd PATH or --ftp HOST is required"
  usage >&2
  exit 1
fi

REPO_ROOT="${ANYKA_REPO_ROOT}"
SD_ROOT="${REPO_ROOT}/SD_card_contents"
SRC_HACK="${SD_ROOT}/anyka_hack"
SRC_FACTORY="${SD_ROOT}/Factory"

require_payload() {
  local missing=0
  if [[ ! -d "${SRC_HACK}" ]]; then
    log_error "Missing ${SRC_HACK}"
    missing=1
  fi
  if [[ ! -d "${SRC_FACTORY}" ]]; then
    log_error "Missing ${SRC_FACTORY}"
    missing=1
  fi
  if [[ ! -f "${SRC_HACK}/onvif/onvif-rust.bin" ]]; then
    log_error "Missing ${SRC_HACK}/onvif/onvif-rust.bin — run ./scripts/build_payload.sh first"
    missing=1
  fi
  if [[ ! -f "${SRC_HACK}/snmp/snmp-agent.bin" ]]; then
    log_error "Missing ${SRC_HACK}/snmp/snmp-agent.bin — run ./scripts/build_payload.sh first"
    missing=1
  fi
  if [[ ! -f "${SRC_HACK}/vendor-daemon/vendor-daemon.bin" ]]; then
    log_error "Missing ${SRC_HACK}/vendor-daemon/vendor-daemon.bin — run ./scripts/build_payload.sh first"
    missing=1
  fi
  if [[ "${missing}" -ne 0 ]]; then
    exit 1
  fi
}

copy_local_sd() {
  local dest="${SD_MOUNT}"
  if [[ ! -d "${dest}" ]]; then
    log_error "SD mount path does not exist or is not a directory: ${dest}"
    exit 1
  fi
  if [[ ! -w "${dest}" ]]; then
    log_error "SD mount path is not writable: ${dest}"
    exit 1
  fi

  anyka_check_commands rsync

  local rsync_opts=(-a)
  if [[ "${DO_DELETE}" = true ]]; then
    rsync_opts+=(--delete)
  fi
  if [[ "${DRY_RUN}" = true ]]; then
    rsync_opts+=(--dry-run --stats)
  fi

  log_info "Syncing anyka_hack/ → ${dest}/anyka_hack/"
  rsync "${rsync_opts[@]}" "${SRC_HACK}/" "${dest}/anyka_hack/"
  log_info "Syncing Factory/ → ${dest}/Factory/"
  rsync "${rsync_opts[@]}" "${SRC_FACTORY}/" "${dest}/Factory/"

  if [[ "${DRY_RUN}" = false ]]; then
    sync
    log_success "SD card sync complete at ${dest}"
  else
    log_success "Dry-run complete (no files written)"
  fi
}

# Escape backslash and double-quote for embedding in lftp's URL argument.
ftp_escape() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  printf '%s' "${s}"
}

# Escape backslash and space for lftp's -e script parser. The 4.9.x build on
# the camera does not strip shell-style quotes inside -e scripts (double
# quotes stay literal, single quotes fail silently) but does honor backslash
# escapes, so this is the only way to keep a path with a space as one arg.
lftp_escape() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s// /\\ }"
  printf '%s' "${s}"
}

# Print the TOML string value of a key (e.g. the ssid/password lines under
# [wifi] in anyka.toml). Uses a real TOML parser (python3 ≥3.11's tomllib),
# not a line pattern: values containing escaped quotes or backslashes
# round-trip correctly. A parse failure is fatal — an unreadable live file
# must never look like "no creds to preserve" (that would let the mirror
# write CHANGE_ME over the camera's real [wifi]).
toml_get() {
  local file="$1" key="$2"
  python3 -c 'import sys, tomllib; d = tomllib.load(open(sys.argv[1], "rb")); v = d.get("wifi", {}).get(sys.argv[2]); sys.stdout.write(v if isinstance(v, str) else "")' "${file}" "${key}"
}

# Fetch the camera's live anyka.toml and, if it carries real [wifi] creds, build
# a replacement anyka.toml = the repo's file with those ssid/password values
# restored. The repo ships CHANGE_ME placeholders (it is public, so real creds
# are never committed), and the payload mirror would clobber the live creds —
# which reverts the camera to the vendor path on the next reboot because
# anyka-init's own wpa can't join. Sets WIFI_FINAL (the dir to push) when there
# is something to preserve; leaves it empty otherwise. Aborts the push when the
# live file cannot be fetched for any reason other than a confirmed absence:
# overwriting unverified live creds with CHANGE_ME would cut the camera off.
fetch_live_wifi() {
  local remote_hack host_esc
  remote_hack="$(lftp_escape "${REMOTE_ROOT}/anyka_hack")"
  host_esc="$(ftp_escape "${FTP_HOST}")"
  local workdir; workdir="$(mktemp -d)"
  WIFI_WORKDIR="${workdir}"
  WIFI_FINAL="${workdir}/final"
  # Single-arg get lands in lftp's local cwd; run it from the scratch dir and
  # capture the output so a failed fetch can be classified below.
  local fetch_out="${workdir}/.lftp-fetch.out"
  local fetch_rc=0
  set +e
  ( cd "${workdir}" && lftp -u "${FTP_USER},${FTP_PASS}" ftp://${host_esc} -e \
      "set ftp:ssl-allow no; set net:timeout 20; cd ${remote_hack}; get anyka.toml; bye" ) \
      >"${fetch_out}" 2>&1
  fetch_rc=$?
  set -e
  # A nonzero lftp exit (a 550 is ambiguous between "missing" and "denied",
  # so only an explicit "no such file" in the output counts as absence), a
  # missing local file, or a zero-byte "success" (corrupt transfer) all mean
  # the live state is unverified: abort so nothing overwrites it.
  if [[ ${fetch_rc} -ne 0 || ! -f "${workdir}/anyka.toml" ]]; then
    if [[ ! -f "${workdir}/anyka.toml" ]] && grep -qi 'no such file' "${fetch_out}"; then
      # Confirmed absent on the camera: first deploy, nothing to preserve.
      log_info "No live anyka.toml on ${FTP_HOST} (first deploy); nothing to preserve"
      wifi_scratch_cleanup
      WIFI_WORKDIR=""; WIFI_FINAL=""
      return 0
    fi
    log_error "Could not fetch live anyka.toml from ${FTP_HOST} (exit ${fetch_rc}); aborting so its [wifi] creds are not clobbered:"
    cat "${fetch_out}" >&2
    exit 1
  fi
  if [[ ! -s "${workdir}/anyka.toml" ]]; then
    log_error "Fetched anyka.toml is empty (corrupt transfer); aborting so its [wifi] creds are not clobbered:"
    cat "${fetch_out}" >&2
    exit 1
  fi
  local live_ssid live_pass
  if ! live_ssid="$(toml_get "${workdir}/anyka.toml" ssid)"; then
    log_error "Live anyka.toml is not parseable TOML (python3 with tomllib is required); aborting so its [wifi] creds are not clobbered"
    exit 1
  fi
  if ! live_pass="$(toml_get "${workdir}/anyka.toml" password)"; then
    log_error "Live anyka.toml is not parseable TOML (python3 with tomllib is required); aborting so its [wifi] creds are not clobbered"
    exit 1
  fi
  if [[ -z "${live_ssid}" || "${live_ssid}" == "CHANGE_ME" || -z "${live_pass}" || "${live_pass}" == "CHANGE_ME" ]]; then
    log_info "Live anyka.toml has no complete [wifi] ssid/password; nothing to preserve"
    wifi_scratch_cleanup
    WIFI_WORKDIR=""; WIFI_FINAL=""
    return 0
  fi
  mkdir -p "${WIFI_FINAL}"
  cp "${SRC_HACK}/anyka.toml" "${WIFI_FINAL}/anyka.toml"
  # Rewrite the two credential lines without interpolating the values into a
  # regex replacement (a value containing & or the sed delimiter would corrupt
  # it). Bash escapes the values for TOML basic strings; awk receives them via
  # the environment, where no escaping applies.
  local ssid_toml pass_toml
  ssid_toml="${live_ssid//\\/\\\\}"; ssid_toml="${ssid_toml//\"/\\\"}"
  pass_toml="${live_pass//\\/\\\\}"; pass_toml="${pass_toml//\"/\\\"}"
  LIVE_SSID="${ssid_toml}" LIVE_PASS="${pass_toml}" awk '
    /^[[:space:]]*ssid[[:space:]]*=/ { print "  ssid = \"" ENVIRON["LIVE_SSID"] "\""; next }
    /^[[:space:]]*password[[:space:]]*=/ { print "  password = \"" ENVIRON["LIVE_PASS"] "\""; next }
    { print }
  ' "${WIFI_FINAL}/anyka.toml" >"${WIFI_FINAL}/anyka.toml.tmp"
  mv "${WIFI_FINAL}/anyka.toml.tmp" "${WIFI_FINAL}/anyka.toml"
  log_info "Live [wifi] ssid=${live_ssid} will be preserved across this push"
}

# Re-push only the preserved anyka.toml so the camera keeps its real [wifi]
# creds. A plain put -O, not a mirror: a one-file mirror can skip the upload
# when size and mtime match the remote (FTP timestamps have 1 s granularity
# and both copies were written seconds apart), leaving the CHANGE_ME
# placeholder on the camera. A failed re-push is fatal: the mirror already
# replaced the live file, so the camera would revert to the vendor boot path
# at the next reboot.
push_live_wifi() {
  [[ -n "${WIFI_FINAL}" && -f "${WIFI_FINAL}/anyka.toml" ]] || return 0
  local remote_hack final_esc host_esc
  remote_hack="$(lftp_escape "${REMOTE_ROOT}/anyka_hack")"
  final_esc="$(lftp_escape "${WIFI_FINAL}")"
  host_esc="$(ftp_escape "${FTP_HOST}")"
  log_step "Re-pushing anyka.toml with preserved [wifi] credentials"
  local rc=0
  set +e
  lftp -u "${FTP_USER},${FTP_PASS}" ftp://${host_esc} -e \
    "set ftp:ssl-allow no; set net:timeout 20; set cmd:fail-exit on; put -O ${remote_hack} ${final_esc}/anyka.toml; bye" \
    >/dev/null 2>&1
  rc=$?
  set -e
  wifi_scratch_cleanup
  WIFI_WORKDIR=""; WIFI_FINAL=""
  if [[ ${rc} -ne 0 ]]; then
    log_error "Preserved anyka.toml re-push failed (exit ${rc}) — ${FTP_HOST} now has the CHANGE_ME [wifi] placeholder; re-apply the real credentials before the next reboot"
    exit 1
  fi
  log_success "[wifi] credentials preserved on ${FTP_HOST}"
}

# Upload the payload trees to the camera over FTP: mirror anyka_hack/ and
# Factory/ under REMOTE_ROOT, then re-push the preserved live [wifi] creds.
copy_ftp() {
  if ! command -v lftp &>/dev/null; then
    log_error "lftp is required for --ftp tree uploads (install: sudo apt-get install -y lftp)"
    exit 1
  fi

  local remote_hack="${REMOTE_ROOT}/anyka_hack"
  local remote_factory="${REMOTE_ROOT}/Factory"
  local remote_hack_esc remote_factory_esc host_esc
  remote_hack_esc="$(lftp_escape "${remote_hack}")"
  remote_factory_esc="$(lftp_escape "${remote_factory}")"
  host_esc="$(ftp_escape "${FTP_HOST}")"

  local mirror_flags="-R --verbose --no-perms --no-umask"
  if [[ "${DO_DELETE}" = true ]]; then
    mirror_flags+=" --delete"
  fi
  if [[ "${DRY_RUN}" = true ]]; then
    mirror_flags+=" --dry-run"
  fi

  log_info "FTP host: ${FTP_HOST}"
  log_info "Remote roots: ${remote_hack}  ${remote_factory}"
  log_info "User: ${FTP_USER}"
  if [[ "${DRY_RUN}" = true ]]; then
    log_warn "Dry-run: no remote writes"
  fi

  if [[ "${DRY_RUN}" = false ]]; then
    fetch_live_wifi
  fi

  # lftp 4.9.x dropped -c on the camera build and its in-script `open` fails
  # login (530); the -u + URL form works and keeps credentials out of the URL.
  # Paths are backslash-escaped because the -e parser does not strip quotes.
  local -a lftp_lines=(
    "set ftp:ssl-allow no"
    "set net:max-retries 2"
    "set net:timeout 20"
    "mkdir -p ${remote_hack_esc}"
    "mkdir -p ${remote_factory_esc}"
    "mirror ${mirror_flags} $(lftp_escape "${SRC_HACK}") ${remote_hack_esc}"
    "mirror ${mirror_flags} $(lftp_escape "${SRC_FACTORY}") ${remote_factory_esc}"
    "bye"
  )
  local lftp_script
  lftp_script="$(printf '%s\n' "${lftp_lines[@]}")"

  log_step "Uploading via lftp mirror"
  local lftp_output lftp_rc=0
  set +e
  lftp_output=$(lftp -u "${FTP_USER},${FTP_PASS}" ftp://${host_esc} -e "${lftp_script}" 2>&1)
  lftp_rc=$?
  set -e

  if [[ "${lftp_rc}" -ne 0 ]] || echo "${lftp_output}" | grep -qiE '^([[:space:]]*)?(error|fatal)|login failed|access denied|530 '; then
    log_error "lftp upload failed (exit ${lftp_rc}):"
    echo "${lftp_output}" >&2
    # The anyka_hack mirror may already have written the CHANGE_ME anyka.toml
    # before it failed (lftp keeps going after a failed transfer): restore
    # the live credentials now so the camera is not wifi-less at reboot.
    push_live_wifi
    exit 1
  fi

  # Show a short summary; full mirror -v output can be large.
  echo "${lftp_output}" | tail -n 40 || true

  if [[ "${DRY_RUN}" = true ]]; then
    log_success "FTP dry-run complete"
  else
    push_live_wifi
    log_success "FTP upload complete → ${FTP_HOST}:${REMOTE_ROOT}/{anyka_hack,Factory}"
    log_info "Reboot the camera or re-insert the SD card so Factory/config.sh can pick up changes."
  fi
}

log_info "=== Copy SD card contents ==="
require_payload
echo ""

case "${MODE}" in
  sd)
    log_info "Mode: local SD mount (${SD_MOUNT})"
    copy_local_sd
    ;;
  ftp)
    log_info "Mode: FTP to camera"
    copy_ftp
    ;;
  *)
    log_error "Internal error: unknown mode ${MODE}"
    exit 1
    ;;
esac
