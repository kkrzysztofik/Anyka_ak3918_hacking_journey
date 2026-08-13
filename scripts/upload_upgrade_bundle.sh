#!/usr/bin/env bash
# Upload an upgrade bundle.tar to a camera via PUT /api/update.
#
# Expects HTTP 202 (queued). The applier stages the inactive slot, flips
# `active`, and reboots on its next poll — this script does not wait.
#
# The password is NEVER a command-line argument (it would land in shell
# history and `ps`). Pass it via CAMERA_PASS env or --pass-file FILE:
#   ./scripts/upload_upgrade_bundle.sh --host 192.168.2.198 --user admin \
#       --pass-file <(echo "$CAMERA_PASS") bundle.tar
#   CAMERA_PASS=SECRET ./scripts/upload_upgrade_bundle.sh --host 192.168.2.198 \
#       --user admin bundle.tar
#   ./scripts/upload_upgrade_bundle.sh --host 192.168.30.10 --jumphost root@192.168.3.137 \
#       --user admin --pass-file /path/to/netrc-credential bundle.tar
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
PASS_FILE=""

usage() {
  cat <<'EOF'
Usage: upload_upgrade_bundle.sh [OPTIONS] BUNDLE.tar

PUT the upgrade bundle to http://<host>/api/update (Administrator Basic auth).
Success is HTTP 202 only — the camera queues the apply and will reboot later.

Options:
  --host HOST         Camera IP/hostname (or CAMERA_HOST)
  --user USER         ONVIF/WebUI admin user (or CAMERA_USER)
  --pass-file FILE    File containing the password (no leading/trailing spaces
                      trimmed). Use process substitution to avoid a temp file.
  --jumphost SPEC     ssh target, e.g. root@192.168.3.137 (or CAMERA_JUMPHOST)
  --timeout SEC       curl max-time seconds (default 600, or CAMERA_UPLOAD_TIMEOUT)
  -h, --help          Show this help

Password is taken from CAMERA_PASS or --pass-file only — never from argv, so
it stays out of shell history and `ps`.

Examples:
  CAMERA_PASS=SECRET ./scripts/upload_upgrade_bundle.sh --host 192.168.2.198 \
      --user admin bundle.tar
  ./scripts/upload_upgrade_bundle.sh --host 192.168.2.198 --user admin \
      --pass-file <(echo -n "$CAMERA_PASS") bundle.tar
  ./scripts/upload_upgrade_bundle.sh --host 192.168.30.10 --jumphost root@192.168.3.137 \
      --user admin --pass-file <(echo -n "$CAMERA_PASS") bundle.tar

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
    --pass-file)
      PASS_FILE="${2:-}"
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

if [[ -z "${HOST}" || -z "${USER_NAME}" || -z "${BUNDLE}" ]]; then
  log_error "--host, --user, and BUNDLE are required (or CAMERA_* env + BUNDLE)"
  usage >&2
  exit 1
fi

# Password from argv is forbidden; require CAMERA_PASS or --pass-file.
if [[ -n "${PASS_FILE}" ]]; then
  PASS="$(cat "${PASS_FILE}")"
fi
if [[ -z "${PASS}" ]]; then
  log_error "Password required: set CAMERA_PASS or pass --pass-file FILE"
  usage >&2
  exit 1
fi

# TIMEOUT_SEC is interpolated into a remote shell command below; validate it as
# a positive integer so a hostile value cannot inject extra commands.
if ! [[ "${TIMEOUT_SEC}" =~ ^[0-9]+$ ]] || (( TIMEOUT_SEC < 1 )); then
  log_error "--timeout must be a positive integer (got '${TIMEOUT_SEC}')"
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
NETRC="$(mktemp)"
chmod 600 "${NETRC}"
# netrc: curl reads credentials from here, so the password never appears in
# argv (local or, after the jumphost hop, on the jumphost).
printf 'machine %s login %s password %s\n' "${HOST}" "${USER_NAME}" "${PASS}" > "${NETRC}"
REMOTE_NETRC=""

cleanup() {
  rm -f "${RESP_BODY}" "${NETRC}"
  # Set as soon as the remote file exists, so a failure anywhere after that
  # still wipes the credential off the jumphost.
  if [[ -n "${REMOTE_NETRC}" ]]; then
    ssh -o BatchMode=yes "${JUMPHOST}" "rm -f $(printf '%q' "${REMOTE_NETRC}")" || true
  fi
}
trap cleanup EXIT

log_info "Uploading $(du -h "${BUNDLE}" | cut -f1) → ${URL}"
if [[ -n "${JUMPHOST}" ]]; then
  log_info "via jumphost ${JUMPHOST}"
fi

upload_lan() {
  curl -sS -o "${RESP_BODY}" -w '%{http_code}' \
    --max-time "${TIMEOUT_SEC}" \
    --netrc-file "${NETRC}" \
    -T "${BUNDLE}" \
    "${URL}"
}

# Stage the credential file on the jumphost so curl -u never appears in its
# `ps`. mktemp under umask 077 means the name is unpredictable and the file is
# never group/world-readable, not even momentarily; the contents arrive over
# stdin, so the password stays out of the remote argv too.
#
# Must run OUTSIDE the "$(upload_jumphost)" command substitution: that is a
# subshell, and REMOTE_NETRC set there would never reach the EXIT trap.
stage_remote_netrc() {
  REMOTE_NETRC="$(ssh -o BatchMode=yes "${JUMPHOST}" \
    'umask 077; f="$(mktemp "${TMPDIR:-/tmp}/upgrade_netrc.XXXXXX")" \
       && cat >"$f" && printf %s "$f"' \
    <"${NETRC}")" || true
  if [[ -z "${REMOTE_NETRC}" ]]; then
    log_error "Failed to stage credentials on ${JUMPHOST}"
    exit 1
  fi
  # From here the EXIT trap owns removal of REMOTE_NETRC.
}

upload_jumphost() {
  local remote_url remote_netrc
  remote_url="$(printf '%q' "${URL}")"
  remote_netrc="$(printf '%q' "${REMOTE_NETRC}")"
  # HTTP body → local RESP_BODY (remote stderr); status code alone on stdout.
  ssh -o BatchMode=yes "${JUMPHOST}" \
    "curl -sS -o /dev/stderr -w '%{http_code}' --max-time ${TIMEOUT_SEC} \
       --netrc-file ${remote_netrc} -T - ${remote_url}" \
    <"${BUNDLE}" 2>"${RESP_BODY}"
}

http_code=""
if [[ -n "${JUMPHOST}" ]]; then
  stage_remote_netrc
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
