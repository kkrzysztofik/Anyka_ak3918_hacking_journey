#!/bin/sh

LOG_FILE=onvif_rust.log

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

start_onvif_rust() {
  log INFO 'Starting ONVIF Rust server (onvif-rust)'

  # Enable core dumps for this process
  ulimit -c unlimited 2>/dev/null || log WARN "Failed to set ulimit -c unlimited"

  # Configure kernel core pattern to write to /mnt/coredumps
  mkdir -p /mnt/coredumps 2>/dev/null || log WARN "Failed to create /mnt/coredumps"
  echo '/mnt/coredumps/core.%e.%p.%t' > /proc/sys/kernel/core_pattern 2>/dev/null \
    || log WARN "Failed to set core_pattern (non-root or read-only procfs)"

  # Verify sufficient disk space for coredumps (need at least 5MB free)
  FREE_KB=$(df /mnt/coredumps 2>/dev/null | awk 'NR==2{print $4}')
  [ -z "$FREE_KB" ] && FREE_KB=$(df /mnt 2>/dev/null | awk 'NR==2{print $4}')
  if [ -n "$FREE_KB" ] && [ "$FREE_KB" -lt 5120 ]; then
    log WARN "Low disk space for coredumps: ${FREE_KB}KB free — coredumps may be truncated"
  fi

  # Binary has RPATH embedded — no LD_LIBRARY_PATH needed.
  # Do NOT export LD_LIBRARY_PATH here; it poisons child processes
  # (busybox/date linked against stock uClibc 0.9.33.2).

  # Guard against accidental debug build deployment (debug builds are ~179MB)
  BIN_SIZE=$(wc -c < /mnt/anyka_hack/onvif/onvif-rust.bin 2>/dev/null || echo 0)
  if [ "$BIN_SIZE" -gt 20000000 ]; then
    log ERROR "onvif-rust.bin is ${BIN_SIZE} bytes — debug build? Expected <15MB release build"
    exit 1
  fi

  # Check if configuration file exists
  if [ ! -f /mnt/anyka_hack/onvif/config.toml ]; then
    log WARN "Configuration file not found: /mnt/anyka_hack/onvif/config.toml"
  fi

  # Start ONVIF Rust server with config path
  /mnt/anyka_hack/onvif/onvif-rust /mnt/anyka_hack/onvif/config.toml
  rc=$?

  if [ $rc -ne 0 ]; then
    log ERROR "ONVIF Rust server (onvif-rust) exited with code $rc"
    return $rc
  else
    log INFO 'ONVIF Rust server (onvif-rust) exited normally'
    return 0
  fi
}

# Check if ONVIF Rust server is already running
if pidof onvif-rust >/dev/null 2>&1; then
  log INFO 'ONVIF Rust server already running'
  exit 0
fi

# Verify vendor-daemon is running (required for IPC mode)
# NOTE: vendor-daemon must be started separately via run_vendor_daemon.sh
# because it requires its own library set and LD_LIBRARY_PATH configuration.
if ! pidof vendor-daemon.bin >/dev/null 2>&1; then
  log ERROR "vendor-daemon is not running; start it first with run_vendor_daemon.sh"
  log ERROR "vendor-daemon is required for IPC mode — aborting"
  exit 1
fi
log INFO "vendor-daemon is running"

# Import settings
[ -f /data/gergesettings.txt ] && . /data/gergesettings.txt || log WARN "Missing settings file"

# Validate shared library directory
SHARED_LIB_DIR="/mnt/anyka_hack/lib"
if [ ! -d "${SHARED_LIB_DIR}" ]; then
  log ERROR "Shared library directory not found at: ${SHARED_LIB_DIR}"
  exit 1
fi

# Start ONVIF Rust server
start_onvif_rust >> /mnt/logs/onvif_rust.log 2>&1
