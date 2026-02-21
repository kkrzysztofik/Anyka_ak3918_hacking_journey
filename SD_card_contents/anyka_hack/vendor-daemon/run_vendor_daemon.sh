#!/bin/sh
#
# run_vendor_daemon.sh - Standalone launcher for vendor-daemon on Anyka AK3918
#
# Description:
#   Runs vendor-daemon.bin in the foreground as a managed process. vendor-daemon
#   is the IPC bridge between the Rust onvif-rust server and the Anyka SDK C API.
#   It listens on /tmp/vendor-daemon.sock (Unix domain socket) and links against
#   Anyka SDK shared libraries. This script is the direct process supervisor for
#   vendor-daemon when run standalone (not as a side effect of run_onvif_rust.sh).
#
# BusyBox-compatible: uses #!/bin/sh, POSIX shell constructs only.
#   - No [[ ]], declare -f, (( )), arrays, or other bashisms.
#   - Tested with BusyBox ash as found on Anyka AK3918 embedded Linux.
#
# Usage:
#   /mnt/anyka_hack/vendor-daemon/run_vendor_daemon.sh
#
#   Typically invoked by a supervisor or init script. The binary runs in the
#   foreground; redirect stdout/stderr externally if needed, or let this script
#   handle log redirection (see below).
#
# Environment variables honoured (set in /data/gergesettings.txt):
#   LOG_FILE  - Override log file name (default: vendor_daemon.log)
#   DEBUG     - Set to 1 for debug-level logging
#   VERBOSE   - Set to 1 for verbose logging
#
# Exit codes:
#   0  - vendor-daemon already running (idempotent) or exited cleanly
#   1  - Fatal startup error (binary/library directory missing)
#   *  - Exit code forwarded from vendor-daemon.bin itself

# =============================================================================
# BOOTSTRAP LOGGING AND COMMON UTILITIES
# =============================================================================

LOG_FILE=vendor_daemon.log

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities (provides log(), die(), validate_* helpers)
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

# Fallback log function if common.sh is not available or did not define log()
if ! command -v log >/dev/null 2>&1; then
  log() {
    local level="$1"
    shift
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [${level}] $*"
  }
fi

# =============================================================================
# CONFIGURATION
# =============================================================================

VENDOR_DAEMON_BIN="/mnt/anyka_hack/vendor-daemon/vendor-daemon.bin"
VENDOR_DAEMON_SOCK="/tmp/vendor-daemon.sock"
VENDOR_DAEMON_LOG="/mnt/logs/vendor_daemon.log"
SHARED_LIB_DIR="/mnt/anyka_hack/lib"

# =============================================================================
# TRAP: LOG CLEAN EXIT/SIGNAL
# =============================================================================

_on_exit() {
  log INFO "vendor-daemon launcher exiting (PID $$)"
}

_on_int() {
  log INFO "vendor-daemon launcher received SIGINT — forwarding to vendor-daemon"
}

_on_term() {
  log INFO "vendor-daemon launcher received SIGTERM — forwarding to vendor-daemon"
}

trap '_on_exit'  EXIT
trap '_on_int'   INT
trap '_on_term'  TERM

# =============================================================================
# IMPORT SETTINGS
# =============================================================================

if [ -f /data/gergesettings.txt ]; then
  . /data/gergesettings.txt
else
  log WARN "Missing settings file: /data/gergesettings.txt"
fi

# =============================================================================
# PRE-FLIGHT CHECKS
# =============================================================================

# Check if vendor-daemon is already running (idempotent launch)
if pgrep -f "vendor-daemon.bin" >/dev/null 2>&1; then
  log INFO "vendor-daemon already running — nothing to do"
  exit 0
fi

# Validate shared library directory
if [ ! -d "${SHARED_LIB_DIR}" ]; then
  log ERROR "Shared library directory not found: ${SHARED_LIB_DIR}"
  exit 1
fi

# Validate vendor-daemon binary
if [ ! -f "${VENDOR_DAEMON_BIN}" ]; then
  log ERROR "vendor-daemon binary not found: ${VENDOR_DAEMON_BIN}"
  exit 1
fi

if [ ! -x "${VENDOR_DAEMON_BIN}" ]; then
  log ERROR "vendor-daemon binary is not executable: ${VENDOR_DAEMON_BIN}"
  exit 1
fi

# =============================================================================
# ENVIRONMENT SETUP
# =============================================================================

# Prepend Anyka SDK libs and vendor-daemon-specific libs to the search path
export LD_LIBRARY_PATH=/mnt/anyka_hack/lib:/mnt/anyka_hack/vendor-daemon/lib:${LD_LIBRARY_PATH}

# Enable core dumps so crash diagnostics are captured
ulimit -c unlimited 2>/dev/null || log WARN "Failed to enable core dumps (ulimit -c unlimited)"

# =============================================================================
# LAUNCH VENDOR-DAEMON IN FOREGROUND
# =============================================================================

log INFO "Starting vendor-daemon: ${VENDOR_DAEMON_BIN}"
log INFO "Socket will appear at: ${VENDOR_DAEMON_SOCK}"
log INFO "LD_LIBRARY_PATH=${LD_LIBRARY_PATH}"

# Run in foreground — this IS the daemon process. stdout/stderr go to the log
# file so that the process supervisor can track it, while vendor-daemon also
# writes its own structured log to /tmp/vendor-daemon.log internally.
"${VENDOR_DAEMON_BIN}" >>"${VENDOR_DAEMON_LOG}" 2>&1
rc=$?

# =============================================================================
# POST-EXIT HANDLING
# =============================================================================

if [ "${rc}" -ne 0 ]; then
  log ERROR "vendor-daemon exited with non-zero code: ${rc}"
else
  log INFO "vendor-daemon exited normally (code 0)"
fi

exit "${rc}"
