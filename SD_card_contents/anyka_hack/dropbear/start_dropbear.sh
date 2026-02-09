#!/bin/sh
# Start Dropbear SSH service for Anyka SD overlay.

LOG_FILE="dropbear.log"
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"

INIT_LOGS_PATH=""
for candidate in "/mnt/anyka_hack/init_logs.sh" "${SCRIPT_DIR}/../init_logs.sh"; do
  if [ -f "$candidate" ]; then
    INIT_LOGS_PATH="$candidate"
    break
  fi
done
[ -n "$INIT_LOGS_PATH" ] && . "$INIT_LOGS_PATH"

COMMON_SH_PATH=""
for candidate in "/mnt/anyka_hack/common.sh" "${SCRIPT_DIR}/../common.sh"; do
  if [ -f "$candidate" ]; then
    COMMON_SH_PATH="$candidate"
    break
  fi
done

if [ -z "$COMMON_SH_PATH" ]; then
  echo "[ERROR] Required helper script not found: common.sh" >&2
  exit 1
fi

. "$COMMON_SH_PATH"

if ! command -v log >/dev/null 2>&1 || ! command -v is_process_running >/dev/null 2>&1; then
  echo "[ERROR] common.sh loaded but required helpers are missing (log, is_process_running)" >&2
  exit 1
fi

ANYKA_HACK_DIR="$(dirname "$COMMON_SH_PATH")"
DROPBEAR_DIR="${ANYKA_HACK_DIR}/dropbear"
LIB_DIR="${ANYKA_HACK_DIR}/lib"
DROPBEAR_MULTI="${DROPBEAR_DIR}/dropbearmulti"
DROPBEAR_BIN="${DROPBEAR_DIR}/dropbear"
DROPBEARKEY_BIN="${DROPBEAR_DIR}/dropbearkey"

PID_FILE="/mnt/tmp/dropbear.pid"
RUN_LOG="/mnt/logs/dropbear.log"

SSH_PORT="${1:-22}"
SSH_AUTH_MODE="${2:-both}"
SSH_HOST_KEY_PATH="${3:-/mnt/anyka_hack/dropbear/dropbear_ecdsa_host_key}"
SSH_AUTHORIZED_KEYS_PATH="${4:-/data/.ssh/authorized_keys}"

mkdir -p /mnt/tmp /mnt/logs 2>/dev/null || true

if [ -d "$LIB_DIR" ]; then
  if [ -n "${LD_LIBRARY_PATH:-}" ]; then
    export LD_LIBRARY_PATH="${LIB_DIR}:${LD_LIBRARY_PATH}"
  else
    export LD_LIBRARY_PATH="${LIB_DIR}"
  fi
fi

if [ ! -x "$DROPBEAR_MULTI" ]; then
  log ERROR "Dropbear binary missing or not executable: $DROPBEAR_MULTI"
  exit 1
fi

if ! echo "$SSH_PORT" | grep -Eq '^[0-9]+$'; then
  log ERROR "Invalid ssh_port: $SSH_PORT"
  exit 1
fi

if [ "$SSH_PORT" -lt 1 ] || [ "$SSH_PORT" -gt 65535 ]; then
  log ERROR "ssh_port out of range: $SSH_PORT"
  exit 1
fi

# Avoid symlinks so this works on SD cards formatted with FAT filesystems.
if [ ! -x "$DROPBEAR_BIN" ]; then
  cp "$DROPBEAR_MULTI" "$DROPBEAR_BIN" 2>/dev/null || true
  chmod 755 "$DROPBEAR_BIN" 2>/dev/null || true
fi
if [ ! -x "$DROPBEARKEY_BIN" ]; then
  cp "$DROPBEAR_MULTI" "$DROPBEARKEY_BIN" 2>/dev/null || true
  chmod 755 "$DROPBEARKEY_BIN" 2>/dev/null || true
fi

if is_process_running "$PID_FILE" "dropbear"; then
  log INFO "Dropbear already running"
  exit 0
fi

auth_dir="$(dirname "$SSH_AUTHORIZED_KEYS_PATH")"
mkdir -p "$auth_dir" /root/.ssh 2>/dev/null || true
chmod 700 /root/.ssh 2>/dev/null || true

if [ ! -f "$SSH_AUTHORIZED_KEYS_PATH" ]; then
  : > "$SSH_AUTHORIZED_KEYS_PATH" 2>/dev/null || true
fi
chmod 600 "$SSH_AUTHORIZED_KEYS_PATH" 2>/dev/null || true

if ! ln -sf "$SSH_AUTHORIZED_KEYS_PATH" /root/.ssh/authorized_keys 2>/dev/null; then
  cp "$SSH_AUTHORIZED_KEYS_PATH" /root/.ssh/authorized_keys 2>/dev/null || true
fi

if [ ! -f "$SSH_HOST_KEY_PATH" ]; then
  log WARN "Host key not found at $SSH_HOST_KEY_PATH; generating ECDSA host key"
  mkdir -p "$(dirname "$SSH_HOST_KEY_PATH")" 2>/dev/null || true
  if ! "$DROPBEARKEY_BIN" -t ecdsa -f "$SSH_HOST_KEY_PATH" >> "$RUN_LOG" 2>&1; then
    log ERROR "Failed to generate host key at $SSH_HOST_KEY_PATH"
    exit 1
  fi
  chmod 600 "$SSH_HOST_KEY_PATH" 2>/dev/null || true
fi

dropbear_args="-p $SSH_PORT -P $PID_FILE -r $SSH_HOST_KEY_PATH -K 30 -I 300 -j -k -E"

case "$SSH_AUTH_MODE" in
  both)
    log INFO "Starting Dropbear with key+password auth"
    ;;
  key)
    dropbear_args="$dropbear_args -s"
    log INFO "Starting Dropbear with key-only auth"
    ;;
  password)
    : > "$SSH_AUTHORIZED_KEYS_PATH" 2>/dev/null || true
    chmod 600 "$SSH_AUTHORIZED_KEYS_PATH" 2>/dev/null || true
    log INFO "Starting Dropbear with password-only auth (root key file emptied)"
    ;;
  *)
    log WARN "Invalid ssh_auth_mode '$SSH_AUTH_MODE', falling back to both"
    ;;
esac

"$DROPBEAR_BIN" $dropbear_args >> "$RUN_LOG" 2>&1
sleep 1

if is_process_running "$PID_FILE" "dropbear"; then
  log INFO "Dropbear started on port $SSH_PORT"
  exit 0
fi

log ERROR "Dropbear failed to start"
exit 1
