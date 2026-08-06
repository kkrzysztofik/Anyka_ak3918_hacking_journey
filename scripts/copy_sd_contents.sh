#!/usr/bin/env bash
# Copy assembled SD_card_contents/ to a mounted SD card or to the camera via FTP.
#
# Does not build — run ./scripts/build_sd_contents.sh first.
#
# Modes (exactly one required):
#   --sd PATH     Sync anyka_hack/ + Factory/ onto a mounted SD card
#   --ftp HOST    Upload trees to /mnt/anyka_hack and /mnt/Factory on the camera
#
# Usage:
#   ./scripts/copy_sd_contents.sh --sd /media/$USER/SDCARD
#   ./scripts/copy_sd_contents.sh --ftp 192.168.1.100
#   ./scripts/copy_sd_contents.sh --ftp 192.168.1.100 --user root --pass ''
#   ./scripts/copy_sd_contents.sh --ftp 192.168.1.100 --dry-run

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
Usage: copy_sd_contents.sh (--sd PATH | --ftp HOST) [OPTIONS]

Copy SD_card_contents/anyka_hack and SD_card_contents/Factory to a mounted
SD card or to the camera filesystem over FTP (/mnt/...).

Modes (exactly one):
  --sd PATH           Local mount point of the SD card
  --ftp HOST          Camera IP/hostname (uploads under /mnt)

Options:
  --user NAME         FTP username (default: admin)
  --pass PASS         FTP password (default: admin)
  --remote-root PATH  Remote SD mount root (default: /mnt)
  --delete            Remove destination files not present in source
  --dry-run           Show what would be copied without writing
  -h, --help          Show this help

Examples:
  ./scripts/copy_sd_contents.sh --sd /media/kmk/SDCARD
  ./scripts/copy_sd_contents.sh --ftp 192.168.1.100
  ./scripts/copy_sd_contents.sh --ftp 192.168.1.100 --user root --pass mypass --delete
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
    log_error "Missing ${SRC_HACK}/onvif/onvif-rust.bin — run ./scripts/build_sd_contents.sh first"
    missing=1
  fi
  if [[ ! -f "${SRC_HACK}/vendor-daemon/vendor-daemon.bin" ]]; then
    log_error "Missing ${SRC_HACK}/vendor-daemon/vendor-daemon.bin — run ./scripts/build_sd_contents.sh first"
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

  # Prefer open + user over embedding credentials in the URL (safer with special chars).
  local lftp_script
  lftp_script=$(
    cat <<EOF
set ftp:ssl-allow no
set net:max-retries 2
set net:timeout 20
open -u "${user_esc}","${pass_esc}" ${host_esc}
mkdir -p ${remote_hack}
mkdir -p ${remote_factory}
mirror ${mirror_flags} "${SRC_HACK}" ${remote_hack}
mirror ${mirror_flags} "${SRC_FACTORY}" ${remote_factory}
bye
EOF
  )

  log_step "Uploading via lftp mirror"
  local lftp_output lftp_rc=0
  set +e
  lftp_output=$(lftp -c "${lftp_script}" 2>&1)
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
