#!/usr/bin/env bash
# Upload an upgrade bundle.tar to a camera via PUT /api/update.
#
# Expects HTTP 202 (queued). The applier stages the inactive slot, flips
# `active`, and reboots on its next poll — this script does not wait.
#
# Usage:
#   ./scripts/upload_upgrade_bundle.sh --host 192.168.2.198 --user admin --pass SECRET bundle.tar
#   ./scripts/upload_upgrade_bundle.sh --host 192.168.30.10 --jumphost root@192.168.3.137 \
#       --user admin --pass SECRET bundle.tar
#
# Env fallbacks: CAMERA_HOST, CAMERA_USER, CAMERA_PASS, CAMERA_JUMPHOST

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"

HOST="${CAMERA_HOST:-}"
USER_NAME="${CAMERA_USER:-}"
PASS="${CAMERA_PASS:-}"
JUMPHOST="${CAMERA_JUMPHOST:-}"
BUNDLE=""
# Bundles are ~19 MB; device ceiling is 64 MB. Allow slow links / jumphost hops.
TIMEOUT_SEC="${CAMERA_UPLOAD_TIMEOUT:-600}"

usage() {
  cat <<'EOF'
Usage: upload_upgrade_bundle.sh [OPTIONS] BUNDLE.tar

PUT the upgrade bundle to http://<host>/api/update (Administrator Basic auth).
Success is HTTP 202 only — the camera queues the apply and will reboot later.

Options:
  --host HOST         Camera IP/hostname (or CAMERA_HOST)
  --user USER         ONVIF/WebUI admin user (or CAMERA_USER)
  --pass PASS         Password (or CAMERA_PASS)
  --jumphost SPEC     ssh target, e.g. root@192.168.3.137 (or CAMERA_JUMPHOST)
  --timeout SEC       curl max-time seconds (default 600, or CAMERA_UPLOAD_TIMEOUT)
  -h, --help          Show this help

Examples:
  ./scripts/upload_upgrade_bundle.sh --host 192.168.2.198 --user admin --pass SECRET bundle.tar
  CAMERA_PASS=SECRET ./scripts/upload_upgrade_bundle.sh --host 192.168.2.198 --user admin bundle.tar
  ./scripts/upload_upgrade_bundle.sh --host 192.168.30.10 --jumphost root@192.168.3.137 \
      --user admin --pass SECRET bundle.tar

After 202: wait for reboot (~90s), then verify ports 80/554/8080 and
GET /api/diagnostics firmware_version. See skill anyka-firmware-upgrade.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host)
      HOST="${2:-}"
      shift 2
      ;;
    --user)
      USER_NAME="${2:-}"
      shift 2
      ;;
    --pass)
      PASS="${2:-}"
      shift 2
      ;;
    --jumphost)
      JUMPHOST="${2:-}"
      shift 2
      ;;
    --timeout)
      TIMEOUT_SEC="${2:-}"
      shift 2
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
      if [[ -n "${BUNDLE}" ]]; then
        log_error "Unexpected argument: $1"
        usage >&2
        exit 1
      fi
      BUNDLE="$1"
      shift
      ;;
  esac
done

if [[ -z "${HOST}" || -z "${USER_NAME}" || -z "${PASS}" || -z "${BUNDLE}" ]]; then
  log_error "--host, --user, --pass, and BUNDLE are required (or CAMERA_* env + BUNDLE)"
  usage >&2
  exit 1
fi

if [[ ! -f "${BUNDLE}" ]]; then
  log_error "Bundle not found: ${BUNDLE}"
  exit 1
fi

anyka_check_commands curl
if [[ -n "${JUMPHOST}" ]]; then
  anyka_check_commands ssh
fi

URL="http://${HOST}/api/update"
RESP_BODY="$(mktemp)"
trap 'rm -f "${RESP_BODY}"' EXIT

log_info "Uploading $(du -h "${BUNDLE}" | cut -f1) → ${URL}"
if [[ -n "${JUMPHOST}" ]]; then
  log_info "via jumphost ${JUMPHOST}"
fi

upload_lan() {
  curl -sS -o "${RESP_BODY}" -w '%{http_code}' \
    --max-time "${TIMEOUT_SEC}" \
    -u "${USER_NAME}:${PASS}" \
    -T "${BUNDLE}" \
    "${URL}"
}

upload_jumphost() {
  # printf %q for remote shell; curl -u still visible in jumphost `ps` during transfer.
  local remote_user remote_pass remote_url
  remote_user="$(printf '%q' "${USER_NAME}")"
  remote_pass="$(printf '%q' "${PASS}")"
  remote_url="$(printf '%q' "${URL}")"
  # HTTP body → local RESP_BODY (remote stderr); status code alone on stdout.
  ssh -o BatchMode=yes "${JUMPHOST}" \
    "curl -sS -o /dev/stderr -w '%{http_code}' --max-time ${TIMEOUT_SEC} \
      -u ${remote_user}:${remote_pass} -T - ${remote_url}" \
    <"${BUNDLE}" 2>"${RESP_BODY}"
}

http_code=""
if [[ -n "${JUMPHOST}" ]]; then
  http_code="$(upload_jumphost)" || true
else
  http_code="$(upload_lan)" || true
fi

if [[ -z "${http_code}" ]]; then
  log_error "Upload failed (no HTTP status)"
  [[ -s "${RESP_BODY}" ]] && cat "${RESP_BODY}" >&2 || true
  exit 1
fi

if [[ "${http_code}" != "202" ]]; then
  log_error "Expected HTTP 202, got ${http_code}"
  if [[ -s "${RESP_BODY}" ]]; then
    log_error "Body: $(head -c 500 "${RESP_BODY}" | tr '\n' ' ')"
  fi
  case "${http_code}" in
    401) log_info "Check Admin credentials (--user/--pass)" ;;
    409) log_info "Another upload in progress or bundle already queued in spool/" ;;
    413) log_info "Bundle exceeds device MAX_BUNDLE_BYTES (64 MiB)" ;;
  esac
  exit 1
fi

log_success "Queued (HTTP 202). Camera will apply on next applier poll and reboot."
log_info "Verify after reconnect: GET /api/diagnostics → firmware_version; ports 80/554/8080"
