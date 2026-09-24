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

ftp_escape() {
  # Escape backslash and double-quote for embedding in lftp -c strings.
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  printf '%s' "${s}"
}

# Fetch the camera's live anyka.toml and, if it carries real [wifi] creds, build
# a replacement anyka.toml = the repo's file with those ssid/password values
# restored. The repo ships CHANGE_ME placeholders (it is public, so real creds
# are never committed), and the payload mirror would clobber the live creds —
# which reverts the camera to the vendor path on the next reboot because
# anyka-init's own wpa can't join. Set WIFI_FINAL (the dir to push) when there
# is something to preserve; leave it empty otherwise.
fetch_live_wifi() {
  local remote_hack="${REMOTE_ROOT}/anyka_hack"
  local user_esc pass_esc host_esc
  user_esc="$(ftp_escape "${FTP_USER}")"
  pass_esc="$(ftp_escape "${FTP_PASS}")"
  host_esc="$(ftp_escape "${FTP_HOST}")"
  local workdir; workdir="$(mktemp -d)"
  WIFI_WORKDIR="${workdir}"
  WIFI_FINAL="${workdir}/final"
  # Single-arg get lands in lftp's local cwd; run it from a scratch dir.
  ( cd "${workdir}" && lftp -u "${FTP_USER},${FTP_PASS}" ftp://${host_esc} -e \
      "set ftp:ssl-allow no; set net:timeout 20; cd ${remote_hack}; get anyka.toml; bye" \
      >/dev/null 2>&1 )
  if [[ ! -f "${workdir}/anyka.toml" ]]; then
    log_info "No live anyka.toml on ${FTP_HOST} (first deploy); nothing to preserve"
    rm -rf "${workdir}"; WIFI_WORKDIR=""; WIFI_FINAL=""
    return 0
  fi
  local live_ssid live_pass
  live_ssid="$(sed -nE 's/^[[:space:]]*ssid[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/p' "${workdir}/anyka.toml" | head -1)"
  live_pass="$(sed -nE 's/^[[:space:]]*password[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/p' "${workdir}/anyka.toml" | head -1)"
  if [[ -z "${live_ssid}" || "${live_ssid}" == "CHANGE_ME" ]]; then
    log_info "Live anyka.toml has placeholder/absent [wifi] ssid; nothing to preserve"
    rm -rf "${workdir}"; WIFI_WORKDIR=""; WIFI_FINAL=""
    return 0
  fi
  mkdir -p "${WIFI_FINAL}"
  cp "${SRC_HACK}/anyka.toml" "${WIFI_FINAL}/anyka.toml"
  local ssid_esc pass_esc2
  ssid_esc="${live_ssid//\\/\\\\}"; ssid_esc="${ssid_esc//\"/\\\"}"
  pass_esc2="${live_pass//\\/\\\\}"; pass_esc2="${pass_esc2//\"/\\\"}"
  sed -i -E "s|^([[:space:]]*ssid[[:space:]]*=).*|\1 \"${ssid_esc}\"|" "${WIFI_FINAL}/anyka.toml"
  sed -i -E "s|^([[:space:]]*password[[:space:]]*=).*|\1 \"${pass_esc2}\"|" "${WIFI_FINAL}/anyka.toml"
  log_info "Live [wifi] ssid=${live_ssid} will be preserved across this push"
}

# Re-push only the preserved anyka.toml (a one-file mirror over the just-written
# CHANGE_ME copy) so the camera keeps its real [wifi] creds.
push_live_wifi() {
  [[ -n "${WIFI_FINAL}" && -f "${WIFI_FINAL}/anyka.toml" ]] || return 0
  local remote_hack="${REMOTE_ROOT}/anyka_hack"
  local user_esc pass_esc host_esc
  user_esc="$(ftp_escape "${FTP_USER}")"
  pass_esc="$(ftp_escape "${FTP_PASS}")"
  host_esc="$(ftp_escape "${FTP_HOST}")"
  log_step "Re-pushing anyka.toml with preserved [wifi] credentials"
  local rc=0
  set +e
  lftp -u "${FTP_USER},${FTP_PASS}" ftp://${host_esc} -e "set ftp:ssl-allow no; set net:timeout 20; mirror -R --no-perms --no-umask ${WIFI_FINAL} ${remote_hack}; bye" >/dev/null 2>&1
  rc=$?
  set -e
  [[ -n "${WIFI_WORKDIR}" ]] && rm -rf "${WIFI_WORKDIR}"
  WIFI_WORKDIR=""; WIFI_FINAL=""
  if [[ ${rc} -ne 0 ]]; then
    log_warn "Preserved anyka.toml re-push failed (exit ${rc}); re-apply [wifi] on ${FTP_HOST} if the camera reverts"
  else
    log_success "[wifi] credentials preserved on ${FTP_HOST}"
  fi
}

copy_ftp() {
  if ! command -v lftp &>/dev/null; then
    log_error "lftp is required for --ftp tree uploads (install: sudo apt-get install -y lftp)"
    exit 1
  fi

  local remote_hack="${REMOTE_ROOT}/anyka_hack"
  local remote_factory="${REMOTE_ROOT}/Factory"
  local user_esc pass_esc host_esc
  user_esc="$(ftp_escape "${FTP_USER}")"
  pass_esc="$(ftp_escape "${FTP_PASS}")"
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

  WIFI_WORKDIR=""; WIFI_FINAL=""
  if [[ "${DRY_RUN}" = false ]]; then
    fetch_live_wifi
  fi

  # lftp 4.9.x dropped -c and its in-script `open` fails login (530); the
  # -u + URL form works and keeps credentials out of the URL.
  local lftp_script
  lftp_script=$(
    cat <<EOF
set ftp:ssl-allow no
set net:max-retries 2
set net:timeout 20
mkdir -p ${remote_hack}
mkdir -p ${remote_factory}
mirror ${mirror_flags} ${SRC_HACK} ${remote_hack}
mirror ${mirror_flags} ${SRC_FACTORY} ${remote_factory}
bye
EOF
  )

  log_step "Uploading via lftp mirror"
  local lftp_output lftp_rc=0
  set +e
  lftp_output=$(lftp -u "${FTP_USER},${FTP_PASS}" ftp://${host_esc} -e "${lftp_script}" 2>&1)
  lftp_rc=$?
  set -e

  if [[ "${lftp_rc}" -ne 0 ]] || echo "${lftp_output}" | grep -qiE '^([[:space:]]*)?(error|fatal)|login failed|access denied|530 '; then
    log_error "lftp upload failed (exit ${lftp_rc}):"
    echo "${lftp_output}" >&2
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
