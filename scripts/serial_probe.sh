#!/usr/bin/env bash
# Probe a serial console: for each baud, start a reader, poke with CR, print any reply.
# Usage: sudo scripts/serial_probe.sh [device] [bauds...]
DEV="${1:-/dev/ttyUSB0}"; shift 2>/dev/null
BAUDS="${*:-115200 57600 38400 19200 9600 4800 2400 1200}"
for b in $BAUDS; do
  stty -F "$DEV" "$b" raw -echo -crtscts min 0 time 0 2>/dev/null || { echo "$b: stty failed"; continue; }
  out=$( { timeout 3 cat "$DEV" & sleep 0.3; printf '\r\n' > "$DEV"; sleep 0.5; printf '\r\n' > "$DEV"; wait; } | head -c 512 )
  printf '%-7s %s\n' "$b" "$(printf '%s' "$out" | cat -v | tr -d '\r' | tr '\n' ' ' | head -c 200)"
done
