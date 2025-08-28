#! /bin/sh

# periodic_reboot.sh
# Reboot the camera every 6 hours (default) to mitigate long‑running leaks / instability.
# Configurable via environment variables:
#   REBOOT_INTERVAL_SEC        Base interval in seconds (default 21600 = 6h)
#   REBOOT_JITTER_MAX_SEC      Optional max jitter (added 0..JITTER) to base (default 0 = disabled)
#   ENABLE_PERIODIC_REBOOT     If set to 0, script exits immediately
#
# Safe for busybox environments (no bashisms). Creates a PID file to avoid duplicates.

LOG_FILE=periodic_reboot.log

# Initialize log directories & common helpers if present
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

# Fallback minimal logger if common.sh missing
if ! command -v log >/dev/null 2>&1; then
  log() { # level message...
    ts=$(date '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date)
    echo "$ts [$1] [periodic_reboot] $*" >> "${LOG_DIR:-/tmp}/$LOG_FILE" 2>/dev/null
  }
fi

if [ "${ENABLE_PERIODIC_REBOOT:-1}" != 1 ]; then
  log INFO "Periodic reboot disabled via ENABLE_PERIODIC_REBOOT=${ENABLE_PERIODIC_REBOOT}"
  exit 0
fi

INTERVAL_SEC=${REBOOT_INTERVAL_SEC:-21600} # 6 hours
JITTER_MAX_SEC=${REBOOT_JITTER_MAX_SEC:-0}
PID_FILE=/mnt/tmp/periodic_reboot.pid
mkdir -p /mnt/tmp 2>/dev/null || true

# Prevent duplicate instances
if [ -f "$PID_FILE" ]; then
  oldpid=$(cat "$PID_FILE" 2>/dev/null)
  if [ -n "$oldpid" ] && kill -0 "$oldpid" 2>/dev/null; then
    log WARN "Already running (pid=$oldpid); exiting"
    exit 0
  fi
fi
echo $$ > "$PID_FILE" 2>/dev/null || log WARN "Could not write PID file $PID_FILE"

compute_jitter() {
  # Outputs jitter (0..JITTER_MAX_SEC) using /dev/urandom if available, else pseudo.
  if [ "$JITTER_MAX_SEC" -le 0 ] 2>/dev/null; then
    echo 0; return 0
  fi
  if [ -r /dev/urandom ]; then
    # Read 2 bytes, convert to integer, mod range
    val=$(dd if=/dev/urandom bs=2 count=1 2>/dev/null | od -An -tu2 2>/dev/null | tr -d ' ')
    [ -z "$val" ] && val=0
  else
    # Fallback: simple time-based pseudo number
    val=$$$(date +%S 2>/dev/null)
  fi
  echo $(( val % (JITTER_MAX_SEC + 1) ))
}

log INFO "Starting periodic_reboot pid=$$ base_interval=${INTERVAL_SEC}s jitter_max=${JITTER_MAX_SEC}s"

cleanup() {
  rm -f "$PID_FILE" 2>/dev/null
  log INFO "Exiting periodic_reboot"
  exit 0
}
trap cleanup INT TERM EXIT

while true; do
  jitter=$(compute_jitter)
  sleep_sec=$INTERVAL_SEC
  if [ "$jitter" -gt 0 ] 2>/dev/null; then
    sleep_sec=$((INTERVAL_SEC + jitter))
  fi
  log INFO "Next reboot in ${sleep_sec}s (base=${INTERVAL_SEC}s jitter=${jitter}s)"

  # Sleep in smaller chunks to allow signal handling (avoid very long uninterruptible sleep)
  remaining=$sleep_sec
  while [ "$remaining" -gt 0 ]; do
    chunk=300
    [ "$remaining" -lt $chunk ] && chunk=$remaining
    # Log progress: INFO every hour boundary and for last 5 minutes, DEBUG otherwise
    if [ "$remaining" -le 300 ] 2>/dev/null || [ $((remaining % 3600)) -eq 0 ] 2>/dev/null; then
      log INFO "Reboot countdown: remaining=${remaining}s next_sleep=${chunk}s"
    else
      log DEBUG "Reboot countdown: remaining=${remaining}s next_sleep=${chunk}s"
    fi
    sleep $chunk || true
    remaining=$((remaining - chunk))
  done

  uptime_s=$(awk '{print int($1)}' /proc/uptime 2>/dev/null || echo '?')
  log INFO "Rebooting now (uptime=${uptime_s}s)"
  sync 2>/dev/null || true
  reboot
  # If reboot command fails (rare), wait a bit then retry
  log WARN "Reboot command returned; retrying in 60s" 
  sleep 60
done
